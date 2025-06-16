use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwapOption;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

mod bundle;
mod clap;
mod global;
mod item;
mod manager;

use crate::bilibili::{Credential, DanmakuOption, FilterOption};
pub use crate::config::bundle::ConfigBundle;
pub use crate::config::clap::version;
pub use crate::config::global::{
    init_config_with_database, reload_config, reload_config_bundle, with_config, ARGS, CONFIG, CONFIG_DIR, TEMPLATE,
};
use crate::config::item::ConcurrentLimit;
pub use crate::config::item::{NFOTimeType, PathSafeTemplate, RateLimit, SubmissionRiskControlConfig};
pub use crate::config::manager::ConfigManager;

// 移除不再需要的配置结构体，因为视频源现在存储在数据库中
// #[derive(Serialize, Deserialize, Default, Debug, Clone)]
// pub struct BangumiConfig {
//     pub season_id: Option<String>,
//     pub media_id: Option<String>,
//     pub ep_id: Option<String>,
//     pub path: PathBuf,
//     #[serde(default = "default_download_all_seasons")]
//     pub download_all_seasons: bool,
//     /// 番剧专用的 video_name 模板，如果未设置则使用全局配置
//     #[serde(default)]
//     pub video_name: Option<String>,
//     /// 番剧专用的 page_name 模板，如果未设置则使用全局 bangumi_name 配置
//     #[serde(default)]
//     pub page_name: Option<String>,
// }

// #[derive(Serialize, Deserialize, Default, Debug, Clone)]
// pub struct FavoriteConfig {
//     pub fid: String,
//     pub path: PathBuf,
//     #[serde(default = "default_download_all_seasons")]
//     pub download_all_seasons: bool,
//     #[serde(default = "default_page_name")]
//     pub page_name: Option<String>,
// }

// #[derive(Serialize, Deserialize, Default, Debug, Clone)]
// pub struct CollectionConfig {
//     pub collection_type: String, // "season" 或 "series"
//     pub upper_id: String,
//     pub collection_id: String,
//     pub path: PathBuf,
//     #[serde(default = "default_download_all_seasons")]
//     pub download_all_seasons: bool,
//     #[serde(default = "default_page_name")]
//     pub page_name: Option<String>,
// }

// #[derive(Serialize, Deserialize, Default, Debug, Clone)]
// pub struct SubmissionConfig {
//     pub upper_id: String,
//     pub path: PathBuf,
//     #[serde(default = "default_download_all_seasons")]
//     pub download_all_seasons: bool,
//     #[serde(default = "default_page_name")]
//     pub page_name: Option<String>,
// }

// #[derive(Serialize, Deserialize, Default, Debug, Clone)]
// pub struct WatchLaterConfig {
//     #[serde(default)]
//     pub enabled: bool,
//     #[serde(default)]
//     pub path: PathBuf,
// }

fn default_time_format() -> String {
    "%Y-%m-%d".to_string()
}

fn default_timezone() -> String {
    "Asia/Shanghai".to_string()
}

/// 默认的 auth_token 实现，生成随机 16 位字符串
fn default_auth_token() -> Option<String> {
    let byte_choices = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()_+-=";
    let mut rng = rand::thread_rng();
    Some(
        (0..16)
            .map(|_| *(byte_choices.choose(&mut rng).expect("choose byte failed")) as char)
            .collect(),
    )
}

fn default_bind_address() -> String {
    "0.0.0.0:12345".to_string()
}

// 移除不再需要的默认函数
// fn default_download_all_seasons() -> bool {
//     false
// }

// fn default_page_name() -> Option<String> {
//     Some("{{title}}".to_string())
// }

fn default_multi_page_name() -> Cow<'static, str> {
    Cow::Borrowed("{{title}}-P{{pid_pad}}")
}

fn default_bangumi_name() -> Cow<'static, str> {
    Cow::Borrowed("S{{season_pad}}E{{pid_pad}}-{{pid_pad}}")
}

fn default_folder_structure() -> Cow<'static, str> {
    Cow::Borrowed("Season 1")
}

