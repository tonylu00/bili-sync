use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("CREATE TABLE IF NOT EXISTS following_cache (account_uid BIGINT PRIMARY KEY NOT NULL, fetched_at TEXT NOT NULL, payload TEXT NOT NULL)").await?;
        db.execute_unprepared("CREATE TABLE IF NOT EXISTS uploader_filter (upper_id BIGINT PRIMARY KEY NOT NULL, settings TEXT NOT NULL, updated_at TEXT NOT NULL)").await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE uploader_filter").await?;
        db.execute_unprepared("DROP TABLE following_cache").await?;
        Ok(())
    }
}
