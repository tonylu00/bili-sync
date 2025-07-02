use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::Stream;
use sea_orm::prelude::*;
use sea_orm::ActiveValue::Set;
use tracing::debug;
use tracing::info;

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
    pub scan_deleted_videos: bool,
}

impl BangumiSource {
    /// 渲染番剧的 page_name，优先使用全局 bangumi_name 配置
    /// 智能检测同一集是否有多个版本，自动添加title后缀避免冲突
    pub async fn render_page_name(
        &self,
        video_model: &bili_sync_entity::video::Model,
        page_model: &bili_sync_entity::page::Model,
        db: &sea_orm::DatabaseConnection,
    ) -> Result<String> {
        use crate::utils::format_arg::bangumi_page_format_args;

        // 获取最新的配置，而不是使用静态全局配置
        let current_config = crate::config::reload_config();

        // 优先级：全局 bangumi_name > 番剧自己的 page_name > 默认格式
        let mut template = if !current_config.bangumi_name.is_empty() {
            current_config.bangumi_name.to_string()
        } else if let Some(ref page_name) = self.page_name_template {
            page_name.clone()
        } else {
            "S{{season_pad}}E{{pid_pad}}-{{pid_pad}}".to_string()
        };

        // 智能检测：检查同一番剧源的同一集是否有多个不同版本
        if !template.contains("{{title}}") && !template.contains("{{ title }}") {
            let should_add_title = self.check_multiple_versions(video_model, db).await;

            if should_add_title {
                // 如果检测到多版本，自动添加title后缀
                template = format!("{}-{{{{title}}}}", template);
                info!(
                    "智能检测到番剧第{}集存在多个版本，自动添加title后缀: {}",
                    video_model.episode_number.unwrap_or(page_model.pid),
                    video_model.name
                );
            }
        }

        // 创建配置了辅助函数的 handlebars 实例
        let mut handlebars = handlebars::Handlebars::new();
        // 注册 truncate 辅助函数
        handlebars.register_helper(
            "truncate",
            Box::new(
                |h: &handlebars::Helper,
                 _: &handlebars::Handlebars,
                 _: &handlebars::Context,
                 _: &mut handlebars::RenderContext,
                 out: &mut dyn handlebars::Output|
                 -> handlebars::HelperResult {
                    let s = h.param(0).and_then(|v| v.value().as_str()).unwrap_or("");
                    let len = h.param(1).and_then(|v| v.value().as_u64()).unwrap_or(0) as usize;
                    let result = if s.chars().count() > len {
                        s.chars().take(len).collect::<String>()
                    } else {
                        s.to_string()
                    };
                    out.write(&result)?;
                    Ok(())
                },
            ),
        );

        let format_args = bangumi_page_format_args(video_model, page_model);

        // 使用最终模板渲染
        let final_name = crate::utils::filenamify::filenamify(&handlebars.render_template(&template, &format_args)?);

        Ok(final_name)
    }

    /// 智能检测同一番剧源的同一集是否存在多个版本
    /// 通过检查相同episode_number的视频数量来判断
    async fn check_multiple_versions(
        &self,
        video_model: &bili_sync_entity::video::Model,
        db: &sea_orm::DatabaseConnection,
    ) -> bool {
        use bili_sync_entity::video;
        use sea_orm::*;

        let source_id = self.id;
        let episode_number = video_model.episode_number.unwrap_or(0);
        let season_id = &video_model.season_id;

        // 查询同一番剧源、同一季度、同一集数的视频数量
        let mut query = video::Entity::find()
            .filter(video::Column::SourceId.eq(source_id))
            .filter(video::Column::SourceType.eq(1)); // 番剧类型

        // 如果有episode_number，使用episode_number过滤
        if episode_number > 0 {
            query = query.filter(video::Column::EpisodeNumber.eq(episode_number));
        }

        // 如果有season_id，使用season_id过滤
        if let Some(season_id_value) = season_id {
            query = query.filter(video::Column::SeasonId.eq(season_id_value));
        }

        match query.count(db).await {
            Ok(count) => {
                debug!("番剧源{} 第{}集 共有{}个版本", source_id, episode_number, count);
                count > 1
            }
            Err(e) => {
                debug!("检查多版本失败: {}", e);
                false
            }
        }
    }

    pub async fn video_stream_from(
        &self,
        bili_client: &BiliClient,
        _path: &Path,
        connection: &sea_orm::DatabaseConnection,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<VideoInfo>> + Send>>> {
        // 检查是否是首次扫描：如果该源没有任何视频记录，应该进行全量获取
        let video_count = bili_sync_entity::video::Entity::find()
            .filter(bili_sync_entity::video::Column::SourceId.eq(self.id))
            .filter(bili_sync_entity::video::Column::SourceType.eq(1)) // 番剧类型
            .count(connection)
            .await?;

        let latest_row_at = if video_count == 0 {
            // 首次扫描，使用全量模式
            debug!("检测到新番剧源（无历史记录），启用全量获取模式");
            None
        } else {
            // 已有记录，使用增量模式
            Some(self.latest_row_at.and_utc())
        };
        
        let bangumi = Bangumi::new(
            bili_client,
            self.media_id.clone(),
            self.season_id.clone(),
            self.ep_id.clone(),
        );

        let mode_desc = if latest_row_at.is_some() { "增量" } else { "全量" };
        
        if self.download_all_seasons {
            debug!("正在{}获取所有季度的番剧内容（时间过滤: {:?}）", mode_desc, latest_row_at);
            Ok(Box::pin(bangumi.to_all_seasons_video_stream_incremental(latest_row_at)))
        } else if let Some(ref selected_seasons) = self.selected_seasons {
            // 如果有选中的季度，只下载选中的季度
            debug!("正在{}获取选中的 {} 个季度的番剧内容（时间过滤: {:?}）", mode_desc, selected_seasons.len(), latest_row_at);
            Ok(Box::pin(
                bangumi.to_selected_seasons_video_stream_incremental(selected_seasons.clone(), latest_row_at),
            ))
        } else {
            debug!("正在{}获取当前季度的番剧内容（时间过滤: {:?}）", mode_desc, latest_row_at);
            Ok(Box::pin(bangumi.to_video_stream_incremental(latest_row_at)))
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

    fn scan_deleted_videos(&self) -> bool {
        self.scan_deleted_videos
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

    fn scan_deleted_videos(&self) -> bool {
        self.scan_deleted_videos
    }
}
