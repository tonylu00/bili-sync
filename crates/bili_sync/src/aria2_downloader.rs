use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::bilibili::Client;
use crate::config::{CONFIG, CONFIG_DIR};

/// 嵌入的aria2二进制文件 (编译时自动下载对应平台版本)
#[cfg(target_os = "windows")]
static ARIA2_BINARY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/aria2c.exe"));

#[cfg(target_os = "linux")]
static ARIA2_BINARY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/aria2c"));

#[cfg(any(target_os = "macos", target_os = "ios"))]
static ARIA2_BINARY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/aria2c"));

pub struct Aria2Downloader {
    client: Client,
    aria2_process: Arc<Mutex<Option<tokio::process::Child>>>,
    aria2_binary_path: PathBuf,
    rpc_port: u16,
    rpc_secret: String,
}

impl Aria2Downloader {
    /// 创建新的aria2下载器实例
    pub async fn new(client: Client) -> Result<Self> {
        let aria2_binary_path = Self::extract_aria2_binary().await?;
        let rpc_port = Self::find_available_port().await?;
        let rpc_secret = Self::generate_secret();

        let mut downloader = Self {
            client,
            aria2_process: Arc::new(Mutex::new(None)),
            aria2_binary_path,
            rpc_port,
            rpc_secret,
        };

        downloader.start_aria2_daemon().await?;
        Ok(downloader)
    }

    /// 提取嵌入的aria2二进制文件到临时目录，失败时回退到系统aria2
    async fn extract_aria2_binary() -> Result<PathBuf> {
        // 使用配置文件夹存储aria2二进制文件，而不是临时目录
        let binary_name = if cfg!(target_os = "windows") {
            "aria2c.exe"
        } else {
            "aria2c"
        };
        let binary_path = CONFIG_DIR.join(binary_name);

        // 确保配置目录存在
        if let Err(e) = tokio::fs::create_dir_all(&*CONFIG_DIR).await {
            warn!("创建配置目录失败: {}, 将使用临时目录", e);
            let temp_dir = std::env::temp_dir();
            return Self::extract_aria2_binary_to_temp(temp_dir, binary_name).await;
        }

        // 如果文件已存在且可执行，直接返回
        if binary_path.exists() {
            // 验证文件是否为有效的aria2可执行文件
            if Self::is_valid_aria2_binary(&binary_path).await {
                return Ok(binary_path);
            } else {
                // 如果是无效的文件（如占位文件），删除它
                let _ = tokio::fs::remove_file(&binary_path).await;
            }
        }

        // 尝试写入嵌入的二进制文件
        debug!("尝试提取aria2二进制文件到配置目录: {}", binary_path.display());
        match tokio::fs::write(&binary_path, ARIA2_BINARY).await {
            Ok(_) => {
                debug!("aria2二进制文件写入配置目录成功，大小: {} bytes", ARIA2_BINARY.len());

                // 在Unix系统上设置执行权限
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = tokio::fs::metadata(&binary_path).await {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = tokio::fs::set_permissions(&binary_path, perms).await;
                        debug!("已设置aria2二进制文件执行权限");
                    }
                }

                // 验证提取的文件是否有效
                debug!("开始验证提取到配置目录的aria2二进制文件...");
                if Self::is_valid_aria2_binary(&binary_path).await {
                    info!("aria2二进制文件已提取到配置目录: {}", binary_path.display());
                    return Ok(binary_path);
                } else {
                    warn!("配置目录中的aria2二进制文件无效，尝试使用系统aria2");
                    let _ = tokio::fs::remove_file(&binary_path).await;
                }
            }
            Err(e) => {
                warn!("提取aria2二进制文件到配置目录失败: {}, 尝试使用系统aria2", e);
            }
        }

