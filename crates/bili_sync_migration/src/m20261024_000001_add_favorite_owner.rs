use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_column("favorite", "owner_mid").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new("favorite"))
                        .add_column(ColumnDef::new(Alias::new("owner_mid")).big_integer().null())
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("favorite"))
                    .drop_column(Alias::new("owner_mid"))
                    .to_owned(),
            )
            .await
    }
}
