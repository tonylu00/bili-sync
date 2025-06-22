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
#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum NFOTimeType {
    #[default]
    FavTime,
    PubTime,
}

/// 多线程下载配置
#[derive(Serialize, Deserialize, Debug, Clone)]
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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConcurrentLimit {
    pub video: usize,
    pub page: usize,
    pub rate_limit: Option<RateLimit>,
    #[serde(default)]
    pub parallel_download: ParallelDownloadConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

/// UP主投稿风控配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubmissionRiskControlConfig {
    /// 大量视频UP主的阈值（超过此数量视为大量视频UP主）
    #[serde(default = "default_large_submission_threshold")]
    pub large_submission_threshold: usize,
    /// 基础请求间隔（毫秒）
    #[serde(default = "default_base_request_delay")]
    pub base_request_delay: u64,
    /// 大量视频UP主的额外延迟倍数
    #[serde(default = "default_large_submission_delay_multiplier")]
    pub large_submission_delay_multiplier: u64,
    /// 是否启用渐进式延迟（请求次数越多，延迟越长）
    #[serde(default = "default_enable_progressive_delay")]
    pub enable_progressive_delay: bool,
    /// 渐进式延迟的最大倍数
    #[serde(default = "default_max_delay_multiplier")]
    pub max_delay_multiplier: u64,
    /// 是否启用增量获取（只获取比上次扫描更新的视频）
    #[serde(default = "default_enable_incremental_fetch")]
    pub enable_incremental_fetch: bool,
    /// 增量获取失败时是否回退到全量获取
    #[serde(default = "default_incremental_fallback_to_full")]
    pub incremental_fallback_to_full: bool,
    /// 是否启用分批处理（大量视频的UP主分批请求）
    #[serde(default = "default_enable_batch_processing")]
    pub enable_batch_processing: bool,
    /// 分批处理的批次大小（每批处理的页数）
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    /// 批次间延迟（秒）
    #[serde(default = "default_batch_delay_seconds")]
    pub batch_delay_seconds: u64,
    /// 是否启用自动退避机制（检测到风控时自动增加延迟）
    #[serde(default = "default_enable_auto_backoff")]
    pub enable_auto_backoff: bool,
    /// 自动退避的基础时间（秒）
    #[serde(default = "default_auto_backoff_base_seconds")]
    pub auto_backoff_base_seconds: u64,
    /// 自动退避的最大倍数
    #[serde(default = "default_auto_backoff_max_multiplier")]
    pub auto_backoff_max_multiplier: u64,
}

fn default_large_submission_threshold() -> usize {
    300
}

fn default_base_request_delay() -> u64 {
    200
}

fn default_large_submission_delay_multiplier() -> u64 {
    2
}

fn default_enable_progressive_delay() -> bool {
    true
}

fn default_max_delay_multiplier() -> u64 {
    4
}

fn default_enable_incremental_fetch() -> bool {
    true
}

fn default_incremental_fallback_to_full() -> bool {
    true
}

fn default_enable_batch_processing() -> bool {
    false // 默认不启用，需要手动开启
}

fn default_batch_size() -> usize {
    5 // 每批5页，约150个视频
}

fn default_batch_delay_seconds() -> u64 {
    2 // 批次间延迟2秒
}

fn default_enable_auto_backoff() -> bool {
    true // 默认启用自动退避
}

fn default_auto_backoff_base_seconds() -> u64 {
    10 // 自动退避基础时间10秒
}

fn default_auto_backoff_max_multiplier() -> u64 {
    5 // 最大退避到50秒
}

impl Default for SubmissionRiskControlConfig {
    fn default() -> Self {
        Self {
            large_submission_threshold: default_large_submission_threshold(),
            base_request_delay: default_base_request_delay(),
            large_submission_delay_multiplier: default_large_submission_delay_multiplier(),
            enable_progressive_delay: default_enable_progressive_delay(),
            max_delay_multiplier: default_max_delay_multiplier(),
            enable_incremental_fetch: default_enable_incremental_fetch(),
            incremental_fallback_to_full: default_incremental_fallback_to_full(),
            enable_batch_processing: default_enable_batch_processing(),
            batch_size: default_batch_size(),
            batch_delay_seconds: default_batch_delay_seconds(),
            enable_auto_backoff: default_enable_auto_backoff(),
            auto_backoff_base_seconds: default_auto_backoff_base_seconds(),
            auto_backoff_max_multiplier: default_auto_backoff_max_multiplier(),
        }
    }
}

#[allow(dead_code)]
pub trait PathSafeTemplate {
    fn path_safe_register(&mut self, name: &'static str, template: &'static str) -> Result<()>;
    fn path_safe_render(&self, name: &'static str, data: &serde_json::Value) -> Result<String>;
}

/// 通过将模板字符串中的分隔符替换为自定义的字符串，使得模板字符串中的分隔符得以保留
impl PathSafeTemplate for handlebars::Handlebars<'_> {
    fn path_safe_register(&mut self, name: &'static str, template: &'static str) -> Result<()> {
        // 同时处理正斜杠和反斜杠，确保跨平台兼容性
        let safe_template = template
            .replace(['/', '\\'], "__SEP__");
        Ok(self.register_template_string(name, safe_template)?)
    }

    fn path_safe_render(&self, name: &'static str, data: &serde_json::Value) -> Result<String> {
        Ok(filenamify(&self.render(name, data)?).replace("__SEP__", std::path::MAIN_SEPARATOR_STR))
    }
}
