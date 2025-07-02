use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::pin::Pin;

use anyhow::{anyhow, bail, Context, Result};
use bili_sync_entity::*;
use futures::stream::FuturesUnordered;
use futures::{Stream, StreamExt, TryStreamExt};
use sea_orm::entity::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::TransactionTrait;
use tokio::fs;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::adapter::{video_source_from, Args, VideoSource, VideoSourceEnum};
use crate::bilibili::{BestStream, BiliClient, BiliError, Dimension, PageInfo, Video, VideoInfo};
use crate::config::ARGS;
use crate::error::{DownloadAbortError, ExecutionStatus, ProcessPageError};
use crate::task::{DeleteVideoTask, VIDEO_DELETE_TASK_QUEUE};
use crate::unified_downloader::UnifiedDownloader;
use crate::utils::format_arg::{page_format_args, video_format_args};
use crate::utils::model::{
    create_pages, create_videos, filter_unfilled_videos, filter_unhandled_video_pages,
    get_failed_videos_in_current_cycle, update_pages_model, update_videos_model,
};
use crate::utils::nfo::NFO;
use crate::utils::status::{PageStatus, VideoStatus, STATUS_OK};

// 新增：番剧季信息结构体
#[derive(Debug, Clone)]
struct SeasonInfo {
    title: String,
    episodes: Vec<EpisodeInfo>,
}

#[derive(Debug, Clone)]
struct EpisodeInfo {
    ep_id: String,
    cid: i64,
    duration: u32, // 秒
}

/// 创建一个配置了 truncate 辅助函数的 handlebars 实例
///
/// 完整地处理某个视频来源，返回新增的视频数量
pub async fn process_video_source(
    args: &Args,
    bili_client: &BiliClient,
    path: &Path,
    connection: &DatabaseConnection,
    downloader: &UnifiedDownloader,
    token: CancellationToken,
) -> Result<usize> {
    // 记录当前处理的参数和路径
    if let Args::Bangumi {
        season_id,
        media_id: _,
        ep_id: _,
    } = &args
    {
        // 尝试从API获取真实的番剧标题
        let title = if let Some(season_id) = season_id {
            // 如果有season_id，尝试获取番剧标题
            get_season_title_from_api(bili_client, season_id, token.clone())
                .await
                .unwrap_or_else(|| {
                    // API获取失败，回退到路径名
                    path.file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| "未知番剧".to_string())
                })
        } else {
            // 没有season_id，使用路径名
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "未知番剧".to_string())
        };
        info!("处理番剧下载: {}", title);
    }

    // 定义一个辅助函数来处理-101错误并重试
    let retry_with_refresh = |error_msg: String| async move {
        if error_msg.contains("status code: -101") || error_msg.contains("账号未登录") {
            warn!("检测到登录状态过期，尝试刷新凭据...");
            if let Err(refresh_err) = bili_client.check_refresh().await {
                error!("刷新凭据失败：{:#}", refresh_err);
                return Err(refresh_err);
            } else {
                info!("凭据刷新成功，将重试操作");
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("非登录状态错误，无需刷新凭据"))
    };

    // 从参数中获取视频列表的 Model 与视频流
    let (video_source, video_streams) = match video_source_from(args, path, bili_client, connection).await {
        Ok(result) => result,
        Err(e) => {
            let error_msg = format!("{:#}", e);
            if retry_with_refresh(error_msg).await.is_ok() {
                // 刷新成功，重试
                video_source_from(args, path, bili_client, connection).await?
            } else {
                return Err(e);
            }
        }
    };

    // 从视频流中获取新视频的简要信息，写入数据库，并获取新增视频数量
    let new_video_count = match refresh_video_source(&video_source, video_streams, connection, token.clone()).await {
        Ok(count) => count,
        Err(e) => {
            let error_msg = format!("{:#}", e);
            if retry_with_refresh(error_msg).await.is_ok() {
                // 刷新成功，重新获取视频流并重试
                let (_, video_streams) = video_source_from(args, path, bili_client, connection).await?;
                refresh_video_source(&video_source, video_streams, connection, token.clone()).await?
            } else {
                return Err(e);
            }
        }
    };

    // 单独请求视频详情接口，获取视频的详情信息与所有的分页，写入数据库
    if let Err(e) = fetch_video_details(bili_client, &video_source, connection, token.clone()).await {
        // 新增：检查是否为风控导致的下载中止
        if e.downcast_ref::<DownloadAbortError>().is_some() {
            error!("获取视频详情时触发风控，已终止当前视频源的处理，停止所有后续扫描");
            // 风控时应该返回错误，中断整个扫描循环，而不是继续处理下一个视频源
            return Err(e);
        }

        let error_msg = format!("{:#}", e);
        if retry_with_refresh(error_msg).await.is_ok() {
            // 刷新成功，重试
            fetch_video_details(bili_client, &video_source, connection, token.clone()).await?;
        } else {
            return Err(e);
        }
    }

    if ARGS.scan_only {
        warn!("已开启仅扫描模式，跳过视频下载..");
    } else {
        // 从数据库中查找所有未下载的视频与分页，下载并处理
        if let Err(e) =
            download_unprocessed_videos(bili_client, &video_source, connection, downloader, token.clone()).await
        {
            let error_msg = format!("{:#}", e);
            if retry_with_refresh(error_msg).await.is_ok() {
                // 刷新成功，重试（继续使用原有的取消令牌）
                download_unprocessed_videos(bili_client, &video_source, connection, downloader, token.clone()).await?;
            } else {
                return Err(e);
            }
        }

        // 新增：循环内重试失败的视频
        // 在当前扫描循环结束前，对失败的视频进行一次额外的重试机会
        if let Err(e) =
            retry_failed_videos_once(bili_client, &video_source, connection, downloader, token.clone()).await
        {
            warn!("循环内重试失败的视频时出错: {:#}", e);
            // 重试失败不中断主流程，继续执行
        }
    }
    Ok(new_video_count)
}

/// 请求接口，获取视频列表中所有新添加的视频信息，将其写入数据库
pub async fn refresh_video_source<'a>(
    video_source: &VideoSourceEnum,
    video_streams: Pin<Box<dyn Stream<Item = Result<VideoInfo>> + 'a + Send>>,
    connection: &DatabaseConnection,
    token: CancellationToken,
) -> Result<usize> {
    video_source.log_refresh_video_start();
    let latest_row_at = video_source.get_latest_row_at().and_utc();
    let mut max_datetime = latest_row_at;
    let mut error = Ok(());
    let mut video_streams = video_streams
        .take_while(|res| {
            if token.is_cancelled() {
                return futures::future::ready(false);
            }
            match res {
                Err(e) => {
                    error = Err(anyhow!(e.to_string()));
                    futures::future::ready(false)
                }
                Ok(v) => {
                    // 虽然 video_streams 是从新到旧的，但由于此处是分页请求，极端情况下可能发生访问完第一页时插入了两整页视频的情况
                    // 此时获取到的第二页视频比第一页的还要新，因此为了确保正确，理应对每一页的第一个视频进行时间比较
                    // 但在 streams 的抽象下，无法判断具体是在哪里分页的，所以暂且对每个视频都进行比较，应该不会有太大性能损失
                    let release_datetime = v.release_datetime();
                    if release_datetime > &max_datetime {
                        max_datetime = *release_datetime;
                    }
                    futures::future::ready(video_source.should_take(release_datetime, &latest_row_at))
                }
            }
        })
        .filter_map(|res| futures::future::ready(res.ok()))
        .chunks(10);
    let mut count = 0;
    while let Some(videos_info) = video_streams.next().await {
        // 获取插入前的视频数量
        let before_count = get_video_count_for_source(video_source, connection).await?;

        create_videos(videos_info, video_source, connection).await?;

        // 获取插入后的视频数量，计算实际新增数量
        let after_count = get_video_count_for_source(video_source, connection).await?;
        count += after_count - before_count;
    }
    // 如果获取视频分页过程中发生了错误，直接在此处返回，不更新 latest_row_at
    error?;
    if max_datetime != latest_row_at {
        video_source
            .update_latest_row_at(max_datetime.naive_utc())
            .save(connection)
            .await?;
    }
    video_source.log_refresh_video_end(count);
    Ok(count)
}

