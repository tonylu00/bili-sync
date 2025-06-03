use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::utils::status::{PageStatus, VideoStatus};

#[derive(Debug, Serialize, ToSchema, Default)]
pub struct VideoSourcesResponse {
    #[serde(default)]
    pub collection: Vec<VideoSource>,
    #[serde(default)]
    pub favorite: Vec<VideoSource>,
    #[serde(default)]
    pub submission: Vec<VideoSource>,
    #[serde(default)]
    pub watch_later: Vec<VideoSource>,
    #[serde(default)]
    pub bangumi: Vec<VideoSource>,
}

#[derive(Serialize, ToSchema)]
pub struct VideosResponse {
    pub videos: Vec<VideoInfo>,
    pub total_count: u64,
}

#[derive(Serialize, ToSchema)]
pub struct VideoResponse {
    pub video: VideoInfo,
    pub pages: Vec<PageInfo>,
}

#[derive(Serialize, ToSchema)]
pub struct ResetVideoResponse {
    pub resetted: bool,
    pub video: i32,
    pub pages: Vec<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct AddVideoSourceResponse {
    pub success: bool,
    pub source_id: i32,
    pub source_type: String,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct DeleteVideoSourceResponse {
    pub success: bool,
    pub source_id: i32,
    pub source_type: String,
    pub message: String,
}

#[derive(FromQueryResult, Serialize, ToSchema, Debug)]
pub struct VideoSource {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize, ToSchema)]
pub struct PageInfo {
    pub id: i32,
    pub pid: i32,
    pub name: String,
    pub download_status: [u32; 5],
}

impl From<(i32, i32, String, u32)> for PageInfo {
    fn from((id, pid, name, download_status): (i32, i32, String, u32)) -> Self {
        Self {
            id,
            pid,
            name,
            download_status: PageStatus::from(download_status).into(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct VideoInfo {
    pub id: i32,
    pub name: String,
    pub upper_name: String,
    pub path: String,
    pub download_status: [u32; 5],
}

impl From<(i32, String, String, String, u32)> for VideoInfo {
    fn from((id, name, upper_name, path, download_status): (i32, String, String, String, u32)) -> Self {
        Self {
            id,
            name,
            upper_name,
            path,
            download_status: VideoStatus::from(download_status).into(),
        }
    }
}

// 获取配置的响应结构体
#[derive(Serialize, ToSchema)]
pub struct ConfigResponse {
    pub video_name: String,
    pub page_name: String,
    pub multi_page_name: String,
    pub bangumi_name: String,
    pub folder_structure: String,
    pub time_format: String,
    pub interval: u64,
    pub nfo_time_type: String,
    // 多线程下载配置
    pub parallel_download_enabled: bool,
    pub parallel_download_threads: usize,
    pub parallel_download_min_size: u64,
}

// 更新配置的响应结构体
#[derive(Serialize, ToSchema)]
pub struct UpdateConfigResponse {
    pub success: bool,
    pub message: String,
    pub updated_files: Option<u32>, // 重命名的文件数量
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BangumiSeasonInfo {
    pub season_id: String,
    pub season_title: String,
    pub full_title: Option<String>, // 完整的番剧标题
    pub media_id: Option<String>,
    pub cover: Option<String>,
    pub episode_count: Option<i32>, // 集数
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BangumiSeasonsResponse {
    pub success: bool,
    pub data: Vec<BangumiSeasonInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchResult {
    pub result_type: String,    // video, bili_user, media_bangumi等
    pub title: String,          // 标题
    pub author: String,         // 作者/UP主
    pub bvid: Option<String>,   // 视频BV号
    pub aid: Option<i64>,       // 视频AV号
    pub mid: Option<i64>,       // UP主ID
    pub season_id: Option<String>, // 番剧season_id
    pub media_id: Option<String>,  // 番剧media_id
    pub cover: String,          // 封面图
    pub description: String,    // 描述
    pub duration: Option<String>, // 视频时长
    pub pubdate: Option<i64>,   // 发布时间
    pub play: Option<i64>,      // 播放量
    pub danmaku: Option<i64>,   // 弹幕数
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchResponse {
    pub success: bool,
    pub results: Vec<SearchResult>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserFavoriteFolder {
    /// 收藏夹完整ID（推荐使用）
    #[serde(serialize_with = "serialize_i64_as_string")]
    pub id: i64,
    /// 收藏夹短ID（可能截断，不推荐直接使用）
    #[serde(serialize_with = "serialize_i64_as_string")]
    pub fid: i64,
    /// 收藏夹标题
    pub title: String,
    /// 收藏夹内视频数量
    pub media_count: i32,
}

// 辅助函数：将 i64 序列化为字符串
fn serialize_i64_as_string<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserCollection {
    /// 合集类型：season（合集）或 series（系列）
    pub collection_type: String,
    /// 合集/系列ID
    pub sid: String,
    /// 合集/系列名称
    pub name: String,
    /// 封面图片URL
    pub cover: String,
    /// 描述
    pub description: String,
    /// 视频总数
    pub total: i64,
    /// 发布时间
    pub ptime: Option<i64>,
    /// UP主ID
    pub mid: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserCollectionsResponse {
    pub success: bool,
    pub collections: Vec<UserCollection>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}
