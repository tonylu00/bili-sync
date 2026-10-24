use std::sync::Arc;

use axum::{extract::Path, Extension, Json};
use bili_sync_entity::{submission, video};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, Statement, TransactionTrait};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::error::InnerApiError;
use super::wrapper::{ApiError, ApiResponse};

#[derive(Deserialize, ToSchema)]
pub struct RenameUploaderRequest {
    pub name: String,
    pub expected_upper_id: String,
}

#[derive(Serialize, ToSchema)]
pub struct UploaderNameResponse {
    pub upper_id: String,
    pub name: String,
    pub names: Vec<String>,
    pub video_count: u64,
}

async fn uploader<C: ConnectionTrait>(db: &C, video_id: i32) -> Result<(i64, String), ApiError> {
    let video = video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .ok_or(InnerApiError::NotFound(video_id))?;
    if video.source_type != Some(1) {
        if video.upper_id > 0 {
            return Ok((video.upper_id, video.upper_name));
        }
        if let Some(id) = video.submission_id {
            if let Some(source) = submission::Entity::find_by_id(id).one(db).await? {
                if source.upper_id > 0 {
                    return Ok((source.upper_id, source.upper_name));
                }
            }
        }
    }
    Err(InnerApiError::BadRequest("尚未保存该视频的UP主UID，暂时无法修改名称".into()).into())
}

async fn response<C: ConnectionTrait>(
    db: &C,
    uid: i64,
    current_name: String,
) -> Result<UploaderNameResponse, ApiError> {
    let override_row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT name FROM uploader_name_override WHERE upper_id = ?",
            [uid.into()],
        ))
        .await?;
    let name = match override_row {
        Some(row) => row.try_get("", "name")?,
        None => current_name,
    };
    let names = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT name FROM source_name_history WHERE source_type = 'uploader' AND source_id = ? ORDER BY id DESC",
            [uid.into()],
        ))
        .await?
        .into_iter()
        .map(|row| row.try_get("", "name"))
        .collect::<Result<Vec<String>, _>>()?;
    let count = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT COUNT(*) AS count FROM video WHERE COALESCE(source_type, 0) <> 1 AND (
           upper_id = ? OR (upper_id = 0 AND submission_id IN (SELECT id FROM submission WHERE upper_id = ?))
         )",
            [uid.into(), uid.into()],
        ))
        .await?
        .unwrap()
        .try_get::<i64>("", "count")?;
    Ok(UploaderNameResponse {
        upper_id: uid.to_string(),
        name,
        names,
        video_count: count as u64,
    })
}

#[utoipa::path(get, path = "/api/videos/{id}/uploader", responses((status = 200, body = ApiResponse<UploaderNameResponse>)))]
pub async fn get_uploader_name(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<UploaderNameResponse>, ApiError> {
    let (uid, name) = uploader(db.as_ref(), id).await?;
    Ok(ApiResponse::ok(response(db.as_ref(), uid, name).await?))
}

#[utoipa::path(put, path = "/api/videos/{id}/uploader", request_body = RenameUploaderRequest,
    responses((status = 200, body = ApiResponse<UploaderNameResponse>)))]
