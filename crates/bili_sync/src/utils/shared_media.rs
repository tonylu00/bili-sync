//! One physical media file per BV/CID. Legacy source/page IDs remain presentation records.
//! Lock order: library gate -> part lock -> short database transaction. No network in a transaction.
use anyhow::{bail, ensure, Context, Result};
use rsa::sha2::{Digest, Sha256};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::LazyLock,
};
use tokio::{
    io::AsyncReadExt,
    sync::{Mutex, MutexGuard, RwLock},
};

pub static GATE: RwLock<()> = RwLock::const_new(());
static PART_LOCKS: LazyLock<Vec<Mutex<()>>> = LazyLock::new(|| (0..256).map(|_| Mutex::new(())).collect());
fn statement(sql: &str, values: impl IntoIterator<Item = sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Sqlite, sql, values)
}

pub fn process_lock() -> Result<std::fs::File> {
    std::fs::create_dir_all(&*crate::config::CONFIG_DIR)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(crate::config::CONFIG_DIR.join("media-library.lock"))?;
    file.try_lock()
        .context("Another bili-sync process is using this library")?;
    Ok(file)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub page_id: i32,
    pub video_id: i32,
    pub bvid: String,
    pub cid: i64,
    pub path: String,
    pub name: String,
    pub upper_id: i64,
    pub staff: Vec<i64>,
    pub source_uid: Option<i64>,
    pub source_kind: String,
    pub source_created: String,
    pub source_root: String,
    pub source_id: i32,
    pub completed: bool,
    pub duration: u32,
}
impl Candidate {
    fn priority(&self) -> (u8, usize, &str, i32, i32) {
        let own_location = !self.source_root.is_empty() && Path::new(&self.path).starts_with(&self.source_root);
        let (rank, order) = if !own_location {
            (4, 0)
        } else {
            match self.source_uid {
                Some(uid) if uid == self.upper_id && uid > 0 => (0, 0),
                Some(uid) => self
                    .staff
                    .iter()
                    .position(|id| *id == uid)
                    .map(|i| (1, i))
                    .unwrap_or((4, 0)),
                None if self.source_kind == "collection" || self.source_kind == "favorite" => (2, 0),
                None => (3, 0),
            }
        };
        (rank, order, &self.source_created, self.source_id, self.page_id)
    }
    fn reason(&self) -> String {
        match self.priority().0 {
            0 => "主投稿者的文件夹".into(),
            1 => format!("第 {} 位联合创作者的文件夹", self.priority().1 + 1),
            2 => "最先添加的合集或收藏夹".into(),
            _ => "其他已有来源".into(),
        }
    }
}