/// 筛选出所有未获取到全部信息的视频，尝试补充其详细信息
pub async fn fetch_video_details(
    bili_client: &BiliClient,
    video_source: &VideoSourceEnum,
    connection: &DatabaseConnection,
    token: CancellationToken,
) -> Result<()> {
    video_source.log_fetch_video_start();
    let videos_model = filter_unfilled_videos(video_source.filter_expr(), connection).await?;

    // 分离出番剧和普通视频
    let (bangumi_videos, normal_videos): (Vec<_>, Vec<_>) =
        videos_model.into_iter().partition(|v| v.source_type == Some(1));

    // 优化后的番剧信息获取 - 使用数据库缓存和按季分组
    if !bangumi_videos.is_empty() {
        info!("开始处理 {} 个番剧视频", bangumi_videos.len());

        // 按 season_id 分组番剧视频
        let mut videos_by_season: HashMap<String, Vec<video::Model>> = HashMap::new();
        let mut videos_without_season = Vec::new();

        for video in bangumi_videos {
            if let Some(season_id) = &video.season_id {
                videos_by_season.entry(season_id.clone()).or_default().push(video);
            } else {
                videos_without_season.push(video);
            }
        }

        // 处理每个季
        for (season_id, videos) in videos_by_season {
            // 首先尝试获取番剧季标题用于日志显示
            let season_title = get_season_title_from_api(bili_client, &season_id, token.clone()).await;
            let display_name = season_title.as_deref().unwrap_or(&season_id);

            info!(
                "处理番剧季 {} 「{}」的 {} 个视频",
                season_id,
                display_name,
                videos.len()
            );

            // 1. 首先从现有数据库中查找该季已有的分集信息
            let mut existing_episodes =
                get_existing_episodes_for_season(connection, &season_id, bili_client, token.clone()).await?;

            // 2. 检查哪些ep_id还没有信息
            let missing_ep_ids: Vec<String> = videos
                .iter()
                .filter_map(|v| v.ep_id.as_ref())
                .filter(|ep_id| !existing_episodes.contains_key(*ep_id))
                .cloned()
                .collect();

            // 3. 只对缺失的信息发起API请求（每个季只请求一次）
            if !missing_ep_ids.is_empty() {
                info!(
                    "需要从API获取番剧季 {} 「{}」的信息（包含 {} 个新分集）",
                    season_id,
                    display_name,
                    missing_ep_ids.len()
                );

                match get_season_info_from_api(bili_client, &season_id, token.clone()).await {
                    Ok(season_info) => {
                        // 将新获取的信息添加到映射中
                        for episode in season_info.episodes {
                            existing_episodes.insert(episode.ep_id, (episode.cid, episode.duration));
                        }
                        info!("成功获取番剧季 {} 「{}」的完整信息", season_id, season_info.title);
                    }
                    Err(e) => {
                        error!("获取番剧季 {} 「{}」信息失败: {}", season_id, display_name, e);
                        // 即使API失败，已有缓存的分集仍可正常处理
                    }
                }
            } else {
                info!(
                    "番剧季 {} 「{}」的所有分集信息已缓存，无需API请求",
                    season_id, display_name
                );
            }

            // 4. 使用合并后的信息处理所有视频
            for video_model in videos {
                if let Err(e) = process_bangumi_video(video_model, &existing_episodes, connection, video_source).await {
                    error!("处理番剧视频失败: {}", e);
                }
            }
        }

        // 处理没有season_id的番剧视频（回退到原逻辑）
        if !videos_without_season.is_empty() {
            warn!(
                "发现 {} 个缺少season_id的番剧视频，使用原有逻辑处理",
                videos_without_season.len()
            );
            for video_model in videos_without_season {
                let txn = connection.begin().await?;

                let (actual_cid, duration) = if let Some(ep_id) = &video_model.ep_id {
                    match get_bangumi_info_from_api(bili_client, ep_id, token.clone()).await {
                        Some(info) => info,
                        None => {
                            error!("番剧 {} (EP{}) 信息获取失败，将跳过弹幕下载", &video_model.name, ep_id);
                            (-1, 1440)
                        }
                    }
                } else {
                    error!("番剧 {} 缺少EP ID，无法获取详细信息", &video_model.name);
                    (-1, 1440)
                };

                let page_info = PageInfo {
                    cid: actual_cid,
                    page: 1,
                    name: video_model.name.clone(),
                    duration,
                    first_frame: None,
                    dimension: None,
                };

                create_pages(vec![page_info], &video_model, &txn).await?;

                let mut video_active_model: bili_sync_entity::video::ActiveModel = video_model.into();
                video_source.set_relation_id(&mut video_active_model);
                video_active_model.single_page = Set(Some(true));
                video_active_model.tags = Set(Some(serde_json::Value::Array(vec![])));
                video_active_model.save(&txn).await?;
                txn.commit().await?;
            }
        }
    }

    // 处理普通视频 - 使用并发处理优化性能
    if !normal_videos.is_empty() {
        info!("开始并发处理 {} 个普通视频的详情", normal_videos.len());

        // 使用信号量控制并发数
        let current_config = crate::config::reload_config();
        let semaphore = Semaphore::new(current_config.concurrent_limit.video);

        let tasks = normal_videos
            .into_iter()
            .map(|video_model| {
                let semaphore = &semaphore;
                let token = token.clone();
                async move {
                    // 获取许可以控制并发
                    let _permit = tokio::select! {
                        biased;
                        _ = token.cancelled() => return Err(anyhow!("Download cancelled")),
                        permit = semaphore.acquire() => permit.context("acquire semaphore failed")?,
                    };

                    let video = Video::new(bili_client, video_model.bvid.clone());
                    let info: Result<_> = tokio::select! {
                        biased;
                        _ = token.cancelled() => return Err(anyhow!("Download cancelled")),
                        res = async { Ok((video.get_tags().await?, video.get_view_info().await?)) } => res,
                    };
                    match info {
                        Err(e) => {
                            // 新增：检查是否为风控错误
                            let classified_error = crate::error::ErrorClassifier::classify_error(&e);
                            if classified_error.error_type == crate::error::ErrorType::RiskControl {
                                error!(
                                    "获取视频 {} - {} 的详细信息时触发风控: {}",
                                    &video_model.bvid, &video_model.name, classified_error.message
                                );
                                // 返回一个特定的错误来中止整个批处理
                                return Err(anyhow!(DownloadAbortError()));
                            }

                            error!(
                                "获取视频 {} - {} 的详细信息失败，错误为：{:#}",
                                &video_model.bvid, &video_model.name, e
                            );
                            if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                                let mut video_active_model: bili_sync_entity::video::ActiveModel = video_model.into();
                                video_active_model.valid = Set(false);
                                video_active_model.save(connection).await?;
                            }
                        }
                        Ok((tags, mut view_info)) => {
                            let VideoInfo::Detail { pages, .. } = &mut view_info else {
                                unreachable!()
                            };
                            let pages = std::mem::take(pages);
                            let pages_len = pages.len();
                            let txn = connection.begin().await?;
                            // 将分页信息写入数据库
                            create_pages(pages, &video_model, &txn).await?;
                            let mut video_active_model = view_info.into_detail_model(video_model);
                            video_source.set_relation_id(&mut video_active_model);
                            video_active_model.single_page = Set(Some(pages_len == 1));
                            video_active_model.tags = Set(Some(serde_json::to_value(tags)?));
                            video_active_model.save(&txn).await?;
                            txn.commit().await?;
                        }
                    };
                    Ok::<_, anyhow::Error>(())
                }
            })
            .collect::<FuturesUnordered<_>>();

        // 并发执行所有任务
        let mut stream = tasks;
        while let Some(res) = stream.next().await {
            if let Err(e) = res {
                let error_msg = e.to_string();

                // 检查是否是暂停导致的失败，只有在任务暂停时才将 Download cancelled 视为暂停错误
                if error_msg.contains("任务已暂停")
                    || error_msg.contains("停止下载")
                    || error_msg.contains("用户主动暂停任务")
                    || (error_msg.contains("Download cancelled") && crate::task::TASK_CONTROLLER.is_paused())
                {
                    info!("视频详情获取因用户暂停而终止: {}", error_msg);
                    return Err(e); // 直接返回暂停错误，不取消其他任务
                }

                if e.downcast_ref::<DownloadAbortError>().is_some() || error_msg.contains("Download cancelled") {
                    token.cancel();
                    // drain the rest of the tasks
                    while stream.next().await.is_some() {}
                    return Err(e);
                }
                // for other errors, just log and continue
                error!("获取视频详情时发生错误: {:#}", e);
            }
        }
        info!("完成普通视频详情处理");
    }
    video_source.log_fetch_video_end();
    Ok(())
}

