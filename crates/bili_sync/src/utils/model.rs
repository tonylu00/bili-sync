use anyhow::{Context, Result};
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::DatabaseTransaction;
use tracing::info;

use crate::adapter::{VideoSource, VideoSourceEnum};
use crate::bilibili::{PageInfo, VideoInfo};
use crate::utils::status::STATUS_COMPLETED;

/// 筛选未填充的视频
pub async fn filter_unfilled_videos(
    additional_expr: SimpleExpr,
    conn: &DatabaseConnection,
) -> Result<Vec<video::Model>> {
    video::Entity::find()
        .filter(
            video::Column::Valid
                .eq(true)
                .and(video::Column::DownloadStatus.eq(0))
                .and(video::Column::Category.is_in([1, 2]))
                .and(video::Column::SinglePage.is_null())
                .and(additional_expr),
        )
        .all(conn)
        .await
        .context("filter unfilled videos failed")
}

/// 筛选未处理完成的视频和视频页
pub async fn filter_unhandled_video_pages(
    additional_expr: SimpleExpr,
    connection: &DatabaseConnection,
) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
    video::Entity::find()
        .filter(
            video::Column::Valid
                .eq(true)
                .and(video::Column::DownloadStatus.lt(STATUS_COMPLETED))
                .and(video::Column::Category.is_in([1, 2]))
                .and(video::Column::SinglePage.is_not_null())
                .and(additional_expr),
        )
        .find_with_related(page::Entity)
        .all(connection)
        .await
        .context("filter unhandled video pages failed")
}

/// 尝试创建 Video Model，如果发生冲突则忽略
/// 如果视频源启用了扫描已删除视频设置，则会恢复已删除的视频
pub async fn create_videos(
    videos_info: Vec<VideoInfo>,
    video_source: &VideoSourceEnum,
    connection: &DatabaseConnection,
) -> Result<()> {
    use sea_orm::{Set, Unchanged};
    
    // 检查是否启用了扫描已删除视频
    let scan_deleted = video_source.scan_deleted_videos();
    
    if scan_deleted {
        // 启用扫描已删除视频：需要特别处理已删除的视频
        for video_info in videos_info {
            let mut model = video_info.into_simple_model();
            video_source.set_relation_id(&mut model);
            
            // 查找是否存在已删除的同一视频
            let existing_video = video::Entity::find()
                .filter(video::Column::Bvid.eq(model.bvid.as_ref()))
                .filter(video_source.filter_expr())
                .one(connection)
                .await?;
            
            if let Some(existing) = existing_video {
                if existing.deleted == 1 {
                    // 存在已删除的视频，恢复它并重置下载状态以强制重新下载
                    let update_model = video::ActiveModel {
                        id: Unchanged(existing.id),
                        deleted: Set(0),
                        download_status: Set(0), // 重置下载状态为未开始，强制重新下载
                        path: Set("".to_string()), // 清空原有路径，因为文件可能已经不存在
                        // 更新其他可能变化的字段
                        name: model.name.clone(),
                        intro: model.intro.clone(),
                        cover: model.cover.clone(),
                        tags: model.tags.clone(),
                        ..Default::default()
                    };
                    update_model.save(connection).await?;
                    
                    // 同时重置该视频的所有页面状态，强制重新下载
                    page::Entity::update_many()
                        .col_expr(page::Column::DownloadStatus, sea_orm::prelude::Expr::value(0)) // 重置为未开始状态
                        .col_expr(page::Column::Path, sea_orm::prelude::Expr::value(Option::<String>::None)) // 清空文件路径
                        .filter(page::Column::VideoId.eq(existing.id))
                        .exec(connection)
                        .await?;
                    
                    info!("恢复已删除的视频并重置下载状态: {}", existing.name);
                } else {
                    // 视频存在且未删除，跳过
                    continue;
                }
            } else {
                // 视频不存在，正常插入
                video::Entity::insert(model)
                    .on_conflict(OnConflict::new().do_nothing().to_owned())
                    .do_nothing()
                    .exec(connection)
                    .await?;
            }
        }
    } else {
        // 未启用扫描已删除视频：使用原有逻辑，但需要排除已删除的视频
        let video_models = videos_info
            .into_iter()
            .map(|v| {
                let mut model = v.into_simple_model();
                video_source.set_relation_id(&mut model);
                model
            })
            .collect::<Vec<_>>();
        video::Entity::insert_many(video_models)
            .on_conflict(OnConflict::new().do_nothing().to_owned())
            .do_nothing()
            .exec(connection)
            .await?;
    }
    Ok(())
}

/// 尝试创建 Page Model，如果发生冲突则忽略
pub async fn create_pages(
    pages_info: Vec<PageInfo>,
    video_model: &bili_sync_entity::video::Model,
    connection: &DatabaseTransaction,
) -> Result<()> {
    let page_models = pages_info
        .into_iter()
        .map(|p| p.into_active_model(video_model))
        .collect::<Vec<page::ActiveModel>>();
    for page_chunk in page_models.chunks(50) {
        page::Entity::insert_many(page_chunk.to_vec())
            .on_conflict(
                OnConflict::columns([page::Column::VideoId, page::Column::Pid])
                    .do_nothing()
                    .to_owned(),
            )
            .do_nothing()
            .exec(connection)
            .await?;
    }
    Ok(())
}

/// 更新视频 model 的下载状态
pub async fn update_videos_model(videos: Vec<video::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    video::Entity::insert_many(videos)
        .on_conflict(
            OnConflict::column(video::Column::Id)
                .update_columns([video::Column::DownloadStatus, video::Column::Path])
                .to_owned(),
        )
        .exec(connection)
        .await?;
    Ok(())
}

/// 更新视频页 model 的下载状态
pub async fn update_pages_model(pages: Vec<page::ActiveModel>, connection: &DatabaseConnection) -> Result<()> {
    let query = page::Entity::insert_many(pages).on_conflict(
        OnConflict::column(page::Column::Id)
            .update_columns([page::Column::DownloadStatus, page::Column::Path])
            .to_owned(),
    );
    query.exec(connection).await?;
    Ok(())
}
