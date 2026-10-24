use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;

use crate::utils::time_format::now_standard_string;
use anyhow::{ensure, Context, Result};
use bili_sync_entity::*;
use chrono::Utc;
use futures::Stream;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, Unchanged};

use crate::adapter::{VideoSource, VideoSourceEnum, _ActiveModel};
use crate::bilibili::{BiliClient, FavoriteList, VideoInfo};
use crate::utils::keyword_filter::deserialize_keywords;

pub(crate) static FAVORITE_SOURCE_LOCK: tokio::sync::RwLock<()> = tokio::sync::RwLock::const_new(());

pub type FavoriteSource = super::source_state::SourceState<favorite::Model>;

impl FavoriteSource {
    pub async fn finish_scan(&self, connection: &DatabaseConnection) -> Result<()> {
        if self.is_unavailable() {
            favorite::Entity::update_many()
                .filter(favorite::Column::Id.eq(self.id))
                .col_expr(favorite::Column::Enabled, Expr::value(false))
                .exec(connection)
                .await?;
            info!("视频源「{}」已删除或不可访问，本次扫描结束，已禁用自动扫描", self.name);
        }
        Ok(())
    }
}

impl VideoSource for FavoriteSource {
    fn filter_expr(&self) -> SimpleExpr {
        video::Column::FavoriteId.eq(self.id)
    }

    fn set_relation_id(&self, video_model: &mut video::ActiveModel) {
        video_model.favorite_id = Set(Some(self.id));
    }

    fn path(&self) -> &Path {
        Path::new(self.path.as_str())
    }

    fn get_latest_row_at(&self) -> String {
        self.latest_row_at.clone()
    }

    fn update_latest_row_at(&self, datetime: String) -> _ActiveModel {
        _ActiveModel::Favorite(favorite::ActiveModel {
            id: Unchanged(self.id),
            latest_row_at: Set(datetime),
            ..Default::default()
        })
    }

    fn should_take(&self, _release_datetime: &chrono::DateTime<Utc>, _latest_row_at_string: &str) -> bool {
        true
    }