/// 下载所有未处理成功的视频
pub async fn download_unprocessed_videos(
    bili_client: &BiliClient,
    video_source: &VideoSourceEnum,
    connection: &DatabaseConnection,
    downloader: &UnifiedDownloader,
    token: CancellationToken,
) -> Result<()> {
    video_source.log_download_video_start();
    let current_config = crate::config::reload_config();
    let semaphore = Semaphore::new(current_config.concurrent_limit.video);
    let unhandled_videos_pages = filter_unhandled_video_pages(video_source.filter_expr(), connection).await?;

    // 只有当有未处理视频时才显示日志
    if !unhandled_videos_pages.is_empty() {
        info!("找到 {} 个未处理完成的视频", unhandled_videos_pages.len());
    }

    let mut assigned_upper = HashSet::new();
    let tasks = unhandled_videos_pages
        .into_iter()
        .map(|(video_model, pages_model)| {
            let should_download_upper = !assigned_upper.contains(&video_model.upper_id);
            assigned_upper.insert(video_model.upper_id);
            debug!("下载视频: {}", video_model.name);
            download_video_pages(
                bili_client,
                video_source,
                video_model,
                pages_model,
                connection,
                &semaphore,
                downloader,
                should_download_upper,
                token.clone(),
            )
        })
        .collect::<FuturesUnordered<_>>();
    let mut download_aborted = false;
    let mut stream = tasks;
    // 使用循环和select来处理任务，以便在检测到取消信号时立即停止
    while let Some(res) = stream.next().await {
        match res {
            Ok(model) => {
                if download_aborted {
                    continue;
                }
                // 任务成功完成，更新数据库
                if let Err(db_err) = update_videos_model(vec![model], connection).await {
                    error!("更新数据库失败: {:#}", db_err);
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // 调试：输出完整的错误信息
                debug!("检查下载错误消息: '{}'", error_msg);
                debug!("完整错误链: {:#}", e);
                debug!("是否包含'任务已暂停': {}", error_msg.contains("任务已暂停"));
                debug!("是否包含'停止下载': {}", error_msg.contains("停止下载"));
                debug!(
                    "是否包含'Download cancelled': {}",
                    error_msg.contains("Download cancelled")
                );

                // 检查是否是暂停导致的失败，只有在任务暂停时才将 Download cancelled 视为暂停错误
                if error_msg.contains("任务已暂停")
                    || error_msg.contains("停止下载")
                    || error_msg.contains("用户主动暂停任务")
                    || (error_msg.contains("Download cancelled") && crate::task::TASK_CONTROLLER.is_paused())
                {
                    info!("下载任务因用户暂停而终止: {}", error_msg);
                    continue; // 跳过暂停相关的错误，不触发风控
                }

                if e.downcast_ref::<DownloadAbortError>().is_some() || error_msg.contains("Download cancelled") {
                    if !download_aborted {
                        debug!("检测到风控或取消信号，开始中止所有下载任务");
                        token.cancel(); // 立即取消所有其他正在运行的任务
                        download_aborted = true;
                    }
                } else {
                    // 检查是否为暂停相关错误
                    let error_msg = e.to_string();
                    if error_msg.contains("用户主动暂停任务") || error_msg.contains("任务已暂停") {
                        info!("下载任务因用户暂停而终止");
                    } else {
                        // 任务返回了非中止的错误
                        error!("下载任务失败: {:#}", e);
                    }
                }
            }
        }
    }

    if download_aborted {
        error!("下载触发风控，已终止所有任务，停止所有后续扫描");

        // 自动重置风控导致的失败任务
        if let Err(reset_err) = auto_reset_risk_control_failures(connection).await {
            error!("自动重置风控失败任务时出错: {:#}", reset_err);
        }

        video_source.log_download_video_end();
        // 风控时返回错误，中断整个扫描循环
        bail!(DownloadAbortError());
    }
    video_source.log_download_video_end();
    Ok(())
}

/// 对当前循环中失败的视频进行一次重试
pub async fn retry_failed_videos_once(
    bili_client: &BiliClient,
    video_source: &VideoSourceEnum,
    connection: &DatabaseConnection,
    downloader: &UnifiedDownloader,
    token: CancellationToken,
) -> Result<()> {
    let failed_videos_pages = get_failed_videos_in_current_cycle(video_source.filter_expr(), connection).await?;

    if failed_videos_pages.is_empty() {
        debug!("当前循环中没有失败的视频需要重试");
        return Ok(());
    }

    info!("开始重试当前循环中的 {} 个失败视频", failed_videos_pages.len());

    let current_config = crate::config::reload_config();
    let semaphore = Semaphore::new(current_config.concurrent_limit.video);
    let mut assigned_upper = HashSet::new();

    let tasks = failed_videos_pages
        .into_iter()
        .map(|(video_model, pages_model)| {
            let should_download_upper = !assigned_upper.contains(&video_model.upper_id);
            assigned_upper.insert(video_model.upper_id);
            debug!("重试视频: {}", video_model.name);
            download_video_pages(
                bili_client,
                video_source,
                video_model,
                pages_model,
                connection,
                &semaphore,
                downloader,
                should_download_upper,
                token.clone(),
            )
        })
        .collect::<FuturesUnordered<_>>();

    let mut download_aborted = false;
    let mut stream = tasks;
    let mut retry_success_count = 0;

    while let Some(res) = stream.next().await {
        match res {
            Ok(model) => {
                if download_aborted {
                    continue;
                }
                retry_success_count += 1;
                if let Err(db_err) = update_videos_model(vec![model], connection).await {
                    error!("重试后更新数据库失败: {:#}", db_err);
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // 检查是否是暂停导致的失败，只有在任务暂停时才将 Download cancelled 视为暂停错误
                if error_msg.contains("任务已暂停")
                    || error_msg.contains("停止下载")
                    || error_msg.contains("用户主动暂停任务")
                    || (error_msg.contains("Download cancelled") && crate::task::TASK_CONTROLLER.is_paused())
                {
                    info!("重试任务因用户暂停而终止: {}", error_msg);
                    continue; // 跳过暂停相关的错误，不触发风控
                }

                if e.downcast_ref::<DownloadAbortError>().is_some() || error_msg.contains("Download cancelled") {
                    if !download_aborted {
                        debug!("重试过程中检测到风控或取消信号，停止重试");
                        token.cancel();
                        download_aborted = true;
                    }
                } else {
                    // 重试失败，但不中断其他重试任务
                    debug!("视频重试失败: {:#}", e);
                }
            }
        }
    }

    if download_aborted {
        warn!("重试过程中触发风控，已停止重试");
        // 不返回错误，避免影响主流程
    } else if retry_success_count > 0 {
        info!("循环内重试完成，成功重试 {} 个视频", retry_success_count);
    } else {
        debug!("循环内重试完成，但没有视频重试成功");
    }

    Ok(())
}

/// 分页下载任务的参数结构体
pub struct DownloadPageArgs<'a> {
    pub should_run: bool,
    pub bili_client: &'a BiliClient,
    pub video_source: &'a VideoSourceEnum,
    pub video_model: &'a video::Model,
    pub pages: Vec<page::Model>,
    pub connection: &'a DatabaseConnection,
    pub downloader: &'a UnifiedDownloader,
    pub base_path: &'a Path,
    #[allow(dead_code)]
    pub token: CancellationToken,
}

#[allow(clippy::too_many_arguments)]
pub async fn download_video_pages(
    bili_client: &BiliClient,
    video_source: &VideoSourceEnum,
    video_model: video::Model,
    pages: Vec<page::Model>,
    connection: &DatabaseConnection,
    semaphore: &Semaphore,
    downloader: &UnifiedDownloader,
    should_download_upper: bool,
    token: CancellationToken,
) -> Result<video::ActiveModel> {
    let _permit = tokio::select! {
        biased;
        _ = token.cancelled() => return Err(anyhow!("Download cancelled")),
        permit = semaphore.acquire() => permit.context("acquire semaphore failed")?,
    };
    let mut status = VideoStatus::from(video_model.download_status);
    let separate_status = status.should_run();

    // 检查是否为番剧
    let is_bangumi = matches!(video_source, VideoSourceEnum::BangumiSource(_));

    // 获取番剧源和季度信息
    let (base_path, season_folder) = if is_bangumi {
        let bangumi_source = match video_source {
            VideoSourceEnum::BangumiSource(source) => source,
            _ => unreachable!(),
        };

        // 番剧直接使用配置的路径，不创建额外的视频标题文件夹
        let base_path = bangumi_source.path();

        // 目录创建策略：
        // 1. 如果启用了下载所有季度 -> 创建季度子目录
        // 2. 如果有选中的季度（且不为空） -> 创建季度子目录
        // 3. 如果没有选择但有season_id -> 创建季度子目录（单季度番剧的情况）
        let should_create_season_folder = bangumi_source.download_all_seasons
            || (bangumi_source
                .selected_seasons
                .as_ref()
                .map(|s| !s.is_empty())
                .unwrap_or(false))
            || video_model.season_id.is_some(); // 单季度番剧：如果有season_id就创建目录

        if should_create_season_folder && video_model.season_id.is_some() {
            let season_id = video_model
                .season_id
                .as_ref()
                .context("season_id should not be None when downloading multiple seasons")?;

            // 从API获取季度标题
            let season_title = match get_season_title_from_api(bili_client, season_id, token.clone()).await {
                Some(title) => title,
                None => {
                    // API请求失败，使用安全的回退策略
                    error!("无法获取season_id={}的标题，跳过创建季度文件夹", season_id);
                    return Err(anyhow::anyhow!(
                        "无法获取番剧季度信息 (season_id: {})，请检查网络连接或番剧是否存在",
                        season_id
                    ));
                }
            };

            (base_path.join(&season_title), Some(season_title))
        } else {
            // 不启用下载所有季度且没有选中特定季度时，直接使用配置路径
            (base_path.to_path_buf(), None)
        }
    } else {
        // 非番剧使用原来的逻辑，但对合集进行特殊处理
        let path = if let VideoSourceEnum::Collection(collection_source) = video_source {
            // 合集的特殊处理
            let config = crate::config::reload_config();
            match config.collection_folder_mode.as_ref() {
                "unified" => {
                    // 统一模式：所有视频放在以合集名称命名的同一个文件夹下
                    video_source.path().join(&collection_source.name)
                }
                _ => {
                    // 分离模式（默认）：每个视频有自己的文件夹
                    let base_folder_name = crate::config::with_config(|bundle| {
                        bundle.render_video_template(&video_format_args(&video_model))
                    })
                    .map_err(|e| anyhow::anyhow!("模板渲染失败: {}", e))?;

                    // **智能判断：根据模板内容决定是否需要去重**
                    let video_template =
                        crate::config::with_config(|bundle| bundle.config.video_name.as_ref().to_string());
                    let needs_deduplication = video_template.contains("title")
                        || (video_template.contains("name") && !video_template.contains("upper_name"));

                    if needs_deduplication {
                        // 智能去重：检查文件夹名是否已存在，如果存在则追加唯一标识符
                        let unique_folder_name = generate_unique_folder_name(
                            video_source.path(),
                            &base_folder_name,
                            &video_model.bvid,
                            &video_model.pubtime.format("%Y-%m-%d").to_string(),
                        );
                        video_source.path().join(&unique_folder_name)
                    } else {
                        // 不使用去重，允许多个视频共享同一文件夹
                        video_source.path().join(&base_folder_name)
                    }
                }
            }
        } else {
            // 其他类型的视频源使用原来的逻辑
            let base_folder_name =
                crate::config::with_config(|bundle| bundle.render_video_template(&video_format_args(&video_model)))
                    .map_err(|e| anyhow::anyhow!("模板渲染失败: {}", e))?;

            // **智能判断：根据模板内容决定是否需要去重**
            let video_template = crate::config::with_config(|bundle| bundle.config.video_name.as_ref().to_string());
            let needs_deduplication = video_template.contains("title")
                || (video_template.contains("name") && !video_template.contains("upper_name"));

            if needs_deduplication {
                // 智能去重：检查文件夹名是否已存在，如果存在则追加唯一标识符
                let unique_folder_name = generate_unique_folder_name(
                    video_source.path(),
                    &base_folder_name,
                    &video_model.bvid,
                    &video_model.pubtime.format("%Y-%m-%d").to_string(),
                );
                video_source.path().join(&unique_folder_name)
            } else {
                // 不使用去重，允许多个视频共享同一文件夹
                video_source.path().join(&base_folder_name)
            }
        };
        (path, None)
    };

    // 确保季度文件夹存在
    if let Some(season_folder_name) = &season_folder {
        let season_path = video_source.path().join(season_folder_name);
        if !season_path.exists() {
            fs::create_dir_all(&season_path).await?;
            info!("创建季度文件夹: {}", season_path.display());
        }
    }

    let upper_id = video_model.upper_id.to_string();
    let current_config = crate::config::reload_config();
    let base_upper_path = &current_config
        .upper_path
        .join(upper_id.chars().next().context("upper_id is empty")?.to_string())
        .join(upper_id);
    let is_single_page = video_model.single_page.context("single_page is null")?;

    // 为多P视频生成基于视频名称的文件名
    let video_base_name = if !is_single_page {
        // 使用video_name模板渲染视频名称
        crate::config::with_config(|bundle| bundle.render_video_template(&video_format_args(&video_model)))
            .map_err(|e| anyhow::anyhow!("模板渲染失败: {}", e))?
    } else {
        String::new() // 单P视频不需要这些文件
    };

    // 对于单页视频，page 的下载已经足够
    // 对于多页视频，page 下载仅包含了分集内容，需要额外补上视频的 poster 的 tvshow.nfo
    // 使用 tokio::join! 替代装箱的 Future，零分配并行执行

    // 首先检查是否取消
    if token.is_cancelled() {
        return Err(anyhow!("Download cancelled"));
    }

    let (res_1, res_2, res_3, res_4, res_5) = tokio::join!(
        // 下载视频封面
        fetch_video_poster(
            separate_status[0] && !is_single_page,
            &video_model,
            downloader,
            base_path.join(format!("{}-poster.jpg", video_base_name)),
            base_path.join(format!("{}-fanart.jpg", video_base_name)),
            token.clone(),
        ),
        // 生成视频信息的 nfo
        generate_video_nfo(
            separate_status[1] && !is_single_page,
            &video_model,
            base_path.join(format!("{}.nfo", video_base_name)),
        ),
        // 下载 Up 主头像（番剧跳过，因为番剧没有UP主信息）
        fetch_upper_face(
            separate_status[2] && should_download_upper && !is_bangumi,
            &video_model,
            downloader,
            base_upper_path.join("folder.jpg"),
            token.clone(),
        ),
        // 生成 Up 主信息的 nfo（番剧跳过，因为番剧没有UP主信息）
        generate_upper_nfo(
            separate_status[3] && should_download_upper && !is_bangumi,
            &video_model,
            base_upper_path.join("person.nfo"),
        ),
        // 分发并执行分 P 下载的任务
        dispatch_download_page(
            DownloadPageArgs {
                should_run: separate_status[4],
                bili_client,
                video_source,
                video_model: &video_model,
                pages,
                connection,
                downloader,
                base_path: &base_path,
                token: token.clone(),
            },
            token.clone()
        )
    );

    let results = [res_1, res_2, res_3, res_4, res_5]
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    status.update_status(&results);

    // 处理87007错误检测和自动删除（在for_each之前）
    let mut has_87007_error = false;
    for (res, task_name) in results.iter().take(4).zip(["封面", "详情", "作者头像", "作者详情"]) {
        if let ExecutionStatus::Ignored(e) = res {
            let error_msg = e.to_string();
            if error_msg.contains("status code: 87007") {
                warn!(
                    "检测到充电专享视频「{}」{}，将自动删除该视频以避免重复尝试: {:#}",
                    &video_model.name, task_name, e
                );
                has_87007_error = true;
                break; // 只需要检测一次即可
            }
        }
    }

    // 如果检测到87007错误，创建自动删除任务
    if has_87007_error {
        let delete_task = DeleteVideoTask {
            video_id: video_model.id,
            task_id: format!("auto_delete_87007_{}", video_model.id),
        };

        if let Err(delete_err) = VIDEO_DELETE_TASK_QUEUE.enqueue_task(delete_task, connection).await {
            error!(
                "无法创建充电专享视频「{}」的自动删除任务: {:#}",
                &video_model.name, delete_err
            );
        } else {
            info!("已为充电专享视频「{}」创建自动删除任务", &video_model.name);
        }
    }

    results
        .iter()
        .take(4)
        .zip(["封面", "详情", "作者头像", "作者详情"])
        .for_each(|(res, task_name)| match res {
            ExecutionStatus::Skipped => debug!("处理视频「{}」{}已成功过，跳过", &video_model.name, task_name),
            ExecutionStatus::Succeeded => debug!("处理视频「{}」{}成功", &video_model.name, task_name),
            ExecutionStatus::Ignored(e) => {
                let error_msg = e.to_string();
                if !error_msg.contains("status code: 87007") {
                    info!(
                        "处理视频「{}」{}出现常见错误，已忽略: {:#}",
                        &video_model.name, task_name, e
                    );
                }
            }
            ExecutionStatus::ClassifiedFailed(classified_error) => {
                // 根据错误分类进行不同级别的日志记录
                match classified_error.error_type {
                    crate::error::ErrorType::NotFound => {
                        debug!(
                            "处理视频「{}」{}失败({}): {}",
                            &video_model.name, task_name, classified_error.error_type, classified_error.message
                        );
                    }
                    crate::error::ErrorType::Permission => {
                        // 对于权限错误（包括充电专享视频），使用info级别记录
                        info!(
                            "跳过视频「{}」{}: {}",
                            &video_model.name, task_name, classified_error.message
                        );
                    }
                    crate::error::ErrorType::Network
                    | crate::error::ErrorType::Timeout
                    | crate::error::ErrorType::RateLimit => {
                        warn!(
                            "处理视频「{}」{}失败({}): {}{}",
                            &video_model.name,
                            task_name,
                            classified_error.error_type,
                            classified_error.message,
                            if classified_error.should_retry {
                                " (可重试)"
                            } else {
                                ""
                            }
                        );
                    }
                    crate::error::ErrorType::RiskControl => {
                        error!(
                            "处理视频「{}」{}触发风控: {}",
                            &video_model.name, task_name, classified_error.message
                        );
                    }
                    _ => {
                        // 检查是否为暂停相关错误
                        if classified_error.message.contains("用户主动暂停任务")
                            || classified_error.message.contains("任务已暂停")
                        {
                            info!("处理视频「{}」{}因用户暂停而终止", &video_model.name, task_name);
                        } else {
                            error!(
                                "处理视频「{}」{}失败({}): {}",
                                &video_model.name, task_name, classified_error.error_type, classified_error.message
                            );
                        }
                    }
                }
            }
            ExecutionStatus::Failed(e) | ExecutionStatus::FixedFailed(_, e) => {
                // 兼容旧的错误处理方式
                if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                    debug!("处理视频「{}」{}失败(404): {:#}", &video_model.name, task_name, e);
                } else {
                    // 检查是否为暂停相关错误
                    let error_msg = e.to_string();
                    if error_msg.contains("用户主动暂停任务") || error_msg.contains("任务已暂停") {
                        info!("处理视频「{}」{}因用户暂停而终止", &video_model.name, task_name);
                    } else {
                        error!("处理视频「{}」{}失败: {:#}", &video_model.name, task_name, e);
                    }
                }
            }
        });
    if let ExecutionStatus::Failed(e) = results.into_iter().nth(4).context("page download result not found")? {
        if e.downcast_ref::<DownloadAbortError>().is_some() {
            return Err(e);
        }
    }
    let mut video_active_model: video::ActiveModel = video_model.into();
    video_active_model.download_status = Set(status.into());
    video_active_model.path = Set(base_path.to_string_lossy().to_string());
    Ok(video_active_model)
}

