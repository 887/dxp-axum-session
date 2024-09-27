use async_trait::async_trait;
use axum_session::{DatabaseError, DatabasePool};
use chrono::{TimeZone, Utc};
use sea_orm::{
    sea_query::{self, ColumnDef, Index, Table},
    ActiveValue, ColumnTrait, ColumnType, ConnectionTrait, DatabaseConnection, EntityName,
    EntityTrait, PaginatorTrait, QueryFilter,
};

use crate::entities::sessions;

#[derive(Clone, Debug, Default)]
pub struct DbPool {
    pool: DatabaseConnection,
}

impl DbPool {
    pub fn new(db: DatabaseConnection) -> DbPool {
        //https://www.sea-ql.org/SeaORM/docs/install-and-config/connection/
        //"Under the hood, a sqlx::Pool is created and owned by DatabaseConnection."
        DbPool { pool: db }
    }
}

//https://github.com/AscendingCreations/AxumSession/blob/main/examples/middleware_layer/src/main.rs
//https://github.com/AscendingCreations/AxumSession/blob/main/databases/sqlx/src/sqlite.rs

#[async_trait]
impl DatabasePool for DbPool {
    #[inline(always)]
    async fn initiate(&self, _table_name: &str) -> Result<(), DatabaseError> {
        let builder = self.pool.get_database_backend();

        let create_table = builder.build(
            &Table::create()
                .if_not_exists()
                .table(sessions::Entity.table_ref())
                .col(
                    ColumnDef::new_with_type(
                        sessions::Column::Id,
                        ColumnType::String(sea_query::StringLen::N(128)),
                    )
                    .not_null(),
                )
                .col(
                    ColumnDef::new_with_type(sessions::Column::Expires, ColumnType::Date)
                        .not_null(),
                )
                .col(
                    ColumnDef::new_with_type(sessions::Column::Session, ColumnType::Text)
                        .not_null(),
                )
                .primary_key(
                    Index::create()
                        .name("sessions_idx")
                        .col(sessions::Column::Id)
                        .primary(),
                )
                .to_owned(),
        );

        self.pool
            .execute(create_table)
            .await
            .map_err(|err| DatabaseError::GenericCreateError(err.to_string()))?;

        let create_index = builder.build(
            &Index::create()
                .if_not_exists()
                .name("sessions_expires_idx")
                .table(sessions::Entity.table_ref())
                .col(sessions::Column::Expires)
                .to_owned(),
        );

        self.pool
            .execute(create_index)
            .await
            .map_err(|err| DatabaseError::GenericCreateError(err.to_string()))?;

        // use sea_orm_migration::{MigrationTrait, SchemaManager};
        // let manager = SchemaManager::new(&self.pool);
        // crate::migration::Migration
        //     .up(&manager)
        //     .await
        //     .map_err(|err| DatabaseError::GenericCreateError(err.to_string()))?;

        // sqlx::query(
        //     &r#"
        //     CREATE TABLE IF NOT EXISTS %%TABLE_NAME%% (
        //         "id" VARCHAR(128) NOT NULL PRIMARY KEY,
        //         "expires" BIGINT NULL,
        //         "session" TEXT NOT NULL
        //     )
        // "#
        //     .replace("%%TABLE_NAME%%", table_name),
        // )
        // .execute(&self.pool)
        // .await
        // .map_err(|err| DatabaseError::GenericCreateError(err.to_string()))?;

        Ok(())
    }

