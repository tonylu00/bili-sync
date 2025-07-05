use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use tracing::{debug, error, info, warn};

use crate::adapter::Args;
use crate::bilibili::{self, BiliClient, CollectionItem, CollectionType};
use crate::config::Config;
use crate::initialization;
use crate::task::TASK_CONTROLLER;
use crate::unified_downloader::UnifiedDownloader;
use crate::workflow::process_video_source;
use bili_sync_entity::entities;

/// 从数据库加载所有视频源的函数
async fn load_video_sources_from_db(
    connection: &DatabaseConnection,
) -> Result<Vec<(Args, PathBuf)>, Box<dyn std::error::Error + Send + Sync>> {
    let mut video_sources = Vec::new();

    // 加载合集源（只加载启用的）
    let collections = entities::collection::Entity::find()
        .filter(entities::collection::Column::Enabled.eq(true))
        .all(connection)
        .await?;

    for collection in collections {
        // 创建拥有的CollectionItem来匹配现有的Args结构
        let collection_type = if collection.r#type == 1 {
            CollectionType::Series
        } else {
            CollectionType::Season
        };

        let collection_item = CollectionItem {
            mid: collection.m_id.to_string(),
            sid: collection.s_id.to_string(),
            collection_type,
        };

        video_sources.push((Args::Collection { collection_item }, PathBuf::from(collection.path)));
    }

    // 加载收藏夹源（只加载启用的）
    let favorites = entities::favorite::Entity::find()
        .filter(entities::favorite::Column::Enabled.eq(true))
        .all(connection)
        .await?;

    for favorite in favorites {
        let fid = favorite.f_id.to_string();
        video_sources.push((Args::Favorite { fid }, PathBuf::from(favorite.path)));
    }

    // 加载UP主投稿源（只加载启用的）
    let submissions = entities::submission::Entity::find()
        .filter(entities::submission::Column::Enabled.eq(true))
        .all(connection)
        .await?;

    for submission in submissions {
        let upper_id = submission.upper_id.to_string();
        video_sources.push((Args::Submission { upper_id }, PathBuf::from(submission.path)));
    }

    // 加载稍后观看源（只加载启用的）
    let watch_later_sources = entities::watch_later::Entity::find()
        .filter(entities::watch_later::Column::Enabled.eq(true))
        .all(connection)
        .await?;

    for watch_later in watch_later_sources {
        video_sources.push((Args::WatchLater, PathBuf::from(watch_later.path)));
    }

    // 加载番剧源（只加载启用的）
    let bangumi_sources = entities::video_source::Entity::find()
        .filter(entities::video_source::Column::Type.eq(1))
        .filter(entities::video_source::Column::Enabled.eq(true))
        .all(connection)
        .await?;

    for bangumi in bangumi_sources {
        video_sources.push((
            Args::Bangumi {
                season_id: bangumi.season_id,
                media_id: bangumi.media_id,
                ep_id: bangumi.ep_id,
            },
            PathBuf::from(bangumi.path),
        ));
    }

    Ok(video_sources)
}

/// 统计所有视频源的数量（包括禁用的）
async fn count_all_video_sources(
    connection: &DatabaseConnection,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let mut total_count = 0;

    // 统计合集源
    let collections_count = entities::collection::Entity::find().count(connection).await?;
    total_count += collections_count as usize;

    // 统计收藏夹源
    let favorites_count = entities::favorite::Entity::find().count(connection).await?;
    total_count += favorites_count as usize;

    // 统计UP主投稿源
    let submissions_count = entities::submission::Entity::find().count(connection).await?;
    total_count += submissions_count as usize;

    // 统计稍后观看源
    let watch_later_count = entities::watch_later::Entity::find().count(connection).await?;
    total_count += watch_later_count as usize;

    // 统计番剧源
    let bangumi_count = entities::video_source::Entity::find()
        .filter(entities::video_source::Column::Type.eq(1))
        .count(connection)
        .await?;
    total_count += bangumi_count as usize;

    Ok(total_count)
}

/// 初始化所有视频源的辅助函数
async fn init_all_sources(
    config: &Config,
    connection: &DatabaseConnection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 初始化番剧源
    if let Err(e) = initialization::init_sources(config, connection).await {
        error!("初始化番剧源失败: {}", e);
        return Err(e.into());
    }

    // 注意：其他源的初始化现在依赖数据库，而不是配置文件
    // 这些初始化函数可能需要修改以适应新的架构

    Ok(())
}

