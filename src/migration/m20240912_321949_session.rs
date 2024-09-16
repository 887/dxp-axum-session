use sea_orm::DbBackend;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        let mut result = manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Sessions::Id)
                            .string_len(128)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Sessions::Expires).date_time().null())
                    .col(ColumnDef::new(Sessions::Session).text().not_null())
                    .to_owned(),
            )
            .await?;

        if backend != DbBackend::Sqlite {
            let foreign_key = sea_query::Index::create()
                .name("sessions_expires_idx")
                .col(Sessions::Expires)
                .if_not_exists()
                .to_owned();

            result = manager.create_index(foreign_key).await?;
        }

        Ok(result)
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
#[iden = "sessions"]
enum Sessions {
    Table,
    Id,
    Expires,
    Session,
}
