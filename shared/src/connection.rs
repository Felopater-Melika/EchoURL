use sea_orm::{Database, DatabaseConnection};
use std::sync::Arc;

pub type DbPool = Arc<DatabaseConnection>;

pub async fn connect_db() -> DbPool {
    let db_url = "postgres://echo:1234@localhost:5432/echodb";

    let db = Database::connect(db_url)
        .await
        .expect("Failed to connect to database");

    Arc::new(db)
}
