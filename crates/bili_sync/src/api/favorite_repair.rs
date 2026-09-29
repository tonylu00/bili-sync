use std::sync::Arc;

use anyhow::{ensure, Context, Result};
use axum::{extract::Path, Extension, Json};
use bili_sync_entity::favorite;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::adapter::favorite::FAVORITE_SOURCE_LOCK;
use crate::api::wrapper::ApiResponse;
use crate::bilibili::{BiliClient, FavoriteList, FavoriteListInfo};

#[derive(Deserialize, ToSchema)]
pub struct RepairFavoriteRequest {
    /// 当前保存的ID，用于防止覆盖其他客户端的修复。
    pub expected_f_id: String,
    /// 正确的收藏夹完整ID或B站收藏夹URL。
    pub favorite: String,
}

#[derive(Serialize, ToSchema)]
pub struct RepairFavoriteResponse {
    pub success: bool,
    pub message: String,
}

struct RepairTarget {
    media_id: i64,
    owner_mid: Option<i64>,
}

fn positive_id(value: &str) -> Result<i64> {
    ensure!(
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()),
        "请输入有效的收藏夹ID"
    );
    let id = value.parse::<i64>().context("收藏夹ID超出范围")?;
    ensure!(id > 0, "收藏夹ID必须大于0");
    Ok(id)
}

fn parse_target(input: &str) -> Result<RepairTarget> {
    let input = input.trim();
    if input.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(RepairTarget {
            media_id: positive_id(input)?,
            owner_mid: None,
        });
    }
    let url = reqwest::Url::parse(input).context("请输入完整收藏夹ID或B站收藏夹URL")?;
    ensure!(
        matches!(url.scheme(), "http" | "https") && url.host_str() == Some("space.bilibili.com"),
        "请输入B站个人空间中的收藏夹URL"
    );
    let segments: Vec<_> = url.path().trim_matches('/').split('/').collect();
    ensure!(
        segments.len() == 2 && segments[1] == "favlist",
        "请输入包含fid参数的收藏夹URL"
    );
    let ids: Vec<_> = url.query_pairs().filter(|(key, _)| key == "fid").collect();
    ensure!(ids.len() == 1, "收藏夹URL必须包含一个fid参数");
    Ok(RepairTarget {
        media_id: positive_id(&ids[0].1)?,
        owner_mid: Some(positive_id(segments[0])?),
    })
}

async fn save_repair(
    db: &DatabaseConnection,
    source_id: i32,
    expected_f_id: i64,
    target: &RepairTarget,
    info: FavoriteListInfo,
) -> Result<String> {
    ensure!(info.id == target.media_id, "B站返回的收藏夹ID与输入不一致");
    ensure!(
        target.owner_mid.is_none_or(|mid| Some(mid) == info.mid),
        "收藏夹所属UP主与URL不一致"
    );
    // Do not guess by ID length, title or appending an owner's digits. Short IDs
    // can also be valid full IDs for unrelated folders. Require upstream proof.
    ensure!(
        info.id != expected_f_id && info.fid == Some(expected_f_id),
        "该收藏夹的原始ID与当前源不匹配，请复制原收藏夹的完整URL"
    );
    let txn = db.begin().await?;
    let source = favorite::Entity::find_by_id(source_id)
        .one(&txn)
        .await?
        .context("收藏夹源不存在")?;
    ensure!(source.f_id == expected_f_id, "收藏夹ID已变更，请刷新页面后重试");
    ensure!(
        favorite::Entity::find()
            .filter(favorite::Column::FId.eq(info.id))
            .one(&txn)
            .await?
            .is_none(),
        "完整ID的收藏夹源已经存在，请先检查重复源；现有记录未修改"
    );
    let message = format!(
        "已修复收藏夹「{}」：{} → {}，下次扫描生效",
        info.title, source.f_id, info.id
    );
    favorite::ActiveModel {
        id: Set(source.id),
        f_id: Set(info.id),
        name: Set(info.title),
        ..Default::default()
    }
    .update(&txn)
    .await?;
    txn.commit().await?;
    tracing::info!("收藏夹源 {}: {}", source_id, message);
    Ok(message)
}

