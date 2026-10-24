use super::{
    error::InnerApiError,
    wrapper::{ApiError, ApiResponse},
};
use crate::utils::{
    creators::{self, Creator},
    uploader_filter::UploaderFilter,
};
use axum::{extract::Path, Extension, Json};
use bili_sync_entity::{page, video};
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QueryFilter, Statement};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

#[derive(Serialize, Default)]
pub struct UploaderUsage {
    uid: String,
    name: String,
    primary_video_count: usize,
    primary_bytes: u64,
    coauthored_video_count: usize,
    coauthored_bytes: u64,
    filters: UploaderFilter,
}
#[derive(Serialize)]
pub struct LibraryUsage {
    uploaders: Vec<UploaderUsage>,
    unreadable_files: usize,
    missing_files: usize,
}
#[derive(Serialize)]
pub struct LibraryVideo {
    id: i32,
    name: String,
    bvid: String,
    creators: Vec<Creator>,
    sources: Vec<LibraryVideoSource>,
}
#[derive(Serialize)]
pub struct LibraryVideoSource {
    video_id: i32,
    source_type: &'static str,
    source_id: Option<i32>,
}

async fn rules(db: &DatabaseConnection) -> anyhow::Result<HashMap<String, UploaderFilter>> {
    db.query_all(Statement::from_string(
        DbBackend::Sqlite,
        "SELECT upper_id, settings FROM uploader_filter",
    ))
    .await?
    .into_iter()
    .map(|r| {
        Ok((
            r.try_get::<i64>("", "upper_id")?.to_string(),
            serde_json::from_str(&r.try_get::<String>("", "settings")?)?,
        ))
    })
    .collect()
}

// File identity deduplicates paths and hard links. Directories are never recursively counted:
// a source folder may contain videos belonging to many unrelated creators.
fn file_size(path: &str) -> std::io::Result<Option<(Arc<same_file::Handle>, u64)>> {
    let meta = std::fs::metadata(path)?;
    if !meta.is_file() {
        return Ok(None);
    }
    Ok(Some((
        crate::utils::shared_media::file_identity(std::path::Path::new(path))?,
        meta.len(),
    )))
}

fn aggregate(
    videos: Vec<video::Model>,
    pages: Vec<page::Model>,
    mut filters: HashMap<String, UploaderFilter>,
) -> LibraryUsage {
    let mut paths: HashMap<i32, Vec<String>> = HashMap::new();
    for p in pages {
        if let Some(path) = p.path {
            paths.entry(p.video_id).or_default().push(path);
        }
    }
    let mut rows: BTreeMap<String, UploaderUsage> = BTreeMap::new();
    let mut counts = HashSet::new();
    let mut files = HashSet::new();
    let mut measurements = HashMap::new();
    let mut missing_files = 0;
    let mut unreadable_files = 0;
    for v in videos {
        let authors = creators::from_video(&v);
        if authors.is_empty() {
            continue;
        }
        let video_paths = paths.remove(&v.id).unwrap_or_default();
        let mut sizes = Vec::new();
        // video.path is normally a directory; include it only if it actually is a file.
        for path in video_paths
            .into_iter()
            .chain(std::iter::once(v.path.clone()))
            .filter(|p| !p.is_empty())
        {
            let size = measurements
                .entry(path.clone())
                .or_insert_with(|| match file_size(&path) {
                    Ok(value) => value,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            missing_files += 1;
                        } else {
                            unreadable_files += 1;
                        }
                        None
                    }
                });
            if let Some(size) = size {
                sizes.push(size.clone());
            }
        }
        for author in authors {
            let row = rows.entry(author.uid.clone()).or_insert_with(|| UploaderUsage {
                uid: author.uid.clone(),
                name: author.name.clone(),
                filters: filters.remove(&author.uid).unwrap_or_default(),
                ..Default::default()
            });
            // Copies across different sources count as one video, but their distinct files count towards space.
            let identity = if v.bvid.is_empty() {
                format!("row:{}", v.id)
            } else {
                v.bvid.clone()
            };
            if counts.insert((author.uid.clone(), author.primary, identity)) {
                if author.primary {
                    row.primary_video_count += 1;
                } else {
                    row.coauthored_video_count += 1;
                }
            }
            for (key, bytes) in &sizes {
                if files.insert((author.uid.clone(), author.primary, key.clone())) {
                    if author.primary {
                        row.primary_bytes += bytes;
                    } else {
                        row.coauthored_bytes += bytes;
                    }
                }
            }
        }
    }
    LibraryUsage {
        uploaders: rows.into_values().collect(),
        missing_files,
        unreadable_files,
    }
}

