use anyhow::{anyhow, Result};
use reqwest::Client;
use std::fmt;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use super::notification_bark::{self, BarkLevel, DeviceKeySelection};
use super::notification_serverchan;
use crate::config::{NotificationConfig, NotificationMethod};

// 推送通知客户端
pub struct NotificationClient {
    client: Client,
    config: NotificationConfig,
}

// 扫描结果数据结构
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NewVideoInfo {
    pub title: String,
    pub bvid: String,
    pub upper_name: String,
    pub source_type: String,
    pub source_name: String,
    pub pubtime: Option<String>, // 使用字符串格式的北京时间
    pub episode_number: Option<i32>,
    pub season_number: Option<i32>,
    pub video_id: Option<i32>, // 添加视频ID字段，用于过滤删除队列中的视频
}

#[derive(Debug, Clone)]
pub struct SourceScanResult {
    pub source_type: String,
    pub source_name: String,
    pub new_videos: Vec<NewVideoInfo>,
}

#[derive(Debug, Clone)]
pub struct ScanSummary {
    pub total_sources: usize,
    pub total_new_videos: usize,
    pub scan_duration: Duration,
    pub source_results: Vec<SourceScanResult>,
}

#[derive(Debug, Clone)]
pub struct DownloadFailureNotification {
    pub source_type: String,
    pub source_name: String,
    pub video_title: Option<String>,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct RiskControlNotification {
    pub source_type: Option<String>,
    pub source_name: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct NotificationMessage {
    pub title: String,
    pub subtitle: Option<String>,
    pub body_markdown: String,
    pub body_plain: String,
    pub level: Option<BarkLevel>,
    pub volume: Option<u8>,
    pub badge: Option<u32>,
    pub call: Option<bool>,
    pub auto_copy: Option<bool>,
    pub copy: Option<String>,
    pub sound: Option<String>,
    pub icon: Option<String>,
    pub group: Option<String>,
    pub ciphertext: Option<String>,
    pub is_archive: Option<bool>,
    pub url: Option<String>,
    pub action: Option<String>,
    pub id: Option<String>,
    pub delete: Option<bool>,
}

impl NotificationMessage {
    pub fn new(title: impl Into<String>, body_markdown: impl Into<String>) -> Self {
        let title = sanitize_text(title.into().trim());
        let body_markdown = sanitize_text(body_markdown.into().trim());
        let body_plain = markdown_to_plain_text(&body_markdown);

        Self {
            title,
            subtitle: None,
            body_markdown,
            body_plain,
            level: None,
            volume: None,
            badge: None,
            call: None,
            auto_copy: None,
            copy: None,
            sound: None,
            icon: None,
            group: None,
            ciphertext: None,
            is_archive: None,
            url: None,
            action: None,
            id: None,
            delete: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum NotificationEventKind {
    ScanSummary,
    SourceUpdate,
    DownloadFailure,
    RiskControl,
    Custom(&'static str),
}

impl NotificationEventKind {
    fn as_str(self) -> &'static str {
        match self {
            NotificationEventKind::ScanSummary => "scan_summary",
            NotificationEventKind::SourceUpdate => "source_update",
            NotificationEventKind::DownloadFailure => "download_failure",
            NotificationEventKind::RiskControl => "risk_control",
            NotificationEventKind::Custom(label) => label,
        }
    }
}

impl fmt::Display for NotificationEventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl NotificationClient {
    pub fn new(config: NotificationConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.notification_timeout))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    pub async fn send_scan_completion(&self, summary: &ScanSummary) -> Result<()> {
        if !self.should_send(NotificationEventKind::ScanSummary) {
            debug!("扫描摘要推送已禁用，跳过发送");
            return Ok(());
        }

        if summary.total_new_videos < self.config.notification_min_videos {
            debug!(
                "新增视频数量({})未达到推送阈值({})",
                summary.total_new_videos, self.config.notification_min_videos
            );
            return Ok(());
        }

        let mut result = Ok(());
        let summary_message = self.build_scan_summary_message(summary);
        if let Err(err) = self
            .dispatch_with_retry(NotificationEventKind::ScanSummary, summary_message)
            .await
        {
            error!("扫描摘要推送发送失败: {}", err);
            result = Err(err);
        }

        if self.should_send(NotificationEventKind::SourceUpdate) {
            for source in summary.source_results.iter().filter(|s| !s.new_videos.is_empty()) {
                let message = self.build_source_update_message(source);
                if let Err(err) = self
                    .dispatch_with_retry(NotificationEventKind::SourceUpdate, message)
                    .await
                {
                    warn!(
                        "源更新推送发送失败 (源: {} - {}): {}",
                        source.source_type, source.source_name, err
                    );
                    result = Err(err);
                }
            }
        }

        result
    }

    pub async fn send_download_failure(&self, details: DownloadFailureNotification) -> Result<()> {
        if !self.should_send(NotificationEventKind::DownloadFailure) {
            debug!("下载失败推送已禁用，跳过发送");
            return Ok(());
        }

        let message = self.build_download_failure_message(&details);
        self.dispatch_with_retry(NotificationEventKind::DownloadFailure, message)
            .await
    }

    pub async fn send_risk_control(&self, details: RiskControlNotification) -> Result<()> {
        if !self.should_send(NotificationEventKind::RiskControl) {
            debug!("风控推送已禁用，跳过发送");
            return Ok(());
        }

        let message = self.build_risk_control_message(&details);
        self.dispatch_with_retry(NotificationEventKind::RiskControl, message)
            .await
    }

    pub async fn test_notification(&self) -> Result<()> {
        let message = NotificationMessage::new(
            "Bili Sync 测试推送",
            "这是一条测试推送消息，如果您收到此消息，说明推送配置正确。\n\n🎉 推送功能工作正常！",
        );
        self.dispatch_with_retry(NotificationEventKind::Custom("test"), message)
            .await
    }

    pub async fn send_custom_test(&self, message: &str) -> Result<()> {
        let message = NotificationMessage::new("Bili Sync 自定义测试推送", format!("🧪 自定义测试消息\n\n{}", message));
        self.dispatch_with_retry(NotificationEventKind::Custom("custom_test"), message)
            .await
    }

    fn should_send(&self, kind: NotificationEventKind) -> bool {
        match kind {
            NotificationEventKind::Custom(_) => true,
            _ => {
                if !self.config.enable_scan_notifications {
                    return false;
                }

                match kind {
                    NotificationEventKind::ScanSummary => self.config.events.scan_summary,
                    NotificationEventKind::SourceUpdate => self.config.events.source_updates,
                    NotificationEventKind::DownloadFailure => self.config.events.download_failures,
                    NotificationEventKind::RiskControl => self.config.events.risk_control,
                    NotificationEventKind::Custom(_) => true,
                }
            }
        }
    }

    async fn dispatch_with_retry(&self, kind: NotificationEventKind, message: NotificationMessage) -> Result<()> {
        let retry_count = self.config.notification_retry_count.max(1) as usize;
        let mut last_error: Option<anyhow::Error> = None;

        for attempt in 1..=retry_count {
            match self.send_once(kind, message.clone()).await {
                Ok(_) => {
                    info!("{} 推送发送成功", kind);
                    return Ok(());
                }
                Err(err) => {
                    warn!("{} 推送发送失败 (尝试 {}/{}): {}", kind, attempt, retry_count, err);
                    last_error = Some(err);

                    if attempt < retry_count {
                        sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("{} 推送发送失败", kind)))
    }

    async fn send_once(&self, _kind: NotificationEventKind, message: NotificationMessage) -> Result<()> {
        match self.config.method {
            NotificationMethod::Serverchan => {
                let key = self
                    .config
                    .serverchan_key
                    .as_ref()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| anyhow!("未配置Server酱 SendKey"))?;

                notification_serverchan::send(&self.client, key, &message.title, &message.body_markdown).await
            }
            NotificationMethod::Bark => {
                let keys = self.bark_device_selection()?;
                let payload = notification_bark::BarkPayload::from_message(&message, &self.config.bark_defaults, keys)?;
                notification_bark::send(&self.client, &self.effective_bark_server(), payload).await
            }
        }
    }

    fn bark_device_selection(&self) -> Result<DeviceKeySelection> {
        let single = self
            .config
            .bark_device_key
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());

        let multi: Vec<String> = self
            .config
            .bark_device_keys
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .collect();

        if single.is_none() && multi.is_empty() {
            return Err(anyhow!("未配置 Bark Device Key"));
        }

        Ok(DeviceKeySelection {
            device_key: single,
            device_keys: multi,
        })
    }

    fn effective_bark_server(&self) -> String {
        let server = self.config.bark_server.trim();
        if server.is_empty() {
            "https://api.day.app".to_string()
        } else {
            server.trim_end_matches('/').to_string()
        }
    }

    fn build_scan_summary_message(&self, summary: &ScanSummary) -> NotificationMessage {
        let title = "Bili Sync 扫描完成";
        let body = format_scan_summary(summary);
        NotificationMessage::new(title, body)
    }

    fn build_source_update_message(&self, source: &SourceScanResult) -> NotificationMessage {
        let sanitized_source = sanitize_text(&source.source_name);
        let title = format!("{} 有 {} 个新视频", sanitized_source, source.new_videos.len());

        let mut body = format!(
            "**源类型**: {}\n**源名称**: {}\n\n",
            source.source_type, sanitized_source
        );

        const MAX_VIDEOS: usize = 10;
        for video in source.new_videos.iter().take(MAX_VIDEOS) {
            let clean_title = sanitize_text(&video.title);
            let mut line = format!("- [{}](https://www.bilibili.com/video/{})", clean_title, video.bvid);

            if let Some(pubtime) = &video.pubtime {
                if let Some(date) = pubtime.split(' ').next() {
                    line.push_str(&format!(" ({})", date));
                }
            }

            body.push_str(&line);
            body.push('\n');
        }

        if source.new_videos.len() > MAX_VIDEOS {
            body.push_str(&format!(
                "...还有 {} 个视频（内容过长已省略）\n",
                source.new_videos.len() - MAX_VIDEOS
            ));
        }

        NotificationMessage::new(title, body)
    }

    fn build_download_failure_message(&self, details: &DownloadFailureNotification) -> NotificationMessage {
        let title = format!("{} 下载失败", sanitize_text(&details.source_name));
        let mut body = format!(
            "**源类型**: {}\n**源名称**: {}\n",
            sanitize_text(&details.source_type),
            sanitize_text(&details.source_name)
        );

        if let Some(title) = &details.video_title {
            body.push_str(&format!("**视频标题**: {}\n", sanitize_text(title)));
        }

        body.push_str("\n**错误信息**:\n");
        body.push_str("````\n");
        body.push_str(&sanitize_text(&details.error));
        body.push_str("\n````");

        NotificationMessage::new(title, body)
    }

    fn build_risk_control_message(&self, details: &RiskControlNotification) -> NotificationMessage {
        let mut title = "检测到风控".to_string();
        if let Some(source_name) = &details.source_name {
            title = format!("{} 触发风控", sanitize_text(source_name));
        }

        let mut body = String::new();
        if let Some(source_type) = &details.source_type {
            body.push_str(&format!("**源类型**: {}\n", sanitize_text(source_type)));
        }
        if let Some(source_name) = &details.source_name {
            body.push_str(&format!("**源名称**: {}\n", sanitize_text(source_name)));
        }

        body.push_str("\n**详细信息**:\n");
        body.push_str(&sanitize_text(&details.message));

        NotificationMessage::new(title, body)
    }
}

fn sanitize_text(text: &str) -> String {
    text.replace('「', "[")
        .replace('」', "]")
        .replace('【', "[")
        .replace('】', "]")
        .replace('〖', "[")
        .replace('〗', "]")
        .replace('〔', "[")
        .replace('〕', "]")
        .chars()
        .filter(|c| c.is_ascii() || (*c as u32) < 0x10000)
        .collect()
}

fn format_scan_summary(summary: &ScanSummary) -> String {
    const MAX_CONTENT_LENGTH: usize = 30_000;

    let mut content = format!(
        "📊 **扫描摘要**\n\n- 扫描视频源: {}个\n- 新增视频: {}个\n- 扫描耗时: {:.1}分钟\n\n",
        summary.total_sources,
        summary.total_new_videos,
        summary.scan_duration.as_secs_f64() / 60.0
    );

    if summary.total_new_videos > 0 {
        content.push_str("📹 **新增视频详情**\n\n");

        let mut videos_shown = 0;
        let mut sources_shown = 0;

        for source_result in &summary.source_results {
            if source_result.new_videos.is_empty() {
                continue;
            }

            if content.len() > MAX_CONTENT_LENGTH - 500 {
                let remaining_videos = summary.total_new_videos - videos_shown;
                let remaining_sources = summary
                    .source_results
                    .iter()
                    .filter(|s| !s.new_videos.is_empty())
                    .count()
                    - sources_shown;
                content.push_str(&format!(
                    "\n...还有 {} 个视频源的 {} 个新视频（内容过长已省略）\n",
                    remaining_sources, remaining_videos
                ));
                break;
            }

            sources_shown += 1;

            let icon = match source_result.source_type.as_str() {
                "收藏夹" => "🎬",
                "合集" => "📁",
                "UP主投稿" => "🎯",
                "稍后再看" => "⏰",
                "番剧" => "📺",
                _ => "📄",
            };

            let clean_source_name = sanitize_text(&source_result.source_name);
            content.push_str(&format!(
                "{} **{}** - {} ({}个新视频):\n",
                icon,
                source_result.source_type,
                clean_source_name,
                source_result.new_videos.len()
            ));

            let mut sorted_videos = source_result.new_videos.clone();
            if source_result.source_type == "番剧" {
                sorted_videos.sort_by(|a, b| b.episode_number.unwrap_or(0).cmp(&a.episode_number.unwrap_or(0)));
            } else {
                sorted_videos.sort_by(|a, b| {
                    b.pubtime
                        .as_ref()
                        .unwrap_or(&String::new())
                        .cmp(a.pubtime.as_ref().unwrap_or(&String::new()))
                });
            }

            let max_videos_per_source = 20;
            let videos_to_show = sorted_videos.len().min(max_videos_per_source);

            for (idx, video) in sorted_videos.iter().take(videos_to_show).enumerate() {
                if content.len() > MAX_CONTENT_LENGTH - 1000 {
                    content.push_str(&format!(
                        "...还有 {} 个视频（内容过长已省略）\n",
                        sorted_videos.len() - idx
                    ));
                    break;
                }

                videos_shown += 1;

                let clean_title = sanitize_text(&video.title);
                let mut line = format!("- [{}](https://www.bilibili.com/video/{})", clean_title, video.bvid);

                if source_result.source_type == "番剧" && video.episode_number.is_some() {
                    line.push_str(&format!(" (第{}集", video.episode_number.unwrap()));
                    if let Some(pubtime) = &video.pubtime {
                        if let Some(date_part) = pubtime.split(' ').next() {
                            line.push_str(&format!(", {}", date_part));
                        }
                    }
                    line.push(')');
                } else if let Some(pubtime) = &video.pubtime {
                    if let Some(date_part) = pubtime.split(' ').next() {
                        line.push_str(&format!(" ({})", date_part));
                    }
                }

                content.push_str(&line);
                content.push('\n');
            }

            if sorted_videos.len() > videos_to_show {
                content.push_str(&format!("...还有 {} 个视频\n", sorted_videos.len() - videos_to_show));
            }

            content.push('\n');
        }
    }

    let clean_content = sanitize_text(&content);

    if clean_content.len() > MAX_CONTENT_LENGTH {
        let mut truncated = clean_content.chars().take(MAX_CONTENT_LENGTH - 100).collect::<String>();
        truncated.push_str("\n\n...内容过长，已截断");
        truncated
    } else {
        clean_content
    }
}

fn markdown_to_plain_text(markdown: &str) -> String {
    let mut plain = String::with_capacity(markdown.len());
    let mut chars = markdown.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '[' => {
                let mut label = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == ']' {
                        break;
                    }
                    label.push(next);
                }

                if let Some(&'(') = chars.peek() {
                    chars.next();
                    let mut depth = 1;
                    while let Some(next) = chars.next() {
                        if next == '(' {
                            depth += 1;
                        } else if next == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                }

                plain.push_str(&label);
            }
            '*' | '`' | '_' => {}
            '-' => {
                if matches!(chars.peek(), Some(' ')) {
                    plain.push('•');
                    plain.push(' ');
                    chars.next();
                } else {
                    plain.push('-');
                }
            }
            _ => plain.push(c),
        }
    }

    plain
}

// 便捷函数
pub async fn send_scan_notification(summary: ScanSummary) -> Result<()> {
    let config = crate::config::reload_config().notification;
    let client = NotificationClient::new(config);
    client.send_scan_completion(&summary).await
}

pub async fn send_download_failure_notification(details: DownloadFailureNotification) -> Result<()> {
    let config = crate::config::reload_config().notification;
    let client = NotificationClient::new(config);
    client.send_download_failure(details).await
}

pub async fn send_risk_control_notification(details: RiskControlNotification) -> Result<()> {
    let config = crate::config::reload_config().notification;
    let client = NotificationClient::new(config);
    client.send_risk_control(details).await
}

#[allow(dead_code)]
pub async fn test_notification() -> Result<()> {
    let config = crate::config::reload_config().notification;
    let client = NotificationClient::new(config);
    client.test_notification().await
}

#[allow(dead_code)]
pub async fn send_custom_test_notification(message: &str) -> Result<()> {
    let config = crate::config::reload_config().notification;
    let client = NotificationClient::new(config);
    client.send_custom_test(message).await
}
