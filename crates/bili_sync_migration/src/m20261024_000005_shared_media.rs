use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Source video/page IDs stay stable. The registry owns media independently of a source.
        db.execute_unprepared("CREATE TABLE media_part (bvid TEXT NOT NULL, cid BIGINT NOT NULL CHECK(cid > 0), path TEXT NOT NULL, bytes BIGINT NOT NULL, sha256 TEXT NOT NULL, pinned BOOLEAN NOT NULL DEFAULT 0, PRIMARY KEY(bvid, cid))").await?;
        db.execute_unprepared("CREATE TABLE media_view (page_id INTEGER PRIMARY KEY REFERENCES page(id) ON DELETE CASCADE, requested_path TEXT NOT NULL)").await?;
        db.execute_unprepared("CREATE TABLE storage_operation (id TEXT PRIMARY KEY NOT NULL, kind TEXT NOT NULL, state TEXT NOT NULL, payload TEXT NOT NULL, error TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)").await?;
        db.execute_unprepared("CREATE INDEX idx_page_path ON page(path)")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP INDEX idx_page_path").await?;
        db.execute_unprepared("DROP TABLE media_view").await?;
        db.execute_unprepared("DROP TABLE media_part").await?;
        db.execute_unprepared("DROP TABLE storage_operation").await?;
        Ok(())
    }
}