        // 回退到系统安装的aria2
        Self::find_system_aria2().await
    }

    /// 备用方案：提取到临时目录
    async fn extract_aria2_binary_to_temp(temp_dir: PathBuf, binary_name: &str) -> Result<PathBuf> {
        let binary_path = temp_dir.join(format!("bili-sync-{}", binary_name));

        debug!("尝试提取aria2二进制文件到临时目录: {}", binary_path.display());

        // 如果文件已存在且可执行，直接返回
        if binary_path.exists() {
            if Self::is_valid_aria2_binary(&binary_path).await {
                return Ok(binary_path);
            } else {
                let _ = tokio::fs::remove_file(&binary_path).await;
            }
        }

        // 尝试写入嵌入的二进制文件
        match tokio::fs::write(&binary_path, ARIA2_BINARY).await {
            Ok(_) => {
                debug!("aria2二进制文件写入临时目录成功，大小: {} bytes", ARIA2_BINARY.len());

                // 在Unix系统上设置执行权限
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = tokio::fs::metadata(&binary_path).await {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = tokio::fs::set_permissions(&binary_path, perms).await;
                    }
                }

                // 验证提取的文件是否有效
                if Self::is_valid_aria2_binary(&binary_path).await {
                    info!("aria2二进制文件已提取到临时目录: {}", binary_path.display());
                    return Ok(binary_path);
                } else {
                    warn!("临时目录中的aria2二进制文件无效");
                    let _ = tokio::fs::remove_file(&binary_path).await;
                }
            }
            Err(e) => {
                warn!("提取aria2二进制文件到临时目录失败: {}", e);
            }
        }

        // 最终回退到系统安装的aria2
        Self::find_system_aria2().await
    }

    /// 验证aria2二进制文件是否有效
    async fn is_valid_aria2_binary(path: &Path) -> bool {
        if !path.exists() {
            warn!("aria2二进制文件不存在: {}", path.display());
            return false;
        }

        // 尝试执行 aria2c --version 来验证
        match tokio::process::Command::new(path).arg("--version").output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() && stdout.contains("aria2") {
                    debug!("aria2二进制文件验证成功: {}", path.display());
                    true
                } else {
                    warn!(
                        "aria2二进制文件验证失败: {}，退出码: {:?}，stdout: {}，stderr: {}",
                        path.display(),
                        output.status.code(),
                        stdout.trim(),
                        stderr.trim()
                    );
                    false
                }
            }
            Err(e) => {
                warn!("无法执行aria2二进制文件 {}: {}", path.display(), e);
                false
            }
        }
    }

    /// 查找系统安装的aria2
    async fn find_system_aria2() -> Result<PathBuf> {
        let _binary_name = if cfg!(target_os = "windows") {
            "aria2c.exe"
        } else {
            "aria2c"
        };

        // 尝试使用which命令查找
        match tokio::process::Command::new("which").arg("aria2c").output().await {
            Ok(output) if output.status.success() => {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let system_path = PathBuf::from(path_str);

                if Self::is_valid_aria2_binary(&system_path).await {
                    info!("使用系统安装的aria2: {}", system_path.display());
                    return Ok(system_path);
                }
            }
            _ => {}
        }

        // 在Windows上尝试where命令
        #[cfg(target_os = "windows")]
        {
            match tokio::process::Command::new("where").arg("aria2c").output().await {
                Ok(output) if output.status.success() => {
                    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let system_path = PathBuf::from(path_str);

                    if Self::is_valid_aria2_binary(&system_path).await {
                        info!("使用系统安装的aria2: {}", system_path.display());
                        return Ok(system_path);
                    }
                }
                _ => {}
            }
        }

        // 尝试常见的安装路径
        let common_paths = if cfg!(target_os = "windows") {
            vec![
                PathBuf::from("C:\\Program Files\\aria2\\aria2c.exe"),
                PathBuf::from("C:\\Program Files (x86)\\aria2\\aria2c.exe"),
            ]
        } else {
            vec![
                PathBuf::from("/usr/bin/aria2c"),
                PathBuf::from("/usr/local/bin/aria2c"),
                PathBuf::from("/opt/homebrew/bin/aria2c"),
            ]
        };

        for path in common_paths {
            if Self::is_valid_aria2_binary(&path).await {
                info!("使用系统安装的aria2: {}", path.display());
                return Ok(path);
            }
        }

        bail!("未找到可用的aria2二进制文件，请确保系统已安装aria2")
    }

    /// 查找可用的端口
    async fn find_available_port() -> Result<u16> {
        use std::net::TcpListener;

        // 尝试绑定到随机端口
        let listener = TcpListener::bind("127.0.0.1:0").context("Failed to bind to random port")?;
        let port = listener.local_addr()?.port();
        drop(listener);

        Ok(port)
    }

    /// 生成随机密钥
    fn generate_secret() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .hash(&mut hasher);

        format!("bili-sync-{:x}", hasher.finish())
    }

    /// 启动aria2守护进程
    async fn start_aria2_daemon(&mut self) -> Result<()> {
        let mut process_guard = self.aria2_process.lock().await;

        if process_guard.is_some() {
            return Ok(());
        }

        // 从配置文件获取线程数
        let threads = CONFIG.concurrent_limit.parallel_download.threads;

        let args = vec![
            "--enable-rpc".to_string(),
            format!("--rpc-listen-port={}", self.rpc_port),
            "--rpc-allow-origin-all".to_string(),
            format!("--rpc-secret={}", self.rpc_secret),
            "--continue=true".to_string(),
            format!("--max-connection-per-server={}", threads),
            "--min-split-size=1M".to_string(),
            format!("--split={}", threads),
            "--max-concurrent-downloads=5".to_string(),
            "--disable-ipv6=true".to_string(),
            "--summary-interval=0".to_string(),
            "--quiet=true".to_string(),
        ];

        debug!(
            "启动aria2守护进程: {} {}",
            self.aria2_binary_path.display(),
            args.join(" ")
        );

        let child = tokio::process::Command::new(&self.aria2_binary_path)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start aria2 daemon")?;

        *process_guard = Some(child);
        drop(process_guard);

        // 等待aria2启动
        for i in 0..10 {
            sleep(Duration::from_millis(500)).await;
            if self.test_aria2_connection().await.is_ok() {
                info!("aria2守护进程已启动，端口: {}，线程数: {}", self.rpc_port, threads);
                return Ok(());
            }
            if i == 9 {
                bail!("aria2守护进程启动超时");
            }
        }

        Ok(())
    }

    /// 测试aria2连接
    async fn test_aria2_connection(&self) -> Result<()> {
        let url = format!("http://127.0.0.1:{}/jsonrpc", self.rpc_port);
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "aria2.getVersion",
            "id": "test",
            "params": [format!("token:{}", self.rpc_secret)]
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to connect to aria2")?;

        if response.status().is_success() {
            Ok(())
        } else {
            bail!("aria2 connection test failed: {}", response.status())
        }
    }

    /// 使用aria2下载文件，支持多个URL备选
    pub async fn fetch_with_aria2_fallback(&self, urls: &[&str], path: &Path) -> Result<()> {
        if urls.is_empty() {
            bail!("No URLs provided");
        }

        // 确保目标目录存在
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).context("Invalid file name")?;

        let dir = path
            .parent()
            .and_then(|p| p.to_str())
            .context("Invalid directory path")?;

        // 构建aria2 RPC请求
        let gid = self.add_download_task(urls, dir, file_name).await?;

        // 等待下载完成
        self.wait_for_download(&gid).await?;

        // 验证文件是否存在
        if !path.exists() {
            bail!("Download completed but file not found: {}", path.display());
        }

        Ok(())
    }

    /// 添加下载任务
    async fn add_download_task(&self, urls: &[&str], dir: &str, file_name: &str) -> Result<String> {
        let url = format!("http://127.0.0.1:{}/jsonrpc", self.rpc_port);

        // 从配置文件获取线程数
        let threads = CONFIG.concurrent_limit.parallel_download.threads;

        let options = serde_json::json!({
            "dir": dir,
            "out": file_name,
            "continue": "true",
            "max-connection-per-server": threads.to_string(),
            "split": threads.to_string(),
            "min-split-size": "1M",
            "header": [
                "User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
                "Referer: https://www.bilibili.com"
            ]
        });

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "aria2.addUri",
            "id": "add_download",
            "params": [
                format!("token:{}", self.rpc_secret),
                urls,
                options
            ]
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to add download task")?;

        let json: serde_json::Value = response.json().await?;

        if let Some(error) = json.get("error") {
            bail!("aria2 error: {}", error);
        }

        let gid = json["result"]
            .as_str()
            .context("Invalid response from aria2")?
            .to_string();

        info!("开始aria2下载: {} (线程数: {})", file_name, threads);
        debug!("添加下载任务成功，GID: {}", gid);
        Ok(gid)
    }

    /// 等待下载完成
    async fn wait_for_download(&self, gid: &str) -> Result<()> {
        let url = format!("http://127.0.0.1:{}/jsonrpc", self.rpc_port);

        loop {
            let payload = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "aria2.tellStatus",
                "id": "check_status",
                "params": [
                    format!("token:{}", self.rpc_secret),
                    gid
                ]
            });

            let response = self
                .client
                .post(&url)
                .json(&payload)
                .send()
                .await
                .context("Failed to check download status")?;

            let json: serde_json::Value = response.json().await?;

            if let Some(error) = json.get("error") {
                bail!("aria2 status check error: {}", error);
            }

            let result = &json["result"];
            let status = result["status"].as_str().unwrap_or("unknown");

            match status {
                "complete" => {
                    // 获取下载统计信息
                    let total_length = result["totalLength"].as_str().unwrap_or("0");
                    let completed_length = result["completedLength"].as_str().unwrap_or("0");

                    if let (Ok(total), Ok(completed)) = (total_length.parse::<u64>(), completed_length.parse::<u64>()) {
                        let total_mb = total as f64 / 1_048_576.0;
                        let completed_mb = completed as f64 / 1_048_576.0;
                        debug!(
                            "aria2下载完成，GID: {}，总大小: {:.2} MB，已完成: {:.2} MB",
                            gid, total_mb, completed_mb
                        );
                    } else {
                        debug!("aria2下载完成，GID: {}", gid);
                    }
                    return Ok(());
                }
                "error" => {
                    let error_msg = result["errorMessage"].as_str().unwrap_or("Unknown error");
                    bail!("Download failed: {}", error_msg);
                }
                "removed" => {
                    bail!("Download was removed");
                }
                "active" | "waiting" | "paused" => {
                    // 继续等待
                    sleep(Duration::from_millis(1000)).await;
                }
                _ => {
                    warn!("Unknown download status: {}", status);
                    sleep(Duration::from_millis(1000)).await;
                }
            }
        }
    }

    /// 获取文件的Content-Length
    #[allow(dead_code)]
    pub async fn get_content_length(&self, url: &str) -> Result<u64> {
        let response = self.client
            .head(url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Referer", "https://www.bilibili.com")
            .send()
            .await?;

        if let Some(content_length) = response.headers().get("content-length") {
            let length_str = content_length.to_str()?;
            let length = length_str.parse::<u64>()?;
            Ok(length)
        } else {
            bail!("Content-Length header not found");
        }
    }

    /// 智能下载：现在总是使用aria2下载
    #[allow(dead_code)]
    pub async fn smart_fetch(&self, url: &str, path: &Path) -> Result<()> {
        // 不再基于文件大小判断，直接使用aria2下载
        self.fetch_with_aria2_fallback(&[url], path).await
    }

    /// 简单的HTTP下载（用于小文件）
    #[allow(dead_code)]
    async fn simple_fetch(&self, url: &str, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36")
            .header("Referer", "https://www.bilibili.com")
            .send()
            .await?;
        let bytes = response.bytes().await?;
        tokio::fs::write(path, bytes).await?;

        Ok(())
    }

    /// 合并视频和音频文件（使用ffmpeg）
    pub async fn merge(&self, video_path: &Path, audio_path: &Path, output_path: &Path) -> Result<()> {
        // 这里需要调用ffmpeg来合并文件
        // 为了简化，我们先使用系统的ffmpeg命令
        let output = tokio::process::Command::new("ffmpeg")
            .args([
                "-i",
                video_path.to_str().unwrap(),
                "-i",
                audio_path.to_str().unwrap(),
                "-c",
                "copy",
                "-y", // 覆盖输出文件
                output_path.to_str().unwrap(),
            ])
            .output()
            .await
            .context("Failed to execute ffmpeg")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("ffmpeg failed: {}", stderr);
        }

        Ok(())
    }

    /// 重新启动aria2守护进程（用于配置更新）
    #[allow(dead_code)]
    pub async fn restart(&mut self) -> Result<()> {
        info!("配置已更新，重新启动aria2守护进程以应用新设置");

        // 先关闭现有进程
        self.shutdown().await?;

        // 重新启动
        self.start_aria2_daemon().await?;

        Ok(())
    }

    /// 优雅关闭aria2进程
    #[allow(dead_code)]
    pub async fn shutdown(&self) -> Result<()> {
        let mut process_guard = self.aria2_process.lock().await;

        if let Some(mut child) = process_guard.take() {
            // 尝试优雅关闭
            let url = format!("http://127.0.0.1:{}/jsonrpc", self.rpc_port);
            let payload = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "aria2.shutdown",
                "id": "shutdown",
                "params": [format!("token:{}", self.rpc_secret)]
            });

            let _ = self.client.post(&url).json(&payload).send().await;

            // 等待进程结束
            tokio::time::timeout(Duration::from_secs(5), async {
                let _ = child.wait().await;
            })
            .await
            .ok();

            // 如果还没结束，强制杀死
            let _ = child.kill().await;

            info!("aria2进程已关闭");
        }

        Ok(())
    }
}

impl Drop for Aria2Downloader {
    fn drop(&mut self) {
        // 在析构时尝试清理临时文件
        if self.aria2_binary_path.exists() {
            let _ = std::fs::remove_file(&self.aria2_binary_path);
        }
    }
}