/// 分发并执行分页下载任务，当且仅当所有分页成功下载或达到最大重试次数时返回 Ok，否则根据失败原因返回对应的错误
pub async fn dispatch_download_page(args: DownloadPageArgs<'_>, token: CancellationToken) -> Result<ExecutionStatus> {
    if !args.should_run {
        return Ok(ExecutionStatus::Skipped);
    }

    let current_config = crate::config::reload_config();
    let child_semaphore = Semaphore::new(current_config.concurrent_limit.page);
    let tasks = args
        .pages
        .into_iter()
        .map(|page_model| {
            download_page(
                args.bili_client,
                args.video_source,
                args.video_model,
                page_model,
                args.connection,
                &child_semaphore,
                args.downloader,
                args.base_path,
                token.clone(),
            )
        })
        .collect::<FuturesUnordered<_>>();
    let (mut download_aborted, mut target_status) = (false, STATUS_OK);
    let mut stream = tasks;
    while let Some(res) = stream.next().await {
        match res {
            Ok(model) => {
                if download_aborted {
                    continue;
                }
                // 该视频的所有分页的下载状态都会在此返回，需要根据这些状态确认视频层"分 P 下载"子任务的状态
                // 在过去的实现中，此处仅仅根据 page_download_status 的最高标志位来判断，如果最高标志位是 true 则认为完成
                // 这样会导致即使分页中有失败到 MAX_RETRY 的情况，视频层的分 P 下载状态也会被认为是 Succeeded，不够准确
                // 新版本实现会将此处取值为所有子任务状态的最小值，这样只有所有分页的子任务全部成功时才会认为视频层的分 P 下载状态是 Succeeded
                let page_download_status = model.download_status.try_as_ref().expect("download_status must be set");
                let separate_status: [u32; 5] = PageStatus::from(*page_download_status).into();
                for status in separate_status {
                    target_status = target_status.min(status);
                }
                update_pages_model(vec![model], args.connection).await?;
            }
            Err(e) => {
                let error_msg = e.to_string();

                // 检查是否是暂停导致的失败，只有在任务暂停时才将 Download cancelled 视为暂停错误
                if error_msg.contains("任务已暂停")
                    || error_msg.contains("停止下载")
                    || error_msg.contains("用户主动暂停任务")
                    || (error_msg.contains("Download cancelled") && crate::task::TASK_CONTROLLER.is_paused())
                {
                    info!("分页下载任务因用户暂停而终止: {}", error_msg);
                    continue; // 跳过暂停相关的错误，不触发风控
                }

                if e.downcast_ref::<DownloadAbortError>().is_some() || error_msg.contains("Download cancelled") {
                    if !download_aborted {
                        token.cancel();
                        download_aborted = true;
                    }
                } else {
                    // 检查是否为暂停相关错误
                    let error_msg = e.to_string();
                    if error_msg.contains("用户主动暂停任务") || error_msg.contains("任务已暂停") {
                        info!("分页下载因用户暂停而终止");
                    } else {
                        error!("下载分页子任务失败: {:#}", e);
                    }
                }
            }
        }
    }

    if download_aborted {
        error!(
            "下载视频「{}」的分页时触发风控，将异常向上传递..",
            &args.video_model.name
        );
        bail!(DownloadAbortError());
    }
    if target_status != STATUS_OK {
        return Ok(ExecutionStatus::FixedFailed(target_status, ProcessPageError().into()));
    }
    Ok(ExecutionStatus::Succeeded)
}

