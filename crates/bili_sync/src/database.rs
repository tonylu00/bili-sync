use anyhow::Result;
use bili_sync_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::{info, warn};

use crate::config::CONFIG_DIR;

fn database_url() -> String {
    // 确保配置目录存在
    if !CONFIG_DIR.exists() {
        std::fs::create_dir_all(&*CONFIG_DIR).expect("创建配置目录失败");
    }
    format!("sqlite://{}?mode=rwc", CONFIG_DIR.join("data.sqlite").to_string_lossy())
}

async fn database_connection() -> Result<DatabaseConnection> {
    // 动态配置 sqlx 慢查询阈值和日志级别
    configure_sqlx_settings();
    
    let mut option = ConnectOptions::new(database_url());
    
    option
        .max_connections(100)
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(90));

    let connection = Database::connect(option).await?;

    // 确保 WAL 模式已启用并应用额外的性能优化
    use sea_orm::ConnectionTrait;
    connection.execute_unprepared("PRAGMA journal_mode = WAL;").await?;
    connection.execute_unprepared("PRAGMA synchronous = NORMAL;").await?;
    connection.execute_unprepared("PRAGMA cache_size = 1000;").await?;
    connection.execute_unprepared("PRAGMA temp_store = memory;").await?;
    connection.execute_unprepared("PRAGMA mmap_size = 268435456;").await?; // 256MB
    connection
        .execute_unprepared("PRAGMA wal_autocheckpoint = 1000;")
        .await?;
    // 新增性能优化参数
    connection.execute_unprepared("PRAGMA busy_timeout = 30000;").await?; // 30秒超时
    connection.execute_unprepared("PRAGMA optimize;").await?; // 启用查询优化器
    connection.execute_unprepared("PRAGMA analysis_limit = 1000;").await?; // 分析限制

    info!("SQLite WAL 模式已启用，性能优化参数已应用");

    Ok(connection)
}

/// 根据环境变量动态配置 sqlx 慢查询阈值和日志级别
fn configure_sqlx_settings() {
    // 1. 配置慢查询阈值
    if let Ok(threshold_str) = std::env::var("SQLX_SLOW_THRESHOLD_SECONDS") {
        if let Ok(threshold_secs) = threshold_str.parse::<u64>() {
            // sqlx 内部使用 SQLX_SLOW_STATEMENTS_DURATION_THRESHOLD 环境变量（以毫秒为单位）
            let threshold_ms = threshold_secs * 1000;
            std::env::set_var("SQLX_SLOW_STATEMENTS_DURATION_THRESHOLD", threshold_ms.to_string());
            info!("设置慢查询阈值为: {} 秒 ({} 毫秒)", threshold_secs, threshold_ms);
        } else {
            warn!("SQLX_SLOW_THRESHOLD_SECONDS 值无效: {}, 使用默认阈值", threshold_str);
        }
    }
    
    // 2. 配置 sqlx 日志级别
    if let Ok(sqlx_level) = std::env::var("SQLX_LOG_LEVEL") {
        let current_rust_log = std::env::var("RUST_LOG").unwrap_or_default();
        
        // 移除现有的 sqlx 配置
        let rust_log_parts: Vec<&str> = current_rust_log
            .split(',')
            .filter(|part| !part.starts_with("sqlx="))
            .collect();
        
        // 添加新的 sqlx 配置
        let new_rust_log = if rust_log_parts.is_empty() {
            format!("sqlx={}", sqlx_level)
        } else {
            format!("{},sqlx={}", rust_log_parts.join(","), sqlx_level)
        };
        
        std::env::set_var("RUST_LOG", new_rust_log);
        info!("动态设置 sqlx 日志级别为: {}", sqlx_level);
    }
}

async fn migrate_database() -> Result<()> {
    // 检查数据库文件是否存在，不存在则会在连接时自动创建
    let db_path = CONFIG_DIR.join("data.sqlite");
    if !db_path.exists() {
        info!("数据库文件不存在，将创建新的数据库");
    } else {
        info!("检测到现有数据库文件，将在必要时应用迁移");
    }

    // 注意此处使用内部构造的 DatabaseConnection，而不是通过 database_connection() 获取
    // 这是因为使用多个连接的 Connection 会导致奇怪的迁移顺序问题，而使用默认的连接选项不会
    let connection = Database::connect(database_url()).await?;

    // 确保所有迁移都应用
    Ok(Migrator::up(&connection, None).await?)
}

/// 进行数据库迁移并获取数据库连接，供外部使用
pub async fn setup_database() -> DatabaseConnection {
    migrate_database().await.expect("数据库迁移失败");
    database_connection().await.expect("获取数据库连接失败")
}