#[utoipa::path(
    put,
    path = "/api/video-sources/favorite/{id}/repair",
    params(("id" = i32, Path, description = "本地收藏夹源ID")),
    request_body = RepairFavoriteRequest,
    responses((status = 200, body = ApiResponse<RepairFavoriteResponse>))
)]
pub async fn repair_favorite_source(
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Path(source_id): Path<i32>,
    Json(request): Json<RepairFavoriteRequest>,
) -> ApiResponse<RepairFavoriteResponse> {
    let result = async {
        let expected_f_id = positive_id(request.expected_f_id.trim())?;
        let target = parse_target(&request.favorite)?;
        let source = favorite::Entity::find_by_id(source_id)
            .one(db.as_ref())
            .await?
            .context("收藏夹源不存在")?;
        ensure!(source.f_id == expected_f_id, "收藏夹ID已变更，请刷新页面后重试");
        let client = BiliClient::new(String::new());
        let info = FavoriteList::new(&client, target.media_id.to_string())
            .get_info()
            .await
            .context("无法验证收藏夹，请检查URL以及B站登录状态")?;
        let _guard = FAVORITE_SOURCE_LOCK
            .try_write()
            .context("收藏夹正在扫描或下载，请暂停任务并等待当前任务结束后重试")?;
        save_repair(db.as_ref(), source_id, expected_f_id, &target, info).await
    }
    .await;
    ApiResponse::ok(match result {
        Ok(message) => RepairFavoriteResponse { success: true, message },
        Err(error) => RepairFavoriteResponse {
            success: false,
            message: format!("{error:#}"),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bili_sync_entity::video;
    use bili_sync_migration::{Migrator, MigratorTrait};
    use sea_orm::{Database, IntoActiveModel, PaginatorTrait};
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const URL: &str = "https://space.bilibili.com/443891409/favlist?fid=3800171409&ftype=create";

    fn folder() -> FavoriteListInfo {
        serde_json::from_value(json!({
            "id": 3800171409_i64, "fid": 38001714, "mid": 443891409, "title": "神父的自我修养4"
        }))
        .unwrap()
    }

    async fn database() -> (DatabaseConnection, favorite::Model) {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        let source = favorite::ActiveModel {
            f_id: Set(38001714),
            name: Set("默认收藏夹".into()),
            path: Set("/saved/original".into()),
            created_at: Set("2025-01-01 00:00:00".into()),
            latest_row_at: Set("2026-09-01 00:00:00".into()),
            enabled: Set(false),
            scan_deleted_videos: Set(true),
            include_keywords: Set(Some("[\"include\"]".into())),
            exclude_keywords: Set(Some("[\"exclude\"]".into())),
            min_duration_seconds: Set(Some(10)),
            max_duration_seconds: Set(Some(100)),
            min_page_duration_seconds: Set(Some(2)),
            max_page_duration_seconds: Set(Some(80)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        (db, source)
    }

    #[test]
    fn repair_target_accepts_full_ids_and_validates_folder_urls() {
        let url = parse_target(URL).unwrap();
        assert_eq!(url.media_id, 3800171409);
        assert_eq!(url.owner_mid, Some(443891409));
        assert_eq!(parse_target(" 3800171409 ").unwrap().media_id, url.media_id);
        for invalid in [
            "",
            "-1",
            "0",
            "1.2",
            "9223372036854775808",
            "https://example.com/443891409/favlist?fid=3800171409",
            "https://space.bilibili.com/443891409/favlist",
            "https://space.bilibili.com/443891409/favlist?fid=1&fid=2",
        ] {
            assert!(parse_target(invalid).is_err(), "{invalid}");
        }
    }

    #[tokio::test]
    async fn repair_preserves_source_settings_and_video_records_and_queued_scan_uses_full_id() {
        let (db, original) = database().await;
        let queued = crate::adapter::Args::Favorite { source_id: original.id };
        let saved_video = video::Model {
            id: 1,
            favorite_id: Some(original.id),
            bvid: "BVexisting".into(),
            name: "Downloaded video".into(),
            path: "/saved/video".into(),
            download_status: 123,
            ..Default::default()
        }
        .into_active_model()
        .insert(&db)
        .await
        .unwrap();
        save_repair(&db, original.id, original.f_id, &parse_target(URL).unwrap(), folder())
            .await
            .unwrap();
        let mut expected = original.clone();
        expected.f_id = 3800171409;
        expected.name = "神父的自我修养4".into();
        assert_eq!(
            favorite::Entity::find_by_id(original.id)
                .one(&db)
                .await
                .unwrap()
                .unwrap(),
            expected
        );
        assert_eq!(
            video::Entity::find_by_id(saved_video.id)
                .one(&db)
                .await
                .unwrap()
                .unwrap(),
            saved_video
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let api_base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for data in [
                json!({"id": 3800171409_i64, "fid": 38001714, "mid": 443891409, "title": "神父的自我修养4"}),
                json!({"info": {"media_count": 1}, "medias": [{
                    "title": "Recovered include video", "type": 2, "bvid": "BVrecovered", "intro": "", "cover": "",
                    "ctime": 1750000000, "pubtime": 1750000000, "fav_time": 1750000000,
                    "upper": {"mid": 443891409, "name": "步月moon", "face": ""}, "attr": 0
                }], "has_more": false}),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buf = vec![0; 8192];
                let size = socket.read(&mut buf).await.unwrap();
                requests.push(String::from_utf8_lossy(&buf[..size]).lines().next().unwrap().to_owned());
                let body = json!({"code": 0, "message": "OK", "data": data}).to_string();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        let crate::adapter::Args::Favorite { source_id } = queued else {
            unreachable!()
        };
        let client = BiliClient::new(String::new());
        let (source, stream) =
            crate::adapter::favorite::favorite_from_with_api_base(source_id, &client, &db, &api_base)
                .await
                .unwrap();
        let (count, _) = crate::workflow::refresh_video_source(
            &source,
            stream,
            &db,
            tokio_util::sync::CancellationToken::new(),
            &client,
        )
        .await
        .unwrap();
        assert_eq!(count, 1, "the old scan timestamp must not skip recovered videos");
        let recovered = video::Entity::find()
            .filter(video::Column::Bvid.eq("BVrecovered"))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered.favorite_id, Some(original.id));
        assert_eq!(
            video::Entity::find_by_id(saved_video.id)
                .one(&db)
                .await
                .unwrap()
                .unwrap(),
            saved_video
        );
        let crate::adapter::VideoSourceEnum::Favorite(source) = source else {
            unreachable!()
        };
        assert_eq!(source, expected);
        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("GET /x/v3/fav/folder/info?media_id=3800171409 "));
        assert!(requests[1].starts_with("GET /x/v3/fav/resource/list?media_id=3800171409&"));
        assert_eq!(favorite::Entity::find().count(&db).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn repair_rejects_unrelated_folders_wrong_owners_and_stale_requests() {
        let (db, source) = database().await;
        let target = parse_target(URL).unwrap();
        let mut unrelated = folder();
        unrelated.fid = Some(123);
        assert!(save_repair(&db, source.id, source.f_id, &target, unrelated)
            .await
            .is_err());
        let mut wrong_owner = folder();
        wrong_owner.mid = Some(11740014);
        assert!(save_repair(&db, source.id, source.f_id, &target, wrong_owner)
            .await
            .is_err());
        let mut wrong_id = folder();
        wrong_id.id = 3800171400;
        assert!(save_repair(&db, source.id, source.f_id, &target, wrong_id)
            .await
            .is_err());
        assert_eq!(
            favorite::Entity::find_by_id(source.id).one(&db).await.unwrap().unwrap(),
            source
        );
        save_repair(&db, source.id, source.f_id, &target, folder())
            .await
            .unwrap();
        assert!(save_repair(&db, source.id, source.f_id, &target, folder())
            .await
            .is_err());
        assert_eq!(favorite::Entity::find().count(&db).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn repair_rejects_duplicate_target_without_modifying_either_source() {
        let (db, source) = database().await;
        let mut duplicate = source.clone().into_active_model();
        duplicate.id = sea_orm::ActiveValue::NotSet;
        duplicate.f_id = Set(3800171409);
        let duplicate = duplicate.insert(&db).await.unwrap();
        assert!(
            save_repair(&db, source.id, source.f_id, &parse_target(URL).unwrap(), folder())
                .await
                .is_err()
        );
        assert_eq!(
            favorite::Entity::find_by_id(source.id).one(&db).await.unwrap().unwrap(),
            source
        );
        assert_eq!(
            favorite::Entity::find_by_id(duplicate.id)
                .one(&db)
                .await
                .unwrap()
                .unwrap(),
            duplicate
        );
    }
}