/// 下载某个分页，未发生风控且正常运行时返回 Ok(Page::ActiveModel)，其中 status 字段存储了新的下载状态，发生风控时返回 DownloadAbortError
#[allow(clippy::too_many_arguments)]
pub async fn download_page(
    bili_client: &BiliClient,
    video_source: &VideoSourceEnum,
    video_model: &video::Model,
    page_model: page::Model,
    connection: &DatabaseConnection,
    semaphore: &Semaphore,
    downloader: &UnifiedDownloader,
    base_path: &Path,
    token: CancellationToken,
) -> Result<page::ActiveModel> {
    let _permit = tokio::select! {
        biased;
        _ = token.cancelled() => return Err(anyhow!("Download cancelled")),
        permit = semaphore.acquire() => permit.context("acquire semaphore failed")?,
    };
    let mut status = PageStatus::from(page_model.download_status);
    let separate_status = status.should_run();
    let is_single_page = video_model.single_page.context("single_page is null")?;

    // 检查是否为番剧
    let is_bangumi = match video_model.source_type {
        Some(1) => true, // source_type = 1 表示为番剧
        _ => false,
    };

    // 根据视频源类型选择不同的模板渲染方式
    let base_name = if let VideoSourceEnum::Collection(collection_source) = video_source {
        // 合集视频的特殊处理
        let config = crate::config::reload_config();
        if config.collection_folder_mode.as_ref() == "unified" {
            // 统一模式：使用S01E01格式命名
            match get_collection_video_episode_number(connection, collection_source.id, &video_model.bvid).await {
                Ok(episode_number) => {
                    let clean_name = crate::utils::filenamify::filenamify(&video_model.name);
                    format!("S01E{:02} - {}", episode_number, clean_name)
                }
                Err(_) => {
                    // 如果获取序号失败，使用默认命名
                    crate::config::with_config(|bundle| {
                        bundle.render_page_template(&page_format_args(video_model, &page_model))
                    })
                    .map_err(|e| anyhow::anyhow!("模板渲染失败: {}", e))?
                }
            }
        } else {
            // 分离模式：使用原有逻辑
            crate::config::with_config(|bundle| {
                bundle.render_page_template(&page_format_args(video_model, &page_model))
            })
            .map_err(|e| anyhow::anyhow!("模板渲染失败: {}", e))?
        }
    } else if is_bangumi {
        // 番剧使用专用的模板方法
        if let VideoSourceEnum::BangumiSource(bangumi_source) = video_source {
            bangumi_source
                .render_page_name(video_model, &page_model, connection)
                .await?
        } else {
            // 如果类型不匹配，使用最新配置手动渲染
            crate::config::with_config(|bundle| {
                bundle.render_page_template(&page_format_args(video_model, &page_model))
            })
            .map_err(|e| anyhow::anyhow!("模板渲染失败: {}", e))?
        }
    } else if !is_single_page {
        // 对于多P视频（非番剧），使用最新配置中的multi_page_name模板
        let page_args = page_format_args(video_model, &page_model);
        match crate::config::with_config(|bundle| bundle.render_multi_page_template(&page_args)) {
            Ok(rendered) => rendered,
            Err(_) => {
                // 如果渲染失败，使用默认格式
                let season_number = 1;
                let episode_number = page_model.pid;
                format!("S{:02}E{:02}-{:02}", season_number, episode_number, episode_number)
            }
        }
    } else {
        // 单P视频使用最新配置的page_name模板
        crate::config::with_config(|bundle| bundle.render_page_template(&page_format_args(video_model, &page_model)))
            .map_err(|e| anyhow::anyhow!("模板渲染失败: {}", e))?
    };

    let (poster_path, video_path, nfo_path, danmaku_path, fanart_path, subtitle_path) = if is_single_page {
        (
            base_path.join(format!("{}-poster.jpg", &base_name)),
            base_path.join(format!("{}.mp4", &base_name)),
            base_path.join(format!("{}.nfo", &base_name)),
            base_path.join(format!("{}.zh-CN.default.ass", &base_name)),
            Some(base_path.join(format!("{}-fanart.jpg", &base_name))),
            base_path.join(format!("{}.srt", &base_name)),
        )
    } else if is_bangumi {
        // 番剧直接使用基础路径，不创建子文件夹结构
        (
            base_path.join(format!("{}-poster.jpg", &base_name)),
            base_path.join(format!("{}.mp4", &base_name)),
            base_path.join(format!("{}.nfo", &base_name)),
            base_path.join(format!("{}.zh-CN.default.ass", &base_name)),
            None,
            base_path.join(format!("{}.srt", &base_name)),
        )
    } else {
        // 非番剧的多P视频直接使用基础路径，不创建子文件夹
        (
            base_path.join(format!("{}-poster.jpg", &base_name)),
            base_path.join(format!("{}.mp4", &base_name)),
            base_path.join(format!("{}.nfo", &base_name)),
            base_path.join(format!("{}.zh-CN.default.ass", &base_name)),
            // 多P视频的每个分页都应该有自己的fanart
            Some(base_path.join(format!("{}-fanart.jpg", &base_name))),
            base_path.join(format!("{}.srt", &base_name)),
        )
    };
    let dimension = match (page_model.width, page_model.height) {
        (Some(width), Some(height)) => Some(Dimension {
            width,
            height,
            rotate: 0,
        }),
        _ => None,
    };
    let page_info = PageInfo {
        cid: page_model.cid,
        duration: page_model.duration,
        dimension,
        ..Default::default()
    };
    // 使用 tokio::join! 替代装箱的 Future，零分配并行执行
    let (res_1, res_2, res_3, res_4, res_5) = tokio::join!(
        fetch_page_poster(
            separate_status[0],
            video_model,
            &page_model,
            downloader,
            poster_path,
            fanart_path,
            token.clone(),
        ),
        fetch_page_video(
            separate_status[1],
            bili_client,
            video_model,
            downloader,
            &page_info,
            &video_path,
            token.clone(),
        ),
        generate_page_nfo(separate_status[2], video_model, &page_model, nfo_path,),
        fetch_page_danmaku(
            separate_status[3],
            bili_client,
            video_model,
            &page_info,
            danmaku_path,
            token.clone(),
        ),
        fetch_page_subtitle(
            separate_status[4],
            bili_client,
            video_model,
            &page_info,
            &subtitle_path,
            token.clone(),
        )
    );

    let results = [res_1, res_2, res_3, res_4, res_5]
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    status.update_status(&results);

    // 处理87007错误检测和自动删除（在for_each之前）
    let mut has_87007_error = false;
    for (res, task_name) in results.iter().zip(["封面", "视频", "详情", "弹幕", "字幕"]) {
        if let ExecutionStatus::Ignored(e) = res {
            let error_msg = e.to_string();
            if error_msg.contains("status code: 87007") {
                warn!(
                    "检测到充电专享视频「{}」第 {} 页{}，将自动删除该视频以避免重复尝试: {:#}",
                    &video_model.name, page_model.pid, task_name, e
                );
                has_87007_error = true;
                break; // 只需要检测一次即可
            }
        }
    }

    // 如果检测到87007错误，创建自动删除任务
    if has_87007_error {
        let delete_task = DeleteVideoTask {
            video_id: video_model.id,
            task_id: format!("auto_delete_87007_page_{}", video_model.id),
        };

        if let Err(delete_err) = VIDEO_DELETE_TASK_QUEUE.enqueue_task(delete_task, connection).await {
            error!(
                "无法创建充电专享视频「{}」的自动删除任务: {:#}",
                &video_model.name, delete_err
            );
        } else {
            info!("已为充电专享视频「{}」创建自动删除任务", &video_model.name);
        }
    }

    results
        .iter()
        .zip(["封面", "视频", "详情", "弹幕", "字幕"])
        .for_each(|(res, task_name)| match res {
            ExecutionStatus::Skipped => debug!(
                "处理视频「{}」第 {} 页{}已成功过，跳过",
                &video_model.name, page_model.pid, task_name
            ),
            ExecutionStatus::Succeeded => debug!(
                "处理视频「{}」第 {} 页{}成功",
                &video_model.name, page_model.pid, task_name
            ),
            ExecutionStatus::Ignored(e) => {
                let error_msg = e.to_string();
                if !error_msg.contains("status code: 87007") {
                    info!(
                        "处理视频「{}」第 {} 页{}出现常见错误，已忽略: {:#}",
                        &video_model.name, page_model.pid, task_name, e
                    );
                }
            }
            ExecutionStatus::ClassifiedFailed(classified_error) => {
                // 根据错误分类进行不同级别的日志记录
                match classified_error.error_type {
                    crate::error::ErrorType::NotFound => {
                        debug!(
                            "处理视频「{}」第 {} 页{}失败({}): {}",
                            &video_model.name,
                            page_model.pid,
                            task_name,
                            classified_error.error_type,
                            classified_error.message
                        );
                    }
                    crate::error::ErrorType::Permission => {
                        // 对于权限错误（包括充电专享视频），使用info级别记录
                        info!(
                            "跳过视频「{}」第 {} 页{}: {}",
                            &video_model.name, page_model.pid, task_name, classified_error.message
                        );
                    }
                    crate::error::ErrorType::Network
                    | crate::error::ErrorType::Timeout
                    | crate::error::ErrorType::RateLimit => {
                        warn!(
                            "处理视频「{}」第 {} 页{}失败({}): {}{}",
                            &video_model.name,
                            page_model.pid,
                            task_name,
                            classified_error.error_type,
                            classified_error.message,
                            if classified_error.should_retry {
                                " (可重试)"
                            } else {
                                ""
                            }
                        );
                    }
                    crate::error::ErrorType::RiskControl => {
                        error!(
                            "处理视频「{}」第 {} 页{}触发风控: {}",
                            &video_model.name, page_model.pid, task_name, classified_error.message
                        );
                    }
                    _ => {
                        // 检查是否为暂停相关错误
                        if classified_error.message.contains("用户主动暂停任务")
                            || classified_error.message.contains("任务已暂停")
                        {
                            info!(
                                "处理视频「{}」第 {} 页{}因用户暂停而终止",
                                &video_model.name, page_model.pid, task_name
                            );
                        } else {
                            error!(
                                "处理视频「{}」第 {} 页{}失败({}): {}",
                                &video_model.name,
                                page_model.pid,
                                task_name,
                                classified_error.error_type,
                                classified_error.message
                            );
                        }
                    }
                }
            }
            ExecutionStatus::Failed(e) | ExecutionStatus::FixedFailed(_, e) => {
                // 兼容旧的错误处理方式
                if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                    debug!(
                        "处理视频「{}」第 {} 页{}失败(404): {:#}",
                        &video_model.name, page_model.pid, task_name, e
                    );
                } else {
                    // 检查是否为暂停相关错误
                    let error_msg = e.to_string();
                    if error_msg.contains("用户主动暂停任务") || error_msg.contains("任务已暂停") {
                        info!(
                            "处理视频「{}」第 {} 页{}因用户暂停而终止",
                            &video_model.name, page_model.pid, task_name
                        );
                    } else {
                        error!(
                            "处理视频「{}」第 {} 页{}失败: {:#}",
                            &video_model.name, page_model.pid, task_name, e
                        );
                    }
                }
            }
        });
    // 检查下载视频时是否触发风控
    match results.into_iter().nth(1).context("video download result not found")? {
        ExecutionStatus::Failed(e) => {
            if let Ok(BiliError::RiskControlOccurred) = e.downcast::<BiliError>() {
                bail!(DownloadAbortError());
            }
        }
        ExecutionStatus::ClassifiedFailed(ref classified_error) => {
            if classified_error.error_type == crate::error::ErrorType::RiskControl {
                bail!(DownloadAbortError());
            }
        }
        _ => {}
    }
    let mut page_active_model: page::ActiveModel = page_model.into();
    page_active_model.download_status = Set(status.into());
    page_active_model.path = Set(Some(video_path.to_string_lossy().to_string()));
    Ok(page_active_model)
}

pub async fn fetch_page_poster(
    should_run: bool,
    video_model: &video::Model,
    page_model: &page::Model,
    downloader: &UnifiedDownloader,
    poster_path: PathBuf,
    fanart_path: Option<PathBuf>,
    token: CancellationToken,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let single_page = video_model.single_page.context("single_page is null")?;
    let url = if single_page {
        // 单页视频直接用视频的封面
        video_model.cover.as_str()
    } else {
        // 多页视频，如果单页没有封面，就使用视频的封面
        match &page_model.image {
            Some(url) => url.as_str(),
            None => video_model.cover.as_str(),
        }
    };
    let urls = vec![url];
    tokio::select! {
        biased;
        _ = token.cancelled() => return Ok(ExecutionStatus::Skipped),
        res = downloader.fetch_with_fallback(&urls, &poster_path) => res,
    }?;
    if let Some(fanart_path) = fanart_path {
        fs::copy(&poster_path, &fanart_path).await?;
    }
    Ok(ExecutionStatus::Succeeded)
}

/// 下载单个流文件并返回文件大小（使用UnifiedDownloader智能选择下载方式）
async fn download_stream(downloader: &UnifiedDownloader, urls: &[&str], path: &Path) -> Result<u64> {
    // 直接使用UnifiedDownloader，它会智能选择aria2或原生下载器
    // aria2本身就支持多线程，原生下载器作为备选方案使用单线程
    let download_result = downloader.fetch_with_fallback(urls, path).await;

    match download_result {
        Ok(_) => {
            // 获取文件大小
            Ok(tokio::fs::metadata(path)
                .await
                .map(|metadata| metadata.len())
                .unwrap_or(0))
        }
        Err(e) => {
            let error_msg = e.to_string();
            // 检查是否为暂停相关错误
            if error_msg.contains("用户主动暂停任务") || error_msg.contains("任务已暂停") {
                info!("下载因用户暂停而终止");
            } else if let Some(BiliError::RequestFailed(-404, _)) = e.downcast_ref::<BiliError>() {
                // 对于404错误，降级为debug日志
                debug!("下载失败(404): {:#}", e);
            } else {
                let error_msg = e.to_string();
                // 检查是否是暂停导致的连接错误
                if (error_msg.contains("tcp connect error") && error_msg.contains("由于目标计算机积极拒绝"))
                    && crate::task::TASK_CONTROLLER.is_paused()
                {
                    info!("下载因用户暂停导致的连接失败: {:#}", e);
                } else {
                    error!("下载失败: {:#}", e);
                }
            }
            Err(e)
        }
    }
}

