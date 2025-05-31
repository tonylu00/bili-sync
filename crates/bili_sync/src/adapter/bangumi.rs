use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::Stream;
use sea_orm::prelude::*;
use sea_orm::{ActiveValue::Set};
use tracing::debug;
use tracing::info;
use tracing::warn;

use bili_sync_entity::VideoSourceTrait;
use sea_orm::sea_query::SimpleExpr;

use crate::adapter::VideoSource;
use crate::bilibili::bangumi::Bangumi;
use crate::bilibili::{BiliClient, VideoInfo};

#[derive(Clone)]
pub struct BangumiSource {
    pub id: i32,
    pub name: String,
    pub latest_row_at: NaiveDateTime,
    pub season_id: Option<String>,
    pub media_id: Option<String>,
    pub ep_id: Option<String>,
    pub path: PathBuf,
    pub download_all_seasons: bool,
    pub page_name_template: Option<String>,
    pub selected_seasons: Option<Vec<String>>,
}

impl BangumiSource {
    /// 渲染番剧的 page_name，优先使用全局 bangumi_name 配置
    /// 支持重名检测，当发现重名时自动添加title后缀
    pub fn render_page_name(
        &self,
        video_model: &bili_sync_entity::video::Model,
        page_model: &bili_sync_entity::page::Model,
    ) -> Result<String> {
        use crate::utils::format_arg::bangumi_page_format_args;

        // 获取最新的配置，而不是使用静态全局配置
        let current_config = crate::config::reload_config();

        // 优先级：全局 bangumi_name > 番剧自己的 page_name > 默认格式
        let template = if !current_config.bangumi_name.is_empty() {
            current_config.bangumi_name.to_string()
        } else if let Some(ref page_name) = self.page_name_template {
            page_name.clone()
        } else {
            "S{{season_pad}}E{{pid_pad}}-{{pid_pad}}".to_string()
        };

        // 创建配置了辅助函数的 handlebars 实例
        let mut handlebars = handlebars::Handlebars::new();
        // 注册 truncate 辅助函数
        handlebars.register_helper("truncate", Box::new(|h: &handlebars::Helper, _: &handlebars::Handlebars, _: &handlebars::Context, _: &mut handlebars::RenderContext, out: &mut dyn handlebars::Output| -> handlebars::HelperResult {
            let s = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
            let len = h.param(1).and_then(|v| v.value().as_u64()).unwrap_or(0) as usize;
            let result = if s.chars().count() > len {
                s.chars().take(len).collect::<String>()
            } else {
                s.to_string()
            };
            out.write(&result)?;
            Ok(())
        }));

        let format_args = bangumi_page_format_args(video_model, page_model);
        
        // 首先使用原始模板渲染
        let base_name = crate::utils::filenamify::filenamify(&handlebars.render_template(
            &template,
            &format_args,
        )?);

        // 检查是否需要添加title后缀来避免重名
        let final_name = self.resolve_naming_conflict(&base_name, video_model, page_model, &template, &handlebars, &format_args)?;

        Ok(final_name)
    }

    /// 检测并解决文件命名冲突
    /// 当检测到重名时，自动在模板末尾添加 `-{{title}}` 后缀
    fn resolve_naming_conflict(
        &self,
        base_name: &str,
        _video_model: &bili_sync_entity::video::Model,
        page_model: &bili_sync_entity::page::Model,
        original_template: &str,
        handlebars: &handlebars::Handlebars,
        format_args: &serde_json::Value,
    ) -> Result<String> {
        // 检查目标目录是否存在同名的mp4文件
        let target_path = self.path.join(format!("{}.mp4", base_name));
        
        if !target_path.exists() {
            // 如果文件不存在，直接返回基础名称
            return Ok(base_name.to_string());
        }

        // 文件已存在，需要添加区分后缀
        info!(
            "检测到番剧文件名冲突: {}，自动添加title后缀区分不同版本",
            base_name
        );

        // 检查原模板是否已包含title，避免重复添加
        let enhanced_template = if original_template.contains("{{title}}") || original_template.contains("{{ title }}") {
            // 已包含title，直接使用原模板
            original_template.to_string()
        } else {
            // 在模板末尾添加title后缀
            format!("{}-{{{{title}}}}", original_template)
        };

        // 使用增强模板重新渲染
        let enhanced_name = crate::utils::filenamify::filenamify(&handlebars.render_template(
            &enhanced_template,
            format_args,
        )?);

        // 再次检查增强后的名称是否冲突
        let enhanced_path = self.path.join(format!("{}.mp4", enhanced_name));
        if enhanced_path.exists() {
            // 如果仍然冲突，添加更详细的区分信息
            warn!(
                "即使添加title后缀仍有冲突: {}，使用详细区分方案",
                enhanced_name
            );
            
            // 使用CID作为最后的区分手段
            let final_name = format!("{}-CID{}", enhanced_name, page_model.cid);
            return Ok(final_name);
        }

        info!(
            "成功解决文件名冲突: {} -> {}",
            base_name, enhanced_name
        );

        Ok(enhanced_name)
    }

