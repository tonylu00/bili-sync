use anyhow::Result;
use std::path::Path;
use tracing::{info, warn};

use crate::bilibili::Client;
use crate::downloader::Downloader;

#[cfg(feature = "aria2")]
use crate::aria2_downloader::{Aria2Downloader, Aria2LaunchOptions};

/// 统一下载器，可以在原生下载器和aria2下载器之间切换
pub enum UnifiedDownloader {
    Native(Downloader),
    #[cfg(feature = "aria2")]
    Aria2(Aria2Downloader),
}

impl UnifiedDownloader {
    /// 创建原生下载器
    #[allow(dead_code)]
    pub fn new_native(client: Client) -> Self {
        Self::Native(Downloader::new(client))
    }

    /// 创建aria2下载器
    #[cfg(feature = "aria2")]
    #[allow(dead_code)]
    pub async fn new_aria2(client: Client, options: Aria2LaunchOptions) -> Result<Self> {
        let aria2_downloader = Aria2Downloader::new(client, options).await?;
        Ok(Self::Aria2(aria2_downloader))
    }

    /// 智能创建下载器：根据配置决定使用哪种下载器
    pub async fn new_smart(client: Client) -> Result<Self> {
        // 获取最新配置
        let config = crate::config::reload_config();

        // 检查是否启用了多线程下载
        if !config.concurrent_limit.parallel_download.enabled {
            info!("多线程下载已禁用，使用原生下载器");
            return Ok(Self::Native(Downloader::new(client)));
        }

        #[cfg(feature = "aria2")]
        {
            let user_binary = config.concurrent_limit.parallel_download.aria2_binary_path.clone();
            let options = Aria2LaunchOptions::new(user_binary);
            match Aria2Downloader::new(client.clone(), options).await {
                Ok(aria2_downloader) => {
                    info!("成功初始化aria2并行下载器");
                    return Ok(Self::Aria2(aria2_downloader));
                }
                Err(err) => {
                    warn!("初始化aria2失败，改用原生多线程: {:#}", err);
                }
            }
        }

        #[cfg(not(feature = "aria2"))]
        warn!("当前构建未启用aria2特性，使用原生多线程下载器");

        info!("使用原生多线程下载器");
        Ok(Self::Native(Downloader::new(client)))
    }

    /// 下载文件，支持多个URL备选
    pub async fn fetch_with_fallback(&self, urls: &[&str], path: &Path) -> Result<()> {
        match self {
            Self::Native(downloader) => downloader.fetch_with_fallback(urls, path).await,
            #[cfg(feature = "aria2")]
            Self::Aria2(downloader) => downloader.fetch_with_aria2_fallback(urls, path).await,
        }
    }

    /// 合并视频和音频文件
    pub async fn merge(&self, video_path: &Path, audio_path: &Path, output_path: &Path) -> Result<()> {
        match self {
            Self::Native(downloader) => downloader.merge(video_path, audio_path, output_path).await,
            #[cfg(feature = "aria2")]
            Self::Aria2(downloader) => downloader.merge(video_path, audio_path, output_path).await,
        }
    }

    /// 智能下载：根据文件大小和配置决定使用哪种下载方式
    #[allow(dead_code)]
    pub async fn smart_fetch(&self, url: &str, path: &Path) -> Result<()> {
        match self {
            Self::Native(downloader) => {
                // 原生下载器根据配置自动选择单线程或多线程
                info!("原生下载器使用内置多线程逻辑");
                downloader.fetch(url, path).await
            }
            #[cfg(feature = "aria2")]
            Self::Aria2(downloader) => {
                // aria2下载器：使用智能下载功能
                downloader.smart_fetch(url, path).await
            }
        }
    }

    /// 重新启动下载器（用于配置更新）
    #[allow(dead_code)]
    pub async fn restart(&mut self) -> Result<()> {
        match self {
            Self::Native(_) => {
                // 原生下载器不需要重启操作
                Ok(())
            }
            #[cfg(feature = "aria2")]
            Self::Aria2(downloader) => downloader.restart().await,
        }
    }

    /// 优雅关闭下载器
    #[allow(dead_code)]
    pub async fn shutdown(&self) -> Result<()> {
        match self {
            Self::Native(_) => {
                // 原生下载器不需要特殊关闭操作
                Ok(())
            }
            #[cfg(feature = "aria2")]
            Self::Aria2(downloader) => downloader.shutdown().await,
        }
    }

    /// 检查是否为aria2下载器
    #[allow(dead_code)]
    pub fn is_aria2(&self) -> bool {
        #[cfg(feature = "aria2")]
        {
            matches!(self, Self::Aria2(_))
        }
        #[cfg(not(feature = "aria2"))]
        {
            false
        }
    }

    /// 检查是否为原生下载器
    #[allow(dead_code)]
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Native(_))
    }
}
