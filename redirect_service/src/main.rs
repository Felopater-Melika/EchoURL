use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::{extract::Path, routing::get, Router};
use entity::url;
use redis::AsyncCommands;
use sea_orm::{ColumnTrait, EntityTrait};
use sea_orm::{DatabaseConnection, QueryFilter};
use shared::connect_db;
use shared::connection::connect_redis;
use std::env;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info, Level};

#[derive(Error, Debug)]
pub enum RedirectError {
    #[error("Slug not found")]
    NotFound,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

impl IntoResponse for RedirectError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            RedirectError::NotFound => (StatusCode::NOT_FOUND, "Slug not found"),
            RedirectError::DatabaseError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            }
            RedirectError::RedisError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Redis error"),
            RedirectError::InternalServerError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Server error")
            }
        };

        (status, message).into_response()
    }
}

#[derive(Clone)]
struct AppState {
    db: Arc<DatabaseConnection>,
    redis: Arc<redis::Client>,
}

#[tokio::main]
async fn main() -> Result<()> {
    unsafe {
        env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let db = connect_db().await.context("Database connection failed")?;
    let redis = connect_redis().await.context("Redis connection failed")?;

    let state = AppState {
        db: db.clone(),
        redis: redis.clone(),
    };

    let app = Router::new()
        .route("/{slug}", get(handle_redirect))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000")
        .await
        .context("Failed to bind HTTP server to port 4000")?;
    info!("🚀 HTTP server listening on 0.0.0.0:4000");

    axum::serve(listener, app)
        .await
        .context("HTTP server error")?;
    Ok(())
}

async fn handle_redirect(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, RedirectError> {
    let db = state.db.clone();

    let redis_client = state.redis.clone();
    let mut redis_conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| {
            error!("Failed to get Redis connection: {:?}", e);
            RedirectError::RedisError(e)
        })?;

    let cache_key = format!("slug:{}", slug);
    match redis_conn.get::<_, String>(&cache_key).await {
        Ok(original_url) => {
            info!("Cache hit for `{}`", slug);
            return Ok(Redirect::permanent(&original_url));
        }
        Err(e) => {
            error!("Cache miss for `{}`: {:?}", slug, e);
        }
    }

    let url_entry = url::Entity::find()
        .filter(url::Column::Shortened.eq(slug.clone()))
        .one(&*db)
        .await
        .map_err(RedirectError::DatabaseError)?
        .ok_or(RedirectError::NotFound)?;

    info!("Queried DB, caching `{}`", url_entry.original.clone());

    redis_conn
        .set_ex::<_, _, ()>(
            format!("slug:{}", slug.clone()),
            url_entry.original.clone(),
            86_400,
        )
        .await
        .map_err(|e| {
            error!("Failed to cache in Redis: {:?}", e);
            RedirectError::InternalServerError("Redis cache error".into())
        })?;

    Ok(Redirect::temporary(&*url_entry.original))
}
