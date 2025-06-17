use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use arc_swap::ArcSwap;
use clap::Parser;
use once_cell::sync::Lazy;
use tracing::{info, warn};

use crate::config::clap::Args;
use crate::config::{Config, ConfigBundle};

/// 全局的配置包，使用 ArcSwap 支持热重载
/// 包含配置、模板引擎、限流器等所有需要热重载的组件
/// 初始化时使用空配置包，在数据库初始化后再设置真实配置
pub static CONFIG_BUNDLE: Lazy<ArcSwap<ConfigBundle>> =
    Lazy::new(|| ArcSwap::from_pointee(load_minimal_config_bundle()));

/// 全局的配置管理器，用于数据库操作
static CONFIG_MANAGER: Lazy<RwLock<Option<crate::config::ConfigManager>>> = Lazy::new(|| RwLock::new(None));

/// 设置配置管理器（在应用启动时调用）
pub fn set_config_manager(manager: crate::config::ConfigManager) {
    let mut guard = CONFIG_MANAGER.write().unwrap();
    *guard = Some(manager);
    info!("配置管理器已设置");
}

/// 获取配置管理器（用于credential刷新等场景）
pub fn get_config_manager() -> Option<crate::config::ConfigManager> {
    let guard = CONFIG_MANAGER.read().unwrap();
    guard.clone()
}

/// 重新加载配置包（支持热重载）
pub async fn reload_config_bundle() -> Result<()> {
    let manager_opt = {
        let manager_guard = CONFIG_MANAGER.read().unwrap();
        manager_guard.clone()
    };

    let new_bundle = if let Some(manager) = manager_opt {
        // 从数据库加载配置
        manager.load_config_bundle().await?
    } else {
        // 回退到TOML加载
        warn!("配置管理器未初始化，回退到TOML加载");
        let config = load_config();
        ConfigBundle::from_config(config)?
    };

    CONFIG_BUNDLE.store(Arc::new(new_bundle));
    info!("配置已重新加载");
    Ok(())
}

/// 访问配置包的便捷函数
pub fn with_config<F, R>(f: F) -> R
where
    F: FnOnce(&ConfigBundle) -> R,
{
    let bundle = CONFIG_BUNDLE.load();
    f(&bundle)
}

/// 获取配置的便捷函数（向后兼容）
#[allow(dead_code)]
pub fn get_config() -> Arc<ConfigBundle> {
    CONFIG_BUNDLE.load_full()
}

/// 向后兼容的配置重载函数（同步版本）
pub fn reload_config() -> Config {
    // 从当前配置包中提取配置
    with_config(|bundle| bundle.config.clone())
}

/// 向后兼容的全局配置获取函数
#[allow(dead_code)]
pub fn get_current_config() -> Config {
    with_config(|bundle| bundle.config.clone())
}

/// 向后兼容的全局配置引用 - 已弃用，请使用reload_config()函数
#[deprecated(note = "配置现在完全基于数据库，请使用reload_config()函数")]
#[allow(dead_code)]
pub static CONFIG: Lazy<Config> = Lazy::new(load_config);

/// 向后兼容的全局模板引擎引用 - 已弃用，请使用ConfigBundle中的handlebars
#[deprecated(note = "模板引擎现在通过ConfigBundle提供热更新支持，请使用with_config(|bundle| bundle.handlebars)")]
#[allow(dead_code)]
pub static TEMPLATE: Lazy<handlebars::Handlebars<'static>> = Lazy::new(|| {
    use crate::config::PathSafeTemplate;
    use handlebars::handlebars_helper;
    
    let config = load_config();
    let mut handlebars = handlebars::Handlebars::new();
    
    // 注册自定义 helper
    handlebars_helper!(truncate: |s: String, len: usize| {
        if s.chars().count() > len {
            s.chars().take(len).collect::<String>()
        } else {
            s.to_string()
        }
    });
    handlebars.register_helper("truncate", Box::new(truncate));
    
    // 注册所有必需的模板
    let video_name = Box::leak(config.video_name.to_string().into_boxed_str());
    let page_name = Box::leak(config.page_name.to_string().into_boxed_str());
    let multi_page_name = Box::leak(config.multi_page_name.to_string().into_boxed_str());
    let bangumi_name = Box::leak(config.bangumi_name.to_string().into_boxed_str());
    
    handlebars.path_safe_register("video", video_name).expect("注册video模板失败");
    handlebars.path_safe_register("page", page_name).expect("注册page模板失败");
    handlebars.path_safe_register("multi_page", multi_page_name).expect("注册multi_page模板失败");
    handlebars.path_safe_register("bangumi", bangumi_name).expect("注册bangumi模板失败");
    
    handlebars
});