fn default_collection_folder_mode() -> Cow<'static, str> {
    Cow::Borrowed("separate") // 默认为分离模式（向后兼容）
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_auth_token")]
    pub auth_token: Option<String>,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    pub credential: ArcSwapOption<Credential>,
    pub filter_option: FilterOption,
    #[serde(default)]
    pub danmaku_option: DanmakuOption,
    pub video_name: Cow<'static, str>,
    pub page_name: Cow<'static, str>,
    #[serde(default = "default_multi_page_name")]
    pub multi_page_name: Cow<'static, str>,
    #[serde(default = "default_bangumi_name")]
    pub bangumi_name: Cow<'static, str>,
    #[serde(default = "default_folder_structure")]
    pub folder_structure: Cow<'static, str>,
    #[serde(default = "default_collection_folder_mode")]
    pub collection_folder_mode: Cow<'static, str>,
    pub interval: u64,
    pub upper_path: PathBuf,
    #[serde(default)]
    pub nfo_time_type: NFOTimeType,
    #[serde(default)]
    pub concurrent_limit: ConcurrentLimit,
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default)]
    pub cdn_sorting: bool,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default)]
    pub submission_risk_control: crate::config::item::SubmissionRiskControlConfig,
}

impl Clone for Config {
    fn clone(&self) -> Self {
        Self {
            auth_token: self.auth_token.clone(),
            bind_address: self.bind_address.clone(),
            credential: ArcSwapOption::from(self.credential.load_full()),
            filter_option: FilterOption {
                video_max_quality: self.filter_option.video_max_quality,
                video_min_quality: self.filter_option.video_min_quality,
                audio_max_quality: self.filter_option.audio_max_quality,
                audio_min_quality: self.filter_option.audio_min_quality,
                codecs: self.filter_option.codecs.clone(),
                no_dolby_video: self.filter_option.no_dolby_video,
                no_dolby_audio: self.filter_option.no_dolby_audio,
                no_hdr: self.filter_option.no_hdr,
                no_hires: self.filter_option.no_hires,
            },
            danmaku_option: DanmakuOption {
                duration: self.danmaku_option.duration,
                font: self.danmaku_option.font.clone(),
                font_size: self.danmaku_option.font_size,
                width_ratio: self.danmaku_option.width_ratio,
                horizontal_gap: self.danmaku_option.horizontal_gap,
                lane_size: self.danmaku_option.lane_size,
                float_percentage: self.danmaku_option.float_percentage,
                bottom_percentage: self.danmaku_option.bottom_percentage,
                opacity: self.danmaku_option.opacity,
                bold: self.danmaku_option.bold,
                outline: self.danmaku_option.outline,
                time_offset: self.danmaku_option.time_offset,
            },
            video_name: self.video_name.clone(),
            page_name: self.page_name.clone(),
            multi_page_name: self.multi_page_name.clone(),
            bangumi_name: self.bangumi_name.clone(),
            folder_structure: self.folder_structure.clone(),
            collection_folder_mode: self.collection_folder_mode.clone(),
            interval: self.interval,
            upper_path: self.upper_path.clone(),
            nfo_time_type: self.nfo_time_type.clone(),
            concurrent_limit: self.concurrent_limit.clone(),
            time_format: self.time_format.clone(),
            cdn_sorting: self.cdn_sorting,
            timezone: self.timezone.clone(),
            submission_risk_control: self.submission_risk_control.clone(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auth_token: default_auth_token(),
            bind_address: default_bind_address(),
            credential: ArcSwapOption::from(Some(Arc::new(Credential::default()))),
            filter_option: FilterOption::default(),
            danmaku_option: DanmakuOption::default(),
            video_name: Cow::Borrowed("{{title}}"),
            page_name: Cow::Borrowed("{{title}}"),
            multi_page_name: Cow::Borrowed("{{title}}-P{{pid_pad}}"),
            bangumi_name: Cow::Borrowed("S{{season_pad}}E{{pid_pad}}-{{pid_pad}}"),
            folder_structure: Cow::Borrowed("Season 1"),
            collection_folder_mode: Cow::Borrowed("separate"),
            interval: 1200,
            upper_path: CONFIG_DIR.join("upper_face"),
            nfo_time_type: NFOTimeType::FavTime,
            concurrent_limit: ConcurrentLimit::default(),
            time_format: default_time_format(),
            cdn_sorting: true,
            timezone: default_timezone(),
            submission_risk_control: crate::config::item::SubmissionRiskControlConfig::default(),
        }
    }
}

impl Config {
    pub fn save(&self) -> Result<()> {
        let config_path = CONFIG_DIR.join("config.toml");
        std::fs::create_dir_all(&*CONFIG_DIR)?;

        // 使用 toml_edit 库来原生支持注释，而不是手动字符串操作
        let config_content = self.save_with_structured_comments()?;

        std::fs::write(config_path, config_content)?;
        Ok(())
    }

