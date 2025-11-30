use core::str;
use std::io::SeekFrom;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{header, Method};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_util::io::StreamReader;
use tracing::{error, info, warn};

use crate::bilibili::Client;

const HARD_THREAD_CAP: usize = 16;
const MIN_PARALLEL_SIZE_BYTES: u64 = 1 * 1024 * 1024;
pub struct Downloader {
    client: Client,
}

impl Downloader {
    // Downloader 使用带有默认 Header 的 Client 构建
    // 拿到 url 后下载文件不需要任何 cookie 作为身份凭证
    // 但如果不设置默认 Header，下载时会遇到 403 Forbidden 错误
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn fetch(&self, url: &str, path: &Path) -> Result<()> {
        Self::ensure_parent_dir(path).await?;

        let (parallel_enabled, configured_threads) = crate::config::with_config(|bundle| {
            let cfg = &bundle.config.concurrent_limit.parallel_download;
            (cfg.enabled, cfg.threads)
        });

        let requested_threads = configured_threads.clamp(1, HARD_THREAD_CAP);

        if parallel_enabled && requested_threads > 1 {
            match self.fetch_parallel(url, path, requested_threads).await {
                Ok(_) => return Ok(()),
                Err(err) => {
                    warn!("原生并行下载失败，将回退到单线程: {:#}", err);
                    let _ = fs::remove_file(path).await;
                }
            }
        }

        self.fetch_single_thread(url, path).await
    }

