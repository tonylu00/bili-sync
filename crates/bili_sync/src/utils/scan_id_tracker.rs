use std::collections::HashMap;
use anyhow::Result;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

/// 扫描源ID跟踪器，用于记录每个源类型的最后扫描ID
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LastScannedIds {
    #[serde(default)]
    pub collection: Option<i32>,
    #[serde(default)]
    pub favorite: Option<i32>,
    #[serde(default)]
    pub submission: Option<i32>,
    #[serde(default)]
    pub watch_later: Option<i32>,
    #[serde(default)]
    pub bangumi: Option<i32>,
}

const CONFIG_KEY: &str = "last_scanned_ids";

/// 从数据库获取最后扫描的ID记录
pub async fn get_last_scanned_ids(db: &DatabaseConnection) -> Result<LastScannedIds> {
    use bili_sync_entity::entities::{config_item, prelude::ConfigItem};
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
    
    // 查询配置项
    let config_item = ConfigItem::find()
        .filter(config_item::Column::KeyName.eq(CONFIG_KEY))
        .one(db)
        .await?;
    
    match config_item {
        Some(item) => {
            // 解析JSON值
            let ids: LastScannedIds = serde_json::from_str(&item.value_json)?;
            Ok(ids)
        }
        None => {
            // 配置项不存在，返回默认值
            Ok(LastScannedIds::default())
        }
    }
}

/// 更新最后扫描的ID记录到数据库
pub async fn update_last_scanned_ids(db: &DatabaseConnection, ids: &LastScannedIds) -> Result<()> {
    use bili_sync_entity::entities::{config_item, prelude::ConfigItem};
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
    
    let value_json = serde_json::to_string(ids)?;
    
    // 查询是否已存在
    let existing = ConfigItem::find()
        .filter(config_item::Column::KeyName.eq(CONFIG_KEY))
        .one(db)
        .await?;
    
    if let Some(existing_item) = existing {
        // 更新现有记录
        let mut active_model: config_item::ActiveModel = existing_item.into();
        active_model.value_json = Set(value_json);
        active_model.updated_at = Set(chrono::Utc::now());
        active_model.update(db).await?;
    } else {
        // 创建新记录
        let new_item = config_item::ActiveModel {
            key_name: Set(CONFIG_KEY.to_string()),
            value_json: Set(value_json),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        };
        new_item.insert(db).await?;
    }
    
    Ok(())
}

/// 视频源信息，包含ID和其他必要信息
#[derive(Debug, Clone)]
pub struct VideoSourceWithId {
    pub id: i32,
    pub args: crate::adapter::Args,
    pub path: std::path::PathBuf,
    pub source_type: SourceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceType {
    Collection,
    Favorite,
    Submission,
    WatchLater,
    Bangumi,
}


/// 将视频源按新旧分组
pub fn group_sources_by_new_old(
    sources: Vec<VideoSourceWithId>,
    last_scanned_ids: &LastScannedIds,
) -> (Vec<VideoSourceWithId>, Vec<VideoSourceWithId>) {
    let mut new_sources = Vec::new();
    let mut old_sources = Vec::new();
    
    for source in sources {
        let last_id = match source.source_type {
            SourceType::Collection => last_scanned_ids.collection,
            SourceType::Favorite => last_scanned_ids.favorite,
            SourceType::Submission => last_scanned_ids.submission,
            SourceType::WatchLater => last_scanned_ids.watch_later,
            SourceType::Bangumi => last_scanned_ids.bangumi,
        };
        
        // 如果没有记录（首次运行）或ID大于最后扫描的ID，则为新源
        if last_id.is_none() || source.id > last_id.unwrap() {
            new_sources.push(source);
        } else {
            old_sources.push(source);
        }
    }
    
    (new_sources, old_sources)
}

/// 记录本轮扫描的最大ID
pub struct MaxIdRecorder {
    max_ids: HashMap<SourceType, i32>,
}

impl MaxIdRecorder {
    pub fn new() -> Self {
        Self {
            max_ids: HashMap::new(),
        }
    }
    
    /// 记录一个源的ID
    pub fn record(&mut self, source_type: SourceType, id: i32) {
        self.max_ids
            .entry(source_type)
            .and_modify(|max_id| {
                if id > *max_id {
                    *max_id = id;
                }
            })
            .or_insert(id);
    }
    
    /// 获取记录的最大ID，更新到LastScannedIds中
    pub fn merge_into(&self, last_scanned_ids: &mut LastScannedIds) {
        for (source_type, &max_id) in &self.max_ids {
            match source_type {
                SourceType::Collection => {
                    last_scanned_ids.collection = Some(max_id.max(last_scanned_ids.collection.unwrap_or(0)));
                }
                SourceType::Favorite => {
                    last_scanned_ids.favorite = Some(max_id.max(last_scanned_ids.favorite.unwrap_or(0)));
                }
                SourceType::Submission => {
                    last_scanned_ids.submission = Some(max_id.max(last_scanned_ids.submission.unwrap_or(0)));
                }
                SourceType::WatchLater => {
                    last_scanned_ids.watch_later = Some(max_id.max(last_scanned_ids.watch_later.unwrap_or(0)));
                }
                SourceType::Bangumi => {
                    last_scanned_ids.bangumi = Some(max_id.max(last_scanned_ids.bangumi.unwrap_or(0)));
                }
            }
        }
    }
}