    /// 使用结构化方式生成带注释的配置文件内容
    fn save_with_structured_comments(&self) -> Result<String> {
        // 先序列化为基本的 TOML 字符串
        let toml_str = toml::to_string_pretty(self)?;

        // 使用 toml_edit 解析并添加注释
        let mut doc = toml_str.parse::<toml_edit::DocumentMut>()?;

        // 为各个部分添加注释
        self.add_structured_comments(&mut doc);

        Ok(doc.to_string())
    }

    /// 使用 toml_edit 的原生 API 添加注释
    fn add_structured_comments(&self, doc: &mut toml_edit::DocumentMut) {
        // 移除视频源相关的注释，因为现在视频源存储在数据库中
        // if let Some(favorite_item) = doc.get_mut("favorite_list") {
        //     if let Some(table) = favorite_item.as_table_mut() {
        //         table
        //             .decor_mut()
        //             .set_prefix("\n# 收藏夹配置\n# 格式: 收藏夹ID = \"保存路径\"\n# 收藏夹ID可以从收藏夹URL中获取\n");
        //     }
        // }

        // if let Some(collection_item) = doc.get_mut("collection_list") {
        //     if let Some(table) = collection_item.as_table_mut() {
        //         table.decor_mut().set_prefix("\n# 合集配置\n# 格式: 合集类型:UP主ID:合集ID = \"保存路径\"\n# 合集类型: season(视频合集) 或 series(视频列表)\n");
        //     }
        // }

        // if let Some(submission_item) = doc.get_mut("submission_list") {
        //     if let Some(table) = submission_item.as_table_mut() {
        //         table
        //             .decor_mut()
        //             .set_prefix("\n# UP主投稿配置\n# 格式: UP主ID = \"保存路径\"\n# UP主ID可以从UP主空间URL中获取\n");
        //     }
        // }

        // if let Some(bangumi_item) = doc.get_mut("bangumi") {
        //     if let Some(array) = bangumi_item.as_array_mut() {
        //         if !array.is_empty() {
        //             array.decor_mut().set_prefix("\n# 番剧配置，可以添加多个[[bangumi]]块\n# season_id: 番剧的season_id，可以从B站番剧页面URL中获取\n# path: 保存番剧的本地路径，必须是绝对路径\n# 注意: season_id和path不能为空，否则程序会报错\n");
        //         }
        //     }
        // }

        // 为并发限制部分添加注释
        if let Some(concurrent_item) = doc.get_mut("concurrent_limit") {
            if let Some(table) = concurrent_item.as_table_mut() {
                table
                    .decor_mut()
                    .set_prefix("\n# 并发下载配置\n# video: 同时下载的视频数量\n# page: 每个视频同时下载的分页数量\n");

                // 为并行下载子部分添加注释
                if let Some(parallel_item) = table.get_mut("parallel_download") {
                    if let Some(sub_table) = parallel_item.as_table_mut() {
                        sub_table.decor_mut().set_prefix(
                            "\n# 多线程下载配置\n# enabled: 是否启用多线程下载\n# threads: 每个文件的下载线程数\n",
                        );
                    }
                }
            }
        }

        // 为凭据部分添加注释
        if let Some(credential_item) = doc.get_mut("credential") {
            if let Some(table) = credential_item.as_table_mut() {
                table
                    .decor_mut()
                    .set_prefix("\n# B站登录凭据信息\n# 请从浏览器开发者工具中获取这些值\n");
            }
        }

        // 为过滤选项添加注释
        if let Some(filter_item) = doc.get_mut("filter_option") {
            if let Some(table) = filter_item.as_table_mut() {
                table
                    .decor_mut()
                    .set_prefix("\n# 视频质量过滤配置\n# 可以设置视频和音频的质量范围\n");
            }
        }

        // 为弹幕选项添加注释
        if let Some(danmaku_item) = doc.get_mut("danmaku_option") {
            if let Some(table) = danmaku_item.as_table_mut() {
                table
                    .decor_mut()
                    .set_prefix("\n# 弹幕样式配置\n# 用于设置下载弹幕的显示样式\n");
            }
        }

        // 为UP主投稿风控配置添加注释
        if let Some(submission_risk_item) = doc.get_mut("submission_risk_control") {
            if let Some(table) = submission_risk_item.as_table_mut() {
                table
                    .decor_mut()
                    .set_prefix("\n# UP主投稿风控配置\n# 用于优化大量视频UP主的获取策略，避免触发风控\n# large_submission_threshold: 大量视频UP主阈值（默认300个视频）\n# base_request_delay: 基础请求间隔（毫秒，默认200ms）\n# large_submission_delay_multiplier: 大量视频UP主延迟倍数（默认2倍）\n# enable_progressive_delay: 启用渐进式延迟（默认true）\n# max_delay_multiplier: 最大延迟倍数（默认4倍）\n# enable_incremental_fetch: 启用增量获取（默认true）\n# incremental_fallback_to_full: 增量获取失败时回退到全量获取（默认true）\n# enable_batch_processing: 启用分批处理（默认false）\n# batch_size: 分批大小（页数，默认5页）\n# batch_delay_seconds: 批次间延迟（秒，默认2秒）\n# enable_auto_backoff: 启用自动退避（默认true）\n# auto_backoff_base_seconds: 自动退避基础时间（秒，默认10秒）\n# auto_backoff_max_multiplier: 自动退避最大倍数（默认5倍）\n");
            }
        }
    }

