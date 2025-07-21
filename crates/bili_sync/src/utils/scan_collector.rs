use std::collections::HashMap;
use std::time::Instant;

use crate::adapter::{VideoSource, VideoSourceEnum};
use crate::utils::notification::{NewVideoInfo, ScanSummary, SourceScanResult};

/// 扫描收集器，用于收集每次完整扫描的统计信息
pub struct ScanCollector {
    start_time: Instant,
    source_results: HashMap<String, SourceScanResult>,
    total_sources: usize,
}

impl ScanCollector {
    /// 创建新的扫描收集器
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            source_results: HashMap::new(),
            total_sources: 0,
        }
    }

    /// 记录一个视频源的扫描开始
    pub fn start_source(&mut self, video_source: &VideoSourceEnum) {
        self.total_sources += 1;
        
        let key = self.get_source_key(video_source);
        let result = SourceScanResult {
            source_type: video_source.source_type_display(),
            source_name: video_source.source_name_display(),
            new_videos: Vec::new(),
        };
        
        self.source_results.insert(key, result);
    }

    /// 记录新增的视频信息
    pub fn add_new_video(&mut self, video_source: &VideoSourceEnum, video_info: NewVideoInfo) {
        let key = self.get_source_key(video_source);
        if let Some(result) = self.source_results.get_mut(&key) {
            result.new_videos.push(video_info);
        }
    }

    /// 批量添加新增视频信息
    pub fn add_new_videos(&mut self, video_source: &VideoSourceEnum, videos: Vec<NewVideoInfo>) {
        let key = self.get_source_key(video_source);
        if let Some(result) = self.source_results.get_mut(&key) {
            result.new_videos.extend(videos);
        }
    }

    /// 生成扫描摘要
    pub fn generate_summary(self) -> ScanSummary {
        let scan_duration = self.start_time.elapsed();
        let total_new_videos = self.source_results
            .values()
            .map(|result| result.new_videos.len())
            .sum();

        let source_results = self.source_results
            .into_values()
            .collect();

        ScanSummary {
            total_sources: self.total_sources,
            total_new_videos,
            scan_duration,
            source_results,
        }
    }

    /// 获取当前总的新增视频数量
    pub fn total_new_videos(&self) -> usize {
        self.source_results
            .values()
            .map(|result| result.new_videos.len())
            .sum()
    }

    /// 生成视频源的唯一键
    fn get_source_key(&self, video_source: &VideoSourceEnum) -> String {
        format!("{}:{}", video_source.source_type_display(), video_source.source_id_display())
    }
}

impl Default for ScanCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 从视频信息创建NewVideoInfo结构
pub fn create_new_video_info(
    title: &str,
    bvid: &str,
    upper_name: &str,
    video_source: &VideoSourceEnum,
) -> NewVideoInfo {
    NewVideoInfo {
        title: title.to_string(),
        bvid: bvid.to_string(),
        upper_name: upper_name.to_string(),
        source_type: video_source.source_type_display(),
        source_name: video_source.source_name_display(),
    }
}