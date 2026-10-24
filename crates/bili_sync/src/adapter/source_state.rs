use std::ops::Deref;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::Result;
use futures::{Stream, StreamExt};

use crate::bilibili::{BiliError, SourceUnavailable, VideoInfo};

/// Keep the persisted identity when upstream metadata disappears. Like submissions,
/// metadata-only lookups leave scheduling alone; a final scan disables the source.
pub struct SourceState<T> {
    model: T,
    unavailable: Arc<AtomicBool>,
}

impl<T> Deref for SourceState<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.model
    }
}

impl<T> SourceState<T> {
    pub fn new(model: T, unavailable: bool) -> Self {
        Self {
            model,
            unavailable: Arc::new(AtomicBool::new(unavailable)),
        }
    }

    pub fn is_unavailable(&self) -> bool {
        self.unavailable.load(Ordering::Relaxed)
    }

    pub fn track_stream<'a>(
        &self,
        stream: impl Stream<Item = Result<VideoInfo>> + Send + 'a,
        favorite: bool,
    ) -> Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send + 'a>> {
        let unavailable = self.unavailable.clone();
        Box::pin(async_stream::stream! {
            futures::pin_mut!(stream);
            while let Some(item) = stream.next().await {
                if item.as_ref().err().is_some_and(|error| is_unavailable_error(error, favorite)) {
                    unavailable.store(true, Ordering::Relaxed);
                }
                let failed = item.is_err();
                yield item;
                if failed { break; }
            }
        })
    }
}