    pub async fn video_stream_from(
        &self,
        bili_client: &BiliClient,
        _path: &Path,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send>>> {
        let bangumi = Bangumi::new(
            bili_client,
            self.media_id.clone(),
            self.season_id.clone(),
            self.ep_id.clone(),
        );

        if self.download_all_seasons {
            debug!("正在获取所有季度的番剧内容");
            Ok(Box::pin(bangumi.to_all_seasons_video_stream()))
        } else if let Some(ref selected_seasons) = self.selected_seasons {
            // 如果有选中的季度，只下载选中的季度
            debug!("正在获取选中的 {} 个季度的番剧内容", selected_seasons.len());
            Ok(Box::pin(bangumi.to_selected_seasons_video_stream(selected_seasons.clone())))
        } else {
            debug!("仅获取当前季度的番剧内容");
            Ok(Box::pin(bangumi.to_video_stream()))
        }
    }

    // 番剧源的初始化现在通过Web API完成，不再需要这个函数
}

impl VideoSourceTrait for BangumiSource {
    fn get_latest_row_at(&self) -> NaiveDateTime {
        self.latest_row_at
    }

    fn log_refresh_video_start(&self) {
        info!("开始获取番剧 {} 的更新", self.name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        if count > 0 {
            info!("番剧 {} 获取更新完毕，新增 {} 个视频", self.name, count);
        } else {
            info!("番剧 {} 无新视频", self.name);
        }
    }

    fn log_fetch_video_start(&self) {
        debug!("开始获取番剧 {} 的详细信息", self.name);
    }

    fn log_fetch_video_end(&self) {
        debug!("番剧 {} 的详细信息获取完毕", self.name);
    }

    fn log_download_video_start(&self) {
        debug!("开始下载番剧 {} 的视频", self.name);
    }

    fn log_download_video_end(&self) {
        debug!("番剧 {} 的视频下载完毕", self.name);
    }

    fn filter_expr(&self) -> SimpleExpr {
        bili_sync_entity::video::Column::SourceId
            .eq(self.id)
            .and(bili_sync_entity::video::Column::SourceType.eq(1))
    }

    fn should_take(&self, _release_datetime: &DateTime<Utc>, _latest_row_at: &DateTime<Utc>) -> bool {
        true
    }

    fn update_latest_row_at(&self, latest_row_at: NaiveDateTime) -> bili_sync_entity::video_source::ActiveModel {
        let mut model = <bili_sync_entity::video_source::ActiveModel as sea_orm::ActiveModelTrait>::default();
        model.id = Set(self.id);
        model.latest_row_at = Set(latest_row_at);
        model
    }

    fn set_relation_id(&self, model: &mut bili_sync_entity::video::ActiveModel) {
        model.source_id = Set(Some(self.id));
        model.source_type = Set(Some(1));
    }
}

impl VideoSource for BangumiSource {
    fn filter_expr(&self) -> SimpleExpr {
        bili_sync_entity::video::Column::SourceId
            .eq(self.id)
            .and(bili_sync_entity::video::Column::SourceType.eq(1))
    }

    fn set_relation_id(&self, model: &mut bili_sync_entity::video::ActiveModel) {
        model.source_id = Set(Some(self.id));
        model.source_type = Set(Some(1));
    }

    fn get_latest_row_at(&self) -> NaiveDateTime {
        self.latest_row_at
    }

    fn update_latest_row_at(&self, datetime: NaiveDateTime) -> crate::adapter::_ActiveModel {
        let mut model = <bili_sync_entity::video_source::ActiveModel as sea_orm::ActiveModelTrait>::default();
        model.id = Set(self.id);
        model.latest_row_at = Set(datetime);
        crate::adapter::_ActiveModel::Bangumi(model)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    // 总是返回true，表示应该下载所有番剧内容，不管发布时间
    fn should_take(&self, _release_datetime: &chrono::DateTime<Utc>, _latest_row_at: &chrono::DateTime<Utc>) -> bool {
        true
    }

    fn log_refresh_video_start(&self) {
        info!("开始获取番剧 {} 的更新", self.name);
    }

    fn log_refresh_video_end(&self, count: usize) {
        if count > 0 {
            info!("番剧 {} 获取更新完毕，新增 {} 个视频", self.name, count);
        } else {
            info!("番剧 {} 无新视频", self.name);
        }
    }

    fn log_fetch_video_start(&self) {
        debug!("开始获取番剧 {} 的详细信息", self.name);
    }

    fn log_fetch_video_end(&self) {
        debug!("番剧 {} 的详细信息获取完毕", self.name);
    }

    fn log_download_video_start(&self) {
        debug!("开始下载番剧 {} 的视频", self.name);
    }

    fn log_download_video_end(&self) {
        debug!("番剧 {} 的视频下载完毕", self.name);
    }
}