    #[cfg(not(test))]
    fn load() -> Result<Self> {
        let config_path = CONFIG_DIR.join("config.toml");
        let config_content = std::fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_content)?)
    }

    #[cfg(test)]
    fn load() -> Result<Self> {
        // 在测试环境下，返回默认配置
        Ok(Self::default())
    }

    #[cfg(not(test))]
    pub fn check(&self) -> bool {
        let mut ok = true;
        let mut critical_error = false;

        // 移除对视频源的检查，因为现在视频源存储在数据库中
        // let video_sources = self.as_video_sources();
        // if video_sources.is_empty() && self.bangumi.is_empty() {
        //     ok = false;
        //     // 移除错误日志
        //     // error!("没有配置任何需要扫描的内容，程序空转没有意义");
        // }
        // for (args, path) in video_sources {
        //     if !path.is_absolute() {
        //         ok = false;
        //         error!("{:?} 保存的路径应为绝对路径，检测到: {}", args, path.display());
        //     }
        // }
        // // 检查番剧配置的路径
        // for bangumi in &self.bangumi {
        //     if !bangumi.path.is_absolute() {
        //         ok = false;
        //         let season_id_display = match &bangumi.season_id {
        //             Some(id) => id.clone(),
        //             None => "未知".to_string(),
        //         };
        //         error!(
        //             "番剧 {} 保存的路径应为绝对路径，检测到: {}",
        //             season_id_display,
        //             bangumi.path.display()
        //         );
        //     }
        // }

        if !self.upper_path.is_absolute() {
            ok = false;
            error!("up 主头像保存的路径应为绝对路径");
        }
        if self.video_name.is_empty() {
            ok = false;
            error!("未设置 video_name 模板");
        }
        if self.page_name.is_empty() {
            ok = false;
            error!("未设置 page_name 模板");
        }
        if self.multi_page_name.is_empty() {
            ok = false;
            error!("未设置 multi_page_name 模板");
        }
        if self.bangumi_name.is_empty() {
            ok = false;
            error!("未设置 bangumi_name 模板");
        }
        if self.folder_structure.is_empty() {
            ok = false;
            error!("未设置 folder_structure 模板");
        }
        let credential = self.credential.load();
        match credential.as_deref() {
            Some(credential) => {
                if credential.sessdata.is_empty()
                    || credential.bili_jct.is_empty()
                    || credential.buvid3.is_empty()
                    || credential.dedeuserid.is_empty()
                    || credential.ac_time_value.is_empty()
                {
                    ok = false;
                    critical_error = true;
                    warn!("未设置完整的B站登录凭证，程序将以受限模式运行");
                }
            }
            None => {
                ok = false;
                critical_error = true;
                warn!("未设置B站登录凭证，程序将以受限模式运行");
            }
        }
        if !(self.concurrent_limit.video > 0 && self.concurrent_limit.page > 0) {
            ok = false;
            error!("video 和 page 允许的并发数必须大于 0");
        }

        if critical_error {
            warn!(
                "配置文件中检测到凭证未设置，程序将继续运行但功能受限。配置文件位置: {}",
                CONFIG_DIR.join("config.toml").display()
            );
            warn!("请通过Web管理界面或配置文件添加B站登录凭证以启用完整功能");
            // 不再使用 panic!，而是允许程序继续运行
        }

        ok
    }
}