pub async fn rename_uploader(
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(request): Json<RenameUploaderRequest>,
) -> Result<ApiResponse<UploaderNameResponse>, ApiError> {
    let name = request.name.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err(InnerApiError::BadRequest("名称必须为1–100个字符，且不能包含控制字符".into()).into());
    }
    let txn = db.begin().await?;
    let (uid, _) = uploader(&txn, id).await?;
    if request.expected_upper_id != uid.to_string() {
        return Err(InnerApiError::BadRequest("UP主UID已变更，请重新打开编辑窗口".into()).into());
    }
    txn.execute(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO uploader_name_override(upper_id, name) VALUES (?, ?)
         ON CONFLICT(upper_id) DO UPDATE SET name = excluded.name",
        [uid.into(), name.into()],
    ))
    .await?;
    let result = response(&txn, uid, name.to_string()).await?;
    txn.commit().await?;
    Ok(ApiResponse::ok(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bili_sync_migration::{Migrator, MigratorTrait};
    use sea_orm::{
        ActiveModelTrait,
        ActiveValue::{NotSet, Set},
        Database, IntoActiveModel,
    };
    use serde_json::{json, Value};

    async fn database() -> Arc<DatabaseConnection> {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        Arc::new(db)
    }

    async fn add_video(
        db: &DatabaseConnection,
        uid: i64,
        name: &str,
        submission_id: Option<i32>,
        source_type: Option<i32>,
    ) -> video::Model {
        let mut model = video::Model {
            upper_id: uid,
            upper_name: name.into(),
            name: "Video title".into(),
            bvid: format!("BV{uid}{name}{submission_id:?}{source_type:?}"),
            path: "/saved/original.mp4".into(),
            download_status: 123,
            deleted: 1,
            submission_id,
            source_type,
            ..Default::default()
        }
        .into_active_model();
        model.id = NotSet;
        model.insert(db).await.unwrap()
    }

    async fn add_submission(db: &DatabaseConnection) -> submission::Model {
        submission::ActiveModel {
            upper_id: Set(42),
            upper_name: Set("Original".into()),
            path: Set("/saved".into()),
            created_at: Set("2025-01-01 00:00:00".into()),
            latest_row_at: Set("2026-01-01 00:00:00".into()),
            enabled: Set(false),
            scan_deleted_videos: Set(true),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn rename(db: &Arc<DatabaseConnection>, id: i32, uid: &str, name: &str) -> Result<Value, ApiError> {
        let result = rename_uploader(
            Path(id),
            Extension(db.clone()),
            Json(RenameUploaderRequest {
                name: name.into(),
                expected_upper_id: uid.into(),
            }),
        )
        .await?;
        Ok(serde_json::to_value(result).unwrap()["data"].clone())
    }

    async fn read(db: &DatabaseConnection, id: i32) -> video::Model {
        video::Entity::find_by_id(id).one(db).await.unwrap().unwrap()
    }

    #[tokio::test]
    async fn rename_updates_same_uid_only_preserves_files_and_survives_metadata_refresh() {
        let db = database().await;
        let source = add_submission(&db).await;
        let first = add_video(&db, 42, "Original", None, None).await;
        let second = add_video(&db, 42, "Older name", None, None).await;
        let pending = add_video(&db, 0, "", Some(source.id), None).await;
        // Same name, different identity; a collaboration source relation is not the uploader.
        let unrelated = add_video(&db, 43, "Original", Some(source.id), None).await;
        let bangumi = add_video(&db, 42, "Studio", None, Some(1)).await;
        let result = rename(&db, first.id, "42", "  水印原名  ").await.unwrap();
        assert_eq!(result["video_count"], 3);
        assert_eq!(result["name"], "水印原名");
        assert_eq!(result["upper_id"], "42");
        for original in [&first, &second, &pending] {
            let saved = read(&db, original.id).await;
            assert_eq!(saved.upper_name, "水印原名");
            assert_eq!(saved.path, original.path);
            assert_eq!(saved.download_status, original.download_status);
            assert_eq!(saved.deleted, original.deleted);
            assert_eq!(saved.name, original.name);
            assert_eq!(saved.upper_id, original.upper_id);
        }
        assert_eq!(read(&db, unrelated.id).await, unrelated);
        assert_eq!(read(&db, bangumi.id).await, bangumi);
        let saved_source = submission::Entity::find_by_id(source.id)
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved_source.upper_name, "水印原名");
        assert!(!saved_source.enabled);
        assert_eq!(saved_source.path, source.path);
        video::ActiveModel {
            id: Set(first.id),
            upper_name: Set("Remote new name".into()),
            ..Default::default()
        }
        .update(db.as_ref())
        .await
        .unwrap();
        submission::ActiveModel {
            id: Set(source.id),
            upper_name: Set("Another remote name".into()),
            ..Default::default()
        }
        .update(db.as_ref())
        .await
        .unwrap();
        let added = add_video(&db, 42, "Future upstream name", None, None).await;
        let pending_added = add_video(&db, 0, "Pending", Some(source.id), None).await;
        for id in [first.id, added.id, pending_added.id] {
            assert_eq!(read(&db, id).await.upper_name, "水印原名");
        }
        let info = response(db.as_ref(), 42, String::new()).await.unwrap();
        assert_eq!(info.name, "水印原名");
        assert!(info.names.len() >= 5);
        assert!(info.names.contains(&"Original".into()));
        assert!(info.names.contains(&"Remote new name".into()));
        assert!(info.names.contains(&"Another remote name".into()));
        // Restoring an old name is a persistent choice, not a release of the override.
        rename(&db, pending.id, "42", "Original").await.unwrap();
        assert_eq!(read(&db, first.id).await.upper_name, "Original");
    }

    #[tokio::test]
    async fn rename_validates_name_and_uid_without_partial_updates() {
        let db = database().await;
        let video = add_video(&db, 42, "Original", None, None).await;
        for name in ["", "   ", "Line\nBreak", "Nul\0name", &"名".repeat(101)] {
            assert!(rename(&db, video.id, "42", name).await.is_err());
            assert_eq!(read(&db, video.id).await, video);
        }
        assert!(rename(&db, video.id, "43", "Changed").await.is_err());
        assert!(rename(&db, -1, "42", "Changed").await.is_err());
        let unknown = add_video(&db, 0, "Unknown", None, None).await;
        assert!(rename(&db, unknown.id, "0", "Changed").await.is_err());
        let bangumi = add_video(&db, 42, "Studio", None, Some(1)).await;
        assert!(rename(&db, bangumi.id, "42", "Changed").await.is_err());
        assert_eq!(read(&db, video.id).await, video);
        assert!(db
            .query_all(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT * FROM uploader_name_override"
            ))
            .await
            .unwrap()
            .is_empty());
        for name in ["One", "Two", "Three", "Four", "Five", "Six", "Six", "Original"] {
            rename(&db, video.id, "42", name).await.unwrap();
        }
        let result = get_uploader_name(Path(video.id), Extension(db.clone())).await.unwrap();
        let value = serde_json::to_value(result).unwrap();
        let names = value["data"]["names"].as_array().unwrap();
        assert_eq!(names.len(), 7);
        assert!(names.contains(&json!("Original")));
    }

    #[tokio::test]
    async fn uploader_migration_backfills_existing_names_and_rolls_back_cleanly() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(
            &db,
            Some(
                Migrator::migrations()
                    .iter()
                    .position(|m| m.name() == "m20261024_000003_uploader_names")
                    .unwrap() as u32,
            ),
        )
        .await
        .unwrap();
        let source = add_submission(&db).await;
        let video = add_video(&db, 42, "Watermark name", None, None).await;
        submission::ActiveModel {
            id: Set(source.id),
            upper_name: Set("New upstream name".into()),
            ..Default::default()
        }
        .update(&db)
        .await
        .unwrap();
        Migrator::up(&db, None).await.unwrap();
        let db = Arc::new(db);
        let info = response(db.as_ref(), 42, String::new()).await.unwrap();
        assert!(info.names.contains(&"Original".into()));
        assert!(info.names.contains(&"Watermark name".into()));
        assert!(info.names.contains(&"New upstream name".into()));
        rename(&db, video.id, "42", "Custom").await.unwrap();
        let migrations = Migrator::migrations();
        let count = migrations.len() - migrations.iter().position(|m| m.name() == "m20261024_000003_uploader_names").unwrap();
        Migrator::down(db.as_ref(), Some(count as u32)).await.unwrap();
        video::ActiveModel {
            id: Set(video.id),
            upper_name: Set("After rollback".into()),
            ..Default::default()
        }
        .update(db.as_ref())
        .await
        .unwrap();
        assert_eq!(read(&db, video.id).await.upper_name, "After rollback");
    }
}
