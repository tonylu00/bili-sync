use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use tracing::{debug, info, warn};

use super::{GpuInfo, GpuVendor, HardwareFingerprint, HardwareInfo, WebGLInfo};

// 定义数据库实体
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "hardware_fingerprint")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i64,
    pub config_type: String,
    pub gpu_vendor: String,
    pub gpu_model: String,
    pub gpu_device_id: String,
    pub gpu_driver_version: String,
    pub gpu_directx_version: String,
    pub gpu_angle_info: String,
    pub webgl_version: String,
    pub webgl_vendor: String,
    pub webgl_renderer: String,
    pub webgl_shading_language_version: String,
    pub webgl_extensions: String,
    pub screen_width: i32,
    pub screen_height: i32,
    pub device_pixel_ratio: f32,
    pub timezone_offset: i32,
    pub created_at: DateTimeUtc,
    pub last_used_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub fn find() -> Select<Entity> {
        <Entity as EntityTrait>::find()
    }

    pub fn find_by_user_id(user_id: i64) -> Select<Entity> {
        Self::find().filter(Column::UserId.eq(user_id))
    }
}

impl Model {
    /// 将数据库模型转换为硬件指纹
    pub fn to_hardware_fingerprint(&self) -> Result<HardwareFingerprint> {
        let gpu_vendor = match self.gpu_vendor.as_str() {
            "NVIDIA" => GpuVendor::Nvidia,
            "AMD" => GpuVendor::Amd,
            "Intel" => GpuVendor::Intel,
            _ => {
                warn!("未知的GPU厂商: {}, 默认使用NVIDIA", self.gpu_vendor);
                GpuVendor::Nvidia
            }
        };

        let gpu = GpuInfo {
            vendor: gpu_vendor,
            model: self.gpu_model.clone(),
            device_id: self.gpu_device_id.clone(),
            driver_version: self.gpu_driver_version.clone(),
            directx_version: self.gpu_directx_version.clone(),
            angle_info: self.gpu_angle_info.clone(),
        };

        let webgl_extensions: Vec<String> = serde_json::from_str(&self.webgl_extensions)
            .unwrap_or_else(|_| {
                warn!("WebGL扩展反序列化失败，使用默认扩展");
                vec!["ANGLE_instanced_arrays".to_string(), "EXT_blend_minmax".to_string()]
            });

        let webgl = WebGLInfo {
            version: self.webgl_version.clone(),
            shading_language_version: self.webgl_shading_language_version.clone(),
            vendor: self.webgl_vendor.clone(),
            renderer: self.webgl_renderer.clone(),
            extensions: webgl_extensions,
        };

        let hardware = HardwareInfo { gpu, webgl };

        Ok(HardwareFingerprint::from_components(
            hardware,
            (self.screen_width as u32, self.screen_height as u32),
            self.device_pixel_ratio,
            self.timezone_offset,
        ))
    }
}

/// 硬件指纹持久化管理器
pub struct HardwareFingerprintManager {
    db: DatabaseConnection,
}

impl HardwareFingerprintManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// 根据用户ID加载硬件指纹
    pub async fn load_for_user(&self, user_id: i64) -> Result<Option<HardwareFingerprint>> {
        debug!("从数据库加载用户 {} 的硬件指纹", user_id);

        let model = Entity::find_by_user_id(user_id)
            .one(&self.db)
            .await?;

        if let Some(model) = model {
            // 更新最后使用时间
            let mut active_model: ActiveModel = model.clone().into();
            active_model.last_used_at = Set(chrono::Utc::now());
            active_model.update(&self.db).await?;

            info!("成功加载用户 {} 的硬件指纹，配置类型: {}", user_id, model.config_type);
            Ok(Some(model.to_hardware_fingerprint()?))
        } else {
            debug!("用户 {} 没有保存的硬件指纹", user_id);
            Ok(None)
        }
    }

    /// 为用户保存硬件指纹
    pub async fn save_for_user(
        &self,
        user_id: i64,
        fingerprint: &HardwareFingerprint,
        config_type: &str,
    ) -> Result<()> {
        info!("为用户 {} 保存硬件指纹，配置类型: {}", user_id, config_type);

        let hardware_info = fingerprint.get_hardware_info();
        let webgl_extensions_json = serde_json::to_string(&hardware_info.webgl.extensions)?;

        let gpu_vendor_str = match hardware_info.gpu.vendor {
            GpuVendor::Nvidia => "NVIDIA",
            GpuVendor::Amd => "AMD",
            GpuVendor::Intel => "Intel",
        };

        // 先删除现有记录（如果存在）
        Entity::delete_many()
            .filter(Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;

        // 创建新记录
        let (screen_width, screen_height) = fingerprint.get_screen_resolution();
        let active_model = ActiveModel {
            user_id: Set(user_id),
            config_type: Set(config_type.to_string()),
            gpu_vendor: Set(gpu_vendor_str.to_string()),
            gpu_model: Set(hardware_info.gpu.model.clone()),
            gpu_device_id: Set(hardware_info.gpu.device_id.clone()),
            gpu_driver_version: Set(hardware_info.gpu.driver_version.clone()),
            gpu_directx_version: Set(hardware_info.gpu.directx_version.clone()),
            gpu_angle_info: Set(hardware_info.gpu.angle_info.clone()),
            webgl_version: Set(hardware_info.webgl.version.clone()),
            webgl_vendor: Set(hardware_info.webgl.vendor.clone()),
            webgl_renderer: Set(hardware_info.webgl.renderer.clone()),
            webgl_shading_language_version: Set(hardware_info.webgl.shading_language_version.clone()),
            webgl_extensions: Set(webgl_extensions_json),
            screen_width: Set(screen_width as i32),
            screen_height: Set(screen_height as i32),
            device_pixel_ratio: Set(fingerprint.get_device_pixel_ratio()),
            timezone_offset: Set(fingerprint.get_timezone_offset()),
            created_at: Set(chrono::Utc::now()),
            last_used_at: Set(chrono::Utc::now()),
            ..Default::default()
        };

        active_model.insert(&self.db).await?;
        info!("成功保存用户 {} 的硬件指纹到数据库", user_id);
        Ok(())
    }

    /// 删除用户的硬件指纹
    pub async fn delete_for_user(&self, user_id: i64) -> Result<()> {
        info!("删除用户 {} 的硬件指纹", user_id);

        let result = Entity::delete_many()
            .filter(Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;

        if result.rows_affected > 0 {
            info!("成功删除用户 {} 的硬件指纹", user_id);
        } else {
            debug!("用户 {} 没有需要删除的硬件指纹", user_id);
        }

        Ok(())
    }

    /// 获取所有保存的硬件指纹
    pub async fn list_all(&self) -> Result<Vec<Model>> {
        let models = Entity::find().all(&self.db).await?;
        Ok(models)
    }
}