    async fn ensure_parent_dir(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("无法创建下载目录: {}", parent.display()))?;
            }
        }
        Ok(())
    }

    async fn fetch_single_thread(&self, url: &str, path: &Path) -> Result<()> {
        let mut file = match File::create(path).await {
            Ok(f) => f,
            Err(e) => {
                error!("创建文件失败: {:#}", e);
                return Err(e.into());
            }
        };

        let resp = match self.client.request(Method::GET, url, None).send().await {
            Ok(r) => match r.error_for_status() {
                Ok(r) => r,
                Err(e) => {
                    error!("HTTP状态码错误: {:#}", e);
                    return Err(e.into());
                }
            },
            Err(e) => {
                error!("HTTP请求失败: {:#}", e);
                return Err(e.into());
            }
        };

        let expected = resp.content_length().unwrap_or_default();

        let mut stream_reader = StreamReader::new(resp.bytes_stream().map_err(std::io::Error::other));
        let received = match tokio::io::copy(&mut stream_reader, &mut file).await {
            Ok(size) => size,
            Err(e) => {
                error!("下载过程中出错: {:#}", e);
                return Err(e.into());
            }
        };

        file.flush().await?;

        ensure!(
            received >= expected,
            "received {} bytes, expected {} bytes",
            received,
            expected
        );

        Ok(())
    }

    async fn fetch_parallel(&self, url: &str, path: &Path, max_threads: usize) -> Result<()> {
        let head_resp = self
            .client
            .request(Method::HEAD, url, None)
            .send()
            .await
            .context("发送 HEAD 请求以检测分片信息失败")?;

        let total_size = head_resp
            .content_length()
            .context("远端未返回 Content-Length，无法执行多线程下载")?;

        if total_size < MIN_PARALLEL_SIZE_BYTES {
            bail!("文件过小 ({} 字节)，无需启用多线程下载", total_size);
        }

        let supports_range = head_resp
            .headers()
            .get(header::ACCEPT_RANGES)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.eq_ignore_ascii_case("bytes") || value.to_ascii_lowercase().contains("bytes"))
            .unwrap_or(false);

        if !supports_range {
            bail!("远端不支持分片下载 (缺少 Accept-Ranges: bytes)");
        }

        let thread_budget = max_threads.min(HARD_THREAD_CAP);
        let chunk_size = std::cmp::max(
            MIN_PARALLEL_SIZE_BYTES,
            (total_size + thread_budget as u64 - 1) / thread_budget as u64,
        );

        let mut ranges = Vec::new();
        let mut start = 0u64;
        while start < total_size {
            let end = std::cmp::min(start + chunk_size - 1, total_size - 1);
            ranges.push((start, end));
            start = end + 1;
        }

        let concurrency = ranges.len().min(thread_budget);
        info!(
            "使用原生多线程下载，线程数: {}, 总大小: {:.2} MB",
            concurrency,
            total_size as f64 / (1024.0 * 1024.0)
        );

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .await
            .with_context(|| format!("创建文件失败: {}", path.display()))?;
        file.set_len(total_size).await?;
        let shared_file = Arc::new(Mutex::new(file));
        let url_arc: Arc<str> = Arc::from(url.to_string());

        let mut stream = stream::iter(ranges.into_iter().map(|(range_start, range_end)| {
            let client = self.client.clone();
            let file = Arc::clone(&shared_file);
            let url = Arc::clone(&url_arc);
            async move {
                let data = Self::download_range(client, &url, range_start, range_end).await?;
                {
                    let mut file = file.lock().await;
                    file.seek(SeekFrom::Start(range_start)).await?;
                    file.write_all(&data).await?;
                }
                Ok::<u64, anyhow::Error>(data.len() as u64)
            }
        }))
        .buffer_unordered(concurrency);

        let mut received = 0u64;
        while let Some(chunk) = stream.next().await {
            received += chunk?;
        }

        ensure!(
            received == total_size,
            "并行下载的字节数与期望不符 ({} vs {})",
            received,
            total_size
        );

        {
            let mut file = shared_file.lock().await;
            file.flush().await?;
            file.sync_all().await?;
        }

        Ok(())
    }

    async fn download_range(client: Client, url: &str, start: u64, end: u64) -> Result<Vec<u8>> {
        let range_header = format!("bytes={}-{}", start, end);
        let resp = client
            .request(Method::GET, url, None)
            .header(header::RANGE, range_header)
            .send()
            .await
            .with_context(|| format!("下载区间 {}-{} 失败", start, end))?;

        if !(resp.status().is_success() || resp.status() == reqwest::StatusCode::PARTIAL_CONTENT) {
            bail!(
                "服务器未返回成功状态码，区间 {}-{}，状态: {}",
                start,
                end,
                resp.status()
            );
        }

        let bytes = resp.bytes().await?;
        let expected_len = (end - start + 1) as usize;
        ensure!(
            bytes.len() == expected_len,
            "区间 {}-{} 数据长度不符 ({} vs {})",
            start,
            end,
            bytes.len(),
            expected_len
        );

        Ok(bytes.to_vec())
    }

    pub async fn fetch_with_fallback(&self, urls: &[&str], path: &Path) -> Result<()> {
        if urls.is_empty() {
            bail!("no urls provided");
        }

        let mut last_error = None;
        for url in urls.iter() {
            match self.fetch(url, path).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(err) => {
                    warn!("下载失败: {:#}", err);
                    last_error = Some(err);
                }
            }
        }

        error!("所有URL尝试失败");
        match last_error {
            Some(err) => Err(err).with_context(|| format!("failed to download from {:?}", urls)),
            None => bail!("no urls to try"),
        }
    }

    pub async fn merge(&self, video_path: &Path, audio_path: &Path, output_path: &Path) -> Result<()> {
        // 检查输入文件是否存在
        if !video_path.exists() {
            error!("视频文件不存在: {}", video_path.display());
            bail!("视频文件不存在: {}", video_path.display());
        }

        if !audio_path.exists() {
            error!("音频文件不存在: {}", audio_path.display());
            bail!("音频文件不存在: {}", audio_path.display());
        }

        // 增强的文件完整性检查
        if let Err(e) = self.validate_media_file(video_path, "视频").await {
            error!("视频文件完整性检查失败: {:#}", e);
            bail!("视频文件损坏或不完整: {}", e);
        }

        if let Err(e) = self.validate_media_file(audio_path, "音频").await {
            error!("音频文件完整性检查失败: {:#}", e);
            bail!("音频文件损坏或不完整: {}", e);
        }

        // 确保输出目录存在
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }

        // 将Path转换为字符串，防止临时值过早释放
        let video_path_str = video_path.to_string_lossy().to_string();
        let audio_path_str = audio_path.to_string_lossy().to_string();
        let output_path_str = output_path.to_string_lossy().to_string();

        // 构建FFmpeg命令
        let args = [
            "-i",
            &video_path_str,
            "-i",
            &audio_path_str,
            "-c",
            "copy",
            "-strict",
            "unofficial",
            "-y",
            &output_path_str,
        ];

        let ffmpeg_timeout_seconds = crate::config::with_config(|bundle| bundle.config.ffmpeg_timeout_seconds);
        let ffmpeg_timeout_seconds = if ffmpeg_timeout_seconds == 0 {
            60
        } else {
            ffmpeg_timeout_seconds
        };
        let timeout_duration = Duration::from_secs(ffmpeg_timeout_seconds);

        let mut command = tokio::process::Command::new("ffmpeg");
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child: tokio::process::Child = command
            .spawn()
            .with_context(|| format!("启动FFmpeg进程失败: {}", output_path.display()))?;

        let stderr_pipe = child.stderr.take();
        let stderr_handle = tokio::spawn(async move {
            let mut buffer = Vec::new();
            if let Some(mut stderr) = stderr_pipe {
                let _ = stderr.read_to_end(&mut buffer).await;
            }
            buffer
        });

        let status = match timeout(timeout_duration, child.wait()).await {
            Ok(wait_result) => wait_result?,
            Err(_) => {
                warn!(
                    "FFmpeg 合并已执行超过 {} 秒，正在强制终止: {}",
                    ffmpeg_timeout_seconds,
                    output_path.display()
                );
                if let Err(kill_err) = child.start_kill() {
                    error!("终止FFmpeg进程失败: {:#}", kill_err);
                }
                let _ = child.wait().await;

                let stderr_bytes = match stderr_handle.await {
                    Ok(buf) => buf,
                    Err(join_err) => {
                        error!("读取FFmpeg输出失败: {:#}", join_err);
                        Vec::new()
                    }
                };
                let stderr_trimmed = String::from_utf8_lossy(&stderr_bytes).trim().to_string();
                if stderr_trimmed.is_empty() {
                    bail!(
                        "ffmpeg merge timed out after {} seconds and was forcibly terminated",
                        ffmpeg_timeout_seconds
                    );
                } else {
                    bail!(
                        "ffmpeg merge timed out after {} seconds and was forcibly terminated: {}",
                        ffmpeg_timeout_seconds,
                        stderr_trimmed
                    );
                }
            }
        };

        let stderr_bytes = match stderr_handle.await {
            Ok(buf) => buf,
            Err(join_err) => {
                error!("读取FFmpeg输出失败: {:#}", join_err);
                Vec::new()
            }
        };

        if !status.success() {
            let stderr = str::from_utf8(&stderr_bytes).unwrap_or("unknown");
            error!("FFmpeg错误: {}", stderr);
            bail!("ffmpeg error: {}", stderr);
        }

        Ok(())
    }

    /// 验证媒体文件的完整性
    async fn validate_media_file(&self, file_path: &Path, file_type: &str) -> Result<()> {
        // 检查文件大小
        let metadata = tokio::fs::metadata(file_path)
            .await
            .with_context(|| format!("无法读取{}文件元数据: {}", file_type, file_path.display()))?;

        let file_size = metadata.len();
        if file_size == 0 {
            bail!("{}文件为空: {}", file_type, file_path.display());
        }

        if file_size < 1024 {
            // 小于1KB很可能是损坏的
            bail!(
                "{}文件过小({}字节)，可能损坏: {}",
                file_type,
                file_size,
                file_path.display()
            );
        }

        // 使用ffprobe快速验证文件格式
        let file_path_str = file_path.to_string_lossy().to_string();
        let result = tokio::process::Command::new("ffprobe")
            .args([
                "-v",
                "quiet", // 静默模式
                "-print_format",
                "json",          // JSON输出
                "-show_format",  // 显示格式信息
                "-show_streams", // 显示流信息
                &file_path_str,
            ])
            .output()
            .await;

        match result {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = str::from_utf8(&output.stderr).unwrap_or("unknown");
                    bail!("{}文件格式验证失败: {}", file_type, stderr);
                }

                // 检查输出是否包含有效的流信息
                let stdout = str::from_utf8(&output.stdout).unwrap_or("");
                if stdout.len() < 50 || !stdout.contains("streams") {
                    bail!("{}文件缺少有效的媒体流信息", file_type);
                }
            }
            Err(e) => {
                warn!("ffprobe不可用，跳过高级验证: {:#}", e);
                // 如果ffprobe不可用，只做基本的文件大小检查
            }
        }

        Ok(())
    }
}
