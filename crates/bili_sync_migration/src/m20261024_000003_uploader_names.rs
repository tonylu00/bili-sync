use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// A submission's initial video rows have no uploader metadata yet. Only use the
// direct submission relation as a fallback, never the collection owner or staff.
const VIDEO_UID: &str =
    "COALESCE(NULLIF(NEW.upper_id, 0), (SELECT upper_id FROM submission WHERE id = NEW.submission_id))";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared("CREATE TABLE uploader_name_override (upper_id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
            .await?;
        // Existing observed uploader names remain available even without a submission subscription.
        db.execute_unprepared(
            "INSERT INTO source_name_history(source_type, source_id, name)
             SELECT DISTINCT 'uploader', upper_id, upper_name FROM (
               SELECT COALESCE(NULLIF(v.upper_id, 0), s.upper_id) AS upper_id, v.upper_name
               FROM video v LEFT JOIN submission s ON s.id = v.submission_id
               WHERE COALESCE(v.source_type, 0) <> 1
               UNION SELECT upper_id, upper_name FROM submission
               UNION SELECT s.upper_id, h.name FROM source_name_history h
                 JOIN submission s ON h.source_type = 'submission' AND h.source_id = s.id
             ) WHERE upper_id > 0 AND trim(upper_name) <> ''",
        )
        .await?;
        for (table, uid, eligible, columns) in [
            (
                "video",
                VIDEO_UID,
                "COALESCE(NEW.source_type, 0) <> 1",
                "upper_name, upper_id, submission_id, source_type",
            ),
            ("submission", "NEW.upper_id", "1", "upper_name, upper_id"),
        ] {
            for (event, action) in [
                ("insert", "INSERT".to_string()),
                ("update", format!("UPDATE OF {columns}")),
            ] {
                db.execute_unprepared(&format!(
                    "CREATE TRIGGER {table}_uploader_name_{event} AFTER {action} ON {table}
                     WHEN ({uid}) > 0 AND ({eligible}) BEGIN
                       INSERT INTO source_name_history(source_type, source_id, name)
                         SELECT 'uploader', ({uid}), NEW.upper_name
                         WHERE trim(NEW.upper_name) <> '' AND NOT EXISTS (
                           SELECT 1 FROM source_name_history
                           WHERE source_type = 'uploader' AND source_id = ({uid}) AND name = NEW.upper_name
                         );
                       UPDATE {table} SET upper_name = (SELECT name FROM uploader_name_override WHERE upper_id = ({uid}))
                         WHERE id = NEW.id AND EXISTS (
                           SELECT 1 FROM uploader_name_override WHERE upper_id = ({uid}) AND name <> NEW.upper_name
                         );
                     END"
                )).await?;
            }
        }
        for event in ["insert", "update"] {
            db.execute_unprepared(&format!(
                "CREATE TRIGGER uploader_override_{event} AFTER {event} ON uploader_name_override BEGIN
                   INSERT INTO source_name_history(source_type, source_id, name)
                     SELECT 'uploader', NEW.upper_id, NEW.name WHERE NOT EXISTS (
                       SELECT 1 FROM source_name_history
                       WHERE source_type = 'uploader' AND source_id = NEW.upper_id AND name = NEW.name
                     );
                   UPDATE video SET upper_name = NEW.name
                     WHERE COALESCE(source_type, 0) <> 1 AND (
                       upper_id = NEW.upper_id OR (upper_id = 0 AND submission_id IN (
                         SELECT id FROM submission WHERE upper_id = NEW.upper_id
                       ))
                     );
                   UPDATE submission SET upper_name = NEW.name WHERE upper_id = NEW.upper_id;
                 END"
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for event in ["insert", "update"] {
            for table in ["video", "submission"] {
                db.execute_unprepared(&format!("DROP TRIGGER {table}_uploader_name_{event}"))
                    .await?;
            }
            db.execute_unprepared(&format!("DROP TRIGGER uploader_override_{event}"))
                .await?;
        }
        db.execute_unprepared("DROP TABLE uploader_name_override").await?;
        db.execute_unprepared("DELETE FROM source_name_history WHERE source_type = 'uploader'")
            .await?;
        Ok(())
    }
}
