use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const MIN_DURATION_COMMENT: &str = "允许的最小视频时长，单位秒";
const MAX_DURATION_COMMENT: &str = "允许的最大视频时长，单位秒";
const MIN_PAGE_DURATION_COMMENT: &str = "允许的最小分P时长，单位秒";
const MAX_PAGE_DURATION_COMMENT: &str = "允许的最大分P时长，单位秒";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in ["collection", "favorite", "submission", "watch_later", "video_source"] {
            add_duration_filters(manager, table).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in ["collection", "favorite", "submission", "watch_later", "video_source"] {
            drop_duration_filters(manager, table).await?;
        }

        Ok(())
    }
}

async fn add_duration_filters(manager: &SchemaManager<'_>, table: &str) -> Result<(), DbErr> {
    add_integer_column(manager, table, "min_duration_seconds", MIN_DURATION_COMMENT).await?;
    add_integer_column(manager, table, "max_duration_seconds", MAX_DURATION_COMMENT).await?;
    add_integer_column(manager, table, "min_page_duration_seconds", MIN_PAGE_DURATION_COMMENT).await?;
    add_integer_column(manager, table, "max_page_duration_seconds", MAX_PAGE_DURATION_COMMENT).await?;

    Ok(())
}

async fn drop_duration_filters(manager: &SchemaManager<'_>, table: &str) -> Result<(), DbErr> {
    drop_column_if_exists(manager, table, "min_duration_seconds").await?;
    drop_column_if_exists(manager, table, "max_duration_seconds").await?;
    drop_column_if_exists(manager, table, "min_page_duration_seconds").await?;
    drop_column_if_exists(manager, table, "max_page_duration_seconds").await?;

    Ok(())
}

async fn add_integer_column(
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
                .add_column(ColumnDef::new(Alias::new(column)).integer().null().comment(comment))
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
