use chrono::Datelike;
use serde_json::json;

use crate::config;

/// 完全基于API的番剧标题提取，无硬编码回退逻辑
fn extract_series_title_with_context(
    video_model: &bili_sync_entity::video::Model,
    api_title: Option<&str>,
) -> Option<String> {
    println!("🔍 extract_series_title_with_context 输入参数:");
    println!("  - video_model.name: '{}'", video_model.name);
    println!("  - video_model.bvid: '{}'", video_model.bvid);
    println!("  - api_title: {:?}", api_title);
    
    // 只使用API提供的真实番剧标题，无回退逻辑
    if let Some(title) = api_title {
        println!("✅ 使用API标题: '{}'", title);
        return Some(title.to_string());
    }

    // 如果没有API标题，记录警告并返回None
    println!("❌ 没有API标题，返回None");
    tracing::debug!(
        "番剧视频 {} (BVID: {}) 缺少API标题，将跳过处理",
        video_model.name,
        video_model.bvid
    );

    None
}

/// 从番剧标题中提取季度编号（应优先使用API数据）
/// 注意：理想情况下应该使用API中的season信息
fn extract_season_number(episode_title: &str) -> i32 {
    let title = episode_title.trim();

    // 移除开头的下划线（如果有）
    let title = title.strip_prefix('_').unwrap_or(title);

    // 查找季度标识的几种模式
    // 模式1: "第X季"
    if let Some(pos) = title.find("第") {
        let after_di = &title[pos + "第".len()..];
        if let Some(ji_pos) = after_di.find("季") {
            let season_str = &after_di[..ji_pos];
            // 尝试解析中文数字或阿拉伯数字
            match season_str {
                "一" => return 1,
                "二" => return 2,
                "三" => return 3,
                "四" => return 4,
                "五" => return 5,
                "六" => return 6,
                "七" => return 7,
                "八" => return 8,
                "九" => return 9,
                "十" => return 10,
                _ => {
                    // 尝试解析阿拉伯数字
                    if let Ok(season) = season_str.parse::<i32>() {
                        if season > 0 && season <= 50 {
                            // 合理的季度范围
                            return season;
                        }
                    }
                }
            }
        }
    }

    // 模式2: "Season X" 或 "season X"
    for pattern in ["Season ", "season "] {
        if let Some(pos) = title.find(pattern) {
            let after_season = &title[pos + pattern.len()..];
            // 找到第一个非数字字符的位置
            let season_end = after_season
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(after_season.len());
            let season_str = &after_season[..season_end];
            if let Ok(season) = season_str.parse::<i32>() {
                if season > 0 && season <= 50 {
                    return season;
                }
            }
        }
    }

    // 默认返回1
    1
}