/// 启动周期下载视频的任务
pub async fn video_downloader(connection: Arc<DatabaseConnection>) {
    let bili_client = BiliClient::new(String::new());

    // 在启动时初始化所有视频源 - 使用动态配置而非静态CONFIG
    let config = crate::config::reload_config();
    if let Err(e) = init_all_sources(&config, &connection).await {
        error!("启动时初始化视频源失败: {}", e);
    } else {
        info!("启动时视频源初始化成功");
    }

    loop {
        // ========== 扫描任务阶段 ==========
        // 注意：在此阶段不应该中断任务，即使配置更新了也要等待当前扫描完成

        // 检查是否需要暂停扫描任务
        if TASK_CONTROLLER.is_paused() {
            debug!("定时扫描任务已暂停，等待恢复...");
            TASK_CONTROLLER.wait_if_paused().await;
            info!("定时扫描任务已恢复");
        }

        // 重新加载配置并初始化视频源
        // 注意：由于我们使用Lazy，全局CONFIG并不会自动更新
        // 但我们可以获取最新配置用于本轮任务
        let config = crate::config::reload_config();

        // 重新初始化所有视频源（确保源初始化是幂等的）
        if let Err(e) = init_all_sources(&config, &connection).await {
            error!("重新初始化视频源失败: {}", e);
            // 即使初始化失败，也继续使用现有配置进行下载
        }

        // 从数据库加载视频源，而不是从配置文件
        let video_sources = match load_video_sources_from_db(&connection).await {
            Ok(sources) => sources,
            Err(e) => {
                error!("从数据库加载视频源失败: {}", e);
                continue;
            }
        };

        // 统计总的视频源数量（包括禁用的）
        let total_sources_count = match count_all_video_sources(&connection).await {
            Ok(count) => count,
            Err(e) => {
                warn!("统计视频源总数失败: {}", e);
                0
            }
        };

        let enabled_sources_count = video_sources.len();
        let disabled_sources_count = total_sources_count.saturating_sub(enabled_sources_count);

        if disabled_sources_count > 0 {
            info!(
                "开始执行本轮视频下载任务，共 {} 个视频源（启用: {}，禁用: {}）",
                total_sources_count, enabled_sources_count, disabled_sources_count
            );
        } else {
            info!("开始执行本轮视频下载任务，共 {} 个启用的视频源", enabled_sources_count);
        }

        'inner: {
            // 在开始扫描前再次检查是否暂停
            if TASK_CONTROLLER.is_paused() {
                debug!("扫描开始前检测到暂停信号，跳过本轮扫描");
                break 'inner;
            }

            // 标记扫描开始并重置取消令牌
            TASK_CONTROLLER.set_scanning(true);
            TASK_CONTROLLER.reset_cancellation_token().await;

            match bili_client.wbi_img().await.map(|wbi_img| wbi_img.into()) {
                Ok(Some(mixin_key)) => bilibili::set_global_mixin_key(mixin_key),
                Ok(_) => {
                    error!("解析 mixin key 失败，等待下一轮执行");
                    // 扫描失败，标记扫描结束
                    TASK_CONTROLLER.set_scanning(false);
                    break 'inner;
                }
                Err(e) => {
                    let error_msg = format!("{:#}", e);
                    // 检查是否是登录状态过期错误（-101错误码）
                    if error_msg.contains("status code: -101") || error_msg.contains("账号未登录") {
                        warn!("检测到登录状态过期或未登录，请检查配置文件中的SESSDATA等认证信息");

                        // 发送登录状态过期日志
                        crate::api::handler::add_log_entry(
                            crate::api::handler::LogLevel::Warn,
                            "检测到登录状态过期或未登录，请更新配置文件中的SESSDATA等认证信息".to_string(),
                            Some("bili_sync::task::video_downloader".to_string()),
                        );
                    } else {
                        error!("解析 mixin key 失败: {:#}", e);

                        // 发送一般性错误日志
                        crate::api::handler::add_log_entry(
                            crate::api::handler::LogLevel::Error,
                            format!("解析 mixin key 失败: {:#}", e),
                            Some("bili_sync::task::video_downloader".to_string()),
                        );
                    }

                    // 扫描失败，标记扫描结束
                    TASK_CONTROLLER.set_scanning(false);
                    break 'inner;
                }
            }

            // 创建共享的下载器实例，供所有视频源使用
            let downloader = UnifiedDownloader::new_smart(bili_client.client.clone()).await;

            // 设置下载器引用到TaskController中，以便暂停时能停止下载
            let downloader_arc = std::sync::Arc::new(downloader);
            TASK_CONTROLLER.set_downloader(Some(downloader_arc.clone())).await;

            let mut processed_sources = 0;
            let mut sources_with_new_content = 0;
            for (args, path) in &video_sources {
                // 在处理每个视频源前检查是否暂停
                if TASK_CONTROLLER.is_paused() {
                    debug!("在处理视频源时检测到暂停信号，停止当前轮次扫描");
                    // 重要：暂停时必须重置扫描状态
                    TASK_CONTROLLER.set_scanning(false);
                    break;
                }

                // 获取全局取消令牌，用于下载任务控制
                let cancellation_token = TASK_CONTROLLER.get_cancellation_token().await;

                match process_video_source(
                    args,
                    &bili_client,
                    path,
                    &connection,
                    &downloader_arc,
                    cancellation_token,
                )
                .await
                {
                    Ok(new_video_count) => {
                        processed_sources += 1;
                        if new_video_count > 0 {
                            sources_with_new_content += 1;
                        }
                    }
                    Err(e) => {
                        // 检查是否为风控错误，如果是则停止所有后续扫描
                        if e.downcast_ref::<crate::error::DownloadAbortError>().is_some() {
                            error!("检测到风控，停止所有后续视频源的扫描");
                            break; // 跳出循环，停止处理剩余的视频源
                        }
                        error!("处理过程遇到错误：{:#}", e);
                    }
                }
            }

            // 标记扫描结束
            TASK_CONTROLLER.set_scanning(false);

            if processed_sources == video_sources.len() {
                if sources_with_new_content > 0 {
                    info!(
                        "本轮任务执行完毕，成功扫描 {} 个视频源，其中 {} 个源有新内容",
                        processed_sources, sources_with_new_content
                    );
                } else {
                    info!(
                        "本轮任务执行完毕，成功扫描 {} 个视频源（均无新内容）",
                        processed_sources
                    );
                }
            } else if processed_sources > 0 {
                if sources_with_new_content > 0 {
                    info!(
                        "本轮任务执行完毕，成功扫描 {} 个视频源（其中 {} 个有新内容），{} 个源处理失败",
                        processed_sources,
                        sources_with_new_content,
                        video_sources.len() - processed_sources
                    );
                } else {
                    info!(
                        "本轮任务执行完毕，成功扫描 {} 个视频源（均无新内容），{} 个源处理失败",
                        processed_sources,
                        video_sources.len() - processed_sources
                    );
                }
            } else {
                warn!("本轮任务执行完毕，所有 {} 个视频源均处理失败", video_sources.len());
            }
        }

        // ========== 扫描后处理阶段 ==========
        // 只在未暂停时处理后续任务
        if !TASK_CONTROLLER.is_paused() {
            // 安全时机：扫描任务已完成，处理暂存的删除任务
            if let Err(e) = crate::task::process_delete_tasks(connection.clone()).await {
                error!("处理删除任务队列失败: {:#}", e);
            }

            // 处理暂存的视频删除任务
            if let Err(e) = crate::task::process_video_delete_tasks(connection.clone()).await {
                error!("处理视频删除任务队列失败: {:#}", e);
            }

            // 处理暂存的添加任务
            if let Err(e) = crate::task::process_add_tasks(connection.clone()).await {
                error!("处理添加任务队列失败: {:#}", e);
            }

            // 处理暂存的配置任务
            if let Err(e) = crate::task::process_config_tasks(connection.clone()).await {
                error!("处理配置任务队列失败: {:#}", e);
            }
        } else {
            debug!("任务已暂停，跳过后处理阶段");
        }

        // ========== 等待阶段 ==========
        // 安全时机：扫描任务已完成，可以安全地检测配置更新并决定是否立即开始下一轮
        // 智能等待：支持配置更新的间隔等待
        // 重要：只在扫描任务完成后才检测配置更新，确保不会中断正在进行的扫描
        let wait_interval = config.interval;
        let check_frequency = 5; // 每5秒检查一次配置是否更新
        let mut remaining_time = wait_interval;

        info!("本轮扫描任务已完成，开始等待 {} 秒后进行下一轮扫描", wait_interval);

        while remaining_time > 0 {
            // 检查是否暂停
            if TASK_CONTROLLER.is_paused() {
                debug!("等待期间检测到暂停信号，等待恢复...");
                TASK_CONTROLLER.wait_if_paused().await;

                // 检查是否刚刚恢复，如果是则立即开始新扫描
                if TASK_CONTROLLER.take_just_resumed() {
                    info!("任务恢复，立即开始新一轮扫描");
                    break; // 跳出等待循环，立即开始新扫描
                }

                info!("等待期间暂停任务已恢复，继续等待");
                continue; // 暂停期间不计入等待时间
            }

            let sleep_duration = remaining_time.min(check_frequency);
            tokio::time::sleep(tokio::time::Duration::from_secs(sleep_duration)).await;
            remaining_time = remaining_time.saturating_sub(sleep_duration);

            // 检查是否刚刚恢复，如果是则立即开始新扫描
            if TASK_CONTROLLER.take_just_resumed() {
                info!("检测到任务恢复信号，立即开始新一轮扫描");
                break; // 跳出等待循环，立即开始新扫描
            }

            // 检查配置是否更新了（通过比较interval值）
            let current_config = crate::config::reload_config();
            if current_config.interval != wait_interval {
                info!(
                    "检测到扫描间隔时间配置更新：{} -> {} 秒，等待本轮结束后立即开始下一轮扫描",
                    wait_interval, current_config.interval
                );
                break; // 配置更新了，立即开始下一轮
            }

            // 显示剩余等待时间（只在较长等待时显示）
            if remaining_time > 0 && remaining_time % 30 == 0 && remaining_time >= 30 {
                debug!("距离下一轮扫描还有 {} 秒", remaining_time);
            }
        }
    }
}
