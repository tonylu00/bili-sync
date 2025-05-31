use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tracing::{debug, error, info, warn};

use crate::adapter::Args;
use crate::bilibili::{self, BiliClient, CollectionItem, CollectionType};
use crate::config::{CONFIG, Config};
use crate::task::TASK_CONTROLLER;
use crate::workflow::process_video_source;
use crate::initialization;
use bili_sync_entity::entities;

/// 从数据库加载所有视频源的函数
async fn load_video_sources_from_db(
    connection: &DatabaseConnection,
) -> Result<Vec<(Args<'static>, PathBuf)>, Box<dyn std::error::Error + Send + Sync>> {
    let mut video_sources = Vec::new();
    
    // 加载合集源
    let collections = entities::collection::Entity::find()
        .all(connection)
        .await?;
    
    for collection in collections {
        // 创建拥有的CollectionItem来匹配现有的Args结构
        let collection_type = if collection.r#type == 1 { 
            CollectionType::Series 
        } else { 
            CollectionType::Season 
        };
        
        let collection_item = Box::leak(Box::new(CollectionItem {
            mid: collection.m_id.to_string(),
            sid: collection.s_id.to_string(),
            collection_type,
        }));
        
        video_sources.push((
            Args::Collection { 
                collection_item 
            },
            PathBuf::from(collection.path)
        ));
    }
    
    // 加载收藏夹源
    let favorites = entities::favorite::Entity::find()
        .all(connection)
        .await?;
        
    for favorite in favorites {
        let fid = Box::leak(favorite.f_id.to_string().into_boxed_str());
        video_sources.push((
            Args::Favorite { 
                fid 
            },
            PathBuf::from(favorite.path)
        ));
    }
    
    // 加载UP主投稿源
    let submissions = entities::submission::Entity::find()
        .all(connection)
        .await?;
        
    for submission in submissions {
        let upper_id = Box::leak(submission.upper_id.to_string().into_boxed_str());
        video_sources.push((
            Args::Submission { 
                upper_id 
            },
            PathBuf::from(submission.path)
        ));
    }
    
    // 加载稍后观看源
    let watch_later_sources = entities::watch_later::Entity::find()
        .all(connection)
        .await?;
        
    for watch_later in watch_later_sources {
        video_sources.push((
            Args::WatchLater,
            PathBuf::from(watch_later.path)
        ));
    }
    
    // 加载番剧源
    let bangumi_sources = entities::video_source::Entity::find()
        .filter(entities::video_source::Column::Type.eq(1))
        .all(connection)
        .await?;
        
    for bangumi in bangumi_sources {
        let season_id = Box::leak(Box::new(bangumi.season_id));
        let media_id = Box::leak(Box::new(bangumi.media_id));
        let ep_id = Box::leak(Box::new(bangumi.ep_id));
        
        video_sources.push((
            Args::Bangumi { 
                season_id,
                media_id,
                ep_id,
            },
            PathBuf::from(bangumi.path)
        ));
    }
    
    Ok(video_sources)
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

    // 在启动时初始化所有视频源
    if let Err(e) = init_all_sources(&CONFIG, &connection).await {
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

        info!("开始执行本轮视频下载任务..");

        'inner: {
            // 在开始扫描前再次检查是否暂停
            if TASK_CONTROLLER.is_paused() {
                debug!("扫描开始前检测到暂停信号，跳过本轮扫描");
                break 'inner;
            }

            match bili_client.wbi_img().await.map(|wbi_img| wbi_img.into()) {
                Ok(Some(mixin_key)) => bilibili::set_global_mixin_key(mixin_key),
                Ok(_) => {
                    error!("解析 mixin key 失败，等待下一轮执行");
                    break 'inner;
                }
                Err(e) => {
                    let error_msg = format!("{:#}", e);
                    // 检查是否是登录状态过期错误（-101错误码）
                    if error_msg.contains("status code: -101") || error_msg.contains("账号未登录") {
                        warn!("检测到登录状态过期，尝试刷新凭据...");
                        if let Err(refresh_err) = bili_client.check_refresh().await {
                            error!("刷新凭据失败：{:#}，等待下一轮执行", refresh_err);
                            break 'inner;
                        } else {
                            info!("凭据刷新成功，重新尝试获取 mixin key");
                            // 重新尝试获取 mixin key
                            match bili_client.wbi_img().await.map(|wbi_img| wbi_img.into()) {
                                Ok(Some(mixin_key)) => bilibili::set_global_mixin_key(mixin_key),
                                Ok(_) => {
                                    error!("刷新凭据后仍无法解析 mixin key，等待下一轮执行");
                                    break 'inner;
                                }
                                Err(retry_err) => {
                                    error!("刷新凭据后仍无法获取 mixin key：{:#}，等待下一轮执行", retry_err);
                                    break 'inner;
                                }
                            }
                        }
                    } else {
                        error!("获取 mixin key 遇到错误：{:#}，等待下一轮执行", e);
                        break 'inner;
                    }
                }
            };

            // 每轮任务都检查凭据是否需要刷新，而不是只在日期变化时检查
            if let Err(e) = bili_client.check_refresh().await {
                error!("检查刷新 Credential 遇到错误：{:#}，等待下一轮执行", e);
                break 'inner;
            }

            let mut processed_sources = 0;
            let mut sources_with_new_content = 0;
            for (args, path) in &video_sources {
                // 在处理每个视频源前检查是否暂停
                if TASK_CONTROLLER.is_paused() {
                    debug!("在处理视频源时检测到暂停信号，停止当前轮次扫描");
                    break;
                }

                match process_video_source(*args, &bili_client, path, &connection).await {
                    Ok(new_video_count) => {
                        processed_sources += 1;
                        if new_video_count > 0 {
                            sources_with_new_content += 1;
                        }
                    }
                    Err(e) => {
                        error!("处理过程遇到错误：{:#}", e);
                    }
                }
            }

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
                info!("等待期间暂停任务已恢复");
                continue; // 暂停期间不计入等待时间
            }
            
            let sleep_duration = remaining_time.min(check_frequency);
            tokio::time::sleep(tokio::time::Duration::from_secs(sleep_duration)).await;
            remaining_time = remaining_time.saturating_sub(sleep_duration);
            
            // 检查配置是否更新了（通过比较interval值）
            let current_config = crate::config::reload_config();
            if current_config.interval != wait_interval {
                info!("检测到扫描间隔时间配置更新：{} -> {} 秒，等待本轮结束后立即开始下一轮扫描", 
                      wait_interval, current_config.interval);
                break; // 配置更新了，立即开始下一轮
            }
            
            // 显示剩余等待时间（只在较长等待时显示）
            if remaining_time > 0 && remaining_time % 30 == 0 && remaining_time >= 30 {
                debug!("距离下一轮扫描还有 {} 秒", remaining_time);
            }
        }
    }
}