/// 从视频标题中提取版本信息（纯API方案，不使用硬编码）
/// 注意：理想情况下版本信息应该从API episode数据中获取
/// 这里只做最基础的提取，避免硬编码模式匹配
fn extract_version_info(video_title: &str) -> String {
    let title = video_title.trim().strip_prefix('_').unwrap_or(video_title);

    // 如果标题很短且不包含常见的番剧标识符，可能是版本标识
    if title.len() <= 6 && !title.contains("第") && !title.contains("话") && !title.contains("集") {
        return title.to_string();
    }

    // 其他情况返回空字符串，依赖API数据
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_season_number() {
        // 测试中文季度标识
        assert_eq!(extract_season_number("_灵笼 第一季_第001话"), 1);
        assert_eq!(extract_season_number("_灵笼 第二季_第1话 末世桃源"), 2);
        assert_eq!(extract_season_number("_灵笼 第三季_第001话"), 3);

        // 测试阿拉伯数字季度标识
        assert_eq!(extract_season_number("_某番剧 第1季_第001话"), 1);
        assert_eq!(extract_season_number("_某番剧 第2季_第001话"), 2);

        // 测试英文季度标识
        assert_eq!(extract_season_number("_某番剧 Season 1_第001话"), 1);
        assert_eq!(extract_season_number("_某番剧 Season 2_第001话"), 2);

        // 测试默认值
        assert_eq!(extract_season_number("_某番剧_第001话"), 1);
        assert_eq!(extract_season_number("_名侦探柯南 绯色的不在证明_全片"), 1);
    }

    #[test]
    fn test_extract_series_title_with_context() {
        use bili_sync_entity::video::Model;
        use chrono::DateTime;

        // 创建测试用的video model
        let test_time = DateTime::from_timestamp(1640995200, 0).unwrap().naive_utc();
        let mut video_model = Model {
            id: 1,
            collection_id: None,
            favorite_id: None,
            watch_later_id: None,
            submission_id: None,
            source_id: None,
            source_type: Some(1),
            upper_id: 123456,
            upper_name: "官方频道".to_string(),
            upper_face: "".to_string(),
            name: "中配".to_string(),
            path: "".to_string(),
            category: 1,
            bvid: "BV1234567890".to_string(),
            intro: "".to_string(),
            cover: "".to_string(),
            ctime: test_time,
            pubtime: test_time,
            favtime: test_time,
            download_status: 0,
            valid: true,
            tags: None,
            single_page: Some(true),
            created_at: "2024-01-01 00:00:00".to_string(),
            season_id: Some("12345".to_string()),
            ep_id: None,
            season_number: None,
            episode_number: None,
            deleted: 0,
            share_copy: None,
            show_season_type: None,
            actors: None,
            auto_download: false,
        };

        // 测试使用API标题的情况
        let result = extract_series_title_with_context(&video_model, Some("灵笼 第一季"));
        assert_eq!(result, Some("灵笼 第一季".to_string()));

        // 测试无API数据的情况
        let result = extract_series_title_with_context(&video_model, None);
        assert_eq!(result, None);

        // 测试空字符串API数据
        let result = extract_series_title_with_context(&video_model, Some(""));
        assert_eq!(result, Some("".to_string()));
    }

    #[test]
    fn test_extract_version_info() {
        // 测试短标题（可能是版本标识）
        assert_eq!(extract_version_info("中文"), "中文");
        assert_eq!(extract_version_info("_中配"), "中配");
        assert_eq!(extract_version_info("原版"), "原版");
        assert_eq!(extract_version_info("日配"), "日配");

        // 测试包含番剧标识符的标题（不应被识别为版本）
        assert_eq!(extract_version_info("_灵笼 第一季_第001话"), "");
        assert_eq!(extract_version_info("名侦探柯南 第1集"), "");
        assert_eq!(extract_version_info("某番剧 第1话"), "");

        // 测试长标题（应该返回空）
        assert_eq!(extract_version_info("很长的番剧标题名称"), "");
    }
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
    api_title: Option<&str>,
) -> serde_json::Value {
    let current_config = config::reload_config();

    // 直接使用数据库中存储的集数，如果没有则使用page_model.pid
    let episode_number = video_model.episode_number.unwrap_or(page_model.pid);

    // 优先从标题中提取季度编号，如果提取失败则使用数据库中存储的值，最后默认为1
    let season_number = match extract_season_number(&video_model.name) {
        1 => video_model.season_number.unwrap_or(1), // 如果从标题提取到1，可能是默认值，使用数据库值
        extracted => extracted,                      // 从标题提取到了明确的季度信息，使用提取的值
    };

    // 从发布时间提取年份
    let year = video_model.pubtime.year();

    // 提取番剧系列标题用于文件夹命名，完全依赖API数据
    let series_title = match extract_series_title_with_context(video_model, api_title) {
        Some(title) => {
            println!("🎯 extract_series_title_with_context 成功提取: '{}'", title);
            title
        },
        None => {
            // 无API数据时记录警告，使用空字符串作为series_title
            // 这样调用方可以根据空字符串判断是否缺少API数据
            println!("⚠️ extract_series_title_with_context 失败，api_title: {:?}", api_title);
            tracing::debug!(
                "番剧视频 {} (BVID: {}) 缺少API标题，series_title将为空",
                video_model.name,
                video_model.bvid
            );
            String::new()
        }
    };

    // 提取版本信息用于文件名区分
    let version_info = extract_version_info(&video_model.name);

    // 智能处理版本信息重复问题
    // 如果page_model.name就是版本信息，那么在version字段中不重复显示
    let final_version = if !version_info.is_empty() && page_model.name.trim() == version_info {
        String::new() // 避免重复，清空version字段
    } else {
        version_info
    };

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
        "series_title": &series_title, // 系列标题，从单集标题中提取
        "version": &final_version, // 版本信息（智能处理重复）
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
        bangumi_page_format_args(video_model, page_model, None)
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
