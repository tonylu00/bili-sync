use anyhow::{Context, Result};
use bili_sync_entity::*;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{OnConflict, SimpleExpr};
use sea_orm::DatabaseTransaction;
use tracing::{debug, info, warn};

use crate::adapter::{VideoSource, VideoSourceEnum};
use crate::bilibili::{PageInfo, VideoInfo};
use crate::config::with_config;
use crate::utils::status::STATUS_COMPLETED;

/// 检查actors字段是否需要初始化
/// 返回 (需要初始化, 已有actors字段的视频数量, 总番剧视频数量)
pub async fn check_actors_field_initialization_needed(connection: &DatabaseConnection) -> Result<(bool, i64, i64)> {
    // 检查配置中的初始化标记
    let config_initialized = with_config(|config| config.config.actors_field_initialized);

    if config_initialized {
        return Ok((false, 0, 0));
    }

    // 统计番剧视频的actors字段完成情况
    let total_bangumi_count = video::Entity::find()
        .filter(video::Column::Category.eq(1)) // 只统计番剧
        .count(connection)
        .await?;

    let filled_actors_count = video::Entity::find()
        .filter(video::Column::Category.eq(1))
        .filter(video::Column::Actors.is_not_null())
        .filter(video::Column::Actors.ne(""))
        .count(connection)
        .await?;

    // 如果大部分番剧都有actors数据，认为已经初始化完成
    let needs_init = if total_bangumi_count > 0 {
        let completion_rate = filled_actors_count as f64 / total_bangumi_count as f64;
        completion_rate < 0.8 // 少于80%的完成率时需要初始化
    } else {
        false // 没有番剧时不需要初始化
    };

    info!(
        "Actors字段初始化检查: 需要初始化={}, 已填充={}/{} ({:.1}%)",
        needs_init,
        filled_actors_count,
        total_bangumi_count,
        if total_bangumi_count > 0 {
            filled_actors_count as f64 / total_bangumi_count as f64 * 100.0
        } else {
            0.0
        }
    );

    Ok((needs_init, filled_actors_count as i64, total_bangumi_count as i64))
}

/// 标记actors字段初始化完成
pub async fn mark_actors_field_initialization_complete() -> Result<()> {
    use crate::config::{get_config_manager, reload_config};

    if let Some(config_manager) = get_config_manager() {
        // 更新数据库中的配置
        config_manager
            .set_config_value("actors_field_initialized", &true)
            .await?;

        // 重新加载配置以应用更改
        reload_config();

        info!("已标记actors字段初始化完成");
    } else {
        warn!("无法获取配置管理器，无法标记actors字段初始化完成");
    }

    Ok(())
}