    fn log_refresh_video_start(&self) {
        info!("开始扫描收藏夹「{}」..", self.name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        if count > 0 {
            info!("扫描收藏夹「{}」完成，获取到 {} 条新视频", self.name, count);
        } else {
            info!("收藏夹「{}」无新视频", self.name);
        }
    }

    fn log_fetch_video_start(&self) {
        debug!("开始填充收藏夹「{}」视频详情..", self.name);
    }

    fn log_fetch_video_end(&self) {
        debug!("填充收藏夹「{}」视频详情完成", self.name);
    }

    fn log_download_video_start(&self) {
        debug!("开始下载收藏夹「{}」视频..", self.name);
    }

    fn log_download_video_end(&self) {
        debug!("下载收藏夹「{}」视频完成", self.name);
    }

    fn scan_deleted_videos(&self) -> bool {
        self.scan_deleted_videos
    }

    fn source_type_display(&self) -> String {
        "收藏夹".to_string()
    }

    fn source_name_display(&self) -> String {
        self.name.clone()
    }

    fn include_keywords(&self) -> Option<Vec<String>> {
        deserialize_keywords(&self.include_keywords)
    }

    fn exclude_keywords(&self) -> Option<Vec<String>> {
        deserialize_keywords(&self.exclude_keywords)
    }

    fn min_duration_seconds(&self) -> Option<i32> {
        self.min_duration_seconds
    }

    fn max_duration_seconds(&self) -> Option<i32> {
        self.max_duration_seconds
    }

    fn min_page_duration_seconds(&self) -> Option<i32> {
        self.min_page_duration_seconds
    }

    fn max_page_duration_seconds(&self) -> Option<i32> {
        self.max_page_duration_seconds
    }
}

#[allow(dead_code)]
pub async fn init_favorite_sources(
    conn: &DatabaseConnection,
    favorite_list: &HashMap<String, std::path::PathBuf>,
) -> Result<()> {
    for (fid, path) in favorite_list {
        let fid_i64 = match fid.parse::<i64>() {
            Ok(id) => id,
            Err(e) => {
                warn!("无效的收藏夹ID {}: {}, 跳过", fid, e);
                continue;
            }
        };

        let existing = favorite::Entity::find()
            .filter(favorite::Column::FId.eq(fid_i64))
            .one(conn)
            .await?;

        if existing.is_none() {
            let bili_client = crate::bilibili::BiliClient::new(String::new());
            let favorite = FavoriteList::new(&bili_client, fid.to_owned());

            match favorite.get_info().await {
                Ok(favorite_info) => {
                    let model = favorite::ActiveModel {
                        id: Set(Default::default()),
                        f_id: Set(favorite_info.id),
                        owner_mid: Set(favorite_info.mid),
                        name: Set(favorite_info.title.clone()),
                        path: Set(path.to_string_lossy().to_string()),
                        created_at: Set(now_standard_string()),
                        latest_row_at: Set("1970-01-01 00:00:00".to_string()),
                        enabled: Set(true),
                        scan_deleted_videos: Set(false),
                        ..Default::default()
                    };

                    let result = favorite::Entity::insert(model)
                        .exec(conn)
                        .await
                        .context("Failed to insert favorite source")?;

                    info!(
                        "初始化收藏夹源: {} (ID: {})",
                        favorite_info.title, result.last_insert_id
                    );
                }
                Err(e) => {
                    warn!("获取收藏夹 {} 信息失败: {}, 创建临时记录", fid, e);

                    let model = favorite::ActiveModel {
                        id: Set(Default::default()),
                        f_id: Set(fid_i64),
                        name: Set(format!("收藏夹 {}", fid)),
                        path: Set(path.to_string_lossy().to_string()),
                        created_at: Set(now_standard_string()),
                        latest_row_at: Set("1970-01-01 00:00:00".to_string()),
                        enabled: Set(true),
                        scan_deleted_videos: Set(false),
                        ..Default::default()
                    };

                    let result = favorite::Entity::insert(model)
                        .exec(conn)
                        .await
                        .context("Failed to insert temporary favorite source")?;

                    info!("初始化临时收藏夹源: {} (ID: {})", fid, result.last_insert_id);
                }
            }
        } else if let Some(existing) = existing {
            if existing.path != path.to_string_lossy() {
                let model = favorite::ActiveModel {
                    id: Set(existing.id),
                    path: Set(path.to_string_lossy().to_string()),
                    ..Default::default()
                };

                favorite::Entity::update(model)
                    .exec(conn)
                    .await
                    .context("Failed to update favorite source")?;

                info!("更新收藏夹源: {} (ID: {})", existing.name, existing.id);
            }
        }
    }

    Ok(())
}

pub(super) async fn favorite_from<'a>(
    source_id: i32,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
) -> Result<(
    VideoSourceEnum,
    Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a + Send>>,
)> {
    favorite_from_with_api_base(source_id, bili_client, connection, "https://api.bilibili.com").await
}

pub(crate) async fn favorite_from_with_api_base<'a>(
    source_id: i32,
    bili_client: &'a BiliClient,
    connection: &DatabaseConnection,
    api_base: &str,
) -> Result<(
    VideoSourceEnum,
    Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a + Send>>,
)> {
    // Notification/error-context callers also refresh metadata outside a full scan.
    let _guard = FAVORITE_SOURCE_LOCK.read().await;
    let source = favorite::Entity::find_by_id(source_id)
        .one(connection)
        .await?
        .context("favorite not found")?;
    let favorite = FavoriteList::with_api_base(bili_client, source.f_id.to_string(), api_base);
    let favorite_info = match favorite.get_info().await {
        Ok(info) => info,
        Err(error) if super::source_state::is_unavailable_error(&error, true) => {
            let source = FavoriteSource::new(source, true);
            let stream = source.track_stream(favorite.into_video_stream(), true);
            return Ok((source.into(), stream));
        }
        Err(error) => return Err(error),
    };
    ensure!(favorite_info.id == source.f_id, "收藏夹返回的ID与保存的ID不一致");
    ensure!(
        source.owner_mid.is_none_or(|mid| Some(mid) == favorite_info.mid),
        "收藏夹创建者UID与保存的UID不一致"
    );
    let source = favorite::ActiveModel {
        id: Unchanged(source.id),
        name: Set(favorite_info.title),
        // Legacy rows may contain a short ID that is also an unrelated full ID.
        // Populate their owner only through explicit repair, never from a scan.
        owner_mid: Unchanged(source.owner_mid),
        ..Default::default()
    }
    .update(connection)
    .await?;
    let source = FavoriteSource::new(source, false);
    let stream = source.track_stream(favorite.into_video_stream(), true);
    Ok((source.into(), stream))
}
