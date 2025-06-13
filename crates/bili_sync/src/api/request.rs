use serde::Deserialize;
use utoipa::IntoParams;
use utoipa::ToSchema;

#[derive(Deserialize, IntoParams, Default)]
pub struct VideosRequest {
    pub collection: Option<i32>,
    pub favorite: Option<i32>,
    pub submission: Option<i32>,
    pub watch_later: Option<i32>,
    pub bangumi: Option<i32>,
    pub query: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// 添加新视频源的请求结构体
#[derive(Deserialize, IntoParams, ToSchema)]
pub struct AddVideoSourceRequest {
    // 视频源类型: "collection", "favorite", "submission", "watch_later", "bangumi"
    pub source_type: String,
    // 视频源ID: 收藏夹ID、合集ID、UP主ID等
    pub source_id: String,
    // UP主ID: 仅当source_type为"collection"时需要
    pub up_id: Option<String>,
    // 视频源名称
    pub name: String,
    // 保存路径
    pub path: String,
    // 合集类型: "season"(视频合集) 或 "series"(视频列表)，仅当source_type为"collection"时有效
    pub collection_type: Option<String>,
    // 番剧特有字段
    pub media_id: Option<String>,
    pub ep_id: Option<String>,
    // 是否下载全部季度，仅当source_type为"bangumi"时有效
    pub download_all_seasons: Option<bool>,
    // 选中的季度ID列表，仅当source_type为"bangumi"且download_all_seasons为false时有效
    pub selected_seasons: Option<Vec<String>>,
}

// 删除视频源的请求结构体
#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteVideoSourceRequest {
    pub delete_local_files: bool,
}

// 更新视频源启用状态的请求结构体
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateVideoSourceEnabledRequest {
    pub enabled: bool,
}

// 更新配置的请求结构体
#[derive(Deserialize, IntoParams, ToSchema)]
pub struct UpdateConfigRequest {
    // 视频命名模板
    pub video_name: Option<String>,
    // 分页命名模板
    pub page_name: Option<String>,
    // 多P视频分页命名模板
    pub multi_page_name: Option<String>,
    // 番剧分页命名模板
    pub bangumi_name: Option<String>,
    // 文件夹结构模板
    pub folder_structure: Option<String>,
    // 时间格式
    pub time_format: Option<String>,
    // 扫描间隔（秒）
    pub interval: Option<u64>,
    // NFO时间类型
    pub nfo_time_type: Option<String>,
    // 多线程下载配置
    pub parallel_download_enabled: Option<bool>,
    pub parallel_download_threads: Option<usize>,
    // 视频质量设置
    pub video_max_quality: Option<String>,
    pub video_min_quality: Option<String>,
    pub audio_max_quality: Option<String>,
    pub audio_min_quality: Option<String>,
    pub codecs: Option<Vec<String>>,
    pub no_dolby_video: Option<bool>,
    pub no_dolby_audio: Option<bool>,
    pub no_hdr: Option<bool>,
    pub no_hires: Option<bool>,
    // 弹幕设置
    pub danmaku_duration: Option<f64>,
    pub danmaku_font: Option<String>,
    pub danmaku_font_size: Option<u32>,
    pub danmaku_width_ratio: Option<f64>,
    pub danmaku_horizontal_gap: Option<f64>,
    pub danmaku_lane_size: Option<u32>,
    pub danmaku_float_percentage: Option<f64>,
    pub danmaku_bottom_percentage: Option<f64>,
    pub danmaku_opacity: Option<u8>,
    pub danmaku_bold: Option<bool>,
    pub danmaku_outline: Option<f64>,
    pub danmaku_time_offset: Option<f64>,
    // 并发控制设置
    pub concurrent_video: Option<usize>,
    pub concurrent_page: Option<usize>,
    pub rate_limit: Option<usize>,
    pub rate_duration: Option<u64>,
    // 其他设置
    pub cdn_sorting: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchRequest {
    pub keyword: String,     // 搜索关键词
    pub search_type: String, // 搜索类型：video, bili_user, media_bangumi
    #[serde(default = "default_page")]
    pub page: u32, // 页码，默认1
    #[serde(default = "default_page_size")]
    pub page_size: u32, // 每页数量，默认20
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

// 状态更新结构
#[derive(Deserialize, ToSchema)]
pub struct StatusUpdate {
    pub status_index: usize, // 状态位索引 (0-4)
    pub status_value: u32,   // 状态值 (0, 1, 2, 3)
}

// 分页状态更新结构
#[derive(Deserialize, ToSchema)]
pub struct PageStatusUpdate {
    pub page_id: i32,
    pub updates: Vec<StatusUpdate>,
}

// 更新视频状态请求
#[derive(Deserialize, ToSchema)]
pub struct UpdateVideoStatusRequest {
    #[serde(default)]
    pub video_updates: Vec<StatusUpdate>,
    #[serde(default)]
    pub page_updates: Vec<PageStatusUpdate>,
}
