use serde_json::json;
use chrono::Datelike;

use crate::config;

/// 从番剧单集标题中提取系列标题
/// 处理各种常见的番剧标题模式
fn extract_series_title(episode_title: &str) -> String {
    let title = episode_title.trim();
    
    // 移除开头的下划线（如果有）
    let title = title.strip_prefix('_').unwrap_or(title);
    
    // 处理常见的版本标识（可能是独立的版本，如"中文"、"原版"等）
    if matches!(title, "中文" | "中配" | "原版" | "日配" | "国语" | "粤语" | "英文" | "日语") {
        return title.to_string();
    }
    
    // 处理以下划线分隔的标题模式，如 "_灵笼 第一季_第001话"
    if let Some(last_underscore_pos) = title.rfind('_') {
        let potential_series = &title[..last_underscore_pos];
        let episode_part = &title[last_underscore_pos + 1..];
        
        // 如果最后一部分是集数标识，则返回系列部分
        if episode_part.starts_with("第") && (episode_part.contains("话") || episode_part.contains("集")) ||
           episode_part.contains("全片") ||
           episode_part.contains("中章") ||
           episode_part.contains("特别篇") ||
           episode_part.contains("终章") ||
           episode_part.contains("上") ||
           episode_part.contains("下") {
            return potential_series.to_string();
        }
    }
    
    // 处理其他分隔符模式
    if let Some(pos) = title.find(" 第") {
        if title[pos..].contains("话") || title[pos..].contains("集") {
            return title[..pos].to_string();
        }
    }
    
    // 如果没有匹配到特定模式，返回原标题
    title.to_string()
}

pub fn video_format_args(video_model: &bili_sync_entity::video::Model) -> serde_json::Value {
    let current_config = config::reload_config();
    json!({
        "bvid": &video_model.bvid,
        "title": &video_model.name,
        "upper_name": &video_model.upper_name,
        "upper_mid": &video_model.upper_id,
        "pubtime": &video_model.pubtime.and_utc().format(&current_config.time_format).to_string(),
        "fav_time": &video_model.favtime.and_utc().format(&current_config.time_format).to_string(),
        "show_title": &video_model.name,
    })
}

/// 番剧专用的格式化函数，使用番剧自己的模板配置
pub fn bangumi_page_format_args(
    video_model: &bili_sync_entity::video::Model,
    page_model: &bili_sync_entity::page::Model,
) -> serde_json::Value {
    let current_config = config::reload_config();

    // 直接使用数据库中存储的集数，如果没有则使用page_model.pid
    let episode_number = video_model.episode_number.unwrap_or(page_model.pid);

    // 使用数据库中存储的季度编号，如果没有则默认为1
    let season_number = video_model.season_number.unwrap_or(1);

    // 从发布时间提取年份
    let year = video_model.pubtime.year();

    // 生成分辨率信息
    let resolution = match (page_model.width, page_model.height) {
        (Some(w), Some(h)) => format!("{}x{}", w, h),
        _ => "Unknown".to_string(),
    };

    // 内容类型判断
    let content_type = match video_model.category {
        1 => "动画",     // 动画分类
        177 => "纪录片", // 纪录片分类  
        155 => "时尚",   // 时尚分类
        _ => "番剧",     // 默认为番剧
    };

    // 播出状态（根据是否有season_id等信息推断）
    let status = if video_model.season_id.is_some() {
        "连载中" // 有season_id通常表示正在播出
    } else {
        "已完结" // 没有season_id可能表示已完结或单集
    };

    json!({
        "bvid": &video_model.bvid,
        "title": &video_model.name,
        "upper_name": &video_model.upper_name,
        "upper_mid": &video_model.upper_id,
        "ptitle": &page_model.name,
        "pid": episode_number,
        "pid_pad": format!("{:02}", episode_number),
        "season": season_number,
        "season_pad": format!("{:02}", season_number),
        "year": year,
        "studio": &video_model.upper_name,
        "actors": video_model.actors.as_deref().unwrap_or(""),
        "share_copy": video_model.share_copy.as_deref().unwrap_or(""),
        "category": video_model.category,
        "resolution": resolution,
        "content_type": content_type,
        "status": status,
        "ep_id": video_model.ep_id.as_deref().unwrap_or(""),
        "season_id": video_model.season_id.as_deref().unwrap_or(""),
        "pubtime": video_model.pubtime.and_utc().format(&current_config.time_format).to_string(),
        "fav_time": video_model.favtime.and_utc().format(&current_config.time_format).to_string(),
        // 添加更多文件夹命名可能用到的变量
        "show_title": &video_model.name, // 番剧标题（别名）
        "series_title": &video_model.name, // 系列标题（别名）
    })
}

pub fn page_format_args(
    video_model: &bili_sync_entity::video::Model,
    page_model: &bili_sync_entity::page::Model,
) -> serde_json::Value {
    let current_config = config::reload_config();
    // 检查是否为番剧类型
    let is_bangumi = match video_model.source_type {
        Some(1) => true, // source_type = 1 表示为番剧
        _ => false,
    };

    // 检查是否为单P视频
    let is_single_page = video_model.single_page.unwrap_or(true);

    // 对于番剧，使用专门的格式化函数
    if is_bangumi {
        bangumi_page_format_args(video_model, page_model)
    } else if !is_single_page {
        // 对于多P视频（非番剧），使用番剧格式的命名，默认季度为1
        let season_number = 1;

        // 从发布时间提取年份
        let year = video_model.pubtime.year();

        // 生成分辨率信息
        let resolution = match (page_model.width, page_model.height) {
            (Some(w), Some(h)) => format!("{}x{}", w, h),
            _ => "Unknown".to_string(),
        };

        json!({
            "bvid": &video_model.bvid,
            "title": &video_model.name,
            "upper_name": &video_model.upper_name,
            "upper_mid": &video_model.upper_id,
            "ptitle": &page_model.name,
            "pid": page_model.pid,
            "pid_pad": format!("{:02}", page_model.pid),
            "season": season_number,
            "season_pad": format!("{:02}", season_number),
            "year": year,
            "studio": &video_model.upper_name,
            "actors": video_model.actors.as_deref().unwrap_or(""),
            "share_copy": video_model.share_copy.as_deref().unwrap_or(""),
            "category": video_model.category,
            "resolution": resolution,
            "pubtime": video_model.pubtime.and_utc().format(&current_config.time_format).to_string(),
            "fav_time": video_model.favtime.and_utc().format(&current_config.time_format).to_string(),
            "long_title": &page_model.name,
            "show_title": &page_model.name,
        })
    } else {
        // 对于单P视频，使用原有的格式（不包含season_pad）
        json!({
            "bvid": &video_model.bvid,
            "title": &video_model.name,
            "upper_name": &video_model.upper_name,
            "upper_mid": &video_model.upper_id,
            "ptitle": &page_model.name,
            "pid": page_model.pid,
            "pid_pad": format!("{:02}", page_model.pid),
            "pubtime": video_model.pubtime.and_utc().format(&current_config.time_format).to_string(),
            "fav_time": video_model.favtime.and_utc().format(&current_config.time_format).to_string(),
            "long_title": &page_model.name,
            "show_title": &page_model.name,
        })
    }
}
