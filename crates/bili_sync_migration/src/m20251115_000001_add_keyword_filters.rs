use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Collection::Table)
                    .add_column(
                        ColumnDef::new(Collection::IncludeKeywords)
                            .text()
                            .null()
                            .comment("允许的视频标题需包含的关键词(JSON数组)"),
                    )
                    .add_column(
                        ColumnDef::new(Collection::ExcludeKeywords)
                            .text()
                            .null()
                            .comment("禁止的视频标题包含的关键词(JSON数组)"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Favorite::Table)
                    .add_column(
                        ColumnDef::new(Favorite::IncludeKeywords)
                            .text()
                            .null()
                            .comment("允许的视频标题需包含的关键词(JSON数组)"),
                    )
                    .add_column(
                        ColumnDef::new(Favorite::ExcludeKeywords)
                            .text()
                            .null()
                            .comment("禁止的视频标题包含的关键词(JSON数组)"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Submission::Table)
                    .add_column(
                        ColumnDef::new(Submission::IncludeKeywords)
                            .text()
                            .null()
                            .comment("允许的视频标题需包含的关键词(JSON数组)"),
                    )
                    .add_column(
                        ColumnDef::new(Submission::ExcludeKeywords)
                            .text()
                            .null()
                            .comment("禁止的视频标题包含的关键词(JSON数组)"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WatchLater::Table)
                    .add_column(
                        ColumnDef::new(WatchLater::IncludeKeywords)
                            .text()
                            .null()
                            .comment("允许的视频标题需包含的关键词(JSON数组)"),
                    )
                    .add_column(
                        ColumnDef::new(WatchLater::ExcludeKeywords)
                            .text()
                            .null()
                            .comment("禁止的视频标题包含的关键词(JSON数组)"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(VideoSource::Table)
                    .add_column(
                        ColumnDef::new(VideoSource::IncludeKeywords)
                            .text()
                            .null()
                            .comment("允许的视频标题需包含的关键词(JSON数组)"),
                    )
                    .add_column(
                        ColumnDef::new(VideoSource::ExcludeKeywords)
                            .text()
                            .null()
                            .comment("禁止的视频标题包含的关键词(JSON数组)"),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Collection::Table)
                    .drop_column(Collection::IncludeKeywords)
                    .drop_column(Collection::ExcludeKeywords)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Favorite::Table)
                    .drop_column(Favorite::IncludeKeywords)
                    .drop_column(Favorite::ExcludeKeywords)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Submission::Table)
                    .drop_column(Submission::IncludeKeywords)
                    .drop_column(Submission::ExcludeKeywords)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WatchLater::Table)
                    .drop_column(WatchLater::IncludeKeywords)
                    .drop_column(WatchLater::ExcludeKeywords)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(VideoSource::Table)
                    .drop_column(VideoSource::IncludeKeywords)
                    .drop_column(VideoSource::ExcludeKeywords)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Collection {
    Table,
    IncludeKeywords,
    ExcludeKeywords,
}

#[derive(DeriveIden)]
enum Favorite {
    Table,
    IncludeKeywords,
    ExcludeKeywords,
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    IncludeKeywords,
    ExcludeKeywords,
}

#[derive(DeriveIden)]
enum WatchLater {
    Table,
    IncludeKeywords,
    ExcludeKeywords,
}

#[derive(DeriveIden)]
enum VideoSource {
    Table,
    IncludeKeywords,
    ExcludeKeywords,
}
