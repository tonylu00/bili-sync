use super::{
    response::{OfficialVerify, UserFollowing},
    wrapper::{ApiError, ApiResponse},
};
use crate::bilibili::BiliClient;
use anyhow::{ensure, Context, Result};
use axum::Extension;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde::Serialize;
use std::sync::Arc;

static REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Serialize)]
pub struct FollowingSnapshot {
    pub account_uid: String,
    pub fetched_at: String,
    pub followings: Vec<UserFollowing>,
}

pub(crate) async fn read_cache(db: &DatabaseConnection, uid: i64) -> Result<Option<FollowingSnapshot>> {
    db.query_one(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "SELECT fetched_at, payload FROM following_cache WHERE account_uid = ?",
        [uid.into()],
    ))
    .await?
    .map(|row| {
        Ok(FollowingSnapshot {
            account_uid: uid.to_string(),
            fetched_at: row.try_get("", "fetched_at")?,
            followings: serde_json::from_str(&row.try_get::<String>("", "payload")?)?,
        })
    })
    .transpose()
}

async fn save_cache(db: &DatabaseConnection, uid: i64, followings: Vec<UserFollowing>) -> Result<FollowingSnapshot> {
    let fetched_at = crate::utils::time_format::now_standard_string();
    db.execute(Statement::from_sql_and_values(DbBackend::Sqlite,
        "INSERT INTO following_cache(account_uid, fetched_at, payload) VALUES (?, ?, ?) ON CONFLICT(account_uid) DO UPDATE SET fetched_at = excluded.fetched_at, payload = excluded.payload",
        [uid.into(), fetched_at.clone().into(), serde_json::to_string(&followings)?.into()])).await?;
    Ok(FollowingSnapshot {
        account_uid: uid.to_string(),
        fetched_at,
        followings,
    })
}

pub(crate) async fn load(db: &DatabaseConnection, refresh: bool) -> Result<FollowingSnapshot> {
    let client = BiliClient::new(String::new());
    let uid = client.get_current_user_id()?;
    if !refresh {
        if let Some(snapshot) = read_cache(db, uid).await? {
            return Ok(snapshot);
        }
    }
    let _guard = REFRESH_LOCK.lock().await;
    if !refresh {
        if let Some(snapshot) = read_cache(db, uid).await? {
            return Ok(snapshot);
        }
    }
    let followings = client
        .get_user_followings()
        .await
        .context("刷新关注列表失败，数据库中的备份未修改")?;
    ensure!(
        client.get_current_user_id()? == uid,
        "登录账号已变更，请重新载入关注列表"
    );
    let followings = followings
        .into_iter()
        .map(|f| UserFollowing {
            mid: f.mid,
            name: f.name,
            face: f.face,
            sign: f.sign,
            official_verify: f.official_verify.map(|v| OfficialVerify {
                type_: v.type_,
                desc: v.desc,
            }),
        })
        .collect();
    save_cache(db, uid, followings).await
}

pub async fn get_snapshot(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<FollowingSnapshot>, ApiError> {
    Ok(ApiResponse::ok(load(&db, false).await?))
}
pub async fn refresh_snapshot(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<FollowingSnapshot>, ApiError> {
    Ok(ApiResponse::ok(load(&db, true).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bili_sync_migration::{Migrator, MigratorTrait};
    #[tokio::test]
    async fn following_backup_is_account_scoped_and_empty_snapshot_is_saved() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        let sample = || UserFollowing {
            mid: 42,
            name: "Creator".into(),
            face: "".into(),
            sign: "".into(),
            official_verify: None,
        };
        save_cache(&db, 1, vec![sample()]).await.unwrap();
        save_cache(&db, 2, vec![]).await.unwrap();
        assert_eq!(read_cache(&db, 1).await.unwrap().unwrap().followings[0].mid, 42);
        assert!(read_cache(&db, 2).await.unwrap().unwrap().followings.is_empty());
        assert!(read_cache(&db, 3).await.unwrap().is_none());
        save_cache(&db, 1, vec![]).await.unwrap();
        assert!(read_cache(&db, 1).await.unwrap().unwrap().followings.is_empty());
    }
}