/// 加载最小配置包（不进行配置检查，避免重复警告）
fn load_minimal_config_bundle() -> ConfigBundle {
    info!("开始加载配置包..");

    // 创建默认配置但不进行检查
    let config = Config::default();
    let bundle = ConfigBundle::from_config(config).expect("创建配置包失败");
    info!("配置包加载完毕");
    bundle
}

/// 加载初始配置包（已弃用，由数据库配置系统取代）
#[allow(dead_code)]
fn load_initial_config_bundle() -> ConfigBundle {
    info!("开始加载配置包..");

    // 初始加载时，配置管理器可能还没有设置，所以先从TOML加载
    let config = load_config();
    let bundle = ConfigBundle::from_config(config).expect("创建配置包失败");
    info!("配置包加载完毕");
    bundle
}

/// 异步初始化配置系统（在数据库连接建立后调用）
pub async fn init_config_with_database(db: sea_orm::DatabaseConnection) -> Result<()> {
    info!("开始初始化数据库配置系统");

    // 创建配置管理器
    let manager = crate::config::ConfigManager::new(db);

    // 确保配置表存在
    manager.ensure_tables_exist().await?;

    // 尝试从数据库加载配置，如果失败则从TOML迁移
    let new_bundle = manager.load_config_bundle().await?;

    // 设置全局配置管理器
    set_config_manager(manager);

    // 更新全局配置包
    CONFIG_BUNDLE.store(Arc::new(new_bundle));

    // 配置检查已简化，因为配置现在完全基于数据库
    info!("检查配置..");
    #[cfg(not(test))]
    {
        let config = reload_config();
        if config.check() {
            info!("配置检查通过");
        } else {
            info!("您可以访问管理页 http://{}/ 添加视频源", config.bind_address);
        }
    }
    #[cfg(test)]
    {
        info!("配置检查通过（测试模式）");
    }

    info!("数据库配置系统初始化完成");
    Ok(())
}

/// 向后兼容的配置加载函数
pub fn load_config() -> Config {
    #[cfg(not(test))]
    {
        load_config_impl()
    }
    #[cfg(test)]
    {
        load_config_test()
    }
}

/// 全局的 ARGS，用来解析命令行参数
pub static ARGS: Lazy<Args> = Lazy::new(Args::parse);

/// 全局的 CONFIG_DIR，表示配置文件夹的路径
pub static CONFIG_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::config_dir().expect("No config path found").join("bili-sync"));

#[cfg(not(test))]
fn load_config_impl() -> Config {
    info!("开始加载默认配置..");
    // 配置现在完全基于数据库，不再从配置文件加载
    let config = Config::default();
    info!("默认配置加载完毕");
    // 移除配置检查，避免在静态初始化时产生警告
    // 配置检查将在数据库配置系统初始化后进行
    // info!("检查配置..");
    // if config.check() {
    //     info!("配置检查通过");
    // } else {
    //     info!("您可以访问管理页 http://{}/ 添加视频源", config.bind_address);
    // }
    config
}

#[cfg(test)]
fn load_config_test() -> Config {
    let credential = match (
        std::env::var("TEST_SESSDATA"),
        std::env::var("TEST_BILI_JCT"),
        std::env::var("TEST_BUVID3"),
        std::env::var("TEST_DEDEUSERID"),
        std::env::var("TEST_AC_TIME_VALUE"),
    ) {
        (Ok(sessdata), Ok(bili_jct), Ok(buvid3), Ok(dedeuserid), Ok(ac_time_value)) => {
            Some(std::sync::Arc::new(crate::bilibili::Credential {
                sessdata,
                bili_jct,
                buvid3,
                dedeuserid,
                ac_time_value,
            }))
        }
        _ => None,
    };
    Config {
        credential: arc_swap::ArcSwapOption::from(credential),
        cdn_sorting: true,
        ..Default::default()
    }
}
