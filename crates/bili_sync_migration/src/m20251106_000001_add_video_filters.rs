use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 为 collection 表添加时长过滤字段
        manager
            .alter_table(
                Table::alter()
                    .table(Collection::Table)
                    .add_column(
                        ColumnDef::new(Collection::MinDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(Collection::MaxDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(Collection::MinPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小分P时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(Collection::MaxPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大分P时长，单位秒"),
                    )
                    .to_owned(),
            )
            .await?;

        // 为 favorite 表添加时长过滤字段
        manager
            .alter_table(
                Table::alter()
                    .table(Favorite::Table)
                    .add_column(
                        ColumnDef::new(Favorite::MinDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(Favorite::MaxDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(Favorite::MinPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小分P时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(Favorite::MaxPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大分P时长，单位秒"),
                    )
                    .to_owned(),
            )
            .await?;

        // 为 submission 表添加时长过滤字段
        manager
            .alter_table(
                Table::alter()
                    .table(Submission::Table)
                    .add_column(
                        ColumnDef::new(Submission::MinDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(Submission::MaxDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(Submission::MinPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小分P时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(Submission::MaxPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大分P时长，单位秒"),
                    )
                    .to_owned(),
            )
            .await?;

        // 为 watch_later 表添加时长过滤字段
        manager
            .alter_table(
                Table::alter()
                    .table(WatchLater::Table)
                    .add_column(
                        ColumnDef::new(WatchLater::MinDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(WatchLater::MaxDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(WatchLater::MinPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小分P时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(WatchLater::MaxPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大分P时长，单位秒"),
                    )
                    .to_owned(),
            )
            .await?;

        // 为 video_source 表添加时长过滤字段
        manager
            .alter_table(
                Table::alter()
                    .table(VideoSource::Table)
                    .add_column(
                        ColumnDef::new(VideoSource::MinDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(VideoSource::MaxDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大视频时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(VideoSource::MinPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最小分P时长，单位秒"),
                    )
                    .add_column(
                        ColumnDef::new(VideoSource::MaxPageDurationSeconds)
                            .integer()
                            .null()
                            .comment("允许的最大分P时长，单位秒"),
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
                    .drop_column(Collection::MinDurationSeconds)
                    .drop_column(Collection::MaxDurationSeconds)
                    .drop_column(Collection::MinPageDurationSeconds)
                    .drop_column(Collection::MaxPageDurationSeconds)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Favorite::Table)
                    .drop_column(Favorite::MinDurationSeconds)
                    .drop_column(Favorite::MaxDurationSeconds)
                    .drop_column(Favorite::MinPageDurationSeconds)
                    .drop_column(Favorite::MaxPageDurationSeconds)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Submission::Table)
                    .drop_column(Submission::MinDurationSeconds)
                    .drop_column(Submission::MaxDurationSeconds)
                    .drop_column(Submission::MinPageDurationSeconds)
                    .drop_column(Submission::MaxPageDurationSeconds)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(WatchLater::Table)
                    .drop_column(WatchLater::MinDurationSeconds)
                    .drop_column(WatchLater::MaxDurationSeconds)
                    .drop_column(WatchLater::MinPageDurationSeconds)
                    .drop_column(WatchLater::MaxPageDurationSeconds)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(VideoSource::Table)
                    .drop_column(VideoSource::MinDurationSeconds)
                    .drop_column(VideoSource::MaxDurationSeconds)
                    .drop_column(VideoSource::MinPageDurationSeconds)
                    .drop_column(VideoSource::MaxPageDurationSeconds)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Collection {
    Table,
    MinDurationSeconds,
    MaxDurationSeconds,
    MinPageDurationSeconds,
    MaxPageDurationSeconds,
}

#[derive(DeriveIden)]
enum Favorite {
    Table,
    MinDurationSeconds,
    MaxDurationSeconds,
    MinPageDurationSeconds,
    MaxPageDurationSeconds,
}

#[derive(DeriveIden)]
enum Submission {
    Table,
    MinDurationSeconds,
    MaxDurationSeconds,
    MinPageDurationSeconds,
    MaxPageDurationSeconds,
}

#[derive(DeriveIden)]
enum WatchLater {
    Table,
    MinDurationSeconds,
    MaxDurationSeconds,
    MinPageDurationSeconds,
    MaxPageDurationSeconds,
}

#[derive(DeriveIden)]
enum VideoSource {
    Table,
    MinDurationSeconds,
    MaxDurationSeconds,
    MinPageDurationSeconds,
    MaxPageDurationSeconds,
}