async fn candidates(db: &DatabaseConnection, key: Option<(&str, i64)>) -> Result<Vec<Candidate>> {
    let mut sql = String::from("SELECT p.id AS page_id, v.id AS video_id, v.bvid, p.cid, p.path, v.name, v.upper_id, v.staff_info, p.duration, p.download_status, s.upper_id AS source_uid, CASE WHEN s.id IS NOT NULL THEN 'submission' WHEN c.id IS NOT NULL THEN 'collection' WHEN f.id IS NOT NULL THEN 'favorite' ELSE 'other' END AS source_kind, COALESCE(s.path, c.path, f.path, '') AS source_root, COALESCE(s.created_at, c.created_at, f.created_at, v.created_at, '') AS source_created, COALESCE(s.id, c.id, f.id, v.id) AS source_id FROM page p JOIN video v ON v.id=p.video_id LEFT JOIN submission s ON s.id=v.submission_id LEFT JOIN collection c ON c.id=v.collection_id LEFT JOIN favorite f ON f.id=v.favorite_id WHERE v.deleted=0 AND p.path IS NOT NULL AND p.path<>''");
    let mut values = Vec::new();
    if let Some((bvid, cid)) = key {
        sql.push_str(" AND v.bvid=? AND p.cid=?");
        values = vec![bvid.into(), cid.into()];
    }
    db.query_all(statement(&sql, values))
        .await?
        .into_iter()
        .map(|r| {
            let owner: i64 = r.try_get("", "upper_id")?;
            let staff = r
                .try_get::<Option<String>>("", "staff_info")?
                .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|p| p["mid"].as_i64())
                .filter(|id| *id > 0 && *id != owner)
                .collect();
            Ok(Candidate {
                page_id: r.try_get("", "page_id")?,
                video_id: r.try_get("", "video_id")?,
                bvid: r.try_get("", "bvid")?,
                cid: r.try_get("", "cid")?,
                path: r.try_get("", "path")?,
                name: r.try_get("", "name")?,
                upper_id: r.try_get("", "upper_id")?,
                staff,
                source_uid: r.try_get("", "source_uid")?,
                source_kind: r.try_get("", "source_kind")?,
                source_created: r.try_get("", "source_created")?,
                source_root: r.try_get("", "source_root")?,
                source_id: r.try_get("", "source_id")?,
                completed: (r.try_get::<u32>("", "download_status")? >> 3) & 7 == 7,
                duration: r.try_get("", "duration")?,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fingerprint {
    pub bytes: u64,
    pub sha256: String,
}
async fn fingerprint(path: &Path) -> Result<Fingerprint> {
    let before = tokio::fs::symlink_metadata(path).await?;
    ensure!(
        before.is_file() && before.len() > 0,
        "不是完整的普通媒体文件: {}",
        path.display()
    );
    let mut file = tokio::fs::File::open(path).await?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    let mut bytes = 0;
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        bytes += n as u64;
        hash.update(&buffer[..n]);
    }
    let after = tokio::fs::metadata(path).await?;
    ensure!(
        bytes == before.len() && bytes == after.len() && before.modified()? == after.modified()?,
        "文件在校验中发生变化"
    );
    Ok(Fingerprint {
        bytes,
        sha256: hex::encode(hash.finalize()),
    })
}

async fn probe(path: &Path, expected_duration: u32) -> Result<()> {
    let mut cmd = tokio::process::Command::new("ffprobe");
    cmd.kill_on_drop(true)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type",
            "-of",
            "json",
        ])
        .arg(path);
    let output = tokio::time::timeout(std::time::Duration::from_secs(60), cmd.output()).await??;
    ensure!(output.status.success(), "媒体完整性检查失败: {}", path.display());
    let info: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let duration = info["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    ensure!(duration.is_finite() && duration > 0.0, "媒体时长无效");
    ensure!(
        expected_duration == 0 || duration + 3.0 >= expected_duration as f64 * 0.98,
        "媒体比记录的时长短，保留为冲突"
    );
    ensure!(
        info["streams"]
            .as_array()
            .is_some_and(|s| s.iter().any(|v| v["codec_type"] == "video")),
        "没有有效的视频轨道"
    );
    Ok(())
}

async fn register<C: ConnectionTrait>(db: &C, bvid: &str, cid: i64, path: &str, fp: &Fingerprint) -> Result<()> {
    db.execute(statement("INSERT INTO media_part(bvid,cid,path,bytes,sha256) VALUES(?,?,?,?,?) ON CONFLICT(bvid,cid) DO UPDATE SET path=excluded.path, bytes=excluded.bytes, sha256=excluded.sha256",
        vec![bvid.into(), cid.into(), path.into(), (fp.bytes as i64).into(), fp.sha256.clone().into()])).await?;
    Ok(())
}
async fn bind<C: ConnectionTrait>(db: &C, page_id: i32, requested: &str, canonical: &str) -> Result<()> {
    db.execute(statement(
        "INSERT INTO media_view(page_id,requested_path) VALUES(?,?) ON CONFLICT(page_id) DO NOTHING",
        vec![page_id.into(), requested.into()],
    ))
    .await?;
    db.execute(statement(
        "UPDATE page SET path=?, download_status=(download_status & ~56) | 56 WHERE id=?",
        vec![canonical.into(), page_id.into()],
    ))
    .await?;
    Ok(())
}

/// A lease spans both network download and durable publication. The caller holds GATE.read().
pub struct Lease {
    _lock: MutexGuard<'static, ()>,
    pub path: PathBuf,
    pub ready: bool,
    bvid: String,
    cid: i64,
    page_id: i32,
    requested: PathBuf,
    operation: String,
}
pub async fn acquire(db: &DatabaseConnection, bvid: &str, cid: i64, page_id: i32, requested: &Path) -> Result<Lease> {
    ensure!(bvid.starts_with("BV") && cid > 0, "无法共享媒体：尚未确认 BV/CID");
    let digest = Sha256::digest(format!("{bvid}:{cid}"));
    let lock = PART_LOCKS[digest[0] as usize].lock().await;
    let mut lease = Lease {
        _lock: lock,
        path: requested.with_extension(format!("{}.part.mp4", uuid::Uuid::new_v4())),
        ready: false,
        bvid: bvid.into(),
        cid,
        page_id,
        requested: requested.into(),
        operation: uuid::Uuid::new_v4().to_string(),
    };
    if let Some(row) = db
        .query_one(statement(
            "SELECT path,bytes,sha256 FROM media_part WHERE bvid=? AND cid=?",
            vec![bvid.into(), cid.into()],
        ))
        .await?
    {
        let path: String = row.try_get("", "path")?;
        let bytes: i64 = row.try_get("", "bytes")?;
        if tokio::fs::metadata(&path)
            .await
            .is_ok_and(|m| m.is_file() && m.len() == bytes as u64 && bytes > 0)
        {
            let expected = Fingerprint {
                bytes: bytes as u64,
                sha256: row.try_get("", "sha256")?,
            };
            ensure!(
                fingerprint(Path::new(&path)).await? == expected,
                "共享媒体内容已改变，请检查后重新迁移: {path}"
            );
            lease.path = path.into();
            lease.ready = true;
        } else {
            // An unavailable registered volume must never cause another full download.
            bail!("共享媒体不可用，请恢复磁盘或在清理页检查: {path}");
        }
    } else {
        let mut copies = candidates(db, Some((bvid, cid))).await?;
        copies.sort_by(|a, b| a.priority().cmp(&b.priority()));
        for copy in copies.into_iter().filter(|c| c.completed) {
            let path = Path::new(&copy.path);
            // Fail closed: an offline/invalid existing copy needs review, not a duplicate download.
            probe(path, copy.duration).await?;
            let fp = fingerprint(path).await?;
            register(db, bvid, cid, &copy.path, &fp).await?;
            lease.path = path.into();
            lease.ready = true;
            break;
        }
    }
    if lease.ready {
        bind(db, page_id, &requested.to_string_lossy(), &lease.path.to_string_lossy()).await?;
    } else {
        ensure!(
            !requested.try_exists()?,
            "目标路径已有未确认的文件，请先在视频详情检查或删除: {}",
            requested.display()
        );
        let pending = db.query_one(statement("SELECT id FROM storage_operation WHERE kind='publish' AND state IN ('prepared','interrupted') AND json_extract(payload,'$.bvid')=? AND json_extract(payload,'$.cid')=? LIMIT 1", vec![bvid.into(),cid.into()])).await?;
        ensure!(pending.is_none(), "有尚未恢复的媒体发布操作，请重启后重试");
        let payload =
            serde_json::json!({"bvid":bvid,"cid":cid,"page_id":page_id,"staging":lease.path,"target":requested});
        db.execute(statement(
            "INSERT INTO storage_operation(id,kind,state,payload) VALUES(?,'publish','downloading',?)",
            vec![lease.operation.clone().into(), payload.to_string().into()],
        ))
        .await?;
    }
    Ok(lease)
}
impl Lease {
    pub async fn abort(&self, db: &DatabaseConnection) -> Result<()> {
        if self.ready {
            return Ok(());
        }
        remove_staging(&self.path).await?;
        db.execute(statement(
            "UPDATE storage_operation SET state='cancelled' WHERE id=? AND state='downloading'",
            vec![self.operation.clone().into()],
        ))
        .await?;
        Ok(())
    }
    pub async fn publish(&mut self, db: &DatabaseConnection, duration: u32) -> Result<()> {
        if self.ready {
            return Ok(());
        }
        probe(&self.path, duration).await?;
        let fp = fingerprint(&self.path).await?;
        let op = self.operation.clone();
        let payload = serde_json::json!({"bvid": self.bvid, "cid": self.cid, "page_id":self.page_id, "staging":self.path, "target":self.requested, "fingerprint":fp});
        db.execute(statement(
            "UPDATE storage_operation SET state='prepared',payload=? WHERE id=?",
            vec![payload.to_string().into(), op.clone().into()],
        ))
        .await?;
        // hard_link is atomic and refuses to overwrite an existing target, on Windows too.
        // It is only a publication primitive: staging and target are in the same directory.
        tokio::fs::hard_link(&self.path, &self.requested)
            .await
            .context("目标路径已存在或磁盘不支持原子发布；未覆盖旧文件")?;
        tokio::fs::remove_file(&self.path).await?;
        let txn = db.begin().await?;
        register(&txn, &self.bvid, self.cid, &self.requested.to_string_lossy(), &fp).await?;
        bind(
            &txn,
            self.page_id,
            &self.requested.to_string_lossy(),
            &self.requested.to_string_lossy(),
        )
        .await?;
        txn.execute(statement(
            "UPDATE storage_operation SET state='complete' WHERE id=?",
            vec![op.into()],
        ))
        .await?;
        txn.commit().await?;
        self.path = self.requested.clone();
        self.ready = true;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub bvid: String,
    pub cid: i64,
    pub keeper: Candidate,
    pub copies: Vec<Candidate>,
    pub reason: String,
    pub fingerprint: Option<Fingerprint>,
    pub reclaimable_bytes: u64,
    pub conflict: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub groups: Vec<Group>,
    pub unresolved_pages: usize,
}

/// Caller holds the exclusive gate. The preview is durable; execution accepts only its ID.
pub async fn preview(db: &DatabaseConnection) -> Result<Plan> {
    let mut parts: BTreeMap<(String, i64), Vec<Candidate>> = BTreeMap::new();
    let mut unresolved_pages = 0;
    for c in candidates(db, None).await? {
        if !c.bvid.starts_with("BV") || c.cid <= 0 {
            unresolved_pages += 1;
            continue;
        }
        parts.entry((c.bvid.clone(), c.cid)).or_default().push(c);
    }
    let mut groups = Vec::new();
    for ((bvid, cid), mut copies) in parts {
        if copies.iter().map(|c| &c.path).collect::<HashSet<_>>().len() < 2 {
            continue;
        }
        copies.sort_by(|a, b| a.priority().cmp(&b.priority()));
        let keeper = copies[0].clone();
        let mut group = Group {
            bvid,
            cid,
            reason: keeper.reason(),
            keeper,
            copies,
            fingerprint: None,
            reclaimable_bytes: 0,
            conflict: None,
        };
        match inspect_group(&group).await {
            Ok((fp, bytes)) => {
                group.fingerprint = Some(fp);
                group.reclaimable_bytes = bytes;
            }
            Err(e) => group.conflict = Some(format!("{e:#}")),
        }
        groups.push(group);
    }
    let plan = Plan {
        id: uuid::Uuid::new_v4().to_string(),
        groups,
        unresolved_pages,
    };
    db.execute(statement(
        "INSERT INTO storage_operation(id,kind,state,payload) VALUES(?,'dedup','preview',?)",
        vec![plan.id.clone().into(), serde_json::to_string(&plan)?.into()],
    ))
    .await?;
    Ok(plan)
}

async fn inspect_group(group: &Group) -> Result<(Fingerprint, u64)> {
    ensure!(group.copies.iter().all(|c| c.completed), "包含未完成的下载，未合并");
    let owners: HashSet<_> = group.copies.iter().map(|c| c.upper_id).filter(|id| *id > 0).collect();
    ensure!(owners.len() <= 1, "主投稿者记录不一致，请先刷新视频详情");
    probe(
        Path::new(&group.keeper.path),
        group.copies.iter().map(|c| c.duration).max().unwrap_or(0),
    )
    .await?;
    let fp = fingerprint(Path::new(&group.keeper.path)).await?;
    let mut paths = HashSet::new();
    let mut physical = HashSet::new();
    let mut bytes = 0;
    physical.insert(file_identity(Path::new(&group.keeper.path))?);
    for c in &group.copies {
        if !paths.insert(&c.path) {
            continue;
        }
        let other = fingerprint(Path::new(&c.path)).await?;
        ensure!(other == fp, "副本内容或质量不同，保留所有文件等待选择");
        if physical.insert(file_identity(Path::new(&c.path))?) {
            bytes += other.bytes;
        }
    }
    Ok((fp, bytes))
}

pub fn file_identity(path: &Path) -> std::io::Result<std::sync::Arc<same_file::Handle>> {
    same_file::Handle::from_path(path).map(std::sync::Arc::new)
}

/// Recheck every file and database mapping, commit references, then unlink exact duplicates.
/// No directory removal. A retry uses the saved plan, and verifies the keeper again.
pub async fn execute(db: &DatabaseConnection, id: &str) -> Result<Plan> {
    let row = db
        .query_one(statement(
            "SELECT payload,state FROM storage_operation WHERE id=? AND kind='dedup'",
            vec![id.into()],
        ))
        .await?
        .context("迁移预览不存在")?;
    let plan: Plan = serde_json::from_str(&row.try_get::<String>("", "payload")?)?;
    if row.try_get::<String>("", "state")? == "complete" {
        return Ok(plan);
    }
    db.execute(statement(
        "UPDATE storage_operation SET state='running', error=NULL WHERE id=?",
        vec![id.into()],
    ))
    .await?;
    for group in plan.groups.iter().filter(|g| g.conflict.is_none()) {
        if let Err(e) = execute_group(db, group).await {
            db.execute(statement(
                "UPDATE storage_operation SET state='interrupted',error=? WHERE id=?",
                vec![format!("{e:#}").into(), id.into()],
            ))
            .await?;
            return Err(e);
        }
    }
    db.execute(statement(
        "UPDATE storage_operation SET state='complete' WHERE id=?",
        vec![id.into()],
    ))
    .await?;
    Ok(plan)
}
async fn execute_group(db: &DatabaseConnection, group: &Group) -> Result<()> {
    let fp = group.fingerprint.as_ref().context("缺少预览校验值")?;
    ensure!(
        &fingerprint(Path::new(&group.keeper.path)).await? == fp,
        "保留文件发生变化，请重新预览"
    );
    let current = candidates(db, Some((&group.bvid, group.cid))).await?;
    for copy in &group.copies {
        let now = current
            .iter()
            .find(|c| c.page_id == copy.page_id)
            .context("来源已改变，请重新预览")?;
        ensure!(
            now.completed && (now.path == copy.path || now.path == group.keeper.path),
            "文件路径或下载状态已改变，请重新预览"
        );
        if copy.path != group.keeper.path && Path::new(&copy.path).try_exists()? {
            ensure!(
                &fingerprint(Path::new(&copy.path)).await? == fp,
                "副本发生变化，请重新预览"
            );
            ensure!(
                !path_has_other_identity(db, &copy.path, &group.bvid, group.cid).await?,
                "路径还被其他媒体使用"
            );
        } else if copy.path != group.keeper.path {
            ensure!(now.path == group.keeper.path, "副本已丢失，请重新预览");
        }
    }
    let txn = db.begin().await?;
    register(&txn, &group.bvid, group.cid, &group.keeper.path, fp).await?;
    for copy in &group.copies {
        bind(&txn, copy.page_id, &copy.path, &group.keeper.path).await?;
    }
    txn.commit().await?;
    for path in group.copies.iter().map(|c| &c.path).collect::<HashSet<_>>() {
        if path == &group.keeper.path {
            continue;
        }
        if Path::new(path).try_exists()? {
            ensure!(
                &fingerprint(Path::new(&group.keeper.path)).await? == fp,
                "保留文件发生变化，停止清理"
            );
            ensure!(&fingerprint(Path::new(path)).await? == fp, "副本发生变化，停止清理");
            tokio::fs::remove_file(path).await?;
        }
    }
    Ok(())
}
async fn path_has_other_identity(db: &DatabaseConnection, path: &str, bvid: &str, cid: i64) -> Result<bool> {
    Ok(db.query_one(statement("SELECT p.id FROM page p JOIN video v ON v.id=p.video_id WHERE p.path=? AND (v.bvid<>? OR p.cid<>?) LIMIT 1", vec![path.into(), bvid.into(), cid.into()])).await?.is_some())
}

/// Remove only the selected source's reference. Exact paths shared by another row or pinned are retained.
pub async fn can_remove<C: ConnectionTrait>(db: &C, path: &str, video_id: i32) -> Result<bool> {
    let other = db.query_one(statement("SELECT p.id FROM page p JOIN video v ON v.id=p.video_id WHERE p.path=? AND p.video_id<>? AND v.deleted=0 LIMIT 1", vec![path.into(), video_id.into()])).await?;
    let pin = db
        .query_one(statement(
            "SELECT bvid FROM media_part WHERE path=? AND pinned=1 LIMIT 1",
            vec![path.into()],
        ))
        .await?;
    Ok(other.is_none() && pin.is_none())
}
pub async fn forget_removed<C: ConnectionTrait>(db: &C, path: &str) -> Result<()> {
    db.execute(statement(
        "DELETE FROM media_part WHERE path=? AND pinned=0",
        vec![path.into()],
    ))
    .await?;
    Ok(())
}
pub async fn pin_video<C: ConnectionTrait>(db: &C, video_id: i32) -> Result<()> {
    // Adopt completed legacy files before their source rows disappear, so future sources can reuse them.
    for row in db.query_all(statement("SELECT v.bvid,p.cid,p.path,p.duration FROM page p JOIN video v ON v.id=p.video_id WHERE v.id=? AND p.cid>0 AND p.path IS NOT NULL AND ((p.download_status >> 3) & 7)=7",vec![video_id.into()])).await? {
        let path:String=row.try_get("","path")?;
        let bvid:String=row.try_get("","bvid")?;
        let cid:i64=row.try_get("","cid")?;
        let existing=db.query_one(statement("SELECT path FROM media_part WHERE bvid=? AND cid=?",vec![bvid.clone().into(),cid.into()])).await?;
        if existing.is_none() && bvid.starts_with("BV") {
            // Keep files even if offline/invalid; those cannot be adopted as reusable media yet.
            if probe(Path::new(&path),row.try_get("","duration")?).await.is_ok() {
                if let Ok(fp)=fingerprint(Path::new(&path)).await { register(db,&bvid,cid,&path,&fp).await?; }
            }
        }
        db.execute(statement("UPDATE media_part SET pinned=1 WHERE bvid=? AND cid=?",vec![bvid.into(),cid.into()])).await?;
    }
    Ok(())
}

/// Any directory containing a canonical file is independent of source folder lifecycle.
pub async fn protects_directory<C: ConnectionTrait>(db: &C, directory: &str) -> Result<bool> {
    let dir = Path::new(directory);
    for row in db.query_all(statement("SELECT path FROM media_part", [])).await? {
        let path: String = row.try_get("", "path")?;
        if Path::new(&path).starts_with(dir) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn managed_video<C: ConnectionTrait>(db: &C, video_id: i32) -> Result<bool> {
    Ok(db
        .query_one(statement(
            "SELECT p.id FROM page p JOIN media_view m ON m.page_id=p.id WHERE p.video_id=? LIMIT 1",
            vec![video_id.into()],
        ))
        .await?
        .is_some())
}
pub async fn remove_orphan_files<C: ConnectionTrait>(db: &C, video_id: i32) -> Result<()> {
    for row in db
        .query_all(statement(
            "SELECT path FROM page WHERE video_id=? AND path IS NOT NULL",
            vec![video_id.into()],
        ))
        .await?
    {
        let path: String = row.try_get("", "path")?;
        if !path.is_empty() && can_remove(db, &path, video_id).await? {
            match tokio::fs::remove_file(&path).await {
                Ok(()) => forget_removed(db, &path).await?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => forget_removed(db, &path).await?,
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(())
}
async fn remove_staging(path: &Path) -> Result<()> {
    for file in [
        path.to_path_buf(),
        path.with_extension("tmp_video"),
        path.with_extension("tmp_audio"),
    ] {
        match tokio::fs::remove_file(file).await {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Run before queues/downloaders start. Never remove a target that does not match its journal.
pub async fn recover(db: &DatabaseConnection) -> Result<()> {
    db.execute_unprepared("UPDATE storage_operation SET state='interrupted',error='进程中断；重新校验后可继续' WHERE kind='dedup' AND state='running'").await?;
    let rows = db.query_all(statement("SELECT id,state,payload FROM storage_operation WHERE kind='publish' AND state IN ('downloading','prepared','interrupted')", [])).await?;
    for row in rows {
        let id: String = row.try_get("", "id")?;
        let state: String = row.try_get("", "state")?;
        let payload: serde_json::Value = serde_json::from_str(&row.try_get::<String>("", "payload")?)?;
        let result: Result<()> = async {
            let staging = Path::new(payload["staging"].as_str().context("Missing staging path")?);
            if state == "downloading" {
                remove_staging(staging).await?;
                db.execute(statement("UPDATE storage_operation SET state='cancelled' WHERE id=?", vec![id.clone().into()])).await?;
                return Ok(());
            }
            let target = Path::new(payload["target"].as_str().context("Missing target path")?);
            let fp: Fingerprint = serde_json::from_value(payload["fingerprint"].clone())?;
            if !target.try_exists()? {
                ensure!(fingerprint(staging).await? == fp, "Staging file changed");
                tokio::fs::hard_link(staging, target).await?;
            }
            ensure!(fingerprint(target).await? == fp, "Published file changed");
            if staging.try_exists()? {
                ensure!(fingerprint(staging).await? == fp, "Staging file changed");
                remove_staging(staging).await?;
            }
            let bvid = payload["bvid"].as_str().context("Missing BV")?;
            let cid = payload["cid"].as_i64().context("Missing CID")?;
            let page_id = payload["page_id"].as_i64().context("Missing page")? as i32;
            let txn = db.begin().await?;
            register(&txn, bvid, cid, &target.to_string_lossy(), &fp).await?;
            let page = txn.query_one(statement("SELECT p.id FROM page p JOIN video v ON v.id=p.video_id WHERE p.id=? AND p.cid=? AND v.bvid=? AND v.deleted=0", vec![page_id.into(),cid.into(),bvid.into()])).await?;
            if page.is_some() {
                bind(&txn, page_id, &target.to_string_lossy(), &target.to_string_lossy()).await?;
            } else {
                txn.execute(statement("UPDATE media_part SET pinned=1 WHERE bvid=? AND cid=?",vec![bvid.into(),cid.into()])).await?;
            }
            txn.execute(statement("UPDATE storage_operation SET state='complete',error=NULL WHERE id=?", vec![id.clone().into()])).await?;
            txn.commit().await?;
            Ok(())
        }.await;
        if let Err(e) = result {
            tracing::error!("Shared media recovery {id}: {e:#}");
            db.execute(statement(
                "UPDATE storage_operation SET state='interrupted',error=? WHERE id=?",
                vec![format!("{e:#}").into(), id.into()],
            ))
            .await?;
        }
    }
    collect_garbage(db).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bili_sync_migration::{Migrator, MigratorTrait};
    use sea_orm::Database;
    const MEDIA: &[u8] = include_bytes!("../../tests/fixtures/shared-media.mp4");
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("shared-media-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn file(&self, name: &str) -> PathBuf {
            let file = self.0.join(name);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, MEDIA).unwrap();
            file
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    async fn database() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        db
    }
    async fn add(db: &DatabaseConnection, id: i32, uid: Option<i64>, path: &Path, cid: i64) {
        let root = path.parent().unwrap().to_string_lossy();
        if let Some(uid) = uid {
            db.execute(statement("INSERT INTO submission(id,upper_id,upper_name,path,created_at,latest_row_at,enabled,scan_deleted_videos) VALUES(?,?,?,?,'2026-01-01 00:00:00','2026-01-01 00:00:00',1,0)", vec![id.into(),uid.into(),"Author".into(),root.to_string().into()])).await.unwrap();
        } else {
            db.execute(statement("INSERT INTO favorite(id,f_id,name,path,created_at,latest_row_at,enabled,scan_deleted_videos) VALUES(?,?,?,?,'2026-01-01 00:00:00','2026-01-01 00:00:00',1,0)", vec![id.into(),id.to_string().into(),"Favorite".into(),root.to_string().into()])).await.unwrap();
        }
        db.execute(statement("INSERT INTO video(id,submission_id,favorite_id,upper_id,upper_name,upper_face,name,path,category,bvid,intro,cover,ctime,pubtime,favtime,download_status,valid,created_at,staff_info) VALUES(?,?,?,11,'Primary','','Title',?,2,'BV1sharedtest','','','2026-01-01 00:00:00','2026-01-01 00:00:00','2026-01-01 00:00:00',0,1,'2026-01-01 00:00:00',?)",
            vec![id.into(), uid.map(|_|id).into(), uid.is_none().then_some(id).into(),root.to_string().into(),r#"[{"mid":11,"name":"Primary"},{"mid":22,"name":"Second"},{"mid":33,"name":"Third"}]"#.into()])).await.unwrap();
        db.execute(statement("INSERT INTO page(id,video_id,cid,pid,name,duration,path,download_status,created_at) VALUES(?,?,?,1,'Page',1,?,56,'2026-01-01 00:00:00')", vec![id.into(),id.into(),cid.into(),path.to_string_lossy().to_string().into()])).await.unwrap();
    }
    async fn page_path(db: &DatabaseConnection, id: i32) -> String {
        db.query_one(statement("SELECT path FROM page WHERE id=?", vec![id.into()]))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "path")
            .unwrap()
    }
    fn copy(id: i32, uid: Option<i64>, kind: &str) -> Candidate {
        Candidate {
            page_id: id,
            video_id: id,
            bvid: "BV1sharedtest".into(),
            cid: 1,
            path: format!("/library/{id}.mp4"),
            name: "Video".into(),
            upper_id: 11,
            staff: vec![22, 33],
            source_uid: uid,
            source_kind: kind.into(),
            source_created: format!("2026-01-{id:02}"),
            source_id: id,
            source_root: "/library".into(),
            completed: true,
            duration: 1,
        }
    }
    #[test]
    fn keeper_priority_primary_then_staff_order_then_first_collection_or_favorite() {
        let mut rows = [
            copy(1, None, "favorite"),
            copy(2, None, "collection"),
            copy(3, Some(33), "submission"),
            copy(4, Some(22), "submission"),
            copy(5, Some(11), "submission"),
        ];
        rows.sort_by(|a, b| a.priority().cmp(&b.priority()));
        assert_eq!(rows.map(|c| c.page_id), [5, 4, 3, 1, 2]);
        let mut borrowed = copy(1, Some(11), "submission");
        borrowed.path = "/other/keeper.mp4".into();
        assert!(borrowed.priority() > copy(2, None, "favorite").priority());
    }
    #[tokio::test]
    async fn migration_preserves_primary_path_references_and_multipart_identity() {
        let db = database().await;
        let dir = Temp::new();
        let favorite = dir.file("favorite/video.mp4");
        let third = dir.file("third/video.mp4");
        let primary = dir.file("primary/video.mp4");
        let second_part = dir.file("favorite/part2.mp4");
        add(&db, 1, None, &favorite, 10).await;
        add(&db, 2, Some(33), &third, 10).await;
        add(&db, 3, Some(11), &primary, 10).await;
        db.execute(statement("INSERT INTO page(id,video_id,cid,pid,name,duration,path,download_status,created_at) VALUES(4,1,20,2,'P2',1,?,56,'2026-01-01 00:00:00')",vec![second_part.to_string_lossy().to_string().into()])).await.unwrap();
        let plan = preview(&db).await.unwrap();
        assert_eq!(plan.groups.len(), 1);
        let group = &plan.groups[0];
        assert_eq!(group.keeper.page_id, 3);
        assert_eq!(group.conflict, None);
        assert_eq!(group.reclaimable_bytes, MEDIA.len() as u64 * 2);
        execute(&db, &plan.id).await.unwrap();
        for id in [1, 2, 3] {
            assert_eq!(page_path(&db, id).await, primary.to_string_lossy());
        }
        assert!(primary.exists() && second_part.exists());
        assert!(!favorite.exists() && !third.exists());
        execute(&db, &plan.id).await.unwrap(); // idempotent, no additional deletions
        assert!(preview(&db).await.unwrap().groups.is_empty());
        assert!(!can_remove(&db, &primary.to_string_lossy(), 3).await.unwrap());
        remove_orphan_files(&db, 3).await.unwrap(); // deleting keeper's source must retain shared media
        db.execute_unprepared("DELETE FROM page WHERE video_id=3; DELETE FROM video WHERE id=3;")
            .await
            .unwrap();
        assert!(primary.exists());
        remove_orphan_files(&db, 2).await.unwrap();
        db.execute_unprepared("DELETE FROM page WHERE video_id=2; DELETE FROM video WHERE id=2;")
            .await
            .unwrap();
        remove_orphan_files(&db, 1).await.unwrap();
        assert!(!primary.exists());
    }
    #[tokio::test]
    async fn changed_file_blocks_execution_without_rebinding_or_deleting() {
        let db = database().await;
        let dir = Temp::new();
        let a = dir.file("a/video.mp4");
        let b = dir.file("b/video.mp4");
        add(&db, 1, Some(11), &a, 10).await;
        add(&db, 2, None, &b, 10).await;
        let plan = preview(&db).await.unwrap();
        std::fs::write(&b, b"changed").unwrap();
        assert!(execute(&db, &plan.id).await.is_err());
        assert!(a.exists() && b.exists());
        assert_eq!(page_path(&db, 2).await, b.to_string_lossy());
        assert!(preview(&db).await.unwrap().groups[0].conflict.is_some());
    }
    #[tokio::test]
    async fn interrupted_migration_resumes_after_references_committed() {
        let db = database().await;
        let dir = Temp::new();
        let a = dir.file("a/video.mp4");
        let b = dir.file("b/video.mp4");
        add(&db, 1, Some(11), &a, 10).await;
        add(&db, 2, None, &b, 10).await;
        let plan = preview(&db).await.unwrap();
        // Simulate crash after DB reference commit and removal, before journal completion.
        bind(&db, 2, &b.to_string_lossy(), &a.to_string_lossy()).await.unwrap();
        std::fs::remove_file(&b).unwrap();
        execute(&db, &plan.id).await.unwrap();
        assert_eq!(page_path(&db, 2).await, a.to_string_lossy());
        assert!(a.exists());
    }
    #[tokio::test]
    async fn pin_keeps_last_file_and_offline_registry_blocks_new_download() {
        let db = database().await;
        let dir = Temp::new();
        let a = dir.file("a/video.mp4");
        add(&db, 1, Some(11), &a, 10).await;
        let fp = fingerprint(&a).await.unwrap();
        register(&db, "BV1sharedtest", 10, &a.to_string_lossy(), &fp)
            .await
            .unwrap();
        pin_video(&db, 1).await.unwrap();
        remove_orphan_files(&db, 1).await.unwrap();
        assert!(a.exists());
        std::fs::remove_file(&a).unwrap();
        assert!(acquire(&db, "BV1sharedtest", 10, 1, &a).await.is_err());
    }
    #[tokio::test]
    async fn concurrent_sources_download_once_and_reuse_the_published_path() {
        let db = database().await;
        let dir = Temp::new();
        let a = dir.file("a/video.mp4");
        let b = dir.file("b/video.mp4");
        add(&db, 1, Some(11), &a, 10).await;
        add(&db, 2, None, &b, 10).await;
        db.execute_unprepared("UPDATE page SET download_status=0,path=NULL")
            .await
            .unwrap();
        std::fs::remove_file(&a).unwrap();
        std::fs::remove_file(&b).unwrap();
        let mut first = acquire(&db, "BV1sharedtest", 10, 1, &a).await.unwrap();
        assert!(!first.ready);
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(20),
            acquire(&db, "BV1sharedtest", 10, 2, &b)
        )
        .await
        .is_err());
        std::fs::write(&first.path, MEDIA).unwrap();
        first.publish(&db, 1).await.unwrap();
        drop(first);
        let second = acquire(&db, "BV1sharedtest", 10, 2, &b).await.unwrap();
        assert!(second.ready);
        assert_eq!(second.path, a);
        assert!(!b.exists());
        assert_eq!(page_path(&db, 2).await, a.to_string_lossy());
    }
    #[tokio::test]
    async fn recovery_finishes_publication_after_atomic_file_publish() {
        let db = database().await;
        let dir = Temp::new();
        let target = dir.file("a/video.mp4");
        add(&db, 1, Some(11), &target, 10).await;
        let staging = dir.file("a/staging.mp4");
        let fp = fingerprint(&target).await.unwrap();
        let payload = serde_json::json!({"bvid":"BV1sharedtest","cid":10,"page_id":1,"target":target,"staging":staging,"fingerprint":fp});
        db.execute(statement(
            "INSERT INTO storage_operation(id,kind,state,payload) VALUES('recover','publish','prepared',?)",
            vec![payload.to_string().into()],
        ))
        .await
        .unwrap();
        recover(&db).await.unwrap();
        recover(&db).await.unwrap();
        assert!(target.exists());
        assert!(!staging.exists());
        assert!(managed_video(&db, 1).await.unwrap());
    }
    #[tokio::test]
    async fn actual_source_delete_api_keeps_other_sources_and_then_reclaims_media() {
        let db = std::sync::Arc::new(database().await);
        let dir = Temp::new();
        let a = dir.file("primary/video.mp4");
        let b = dir.file("favorite/video.mp4");
        add(&db, 1, Some(11), &a, 10).await;
        add(&db, 2, None, &b, 10).await;
        let plan = preview(&db).await.unwrap();
        execute(&db, &plan.id).await.unwrap();
        crate::api::handler::delete_video_source_internal(db.clone(), "submission".into(), 1, true)
            .await
            .unwrap();
        assert!(a.exists());
        assert_eq!(page_path(&db, 2).await, a.to_string_lossy());
        crate::api::handler::delete_video_source_internal(db.clone(), "favorite".into(), 2, true)
            .await
            .unwrap();
        assert!(!a.exists());
    }
    #[tokio::test]
    async fn actual_video_delete_and_restore_reuses_remaining_shared_media() {
        let db = std::sync::Arc::new(database().await);
        let dir = Temp::new();
        let a = dir.file("primary/video.mp4");
        let b = dir.file("favorite/video.mp4");
        add(&db, 1, Some(11), &a, 10).await;
        add(&db, 2, None, &b, 10).await;
        let plan = preview(&db).await.unwrap();
        execute(&db, &plan.id).await.unwrap();
        crate::api::handler::delete_video_internal(db.clone(), 1).await.unwrap();
        assert!(a.exists());
        crate::api::handler::restore_video_internal(db.clone(), 1)
            .await
            .unwrap();
        // Restoring deletes old pages, but the canonical registry remains reusable.
        assert!(db
            .query_one(statement(
                "SELECT bvid FROM media_part WHERE bvid='BV1sharedtest' AND cid=10",
                []
            ))
            .await
            .unwrap()
            .is_some());
        assert!(a.exists());
    }
    #[tokio::test]
    async fn gc_rechecks_references_and_does_not_delete_modified_files() {
        let db = database().await;
        let dir = Temp::new();
        let a = dir.file("primary/video.mp4");
        add(&db, 1, Some(11), &a, 10).await;
        schedule_gc(&db, &[a.to_string_lossy().to_string()]).await.unwrap();
        collect_garbage(&db).await.unwrap();
        assert!(a.exists()); // a live reference cancels GC
        db.execute_unprepared("UPDATE video SET deleted=1").await.unwrap();
        schedule_gc(&db, &[a.to_string_lossy().to_string()]).await.unwrap();
        std::fs::write(&a, b"new content").unwrap();
        collect_garbage(&db).await.unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), b"new content");
    }

    #[tokio::test]
    async fn hardlinks_are_not_reported_as_reclaimable_duplicate_bytes() {
        let db = database().await;
        let dir = Temp::new();
        let a = dir.file("a/video.mp4");
        let b = dir.0.join("b.mp4");
        std::fs::hard_link(&a, &b).unwrap();
        add(&db, 1, Some(11), &a, 10).await;
        add(&db, 2, None, &b, 10).await;
        let plan = preview(&db).await.unwrap();
        assert_eq!(plan.groups[0].reclaimable_bytes, 0);
        assert_eq!(plan.groups[0].conflict, None);
        execute(&db, &plan.id).await.unwrap();
        assert!(a.exists());
        assert!(!b.exists());
    }
}

pub async fn video_paths<C: ConnectionTrait>(db: &C, video_id: i32) -> Result<Vec<String>> {
    db.query_all(statement(
        "SELECT path FROM page WHERE video_id=? AND path IS NOT NULL AND path<>''",
        vec![video_id.into()],
    ))
    .await?
    .into_iter()
    .map(|r| Ok(r.try_get("", "path")?))
    .collect()
}
pub async fn schedule_gc<C: ConnectionTrait>(db: &C, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    // Only capture identities of readable files. An offline path is never deleted later by guesswork.
    let mut files = Vec::new();
    for path in paths.iter().collect::<HashSet<_>>() {
        if let Ok(fp) = fingerprint(Path::new(path)).await {
            files.push((path.clone(), fp));
        }
    }
    db.execute(statement(
        "INSERT INTO storage_operation(id,kind,state,payload) VALUES(?,'gc','prepared',?)",
        vec![
            uuid::Uuid::new_v4().to_string().into(),
            serde_json::to_string(&files)?.into(),
        ],
    ))
    .await?;
    Ok(())
}
pub async fn collect_garbage(db: &DatabaseConnection) -> Result<()> {
    for row in db
        .query_all(statement(
            "SELECT id,payload FROM storage_operation WHERE kind='gc' AND state='prepared'",
            [],
        ))
        .await?
    {
        let id: String = row.try_get("", "id")?;
        let files: Vec<(String, Fingerprint)> = serde_json::from_str(&row.try_get::<String>("", "payload")?)?;
        let result: Result<()> = async {
            for (path, fp) in &files {
                if !can_remove(db, path, -1).await? {
                    continue;
                }
                if Path::new(path).try_exists()? {
                    ensure!(
                        &fingerprint(Path::new(path)).await? == fp,
                        "待清理文件发生变化，保留: {path}"
                    );
                    tokio::fs::remove_file(path).await?;
                }
                forget_removed(db, path).await?;
            }
            Ok(())
        }
        .await;
        match result {
            Ok(()) => {
                db.execute(statement(
                    "UPDATE storage_operation SET state='complete',error=NULL WHERE id=?",
                    vec![id.into()],
                ))
                .await?;
            }
            Err(e) => {
                tracing::warn!("媒体清理等待重试 {id}: {e:#}");
                db.execute(statement(
                    "UPDATE storage_operation SET error=? WHERE id=?",
                    vec![format!("{e:#}").into(), id.into()],
                ))
                .await?;
            }
        }
    }
    Ok(())
}
