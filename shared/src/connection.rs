use anyhow::{Context, Result};
use redis::Client;
use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;

pub type DbPool = Arc<DatabaseConnection>;

pub async fn connect_db() -> Result<DbPool> {
    let db_url = "postgres://echo:1234@localhost:5432/echodb";

    let db = Database::connect(db_url)
        .await
        .context("Failed to connect to database")?;

    Ok(Arc::new(db))
}

pub type RedisPool = Arc<Client>;

pub async fn connect_redis() -> Result<RedisPool> {
    let redis_url = "redis://127.0.0.1:6379";

    let client = Client::open(redis_url).context("Failed to connect to Redis")?;

    Ok(Arc::new(client))
}