pub async fn fetch_page_video(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    downloader: &UnifiedDownloader,
    page_info: &PageInfo,
    page_path: &Path,
    token: CancellationToken,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }

    let bili_video = Video::new(bili_client, video_model.bvid.clone());

    // 获取视频流信息 - 根据视频类型选择不同的API
    let mut streams = tokio::select! {
        biased;
        _ = token.cancelled() => return Err(anyhow!("Download cancelled")),
        res = async {
            // 检查是否为番剧视频
            if video_model.source_type == Some(1) && video_model.ep_id.is_some() {
                // 使用番剧专用API
                let ep_id = video_model.ep_id.as_ref().unwrap();
                debug!("使用番剧专用API获取播放地址: ep_id={}", ep_id);
                bili_video.get_bangumi_page_analyzer(page_info, ep_id).await
            } else {
                // 使用普通视频API
                bili_video.get_page_analyzer(page_info).await
            }
        } => res
    }?;

    // 创建保存目录
    if let Some(parent) = page_path.parent() {
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    // UnifiedDownloader会自动选择最佳下载方式

    // 获取用户配置的筛选选项
    let config = crate::config::reload_config();
    let filter_option = &config.filter_option;

    // 记录开始时间
    let start_time = std::time::Instant::now();

    // 根据流类型进行不同处理
    let total_bytes = match streams.best_stream(filter_option)? {
        BestStream::Mixed(mix_stream) => {
            let urls = mix_stream.urls();
            download_stream(downloader, &urls, page_path).await?
        }
        BestStream::VideoAudio {
            video: video_stream,
            audio: None,
        } => {
            let urls = video_stream.urls();
            download_stream(downloader, &urls, page_path).await?
        }
        BestStream::VideoAudio {
            video: video_stream,
            audio: Some(audio_stream),
        } => {
            let (tmp_video_path, tmp_audio_path) = (
                page_path.with_extension("tmp_video"),
                page_path.with_extension("tmp_audio"),
            );

            let video_urls = video_stream.urls();
            let video_size = download_stream(downloader, &video_urls, &tmp_video_path)
                .await
                .map_err(|e| {
                    let error_msg = e.to_string();
                    if error_msg.contains("用户主动暂停任务") || error_msg.contains("任务已暂停") {
                        info!("视频流下载因用户暂停而终止");
                    } else {
                        // 检查是否是暂停导致的连接错误
                        if (error_msg.contains("tcp connect error") && error_msg.contains("由于目标计算机积极拒绝"))
                            && crate::task::TASK_CONTROLLER.is_paused()
                        {
                            info!("视频流下载因用户暂停导致的连接失败: {:#}", e);
                        } else {
                            error!("视频流下载失败: {:#}", e);
                        }
                    }
                    e
                })?;

            let audio_urls = audio_stream.urls();
            let audio_size = download_stream(downloader, &audio_urls, &tmp_audio_path)
                .await
                .map_err(|e| {
                    let error_msg = e.to_string();
                    if error_msg.contains("用户主动暂停任务") || error_msg.contains("任务已暂停") {
                        info!("音频流下载因用户暂停而终止");
                    } else {
                        // 检查是否是暂停导致的连接错误
                        if (error_msg.contains("tcp connect error") && error_msg.contains("由于目标计算机积极拒绝"))
                            && crate::task::TASK_CONTROLLER.is_paused()
                        {
                            info!("音频流下载因用户暂停导致的连接失败: {:#}", e);
                        } else {
                            error!("音频流下载失败: {:#}", e);
                        }
                    }
                    // 异步删除临时视频文件
                    let video_path_clone = tmp_video_path.clone();
                    tokio::spawn(async move {
                        let _ = fs::remove_file(&video_path_clone).await;
                    });
                    e
                })?;

            // 增强的音视频合并，带损坏文件检测和重试机制
            let res = downloader.merge(&tmp_video_path, &tmp_audio_path, page_path).await;

            // 合并失败时的智能处理
            if let Err(e) = res {
                error!("音视频合并失败: {:#}", e);

                // 检查是否是文件损坏导致的失败
                let error_msg = e.to_string();
                if error_msg.contains("Invalid data found when processing input")
                    || error_msg.contains("ffmpeg error")
                    || error_msg.contains("文件损坏")
                {
                    warn!("检测到文件损坏，清理临时文件并标记为重试: {}", error_msg);

                    // 立即清理损坏的临时文件
                    let _ = fs::remove_file(&tmp_video_path).await;
                    let _ = fs::remove_file(&tmp_audio_path).await;

                    // 返回特殊错误，让上层重试下载
                    return Err(anyhow::anyhow!(
                        "视频文件损坏，已清理临时文件，请重试下载: {}",
                        error_msg
                    ));
                } else {
                    // 其他类型的合并错误，清理临时文件后直接返回
                    let _ = fs::remove_file(&tmp_video_path).await;
                    let _ = fs::remove_file(&tmp_audio_path).await;
                    return Err(e);
                }
            }

            // 合并成功，清理临时文件
            let _ = fs::remove_file(tmp_video_path).await;
            let _ = fs::remove_file(tmp_audio_path).await;

            // 获取合并后文件大小，如果失败则使用视频和音频大小之和
            tokio::fs::metadata(page_path)
                .await
                .map(|metadata| metadata.len())
                .unwrap_or(video_size + audio_size)
        }
    };

    // 计算并记录下载速度
    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();

    if elapsed_secs > 0.0 && total_bytes > 0 {
        let speed_bps = total_bytes as f64 / elapsed_secs;
        let (speed, unit) = if speed_bps >= 1_048_576.0 {
            (speed_bps / 1_048_576.0, "MB/s")
        } else if speed_bps >= 1_024.0 {
            (speed_bps / 1_024.0, "KB/s")
        } else {
            (speed_bps, "B/s")
        };

        info!(
            "视频下载完成，总大小: {:.2} MB，耗时: {:.2} 秒，平均速度: {:.2} {}",
            total_bytes as f64 / 1_048_576.0,
            elapsed_secs,
            speed,
            unit
        );
    }

    Ok(ExecutionStatus::Succeeded)
}