/// 根据show_season_type和其他字段重新计算番剧的智能命名
fn recalculate_bangumi_name(
    title: &str,
    share_copy: Option<&str>,
    show_title: Option<&str>,
    show_season_type: Option<i32>,
) -> String {
    // 参考convert.rs中的智能命名逻辑
    if show_season_type == Some(2) {
        // 番剧影视类型，使用简化命名
        show_title.unwrap_or(title).to_string()
    } else {
        // 常规番剧类型，使用详细命名
        share_copy
            .filter(|s| !s.is_empty() && s.len() > title.len()) // 只有当share_copy更详细时才使用
            .or(show_title)
            .unwrap_or(title)
            .to_string()
    }
}

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
                .and(video::Column::AutoDownload.eq(true))  // 只处理设置为自动下载的视频
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
                .and(video::Column::AutoDownload.eq(true))  // 只处理设置为自动下载的视频
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
                .and(video::Column::AutoDownload.eq(true))  // 只处理设置为自动下载的视频
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
/// 对于选择性下载模式，只存储选中的视频到数据库
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
            // 选择性下载逻辑：针对 submission 类型视频源 - 需要在 into_simple_model() 之前获取信息
            let should_store_video = if let Some(selected_videos) = video_source.get_selected_videos() {
                // 获取创建时间来判断是否为新投稿
                let is_new_submission = if let Some(created_at) = video_source.get_created_at() {
                    // 如果视频发布时间晚于订阅创建时间，则为新投稿，自动下载
                    video_info.release_datetime() > &created_at
                } else {
                    // 如果无法获取创建时间，保守地认为不是新投稿
                    false
                };

                // 获取视频的 BVID（从 VideoInfo 获取）
                let video_bvid = match &video_info {
                    crate::bilibili::VideoInfo::Submission { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::Detail { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::Favorite { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::WatchLater { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::Collection { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::Bangumi { bvid, .. } => bvid,
                };

                let should_store = if is_new_submission {
                    // 新投稿：存储到数据库并设置自动下载
                    true
                } else {
                    // 历史投稿：只有在选择列表中的才存储到数据库
                    selected_videos.contains(video_bvid)
                };

                debug!(
                    "选择性下载检查(已删除扫描): BVID={}, 是否新投稿={}, 是否在选择列表中={}, 是否存储={}",
                    video_bvid,
                    is_new_submission,
                    selected_videos.contains(video_bvid),
                    should_store
                );

                should_store
            } else {
                // 没有选择性下载，存储所有视频
                true
            };

            // 如果不应该存储此视频，则跳过
            if !should_store_video {
                continue;
            }

            let mut model = video_info.into_simple_model();
            video_source.set_relation_id(&mut model);

            // 对于需要存储的视频，设置 auto_download 为 true
            model.auto_download = Set(true);

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
                    // 视频存在且未删除，检查是否需要更新字段
                    let mut needs_update = false;
                    let mut should_recalculate_name = false;

                    // 检查 share_copy 字段更新
                    let share_copy_changed = if let Some(new_share_copy) = model.share_copy.as_ref() {
                        existing.share_copy.as_ref() != Some(new_share_copy)
                    } else {
                        false
                    };

                    // 检查 show_season_type 字段更新
                    let show_season_type_changed = if let Some(new_show_season_type) = model.show_season_type.as_ref() {
                        existing.show_season_type != Some(*new_show_season_type)
                    } else {
                        false
                    };

                    // 检查 actors 字段更新
                    let actors_changed = match (&existing.actors, model.actors.as_ref()) {
                        (None, Some(new_actors)) => {
                            // 数据库为空，API有数据，需要更新
                            tracing::info!("检测到actors字段从空值更新为有值: {:?}", new_actors);
                            true
                        }
                        (Some(existing_actors), Some(new_actors)) => {
                            // 两者都有值，比较是否不同
                            let changed = existing_actors != new_actors;
                            if changed {
                                tracing::info!(
                                    "检测到actors字段值发生变化: 原值={:?}, 新值={:?}",
                                    existing_actors,
                                    new_actors
                                );
                            }
                            changed
                        }
                        (Some(_), None) => {
                            // 数据库有值，API返回空，保持原值不变
                            tracing::debug!("API未返回actors数据，保持数据库现有值");
                            false
                        }
                        (None, None) => {
                            // 两者都为空，无需更新
                            false
                        }
                    };

                    if share_copy_changed || show_season_type_changed || actors_changed {
                        needs_update = true;
                        should_recalculate_name = true;

                        if share_copy_changed {
                            info!(
                                "检测到需要更新share_copy: 视频={}, 原值={:?}, 新值={:?}",
                                existing.name, existing.share_copy, model.share_copy
                            );
                        }
                        if show_season_type_changed {
                            info!(
                                "检测到需要更新show_season_type: 视频={}, 原值={:?}, 新值={:?}",
                                existing.name, existing.show_season_type, model.show_season_type
                            );
                        }
                        if actors_changed {
                            info!(
                                "检测到需要更新actors: 视频={}, 原值={:?}, 新值={:?}",
                                existing.name, existing.actors, model.actors
                            );
                        }
                    }

                    if needs_update {
                        // 如果需要重新计算name，并且这是番剧类型（category=1）
                        // 但对于番剧影视类型（show_season_type=2），不重新计算name，保持原有的简洁格式
                        let new_name = if should_recalculate_name && existing.category == 1 {
                            let new_show_season_type = match &model.show_season_type {
                                Set(opt) => *opt,
                                _ => existing.show_season_type,
                            };

                            // 如果是番剧影视类型，不重新计算name，保持现有的简洁name
                            if new_show_season_type == Some(2) {
                                sea_orm::ActiveValue::NotSet // 保持现有name不变
                            } else {
                                // 对于常规番剧类型，进行重新计算
                                let title = existing.name.as_str();
                                let share_copy = match &model.share_copy {
                                    Set(Some(s)) => Some(s.as_str()),
                                    Set(None) => None,
                                    _ => existing.share_copy.as_deref(),
                                };

                                let recalculated_name =
                                    recalculate_bangumi_name(title, share_copy, None, new_show_season_type);
                                info!(
                                    "重新计算常规番剧name: 视频={}, 原name={}, 新name={}",
                                    existing.name, existing.name, recalculated_name
                                );
                                Set(recalculated_name)
                            }
                        } else {
                            model.name.clone()
                        };

                        let update_model = video::ActiveModel {
                            id: Unchanged(existing.id),
                            share_copy: model.share_copy.clone(),
                            show_season_type: model.show_season_type.clone(),
                            actors: model.actors.clone(),
                            name: new_name,
                            intro: model.intro.clone(),
                            cover: model.cover.clone(),
                            ..Default::default()
                        };

                        // 详细的数据库更新调试日志
                        tracing::info!(
                            "即将执行数据库更新(启用扫描删除): 视频={}, actors字段={:?}, share_copy={:?}, show_season_type={:?}",
                            existing.name, update_model.actors, update_model.share_copy, update_model.show_season_type
                        );

                        update_model.save(connection).await?;
                        info!("更新视频 {} 的字段完成", existing.name);
                    } else {
                        tracing::debug!(
                            "字段无需更新: 视频={}, share_copy={:?}, show_season_type={:?}, actors={:?}",
                            existing.name,
                            existing.share_copy,
                            existing.show_season_type,
                            existing.actors
                        );
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
            // 选择性下载逻辑：针对 submission 类型视频源 - 需要在 into_simple_model() 之前获取信息
            let should_store_video = if let Some(selected_videos) = video_source.get_selected_videos() {
                // 获取创建时间来判断是否为新投稿
                let is_new_submission = if let Some(created_at) = video_source.get_created_at() {
                    // 如果视频发布时间晚于订阅创建时间，则为新投稿，自动下载
                    video_info.release_datetime() > &created_at
                } else {
                    // 如果无法获取创建时间，保守地认为不是新投稿
                    false
                };

                // 获取视频的 BVID（从 VideoInfo 获取）
                let video_bvid = match &video_info {
                    crate::bilibili::VideoInfo::Submission { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::Detail { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::Favorite { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::WatchLater { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::Collection { bvid, .. } => bvid,
                    crate::bilibili::VideoInfo::Bangumi { bvid, .. } => bvid,
                };

                let should_store = if is_new_submission {
                    // 新投稿：存储到数据库并设置自动下载
                    true
                } else {
                    // 历史投稿：只有在选择列表中的才存储到数据库
                    selected_videos.contains(video_bvid)
                };

                debug!(
                    "选择性下载检查(常规模式): BVID={}, 是否新投稿={}, 是否在选择列表中={}, 是否存储={}",
                    video_bvid,
                    is_new_submission,
                    selected_videos.contains(video_bvid),
                    should_store
                );

                should_store
            } else {
                // 没有选择性下载，存储所有视频
                true
            };

            // 如果不应该存储此视频，则跳过
            if !should_store_video {
                continue;
            }

            let mut model = video_info.into_simple_model();
            video_source.set_relation_id(&mut model);

            // 对于需要存储的视频，设置 auto_download 为 true
            model.auto_download = Set(true);

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
                    // 记录已存在，检查是否需要更新字段
                    let existing_video = video::Entity::find()
                        .filter(video::Column::Bvid.eq(model.bvid.as_ref()))
                        .filter(video_source.filter_expr())
                        .one(connection)
                        .await?;

                    if let Some(existing) = existing_video {
                        let mut needs_update = false;
                        let mut should_recalculate_name = false;

                        // 检查 share_copy 字段更新
                        let share_copy_changed = if let Some(new_share_copy) = model.share_copy.as_ref() {
                            existing.share_copy.as_ref() != Some(new_share_copy)
                        } else {
                            false
                        };

                        // 检查 show_season_type 字段更新
                        let show_season_type_changed =
                            if let Some(new_show_season_type) = model.show_season_type.as_ref() {
                                existing.show_season_type != Some(*new_show_season_type)
                            } else {
                                false
                            };

                        // 检查 actors 字段更新
                        let actors_changed = match (&existing.actors, model.actors.as_ref()) {
                            (None, Some(new_actors)) => {
                                // 数据库为空，API有数据，需要更新
                                tracing::info!("检测到actors字段从空值更新为有值(未启用扫描删除): {:?}", new_actors);
                                true
                            }
                            (Some(existing_actors), Some(new_actors)) => {
                                // 两者都有值，比较是否不同
                                let changed = existing_actors != new_actors;
                                if changed {
                                    tracing::info!(
                                        "检测到actors字段值发生变化(未启用扫描删除): 原值={:?}, 新值={:?}",
                                        existing_actors,
                                        new_actors
                                    );
                                }
                                changed
                            }
                            (Some(_), None) => {
                                // 数据库有值，API返回空，保持原值不变
                                tracing::debug!("API未返回actors数据，保持数据库现有值(未启用扫描删除)");
                                false
                            }
                            (None, None) => {
                                // 两者都为空，无需更新
                                false
                            }
                        };

                        if share_copy_changed || show_season_type_changed || actors_changed {
                            needs_update = true;
                            should_recalculate_name = true;

                            if share_copy_changed {
                                info!(
                                    "检测到需要更新share_copy(未启用扫描删除): 视频={}, 原值={:?}, 新值={:?}",
                                    existing.name, existing.share_copy, model.share_copy
                                );
                            }
                            if show_season_type_changed {
                                info!(
                                    "检测到需要更新show_season_type(未启用扫描删除): 视频={}, 原值={:?}, 新值={:?}",
                                    existing.name, existing.show_season_type, model.show_season_type
                                );
                            }
                            if actors_changed {
                                info!(
                                    "检测到需要更新actors(未启用扫描删除): 视频={}, 原值={:?}, 新值={:?}",
                                    existing.name, existing.actors, model.actors
                                );
                            }
                        }

                        if needs_update {
                            // 如果需要重新计算name，并且这是番剧类型（category=1）
                            // 但对于番剧影视类型（show_season_type=2），不重新计算name，保持原有的简洁格式
                            let new_name = if should_recalculate_name && existing.category == 1 {
                                let new_show_season_type = match &model.show_season_type {
                                    Set(opt) => *opt,
                                    _ => existing.show_season_type,
                                };

                                // 如果是番剧影视类型，不重新计算name，保持现有的简洁name
                                if new_show_season_type == Some(2) {
                                    sea_orm::ActiveValue::NotSet // 保持现有name不变
                                } else {
                                    // 对于常规番剧类型，进行重新计算
                                    let title = existing.name.as_str();
                                    let share_copy = match &model.share_copy {
                                        Set(Some(s)) => Some(s.as_str()),
                                        Set(None) => None,
                                        _ => existing.share_copy.as_deref(),
                                    };

                                    let recalculated_name =
                                        recalculate_bangumi_name(title, share_copy, None, new_show_season_type);
                                    info!(
                                        "重新计算常规番剧name(未启用扫描删除): 视频={}, 原name={}, 新name={}",
                                        existing.name, existing.name, recalculated_name
                                    );
                                    Set(recalculated_name)
                                }
                            } else {
                                model.name.clone()
                            };

                            let update_model = video::ActiveModel {
                                id: Unchanged(existing.id),
                                share_copy: model.share_copy.clone(),
                                show_season_type: model.show_season_type.clone(),
                                actors: model.actors.clone(),
                                name: new_name,
                                intro: model.intro.clone(),
                                cover: model.cover.clone(),
                                ..Default::default()
                            };

                            // 详细的数据库更新调试日志
                            tracing::info!(
                                "即将执行数据库更新(未启用扫描删除): 视频={}, actors字段={:?}, share_copy={:?}, show_season_type={:?}",
                                existing.name, update_model.actors, update_model.share_copy, update_model.show_season_type
                            );

                            update_model.save(connection).await?;
                            info!("更新视频 {} 的字段完成(未启用扫描删除)", existing.name);
                        } else {
                            tracing::debug!(
                                "字段无需更新(未启用扫描删除): 视频={}, share_copy={:?}, show_season_type={:?}, actors={:?}",
                                existing.name,
                                existing.share_copy,
                                existing.show_season_type,
                                existing.actors
                            );
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