pub(super) fn is_unavailable_error(error: &anyhow::Error, favorite: bool) -> bool {
    if error.is::<SourceUnavailable>() {
        return true;
    }
    // Only Bilibili source API codes, never HTTP 403/404 from a proxy or CDN.
    // Authentication, network failures, rate limits and risk control remain retryable.
    match error.downcast_ref::<BiliError>() {
        Some(BiliError::RequestFailed(-403 | -404, _)) => true,
        Some(BiliError::RequestFailed(11010, _)) => favorite,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{VideoSource, VideoSourceEnum};
    use crate::bilibili::{test_support::mock_api, BiliClient, CollectionItem, CollectionType};
    use bili_sync_entity::{collection, favorite};
    use bili_sync_migration::{Migrator, MigratorTrait};
    use sea_orm::{
        ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend,
        EntityTrait, PaginatorTrait, QueryFilter, Statement,
    };
    use serde_json::{json, Value};
    use std::path::Path;

    async fn database() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        favorite::ActiveModel {
            id: Set(1),
            f_id: Set(101),
            owner_mid: Set(Some(42)),
            name: Set("Last valid name".into()),
            path: Set("/saved".into()),
            created_at: Set("2025-01-01 00:00:00".into()),
            latest_row_at: Set("2026-01-01 00:00:00".into()),
            enabled: Set(true),
            scan_deleted_videos: Set(true),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        collection::ActiveModel {
            id: Set(1),
            s_id: Set(101),
            m_id: Set(42),
            r#type: Set(1),
            name: Set("Last valid name".into()),
            path: Set("/saved".into()),
            created_at: Set("2025-01-01 00:00:00".into()),
            latest_row_at: Set("2026-01-01 00:00:00".into()),
            enabled: Set(true),
            scan_deleted_videos: Set(true),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        db
    }

    fn response(data: Value) -> (u16, String) {
        (200, json!({"code": 0, "message": "OK", "data": data}).to_string())
    }

    fn failure(code: i64) -> (u16, String) {
        (200, json!({"code": code, "message": "unavailable"}).to_string())
    }

    fn metadata(favorite: bool, name: &str) -> (u16, String) {
        if favorite {
            response(json!({"id": 101, "mid": 42, "title": name}))
        } else {
            response(json!({"meta": {"series_id": 101, "mid": 42, "name": name}}))
        }
    }

    fn item() -> CollectionItem {
        CollectionItem {
            sid: "101".into(),
            mid: "42".into(),
            collection_type: CollectionType::Series,
        }
    }

    async fn source<'a>(
        favorite: bool,
        client: &'a BiliClient,
        item: &'a CollectionItem,
        db: &DatabaseConnection,
        base: &str,
    ) -> Result<(
        VideoSourceEnum,
        Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send + 'a>>,
    )> {
        if favorite {
            crate::adapter::favorite::favorite_from_with_api_base(1, client, db, base).await
        } else {
            crate::adapter::collection::collection_from_with_api_base(item, Path::new("/saved"), client, db, base).await
        }
    }

    async fn stored(db: &DatabaseConnection, is_favorite: bool) -> (String, bool, String) {
        if is_favorite {
            let row = favorite::Entity::find_by_id(1).one(db).await.unwrap().unwrap();
            assert!(row.scan_deleted_videos);
            (row.name, row.enabled, row.latest_row_at)
        } else {
            let row = collection::Entity::find_by_id(1).one(db).await.unwrap().unwrap();
            assert!(row.scan_deleted_videos);
            (row.name, row.enabled, row.latest_row_at)
        }
    }

    async fn names(db: &DatabaseConnection, favorite: bool) -> Vec<String> {
        db.query_all(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT name FROM source_name_history WHERE source_type = ? AND source_id = 1 ORDER BY id",
            [if favorite { "favorite" } else { "collection" }.into()],
        ))
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get("", "name").unwrap())
        .collect()
    }

    #[tokio::test]
    async fn unavailable_sources_preserve_name_and_disable_after_final_failed_scan() {
        for is_favorite in [true, false] {
            let mut responses = vec![
                failure(-404),
                failure(-403),
                response(Value::Null),
                metadata(
                    is_favorite,
                    if is_favorite {
                        "已失效收藏夹"
                    } else {
                        "合集已失效"
                    },
                ),
            ];
            if is_favorite {
                responses.push(failure(11010));
            }
            for metadata_response in responses {
                let db = database().await;
                let before = stored(&db, is_favorite).await;
                let (base, server) = mock_api(vec![metadata_response, failure(-404)]).await;
                let client = BiliClient::new(String::new());
                let item = item();
                let (source, stream) = source(is_favorite, &client, &item, &db, &base).await.unwrap();
                assert_eq!(source.source_name_display(), "Last valid name");
                assert!(
                    stored(&db, is_favorite).await.1,
                    "metadata lookup must allow final scan"
                );
                let result = crate::workflow::refresh_video_source(
                    &source,
                    stream,
                    &db,
                    tokio_util::sync::CancellationToken::new(),
                    &client,
                )
                .await;
                assert!(result.is_err());
                let after = stored(&db, is_favorite).await;
                assert_eq!(after, (before.0, false, before.2));
                assert_eq!(names(&db, is_favorite).await, vec!["Last valid name"]);
                assert_eq!(server.await.unwrap().len(), 2);
                let count = if is_favorite {
                    favorite::Entity::find()
                        .filter(favorite::Column::Enabled.eq(true))
                        .count(&db)
                        .await
                } else {
                    collection::Entity::find()
                        .filter(collection::Column::Enabled.eq(true))
                        .count(&db)
                        .await
                }
                .unwrap();
                assert_eq!(count, 0);
            }
        }
    }

    #[tokio::test]
    async fn temporary_or_malformed_metadata_does_not_disable_or_rename_sources() {
        for is_favorite in [true, false] {
            for response in [
                failure(-101),
                failure(-352),
                failure(-412),
                failure(-500),
                (403, "forbidden".into()),
                (404, "not found".into()),
                (200, "invalid JSON".into()),
                (200, json!({"code":0,"message":"OK"}).to_string()),
                metadata(is_favorite, ""),
            ] {
                let db = database().await;
                let before = stored(&db, is_favorite).await;
                let (base, server) = mock_api(vec![response]).await;
                let client = BiliClient::new(String::new());
                assert!(source(is_favorite, &client, &item(), &db, &base).await.is_err());
                assert_eq!(stored(&db, is_favorite).await, before);
                assert_eq!(names(&db, is_favorite).await, vec!["Last valid name"]);
                assert_eq!(server.await.unwrap().len(), 1);
            }
        }
    }

    #[tokio::test]
    async fn source_video_api_removal_disables_but_transient_errors_do_not() {
        for is_favorite in [true, false] {
            for (video_response, expected_enabled) in [
                (failure(-404), false),
                (failure(-403), false),
                (response(Value::Null), false),
                (failure(-101), true),
                (failure(-412), true),
                (failure(-500), true),
            ] {
                let db = database().await;
                let (base, server) = mock_api(vec![metadata(is_favorite, "Last valid name"), video_response]).await;
                let client = BiliClient::new(String::new());
                let item = item();
                let (source, stream) = source(is_favorite, &client, &item, &db, &base).await.unwrap();
                assert!(crate::workflow::refresh_video_source(
                    &source,
                    stream,
                    &db,
                    tokio_util::sync::CancellationToken::new(),
                    &client
                )
                .await
                .is_err());
                let after = stored(&db, is_favorite).await;
                assert_eq!(after.1, expected_enabled);
                assert_eq!(after.0, "Last valid name");
                assert_eq!(server.await.unwrap().len(), 2);
            }
        }
    }

    #[tokio::test]
    async fn source_renames_retain_more_than_five_names_and_manual_disable() {
        for is_favorite in [true, false] {
            let db = database().await;
            if is_favorite {
                favorite::ActiveModel {
                    id: Set(1),
                    enabled: Set(false),
                    ..Default::default()
                }
                .update(&db)
                .await
                .unwrap();
            } else {
                collection::ActiveModel {
                    id: Set(1),
                    enabled: Set(false),
                    ..Default::default()
                }
                .update(&db)
                .await
                .unwrap();
            }
            let mut expected = vec!["Last valid name".to_string()];
            for name in ["One", "Two", "Three", "Four", "Five", "Six", "Six", "One"] {
                let (base, server) = mock_api(vec![metadata(is_favorite, name)]).await;
                let client = BiliClient::new(String::new());
                let item = item();
                let (source, _) = source(is_favorite, &client, &item, &db, &base).await.unwrap();
                assert_eq!(source.source_name_display(), name);
                assert!(!stored(&db, is_favorite).await.1);
                if expected.last().unwrap() != name {
                    expected.push(name.to_string());
                }
                assert_eq!(names(&db, is_favorite).await, expected);
                server.await.unwrap();
            }
        }
    }
    #[tokio::test]
    async fn source_history_migration_backfills_and_tracks_atomic_renames() {
        use sea_orm::TransactionTrait;
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(
            &db,
            Some(
                Migrator::migrations()
                    .iter()
                    .position(|m| m.name() == "m20261024_000002_source_name_history")
                    .unwrap() as u32,
            ),
        )
        .await
        .unwrap();
        for (table, key, name_column) in [
            ("favorite", "f_id", "name"),
            ("collection", "s_id, m_id, type", "name"),
            ("submission", "upper_id", "upper_name"),
        ] {
            let key_values = if table == "collection" { "101,42,1" } else { "101" };
            db.execute_unprepared(&format!(
                "INSERT INTO {table} (id, {key}, {name_column}, path, created_at, latest_row_at, enabled, scan_deleted_videos)
                 VALUES (1, {key_values}, 'Original', '/saved', '2025-01-01 00:00:00', '2026-01-01 00:00:00', 1, 0)"
            )).await.unwrap();
        }
        Migrator::up(&db, None).await.unwrap();
        for (table, name_column) in [
            ("favorite", "name"),
            ("collection", "name"),
            ("submission", "upper_name"),
        ] {
            let read_history = || {
                Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "SELECT name FROM source_name_history WHERE source_type = ? ORDER BY id",
                    [table.into()],
                )
            };
            let history = db.query_all(read_history()).await.unwrap();
            assert_eq!(history.len(), 1);
            assert_eq!(history[0].try_get::<String>("", "name").unwrap(), "Original");
            let txn = db.begin().await.unwrap();
            txn.execute_unprepared(&format!("UPDATE {table} SET {name_column} = 'Rolled back'"))
                .await
                .unwrap();
            txn.rollback().await.unwrap();
            assert_eq!(db.query_all(read_history()).await.unwrap().len(), 1);
            for name in [
                "One",
                "Two",
                "Three",
                "Four",
                "Five",
                "Six",
                "收藏 ' name",
                "收藏 ' name",
            ] {
                db.execute(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    format!("UPDATE {table} SET {name_column} = ? WHERE id = 1"),
                    [name.into()],
                ))
                .await
                .unwrap();
            }
            let history = db.query_all(read_history()).await.unwrap();
            assert_eq!(history.len(), 8);
            assert_eq!(
                history.last().unwrap().try_get::<String>("", "name").unwrap(),
                "收藏 ' name"
            );
        }
        db.execute_unprepared("DELETE FROM favorite WHERE id = 1")
            .await
            .unwrap();
        assert!(names(&db, true).await.is_empty());
        assert_eq!(names(&db, false).await.len(), 8);
        // Rolling the migration back must remove triggers before their target table.
        let migrations = Migrator::migrations();
        let count = migrations.len() - migrations.iter().position(|m| m.name() == "m20261024_000002_source_name_history").unwrap();
        Migrator::down(&db, Some(count as u32)).await.unwrap();
        db.execute_unprepared("UPDATE collection SET name = 'After rollback'")
            .await
            .unwrap();
        Migrator::up(&db, None).await.unwrap();
        assert_eq!(names(&db, false).await, vec!["After rollback"]);
    }

    #[tokio::test]
    async fn removed_source_final_scan_can_still_save_remaining_videos() {
        for is_favorite in [true, false] {
            let db = database().await;
            let (base, server) = mock_api(vec![failure(-404)]).await;
            let client = BiliClient::new(String::new());
            let item = item();
            let (source, _) = source(is_favorite, &client, &item, &db, &base).await.unwrap();
            let stream = futures::stream::iter([Ok(VideoInfo::Submission {
                title: "Final video".into(),
                bvid: "BV1finalsource".into(),
                intro: String::new(),
                cover: String::new(),
                ctime: chrono::DateTime::from_timestamp(1_780_000_000, 0).unwrap(),
            })]);
            let (count, _) = crate::workflow::refresh_video_source(
                &source,
                Box::pin(stream),
                &db,
                tokio_util::sync::CancellationToken::new(),
                &client,
            )
            .await
            .unwrap();
            assert_eq!(count, 1);
            let after = stored(&db, is_favorite).await;
            assert_eq!(after.0, "Last valid name");
            assert!(!after.1);
            assert_eq!(bili_sync_entity::video::Entity::find().count(&db).await.unwrap(), 1);
            server.await.unwrap();
        }
    }

    #[tokio::test]
    async fn season_collections_share_unavailable_and_transient_error_handling() {
        for (metadata_response, video_response, expected_name, expected_enabled) in [
            (failure(-404), failure(-404), "Last valid name", false),
            (response(json!({"meta": null})), failure(-404), "Last valid name", false),
            (
                response(json!({"meta": {"season_id":101,"mid":42,"name":"合集已失效"}})),
                failure(-404),
                "Last valid name",
                false,
            ),
            (
                response(json!({"meta": {"season_id":101,"mid":42,"name":"Renamed season"}})),
                failure(-403),
                "Renamed season",
                false,
            ),
            (
                response(json!({"meta": {"season_id":101,"mid":42,"name":"Renamed season"}})),
                failure(-412),
                "Renamed season",
                true,
            ),
        ] {
            let db = database().await;
            collection::ActiveModel {
                id: Set(1),
                r#type: Set(2),
                ..Default::default()
            }
            .update(&db)
            .await
            .unwrap();
            let item = CollectionItem {
                collection_type: CollectionType::Season,
                ..item()
            };
            let client = BiliClient::new(String::new());
            let (base, server) = mock_api(vec![metadata_response, video_response]).await;
            let (source, stream) = source(false, &client, &item, &db, &base).await.unwrap();
            assert!(crate::workflow::refresh_video_source(
                &source,
                stream,
                &db,
                tokio_util::sync::CancellationToken::new(),
                &client
            )
            .await
            .is_err());
            let saved = stored(&db, false).await;
            assert_eq!(saved.0, expected_name);
            assert_eq!(saved.1, expected_enabled);
            let requests = server.await.unwrap();
            assert!(requests
                .iter()
                .all(|r| r.contains("/x/polymer/web-space/seasons_archives_list?")));
        }
    }
}