pub async fn fetch_page_danmaku(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    page_info: &PageInfo,
    danmaku_path: PathBuf,
    token: CancellationToken,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }

    // 检查 CID 是否有效（-1 表示信息获取失败）
    if page_info.cid < 0 {
        warn!(
            "视频 {} 的 CID 无效（{}），跳过弹幕下载",
            &video_model.name, page_info.cid
        );
        return Ok(ExecutionStatus::Ignored(anyhow::anyhow!("CID 无效，无法下载弹幕")));
    }

    // 检查是否为番剧，如果是番剧则需要从API获取正确的 aid
    let bili_video = if video_model.source_type == Some(1) {
        // 番剧：需要从API获取aid
        if let Some(ep_id) = &video_model.ep_id {
            match tokio::select! {
                biased;
                _ = token.cancelled() => None,
                res = get_bangumi_aid_from_api(bili_client, ep_id, token.clone()) => res,
            } {
                Some(aid) => {
                    debug!("使用番剧API获取到的aid: {}", aid);
                    Video::new_with_aid(bili_client, video_model.bvid.clone(), aid)
                }
                None => {
                    warn!("无法获取番剧 {} (EP{}) 的AID，使用bvid转换", &video_model.name, ep_id);
                    Video::new(bili_client, video_model.bvid.clone())
                }
            }
        } else {
            warn!("番剧 {} 缺少EP ID，使用bvid转换aid", &video_model.name);
            Video::new(bili_client, video_model.bvid.clone())
        }
    } else {
        // 普通视频：使用 bvid 转换的 aid
        Video::new(bili_client, video_model.bvid.clone())
    };

    let danmaku_writer = tokio::select! {
        biased;
        _ = token.cancelled() => return Err(anyhow!("Download cancelled")),
        res = bili_video.get_danmaku_writer(page_info, token.clone()) => res?,
    };

    danmaku_writer.write(danmaku_path).await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn fetch_page_subtitle(
    should_run: bool,
    bili_client: &BiliClient,
    video_model: &video::Model,
    page_info: &PageInfo,
    subtitle_path: &Path,
    token: CancellationToken,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let bili_video = Video::new(bili_client, video_model.bvid.clone());
    let subtitles = tokio::select! {
        biased;
        _ = token.cancelled() => return Err(anyhow!("Download cancelled")),
        res = bili_video.get_subtitles(page_info) => res?,
    };
    let tasks = subtitles
        .into_iter()
        .map(|subtitle| async move {
            let path = subtitle_path.with_extension(format!("{}.srt", subtitle.lan));
            tokio::fs::write(path, subtitle.body.to_string()).await
        })
        .collect::<FuturesUnordered<_>>();
    tasks.try_collect::<Vec<()>>().await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn generate_page_nfo(
    should_run: bool,
    video_model: &video::Model,
    page_model: &page::Model,
    nfo_path: PathBuf,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let nfo = match video_model.single_page {
        Some(single_page) => {
            if single_page {
                NFO::Movie(video_model.into())
            } else {
                NFO::Episode(page_model.into())
            }
        }
        None => NFO::Episode(page_model.into()),
    };
    generate_nfo(nfo, nfo_path).await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn fetch_video_poster(
    should_run: bool,
    video_model: &video::Model,
    downloader: &UnifiedDownloader,
    poster_path: PathBuf,
    fanart_path: PathBuf,
    token: CancellationToken,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    let cover_url = video_model.cover.as_str();
    let urls = vec![cover_url];
    tokio::select! {
        biased;
        _ = token.cancelled() => return Ok(ExecutionStatus::Skipped),
        res = downloader.fetch_with_fallback(&urls, &poster_path) => res,
    }?;
    fs::copy(&poster_path, &fanart_path).await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn fetch_upper_face(
    should_run: bool,
    video_model: &video::Model,
    downloader: &UnifiedDownloader,
    upper_face_path: PathBuf,
    token: CancellationToken,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }

    // 检查URL是否有效，避免相对路径或空URL
    let upper_face_url = &video_model.upper_face;
    if upper_face_url.is_empty() || !upper_face_url.starts_with("http") {
        debug!("跳过无效的作者头像URL: {}", upper_face_url);
        return Ok(ExecutionStatus::Ignored(anyhow::anyhow!("无效的作者头像URL")));
    }

    let urls = vec![upper_face_url.as_str()];
    tokio::select! {
        biased;
        _ = token.cancelled() => return Ok(ExecutionStatus::Skipped),
        res = downloader.fetch_with_fallback(&urls, &upper_face_path) => res,
    }?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn generate_upper_nfo(
    should_run: bool,
    video_model: &video::Model,
    nfo_path: PathBuf,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    generate_nfo(NFO::Upper(video_model.into()), nfo_path).await?;
    Ok(ExecutionStatus::Succeeded)
}

pub async fn generate_video_nfo(
    should_run: bool,
    video_model: &video::Model,
    nfo_path: PathBuf,
) -> Result<ExecutionStatus> {
    if !should_run {
        return Ok(ExecutionStatus::Skipped);
    }
    generate_nfo(NFO::TVShow(video_model.into()), nfo_path).await?;
    Ok(ExecutionStatus::Succeeded)
}

async fn generate_nfo(nfo: NFO<'_>, nfo_path: PathBuf) -> Result<()> {
    if let Some(parent) = nfo_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(nfo_path, nfo.generate_nfo().await?.as_bytes()).await?;
    Ok(())
}

async fn get_season_title_from_api(
    bili_client: &BiliClient,
    season_id: &str,
    token: CancellationToken,
) -> Option<String> {
    let url = format!("https://api.bilibili.com/pgc/view/web/season?season_id={}", season_id);

    // 重试配置：最大重试3次，每次重试间隔递增
    let max_retries = 3;
    let mut retry_count = 0;

    while retry_count <= max_retries {
        // 检查是否被取消
        if token.is_cancelled() {
            debug!("请求被取消，停止重试");
            return None;
        }

        let retry_delay = std::time::Duration::from_millis(500 * (retry_count as u64 + 1));
        if retry_count > 0 {
            debug!("第{}次重试获取季度信息，延迟{}ms", retry_count, retry_delay.as_millis());
            tokio::time::sleep(retry_delay).await;
        }

        match tokio::select! {
            biased;
            _ = token.cancelled() => return None,
            res = bili_client.get(&url, token.clone()) => res,
        } {
            Ok(res) => {
                if res.status().is_success() {
                    match res.json::<serde_json::Value>().await {
                        Ok(json) => {
                            // 检查API返回是否成功
                            if json["code"].as_i64().unwrap_or(-1) == 0 {
                                // 获取季度标题
                                if let Some(title) = json["result"]["title"].as_str() {
                                    debug!("获取到季度标题: {} (尝试次数: {})", title, retry_count + 1);
                                    return Some(title.to_string());
                                }
                            } else {
                                warn!(
                                    "获取季度信息失败，API返回错误: {} (尝试次数: {})",
                                    json["message"].as_str().unwrap_or("未知错误"),
                                    retry_count + 1
                                );
                                // API返回错误码通常不是临时性问题，直接返回
                                return None;
                            }
                        }
                        Err(e) => {
                            warn!("解析季度信息JSON失败: {} (尝试次数: {})", e, retry_count + 1);
                            // JSON解析失败通常不是临时性问题，直接返回
                            return None;
                        }
                    }
                } else {
                    warn!(
                        "获取季度信息HTTP请求失败，状态码: {} (尝试次数: {})",
                        res.status(),
                        retry_count + 1
                    );
                }
            }
            Err(e) => {
                warn!("发送季度信息请求失败: {} (尝试次数: {})", e, retry_count + 1);
            }
        }

        retry_count += 1;
    }

    error!("获取season_id={}的季度信息失败，已重试{}次", season_id, max_retries);
    None
}

/// 从番剧API获取指定EP的AID
async fn get_bangumi_aid_from_api(bili_client: &BiliClient, ep_id: &str, token: CancellationToken) -> Option<String> {
    let url = format!("https://api.bilibili.com/pgc/view/web/season?ep_id={}", ep_id);

    // 重试配置：最大重试3次，每次重试间隔递增
    let max_retries = 3;
    let mut retry_count = 0;

    while retry_count <= max_retries {
        // 检查是否被取消
        if token.is_cancelled() {
            debug!("请求被取消，停止重试");
            return None;
        }

        let retry_delay = std::time::Duration::from_millis(500 * (retry_count as u64 + 1));
        if retry_count > 0 {
            debug!("第{}次重试获取EP信息，延时{}ms", retry_count, retry_delay.as_millis());
            tokio::time::sleep(retry_delay).await;
        }

        match tokio::select! {
            biased;
            _ = token.cancelled() => return None,
            res = bili_client.get(&url, token.clone()) => res,
        } {
            Ok(res) => {
                if res.status().is_success() {
                    match res.json::<serde_json::Value>().await {
                        Ok(json) => {
                            // 检查API返回是否成功
                            if json["code"].as_i64().unwrap_or(-1) == 0 {
                                // 在episodes数组中查找对应EP的AID
                                if let Some(episodes) = json["result"]["episodes"].as_array() {
                                    for episode in episodes {
                                        if let Some(episode_id) = episode["id"].as_i64() {
                                            if episode_id.to_string() == ep_id {
                                                debug!("获取到EP {} 的AID (尝试次数: {})", ep_id, retry_count + 1);
                                                return episode["aid"].as_i64().map(|aid| aid.to_string());
                                            }
                                        }
                                    }
                                }
                                // 找不到对应的EP，不是网络问题，直接返回
                                warn!("在episodes数组中找不到EP {}", ep_id);
                                return None;
                            } else {
                                warn!(
                                    "获取EP信息失败，API返回错误: {} (尝试次数: {})",
                                    json["message"].as_str().unwrap_or("未知错误"),
                                    retry_count + 1
                                );
                                // API返回错误码通常不是临时性问题，直接返回
                                return None;
                            }
                        }
                        Err(e) => {
                            warn!("解析番剧API响应失败: {} (尝试次数: {})", e, retry_count + 1);
                            // JSON解析失败通常不是临时性问题，直接返回
                            return None;
                        }
                    }
                } else {
                    warn!(
                        "请求EP信息HTTP失败，状态码: {} (尝试次数: {})",
                        res.status(),
                        retry_count + 1
                    );
                }
            }
            Err(e) => {
                warn!("请求番剧API失败: {} (尝试次数: {})", e, retry_count + 1);
            }
        }

        retry_count += 1;
    }

    error!("获取ep_id={}的AID失败，已重试{}次", ep_id, max_retries);
    None
}

/// 从番剧API获取指定EP的CID和duration
async fn get_bangumi_info_from_api(
    bili_client: &BiliClient,
    ep_id: &str,
    token: CancellationToken,
) -> Option<(i64, u32)> {
    let url = format!("https://api.bilibili.com/pgc/view/web/season?ep_id={}", ep_id);

    // 重试配置：最大重试3次，每次重试间隔递增
    let max_retries = 3;
    let mut retry_count = 0;

    while retry_count <= max_retries {
        // 检查是否被取消
        if token.is_cancelled() {
            debug!("请求被取消，停止重试");
            return None;
        }

        let retry_delay = std::time::Duration::from_millis(500 * (retry_count as u64 + 1));
        if retry_count > 0 {
            debug!(
                "第{}次重试获取EP详细信息，延时{}ms",
                retry_count,
                retry_delay.as_millis()
            );
            tokio::time::sleep(retry_delay).await;
        }

        match tokio::select! {
            biased;
            _ = token.cancelled() => return None,
            res = bili_client.get(&url, token.clone()) => res,
        } {
            Ok(res) => {
                if res.status().is_success() {
                    match res.json::<serde_json::Value>().await {
                        Ok(json) => {
                            // 检查API返回是否成功
                            if json["code"].as_i64().unwrap_or(-1) == 0 {
                                // 在episodes数组中查找对应EP的信息
                                if let Some(episodes) = json["result"]["episodes"].as_array() {
                                    for episode in episodes {
                                        if let Some(episode_id) = episode["id"].as_i64() {
                                            if episode_id.to_string() == ep_id {
                                                let cid = episode["cid"].as_i64().unwrap_or(0);
                                                // duration在API中是毫秒，需要转换为秒
                                                let duration_ms = episode["duration"].as_i64().unwrap_or(0);
                                                let duration_sec = (duration_ms / 1000) as u32;
                                                debug!(
                                                    "获取到番剧EP {} 的CID: {}, 时长: {}秒 (尝试次数: {})",
                                                    ep_id,
                                                    cid,
                                                    duration_sec,
                                                    retry_count + 1
                                                );
                                                return Some((cid, duration_sec));
                                            }
                                        }
                                    }
                                }
                                // 找不到对应的EP，不是网络问题，直接返回
                                warn!("在episodes数组中找不到EP {}", ep_id);
                                return None;
                            } else {
                                warn!(
                                    "获取EP详细信息失败，API返回错误: {} (尝试次数: {})",
                                    json["message"].as_str().unwrap_or("未知错误"),
                                    retry_count + 1
                                );
                                // API返回错误码通常不是临时性问题，直接返回
                                return None;
                            }
                        }
                        Err(e) => {
                            warn!("解析番剧API响应失败: {} (尝试次数: {})", e, retry_count + 1);
                            // JSON解析失败通常不是临时性问题，直接返回
                            return None;
                        }
                    }
                } else {
                    warn!(
                        "请求EP详细信息HTTP失败，状态码: {} (尝试次数: {})",
                        res.status(),
                        retry_count + 1
                    );
                }
            }
            Err(e) => {
                warn!("请求番剧API失败: {} (尝试次数: {})", e, retry_count + 1);
            }
        }

        retry_count += 1;
    }

    error!("获取ep_id={}的详细信息失败，已重试{}次", ep_id, max_retries);
    None
}

/// 从现有数据库中获取该季已有的分集信息
async fn get_existing_episodes_for_season(
    connection: &DatabaseConnection,
    season_id: &str,
    bili_client: &BiliClient,
    token: CancellationToken,
) -> Result<HashMap<String, (i64, u32)>> {
    use sea_orm::*;

    // 查询该season_id下所有已有page信息的视频
    let existing_data = video::Entity::find()
        .filter(video::Column::SeasonId.eq(season_id))
        .filter(video::Column::SourceType.eq(1)) // 番剧类型
        .filter(video::Column::EpId.is_not_null())
        .find_with_related(page::Entity)
        .all(connection)
        .await?;

    let mut episodes_map = HashMap::new();

    for (video, pages) in existing_data {
        if let Some(ep_id) = video.ep_id {
            // 每个番剧视频通常只有一个page（单集）
            if let Some(page) = pages.first() {
                episodes_map.insert(ep_id, (page.cid, page.duration));
            }
        }
    }

    if !episodes_map.is_empty() {
        // 尝试获取番剧标题用于显示
        let season_title = get_season_title_from_api(bili_client, season_id, token.clone()).await;
        let display_name = season_title.as_deref().unwrap_or(season_id);

        info!(
            "从数据库缓存中找到季 {} 「{}」的 {} 个分集信息",
            season_id,
            display_name,
            episodes_map.len()
        );
    }

    Ok(episodes_map)
}

/// 从API获取整个番剧季的信息（单次请求）
async fn get_season_info_from_api(
    bili_client: &BiliClient,
    season_id: &str,
    token: CancellationToken,
) -> Result<SeasonInfo> {
    let url = format!("https://api.bilibili.com/pgc/view/web/season?season_id={}", season_id);

    let res = tokio::select! {
        biased;
        _ = token.cancelled() => return Err(anyhow!("Request cancelled")),
        res = bili_client.get(&url, token.clone()) => res,
    }?;

    if !res.status().is_success() {
        bail!("获取番剧季信息失败，HTTP状态码: {}", res.status());
    }

    let json: serde_json::Value = res
        .json()
        .await
        .with_context(|| format!("解析番剧季 {} 响应失败", season_id))?;

    if json["code"].as_i64().unwrap_or(-1) != 0 {
        bail!("API返回错误: {}", json["message"].as_str().unwrap_or("未知错误"));
    }

    let result = &json["result"];
    // 获取番剧标题
    let title = result["title"]
        .as_str()
        .unwrap_or(&format!("番剧{}", season_id))
        .to_string();

    let episodes: Vec<EpisodeInfo> = result["episodes"]
        .as_array()
        .context("找不到分集列表")?
        .iter()
        .filter_map(|ep| {
            let ep_id = ep["id"].as_i64()?.to_string();
            let cid = ep["cid"].as_i64()?;
            let duration_ms = ep["duration"].as_i64()?;
            let duration = (duration_ms / 1000) as u32;

            Some(EpisodeInfo { ep_id, cid, duration })
        })
        .collect();

    info!(
        "成功获取番剧季 {} 「{}」信息，包含 {} 集",
        season_id,
        title,
        episodes.len()
    );

    Ok(SeasonInfo { title, episodes })
}

/// 处理单个番剧视频
async fn process_bangumi_video(
    video_model: video::Model,
    episodes_map: &HashMap<String, (i64, u32)>,
    connection: &DatabaseConnection,
    video_source: &VideoSourceEnum,
) -> Result<()> {
    let txn = connection.begin().await?;

    let (actual_cid, duration) = if let Some(ep_id) = &video_model.ep_id {
        match episodes_map.get(ep_id) {
            Some(&info) => {
                debug!("使用缓存信息: EP{} -> CID={}, Duration={}s", ep_id, info.0, info.1);
                info
            }
            None => {
                warn!("找不到分集 {} 的信息，使用默认值", ep_id);
                (-1, 1440) // 默认值
            }
        }
    } else {
        error!("番剧 {} 缺少EP ID", video_model.name);
        (-1, 1440)
    };

    let page_info = PageInfo {
        cid: actual_cid,
        page: 1,
        name: video_model.name.clone(),
        duration,
        first_frame: None,
        dimension: None,
    };

    // 创建page记录（这里会自动缓存cid和duration到数据库）
    create_pages(vec![page_info], &video_model, &txn).await?;

    // 更新视频状态
    let mut video_active_model: bili_sync_entity::video::ActiveModel = video_model.into();
    video_source.set_relation_id(&mut video_active_model);
    video_active_model.single_page = Set(Some(true)); // 番剧的每一集都是单页
    video_active_model.tags = Set(Some(serde_json::Value::Array(vec![]))); // 空标签数组
    video_active_model.save(&txn).await?;

    txn.commit().await?;
    Ok(())
}

/// 获取特定视频源的视频数量
async fn get_video_count_for_source(video_source: &VideoSourceEnum, connection: &DatabaseConnection) -> Result<usize> {
    let count = video::Entity::find()
        .filter(video_source.filter_expr())
        .count(connection)
        .await?;
    Ok(count as usize)
}

/// 自动重置风控导致的失败任务
/// 当检测到风控时，将所有失败状态(值为3)、正在进行状态(值为2)以及未完成的任务重置为未开始状态(值为0)
pub async fn auto_reset_risk_control_failures(connection: &DatabaseConnection) -> Result<()> {
    use crate::utils::status::{PageStatus, VideoStatus};
    use bili_sync_entity::{page, video};
    use sea_orm::*;

    info!("检测到风控，开始自动重置失败、进行中和未完成的下载任务...");

    // 查询所有视频和页面数据
    let (all_videos, all_pages) = tokio::try_join!(
        video::Entity::find()
            .select_only()
            .columns([video::Column::Id, video::Column::Name, video::Column::DownloadStatus,])
            .into_tuple::<(i32, String, u32)>()
            .all(connection),
        page::Entity::find()
            .select_only()
            .columns([page::Column::Id, page::Column::Name, page::Column::DownloadStatus,])
            .into_tuple::<(i32, String, u32)>()
            .all(connection)
    )?;

    let mut resetted_videos = 0;
    let mut resetted_pages = 0;

    let txn = connection.begin().await?;

    // 重置视频失败、进行中和未完成状态
    for (id, name, download_status) in all_videos {
        let mut video_status = VideoStatus::from(download_status);
        let mut video_resetted = false;

        // 检查是否为完全成功的状态（所有任务都是1）
        let is_fully_completed = (0..5).all(|task_index| video_status.get(task_index) == 1);

        if !is_fully_completed {
            // 如果不是完全成功，检查所有任务索引，将失败状态(3)、正在进行状态(2)和未开始状态(0)重置为未开始(0)
            for task_index in 0..5 {
                let status_value = video_status.get(task_index);
                if status_value == 3 || status_value == 2 || status_value == 0 {
                    video_status.set(task_index, 0); // 重置为未开始
                    video_resetted = true;
                }
            }
        }

        if video_resetted {
            video::Entity::update(video::ActiveModel {
                id: sea_orm::ActiveValue::Unchanged(id),
                download_status: sea_orm::Set(video_status.into()),
                ..Default::default()
            })
            .exec(&txn)
            .await?;

            resetted_videos += 1;
            debug!("重置视频「{}」的未完成任务状态", name);
        }
    }

    // 重置页面失败、进行中和未完成状态
    for (id, name, download_status) in all_pages {
        let mut page_status = PageStatus::from(download_status);
        let mut page_resetted = false;

        // 检查是否为完全成功的状态（所有任务都是1）
        let is_fully_completed = (0..5).all(|task_index| page_status.get(task_index) == 1);

        if !is_fully_completed {
            // 如果不是完全成功，检查所有任务索引，将失败状态(3)、正在进行状态(2)和未开始状态(0)重置为未开始(0)
            for task_index in 0..5 {
                let status_value = page_status.get(task_index);
                if status_value == 3 || status_value == 2 || status_value == 0 {
                    page_status.set(task_index, 0); // 重置为未开始
                    page_resetted = true;
                }
            }
        }

        if page_resetted {
            page::Entity::update(page::ActiveModel {
                id: sea_orm::ActiveValue::Unchanged(id),
                download_status: sea_orm::Set(page_status.into()),
                ..Default::default()
            })
            .exec(&txn)
            .await?;

            resetted_pages += 1;
            debug!("重置页面「{}」的未完成任务状态", name);
        }
    }

    txn.commit().await?;

    if resetted_videos > 0 || resetted_pages > 0 {
        info!(
            "风控自动重置完成：重置了 {} 个视频和 {} 个页面的未完成任务状态",
            resetted_videos, resetted_pages
        );
    } else {
        info!("风控自动重置完成：所有任务都已完成，无需重置");
    }

    Ok(())
}

/// 获取合集中视频的集数序号
/// 根据视频在合集中的发布时间顺序确定集数
async fn get_collection_video_episode_number(
    connection: &DatabaseConnection,
    collection_id: i32,
    bvid: &str,
) -> Result<i32> {
    use bili_sync_entity::video;
    use sea_orm::*;

    // 获取该合集中所有视频，按发布时间排序
    let videos = video::Entity::find()
        .filter(video::Column::CollectionId.eq(collection_id))
        .order_by_asc(video::Column::Pubtime)
        .select_only()
        .columns([video::Column::Bvid, video::Column::Pubtime])
        .into_tuple::<(String, chrono::NaiveDateTime)>()
        .all(connection)
        .await?;

    // 找到当前视频的位置，返回序号（从1开始）
    for (index, (video_bvid, _)) in videos.iter().enumerate() {
        if video_bvid == bvid {
            return Ok((index + 1) as i32);
        }
    }

    // 如果没找到，返回错误
    Err(anyhow!("视频 {} 在合集 {} 中未找到", bvid, collection_id))
}

/// 生成唯一的文件夹名称，避免同名冲突（公共函数）
pub fn generate_unique_folder_name(parent_dir: &std::path::Path, base_name: &str, bvid: &str, pubtime: &str) -> String {
    let mut unique_name = base_name.to_string();
    let mut counter = 0;

    // 检查基础名称是否已存在
    let base_path = parent_dir.join(&unique_name);
    if !base_path.exists() {
        return unique_name;
    }

    // 如果存在，先尝试追加发布时间
    unique_name = format!("{}-{}", base_name, pubtime);
    let time_path = parent_dir.join(&unique_name);
    if !time_path.exists() {
        info!("检测到下载文件夹名冲突，追加发布时间: {} -> {}", base_name, unique_name);
        return unique_name;
    }

    // 如果发布时间也冲突，追加BVID
    unique_name = format!("{}-{}", base_name, bvid);
    let bvid_path = parent_dir.join(&unique_name);
    if !bvid_path.exists() {
        info!("检测到下载文件夹名冲突，追加BVID: {} -> {}", base_name, unique_name);
        return unique_name;
    }

    // 如果都冲突，使用数字后缀
    loop {
        counter += 1;
        unique_name = format!("{}-{}", base_name, counter);
        let numbered_path = parent_dir.join(&unique_name);
        if !numbered_path.exists() {
            warn!(
                "检测到严重下载文件夹名冲突，使用数字后缀: {} -> {}",
                base_name, unique_name
            );
            return unique_name;
        }

        // 防止无限循环，使用随机后缀
        if counter > 1000 {
            warn!("下载文件夹名冲突解决失败，使用随机后缀");
            let random_suffix: u32 = rand::random::<u32>() % 90000 + 10000;
            unique_name = format!("{}-{}", base_name, random_suffix);
            return unique_name;
        }
    }
}

#[cfg(test)]
mod tests {
    use handlebars::handlebars_helper;
    use serde_json::json;

    use crate::config::PathSafeTemplate;

    #[test]
    fn test_template_usage() {
        let mut template = handlebars::Handlebars::new();
        handlebars_helper!(truncate: |s: String, len: usize| {
            if s.chars().count() > len {
                s.chars().take(len).collect::<String>()
            } else {
                s.to_string()
            }
        });
        template.register_helper("truncate", Box::new(truncate));
        let _ = template.path_safe_register("video", "test{{bvid}}test");
        let _ = template.path_safe_register("test_truncate", "哈哈，{{ truncate title 30 }}");
        let _ = template.path_safe_register("test_path_unix", "{{ truncate title 7 }}/test/a");
        let _ = template.path_safe_register("test_path_windows", r"{{ truncate title 7 }}\\test\\a");
        #[cfg(not(windows))]
        {
            assert_eq!(
                template
                    .path_safe_render("test_path_unix", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                "关注_永雏塔菲/test/a"
            );
            assert_eq!(
                template
                    .path_safe_render("test_path_windows", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                "关注_永雏塔菲_test_a"
            );
        }
        #[cfg(windows)]
        {
            assert_eq!(
                template
                    .path_safe_render("test_path_unix", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                "关注_永雏塔菲_test_a"
            );
            assert_eq!(
                template
                    .path_safe_render("test_path_windows", &json!({"title": "关注/永雏塔菲喵"}))
                    .unwrap(),
                r"关注_永雏塔菲\\test\\a"
            );
        }
        assert_eq!(
            template
                .path_safe_render("video", &json!({"bvid": "BV1b5411h7g7"}))
                .unwrap(),
            "testBV1b5411h7g7test"
        );
        assert_eq!(
            template
                .path_safe_render(
                    "test_truncate",
                    &json!({"title": "你说得对，但是 Rust 是由 Mozilla 自主研发的一款全新的编译期格斗游戏。\
                    编译将发生在一个被称作「Cargo」的构建系统中。在这里，被引用的指针将被授予「生命周期」之力，导引对象安全。\
                    你将扮演一位名为「Rustacean」的神秘角色, 在与「Rustc」的搏斗中邂逅各种骨骼惊奇的傲娇报错。\
                    征服她们、通过编译同时，逐步发掘「C++」程序崩溃的真相。"})
                )
                .unwrap(),
            "哈哈，你说得对，但是 Rust 是由 Mozilla 自主研发的一"
        );
    }
}
