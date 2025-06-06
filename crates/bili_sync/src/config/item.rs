use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::utils::filenamify::filenamify;

/// 稍后再看的配置
#[derive(Serialize, Deserialize, Default)]
pub struct WatchLaterConfig {
    pub enabled: bool,
    pub path: PathBuf,
}

/// NFO 文件使用的时间类型
#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NFOTimeType {
    #[default]
    FavTime,
    PubTime,
}

/// 多线程下载配置
#[derive(Serialize, Deserialize)]
pub struct ParallelDownloadConfig {
    /// 是否启用多线程下载
    #[serde(default = "default_parallel_download_enabled")]
    pub enabled: bool,
    /// 每个文件的下载线程数
    #[serde(default = "default_parallel_download_threads")]
    pub threads: usize,
}

fn default_parallel_download_enabled() -> bool {
    true
}

fn default_parallel_download_threads() -> usize {
    4
}

impl Default for ParallelDownloadConfig {
    fn default() -> Self {
        Self {
            enabled: default_parallel_download_enabled(),
            threads: default_parallel_download_threads(),
        }
    }
}

/// 并发下载相关的配置
#[derive(Serialize, Deserialize)]
pub struct ConcurrentLimit {
    pub video: usize,
    pub page: usize,
    pub rate_limit: Option<RateLimit>,
    #[serde(default)]
    pub parallel_download: ParallelDownloadConfig,
}

#[derive(Serialize, Deserialize)]
pub struct RateLimit {
    pub limit: usize,
    pub duration: u64,
}

impl Default for ConcurrentLimit {
    fn default() -> Self {
        Self {
            video: 3,
            page: 2,
            // 默认的限速配置，每 250ms 允许请求 4 次
            rate_limit: Some(RateLimit {
                limit: 4,
                duration: 250,
            }),
            parallel_download: ParallelDownloadConfig::default(),
        }
    }
}

pub trait PathSafeTemplate {
    fn path_safe_register(&mut self, name: &'static str, template: &'static str) -> Result<()>;
    fn path_safe_render(&self, name: &'static str, data: &serde_json::Value) -> Result<String>;
}

/// 通过将模板字符串中的分隔符替换为自定义的字符串，使得模板字符串中的分隔符得以保留
impl PathSafeTemplate for handlebars::Handlebars<'_> {
    fn path_safe_register(&mut self, name: &'static str, template: &'static str) -> Result<()> {
        Ok(self.register_template_string(name, template.replace(std::path::MAIN_SEPARATOR_STR, "__SEP__"))?)
    }

    fn path_safe_render(&self, name: &'static str, data: &serde_json::Value) -> Result<String> {
        Ok(filenamify(&self.render(name, data)?).replace("__SEP__", std::path::MAIN_SEPARATOR_STR))
    }
}