pub async fn get_usage(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<LibraryUsage>, ApiError> {
    let _storage_guard = crate::utils::shared_media::GATE.read().await;
    let videos = video::Entity::find()
        .filter(video::Column::Deleted.eq(0))
        .all(db.as_ref())
        .await?;
    let pages = page::Entity::find().all(db.as_ref()).await?;
    let filters = rules(&db).await?;
    let result = tokio::task::spawn_blocking(move || aggregate(videos, pages, filters)).await?;
    Ok(ApiResponse::ok(result))
}

pub async fn get_videos(
    Path(uid): Path<i64>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<Vec<LibraryVideo>>, ApiError> {
    let uid = uid.to_string();
    let videos = video::Entity::find()
        .filter(video::Column::Deleted.eq(0))
        .all(db.as_ref())
        .await?;
    let mut grouped: BTreeMap<String, LibraryVideo> = BTreeMap::new();
    for v in videos {
        let creators = creators::from_video(&v);
        if !creators.iter().any(|c| c.uid == uid) {
            continue;
        }
        let source = if v.submission_id.is_some() {
            ("UP主", v.submission_id)
        } else if v.favorite_id.is_some() {
            ("收藏夹", v.favorite_id)
        } else if v.collection_id.is_some() {
            ("合集", v.collection_id)
        } else if v.watch_later_id.is_some() {
            ("稍后观看", v.watch_later_id)
        } else {
            ("番剧", v.source_id)
        };
        let source = LibraryVideoSource {
            video_id: v.id,
            source_type: source.0,
            source_id: source.1,
        };
        let identity = if v.bvid.is_empty() {
            format!("row:{}", v.id)
        } else {
            v.bvid.clone()
        };
        let entry = grouped.entry(identity).or_insert_with(|| LibraryVideo {
            id: v.id,
            name: v.name,
            bvid: v.bvid,
            creators: creators.clone(),
            sources: Vec::new(),
        });
        if creators.len() > entry.creators.len() {
            entry.creators = creators;
        }
        entry.sources.push(source);
    }
    let videos: Vec<_> = grouped.into_values().collect();
    Ok(ApiResponse::ok(videos))
}

pub async fn save_filter(
    Path(uid): Path<i64>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
    Json(mut filter): Json<UploaderFilter>,
) -> Result<ApiResponse<UploaderFilter>, ApiError> {
    if uid <= 0 {
        return Err(InnerApiError::BadRequest("无效的UP主UID".into()).into());
    }
    filter
        .validate()
        .map_err(|e| InnerApiError::BadRequest(e.to_string()))?;
    db.execute(Statement::from_sql_and_values(DbBackend::Sqlite,
        "INSERT INTO uploader_filter(upper_id, settings, updated_at) VALUES (?, ?, ?) ON CONFLICT(upper_id) DO UPDATE SET settings = excluded.settings, updated_at = excluded.updated_at",
        [uid.into(), serde_json::to_string(&filter)?.into(), crate::utils::time_format::now_standard_string().into()])).await?;
    Ok(ApiResponse::ok(filter))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_separate_authors_and_deduplicates_files_and_copies() {
        let dir = std::env::temp_dir().join(format!("bili-library-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("video.mp4");
        std::fs::write(&path, [0; 42]).unwrap();
        let v = video::Model {
            id: 1,
            upper_id: 11,
            upper_name: "Main".into(),
            bvid: "BV1".into(),
            path: dir.to_string_lossy().into(),
            staff_info: Some(serde_json::json!([{ "mid": 11, "name": "Main" }, { "mid": 22, "name": "Partner" }])),
            ..Default::default()
        };
        let p = page::Model {
            video_id: 1,
            path: Some(path.to_string_lossy().into()),
            ..Default::default()
        };
        let result = aggregate(
            vec![v.clone(), video::Model { id: 2, ..v }],
            vec![p.clone(), page::Model { video_id: 2, ..p }],
            HashMap::new(),
        );
        assert_eq!(result.uploaders.len(), 2);
        assert_eq!(result.uploaders[0].primary_bytes, 42);
        assert_eq!(result.uploaders[0].primary_video_count, 1);
        assert_eq!(result.uploaders[1].primary_bytes, 0);
        assert_eq!(result.uploaders[1].coauthored_bytes, 42);
        assert_eq!(result.uploaders[1].coauthored_video_count, 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

pub async fn preview_dedup(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<crate::utils::shared_media::Plan>, ApiError> {
    // Detach execution from the HTTP connection so closing the tab cannot interrupt publication.
    let plan = tokio::spawn(async move {
        let _guard = crate::utils::shared_media::GATE.write().await;
        crate::utils::shared_media::preview(&db).await
    })
    .await??;
    Ok(ApiResponse::ok(plan))
}

pub async fn execute_dedup(
    Path(id): Path<String>,
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<crate::utils::shared_media::Plan>, ApiError> {
    uuid::Uuid::parse_str(&id).map_err(|_| InnerApiError::BadRequest("无效的迁移预览".into()))?;
    let plan = tokio::spawn(async move {
        let _guard = crate::utils::shared_media::GATE.write().await;
        let backup_dir = crate::config::CONFIG_DIR.join("storage-backups");
        tokio::fs::create_dir_all(&backup_dir).await?;
        let backup = backup_dir.join(format!("{id}.sqlite"));
        if !backup.try_exists()? {
            db.execute(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "VACUUM INTO ?",
                [backup.to_string_lossy().to_string().into()],
            ))
            .await?;
        }
        crate::utils::shared_media::execute(&db, &id).await
    })
    .await??;
    Ok(ApiResponse::ok(plan))
}

#[derive(Serialize)]
pub struct StorageOperation {
    id: String,
    state: String,
    error: Option<String>,
    plan: crate::utils::shared_media::Plan,
}
pub async fn dedup_history(
    Extension(db): Extension<Arc<DatabaseConnection>>,
) -> Result<ApiResponse<Vec<StorageOperation>>, ApiError> {
    let rows = db.query_all(Statement::from_string(DbBackend::Sqlite, "SELECT id,state,error,payload FROM storage_operation WHERE kind='dedup' ORDER BY created_at DESC,rowid DESC LIMIT 10")).await?;
    let history: anyhow::Result<Vec<_>> = rows
        .into_iter()
        .map(|r| {
            Ok(StorageOperation {
                id: r.try_get("", "id")?,
                state: r.try_get("", "state")?,
                error: r.try_get("", "error")?,
                plan: serde_json::from_str(&r.try_get::<String>("", "payload")?)?,
            })
        })
        .collect();
    Ok(ApiResponse::ok(history?))
}
