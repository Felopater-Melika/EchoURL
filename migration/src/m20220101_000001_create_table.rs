use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Url::Table)
                    .col(pk_auto(Url::Id))
                    .col(string(Url::Original).not_null())
                    .col(string(Url::Shortened).not_null())
                    .col(integer(Url::Clicks).not_null())
                    .col(
                        timestamp_with_time_zone(Url::CreatedAt)
                            .default(Expr::current_timestamp()) // Default to NOW()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_shortened")
                    .table(Url::Table)
                    .col(Url::Shortened)
                    .unique()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Url::Table).to_owned())
            .await?;

        manager
            .drop_index(Index::drop().name("idx_shortened").to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Url {
    Table,
    Id,
    Original,
    Shortened,
    Clicks,
    CreatedAt,
}
