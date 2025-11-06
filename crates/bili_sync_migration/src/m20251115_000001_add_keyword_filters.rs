use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const INCLUDE_COMMENT: &str = "允许的视频标题需包含的关键词(JSON数组)";
const EXCLUDE_COMMENT: &str = "禁止的视频标题包含的关键词(JSON数组)";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in ["collection", "favorite", "submission", "watch_later", "video_source"] {
            add_keyword_filters(manager, table).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in ["collection", "favorite", "submission", "watch_later", "video_source"] {
            drop_keyword_filters(manager, table).await?;
        }

        Ok(())
    }
}

async fn add_keyword_filters(manager: &SchemaManager<'_>, table: &str) -> Result<(), DbErr> {
    add_text_column(manager, table, "include_keywords", INCLUDE_COMMENT).await?;
    add_text_column(manager, table, "exclude_keywords", EXCLUDE_COMMENT).await?;

    Ok(())
}

async fn drop_keyword_filters(manager: &SchemaManager<'_>, table: &str) -> Result<(), DbErr> {
    drop_column_if_exists(manager, table, "include_keywords").await?;
    drop_column_if_exists(manager, table, "exclude_keywords").await?;

    Ok(())
}

async fn add_text_column(
    manager: &SchemaManager<'_>,
    table: &str,
    column: &str,
    comment: &'static str,
) -> Result<(), DbErr> {
    if manager.has_column(table, column).await? {
        return Ok(());
    }

    manager
        .alter_table(
            Table::alter()
                .table(Alias::new(table))
                .add_column(ColumnDef::new(Alias::new(column)).text().null().comment(comment))
                .to_owned(),
        )
        .await?;

    Ok(())
}

async fn drop_column_if_exists(manager: &SchemaManager<'_>, table: &str, column: &str) -> Result<(), DbErr> {
    if !manager.has_column(table, column).await? {
        return Ok(());
    }

    manager
        .alter_table(
            Table::alter()
                .table(Alias::new(table))
                .drop_column(Alias::new(column))
                .to_owned(),
        )
        .await?;

    Ok(())
}
