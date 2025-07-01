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
                .and(video::Column::Deleted.eq(0))
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
                .and(video::Column::Deleted.eq(0))
                .and(additional_expr),
        )
        .find_with_related(page::Entity)
        .all(connection)
        .await
        .context("filter unhandled video pages failed")
}

/// 筛选在当前循环中失败但可重试的视频（不包括已达到最大重试次数的视频）
pub async fn get_failed_videos_in_current_cycle(
    additional_expr: SimpleExpr,
    connection: &DatabaseConnection,
) -> Result<Vec<(video::Model, Vec<page::Model>)>> {
    use crate::utils::status::STATUS_COMPLETED;

    let all_videos = video::Entity::find()
        .filter(
            video::Column::Valid
                .eq(true)
                .and(video::Column::DownloadStatus.lt(STATUS_COMPLETED))
                .and(video::Column::DownloadStatus.gt(0)) // 排除未开始的视频 (状态为0)
                .and(video::Column::Category.is_in([1, 2]))
                .and(video::Column::SinglePage.is_not_null())
                .and(video::Column::Deleted.eq(0))
                .and(additional_expr),
        )
        .find_with_related(page::Entity)
        .all(connection)
        .await?;

    let result = all_videos
        .into_iter()
        .filter(|(video_model, pages_model)| {
            // 检查视频和分页是否有可重试的失败
            let video_status = crate::utils::status::VideoStatus::from(video_model.download_status);
            let video_should_retry = video_status.should_run().iter().any(|&should_run| should_run);

            let pages_should_retry = pages_model.iter().any(|page_model| {
                let page_status = crate::utils::status::PageStatus::from(page_model.download_status);
                page_status.should_run().iter().any(|&should_run| should_run)
            });

            video_should_retry || pages_should_retry
        })
        .collect::<Vec<_>>();

    Ok(result)
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
                        download_status: Set(0),   // 重置下载状态为未开始，强制重新下载
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
                    // 视频存在且未删除，检查是否需要更新 share_copy 字段
                    if let Some(new_share_copy) = model.share_copy.as_ref() {
                        if existing.share_copy != model.share_copy.as_ref().clone() {
                            // 需要更新 share_copy 字段
                            info!("检测到需要更新share_copy: 视频={}, 原值={:?}, 新值={:?}", 
                                existing.name, existing.share_copy, model.share_copy);
                            let update_model = video::ActiveModel {
                                id: Unchanged(existing.id),
                                share_copy: model.share_copy.clone(),
                                // 同时更新其他可能变化的字段
                                name: model.name.clone(),
                                intro: model.intro.clone(),
                                cover: model.cover.clone(),
                                ..Default::default()
                            };
                            update_model.save(connection).await?;
                            info!("更新视频 {} 的 share_copy 字段完成", existing.name);
                        } else {
                            tracing::debug!("share_copy无需更新: 视频={}, 现有值={:?}", 
                                existing.name, existing.share_copy);
                        }
                    }
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
        // 未启用扫描已删除视频：使用原有逻辑，但增加 share_copy 更新检查
        for video_info in videos_info {
            let mut model = video_info.into_simple_model();
            video_source.set_relation_id(&mut model);

            // 先尝试插入，如果失败说明记录已存在
            let insert_result = video::Entity::insert(model.clone())
                .on_conflict(OnConflict::new().do_nothing().to_owned())
                .do_nothing()
                .exec(connection)
                .await;

            // 如果插入没有影响任何行（即记录已存在），检查是否需要更新 share_copy
            if let Ok(insert_res) = insert_result {
                // 检查插入是否真的生效，如果没有生效说明记录已存在
                let insert_success = match &insert_res {
                    sea_orm::TryInsertResult::Inserted(_) => true,
                    sea_orm::TryInsertResult::Conflicted => false,
                    sea_orm::TryInsertResult::Empty => true, // 空插入视为成功
                };
                if !insert_success {
                    // 记录已存在，检查是否需要更新 share_copy 字段
                    if let Some(_new_share_copy) = model.share_copy.as_ref() {
                        let existing_video = video::Entity::find()
                            .filter(video::Column::Bvid.eq(model.bvid.as_ref()))
                            .filter(video_source.filter_expr())
                            .one(connection)
                            .await?;

                        if let Some(existing) = existing_video {
                            if existing.share_copy != model.share_copy.as_ref().clone() {
                                // 需要更新 share_copy 字段
                                info!("检测到需要更新share_copy(未启用扫描删除): 视频={}, 原值={:?}, 新值={:?}", 
                                    existing.name, existing.share_copy, model.share_copy);
                                let update_model = video::ActiveModel {
                                    id: Unchanged(existing.id),
                                    share_copy: model.share_copy.clone(),
                                    // 同时更新其他可能变化的字段
                                    name: model.name.clone(),
                                    intro: model.intro.clone(),
                                    cover: model.cover.clone(),
                                    ..Default::default()
                                };
                                update_model.save(connection).await?;
                                info!("更新视频 {} 的 share_copy 字段完成", existing.name);
                            } else {
                                tracing::debug!("share_copy无需更新(未启用扫描删除): 视频={}, 现有值={:?}", 
                                    existing.name, existing.share_copy);
                            }
                        }
                    }
                }
            }
        }
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
