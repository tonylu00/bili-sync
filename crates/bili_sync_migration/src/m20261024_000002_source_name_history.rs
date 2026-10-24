use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// SQLite triggers cover every write path, including scans, repairs and UPSERTs.
// Retain every observed name, exceeding the minimum of five previous names.
const SOURCES: [(&str, &str); 3] = [
    ("favorite", "name"),
    ("collection", "name"),
    ("submission", "upper_name"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE TABLE source_name_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_type TEXT NOT NULL,
                source_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            )",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX idx_source_name_history_source ON source_name_history(source_type, source_id, id)",
        )
        .await?;
        for (table, name) in SOURCES {
            db.execute_unprepared(&format!(
                "INSERT INTO source_name_history (source_type, source_id, name)
                 SELECT '{table}', id, {name} FROM {table} WHERE trim({name}) <> ''"
            ))
            .await?;
            db.execute_unprepared(&format!(
                "CREATE TRIGGER {table}_name_history_insert AFTER INSERT ON {table}
                 WHEN trim(NEW.{name}) <> '' BEGIN
                   INSERT INTO source_name_history(source_type, source_id, name)
                   VALUES ('{table}', NEW.id, NEW.{name});
                 END"
            ))
            .await?;
            db.execute_unprepared(&format!(
                "CREATE TRIGGER {table}_name_history_update AFTER UPDATE OF {name} ON {table}
                 WHEN NEW.{name} <> OLD.{name} AND trim(NEW.{name}) <> '' BEGIN
                   INSERT INTO source_name_history(source_type, source_id, name)
                   VALUES ('{table}', NEW.id, NEW.{name});
                 END"
            ))
            .await?;
            // Explicit source deletion removes its history and avoids ID-reuse collisions.
            db.execute_unprepared(&format!(
                "CREATE TRIGGER {table}_name_history_delete AFTER DELETE ON {table} BEGIN
                   DELETE FROM source_name_history WHERE source_type = '{table}' AND source_id = OLD.id;
                 END"
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (table, _) in SOURCES {
            for event in ["insert", "update", "delete"] {
                db.execute_unprepared(&format!("DROP TRIGGER {table}_name_history_{event}"))
                    .await?;
            }
        }
        db.execute_unprepared("DROP TABLE source_name_history").await?;
        Ok(())
    }
}