    #[inline(always)]
    async fn delete_by_expiry(&self, _table_name: &str) -> Result<Vec<String>, DatabaseError> {
        let results = sessions::Entity::find()
            .filter(
                sessions::Column::Expires
                    .is_null()
                    .or(sessions::Column::Expires.lt(Utc::now())),
            )
            .all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        // let result: Vec<(String,)> = sqlx::query_as(
        //     &r#"
        //     SELECT id FROM %%TABLE_NAME%%
        //     WHERE (expires IS NULL OR expires < $1)
        // "#
        //     .replace("%%TABLE_NAME%%", table_name),
        // )
        // .bind(Utc::now().timestamp())
        // .fetch_all(&self.pool)
        // .await
        // .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        // let result: Vec<String> = result.into_iter().map(|(s,)| s).collect();

        let result = results.iter().map(|model| model.id.clone()).collect();

        sessions::Entity::delete_many()
            .filter(sessions::Column::Expires.lt(Utc::now()))
            .exec(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

        // sqlx::query(
        //     &r#"DELETE FROM %%TABLE_NAME%% WHERE expires < $1"#
        //         .replace("%%TABLE_NAME%%", table_name),
        // )
        // .bind(Utc::now().timestamp())
        // .execute(&self.pool)
        // .await
        // .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

        Ok(result)
    }

    #[inline(always)]
    async fn count(&self, _table_name: &str) -> Result<i64, DatabaseError> {
        let count = sessions::Entity::find()
            .count(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        // let (count,) = sqlx::query_as(
        //     &r#"SELECT COUNT(*) FROM %%TABLE_NAME%%"#.replace("%%TABLE_NAME%%", table_name),
        // )
        // .fetch_one(&self.pool)
        // .await
        // .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        return Ok(count as i64);
    }

    //https://github.com/AscendingCreations/AxumSession/blob/main/src/session_data.rs
    //   pub(crate) expires: DateTime<Utc>,

    #[inline(always)]
    async fn store(
        &self,
        id: &str,
        session: &str,
        expires: i64,
        _table_name: &str,
    ) -> Result<(), DatabaseError> {
        //https://www.sea-ql.org/SeaORM/docs/basic-crud/update/
        //https://www.sea-ql.org/SeaORM/docs/basic-crud/insert/

        //should be seconds since 1970-01-01 00:00:00 UTC
        let expires = chrono::DateTime::from_timestamp(expires, 0)
            .map(|expires| Utc.from_utc_datetime(&expires.naive_utc()));

        let model = sessions::ActiveModel {
            id: ActiveValue::set(id.to_owned()),
            session: ActiveValue::set(session.to_string()),
            expires: ActiveValue::set(expires),
        };

        sessions::Entity::insert(model.clone())
            .on_conflict(
                sea_query::OnConflict::column(sessions::Column::Id)
                    .update_columns([sessions::Column::Expires, sessions::Column::Session])
                    .to_owned(),
            )
            .exec(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;

        //     sqlx::query(
        //         &r#"
        //     INSERT INTO %%TABLE_NAME%%
        //         (id, session, expires) SELECT $1, $2, $3
        //     ON CONFLICT(id) DO UPDATE SET
        //         expires = EXCLUDED.expires,
        //         session = EXCLUDED.session
        // "#
        //         .replace("%%TABLE_NAME%%", table_name),
        //     )
        //     .bind(id)
        //     .bind(session)
        //     .bind(expires)
        //     .execute(&self.pool)
        //     .await
        //     .map_err(|err| DatabaseError::GenericInsertError(err.to_string()))?;
        Ok(())
    }

    #[inline(always)]
    async fn load(&self, id: &str, _table_name: &str) -> Result<Option<String>, DatabaseError> {
        let maybe_model = sessions::Entity::find()
            .filter(sessions::Column::Id.eq(id))
            .filter(
                sessions::Column::Expires
                    .is_null()
                    .or(sessions::Column::Expires.gt(Utc::now())),
            )
            .one(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        if let Some(model) = maybe_model {
            Ok(Some(model.session.to_string()))
        } else {
            Ok(None)
        }

        // let result: Option<(String,)> = sqlx::query_as(
        //     &r#"
        //     SELECT session FROM %%TABLE_NAME%%
        //     WHERE id = $1 AND (expires IS NULL OR expires > $2)
        // "#
        //     .replace("%%TABLE_NAME%%", table_name),
        // )
        // .bind(id)
        // .bind(Utc::now().timestamp())
        // .fetch_optional(&self.pool)
        // .await
        // .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        // Ok(result.map(|(session,)| session))
    }

    #[inline(always)]
    async fn delete_one_by_id(&self, id: &str, _table_name: &str) -> Result<(), DatabaseError> {
        sessions::Entity::delete_many()
            .filter(sessions::Column::Id.eq(id))
            .exec(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

        // sqlx::query(
        //     &r#"DELETE FROM %%TABLE_NAME%% WHERE id = $1"#.replace("%%TABLE_NAME%%", table_name),
        // )
        // .bind(id)
        // .execute(&self.pool)
        // .await
        // .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;
        Ok(())
    }

    #[inline(always)]
    async fn exists(&self, id: &str, _table_name: &str) -> Result<bool, DatabaseError> {
        let count = sessions::Entity::find()
            .filter(sessions::Column::Id.eq(id))
            .filter(sessions::Column::Expires.gt(Utc::now()))
            .count(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        // let result: Option<(i64,)> = sqlx::query_as(
        //     &r#"
        //     SELECT COUNT(*) FROM %%TABLE_NAME%%
        //     WHERE id = $1 AND (expires IS NULL OR expires > $2)
        // "#
        //     .replace("%%TABLE_NAME%%", table_name),
        // )
        // .bind(id)
        // .bind(Utc::now().timestamp())
        // .fetch_optional(&self.pool)
        // .await
        // .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        // Ok(result.map(|(o,)| o).unwrap_or(0) > 0)
        Ok(count > 0)
    }

    #[inline(always)]
    async fn delete_all(&self, _table_name: &str) -> Result<(), DatabaseError> {
        sessions::Entity::delete_many()
            .exec(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;

        // sqlx::query(&r#"DELETE FROM %%TABLE_NAME%%"#.replace("%%TABLE_NAME%%", table_name))
        //     .execute(&self.pool)
        //     .await
        //     .map_err(|err| DatabaseError::GenericDeleteError(err.to_string()))?;
        Ok(())
    }

    #[inline(always)]
    async fn get_ids(&self, _table_name: &str) -> Result<Vec<String>, DatabaseError> {
        let results = sessions::Entity::find()
            .filter(
                sessions::Column::Expires
                    .is_null()
                    .or(sessions::Column::Expires.gt(Utc::now())),
            )
            .all(&self.pool)
            .await
            .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        let result = results.iter().map(|model| model.id.clone()).collect();

        // let result: Vec<(String,)> = sqlx::query_as(
        //     &r#"
        //     SELECT id FROM %%TABLE_NAME%%
        //     WHERE (expires IS NULL OR expires > $1)
        // "#
        //     .replace("%%TABLE_NAME%%", table_name),
        // )
        // .bind(Utc::now().timestamp())
        // .fetch_all(&self.pool)
        // .await
        // .map_err(|err| DatabaseError::GenericSelectError(err.to_string()))?;

        // let result: Vec<String> = result.into_iter().map(|(s,)| s).collect();

        Ok(result)
    }

    #[inline(always)]
    fn auto_handles_expiry(&self) -> bool {
        false
    }
}
