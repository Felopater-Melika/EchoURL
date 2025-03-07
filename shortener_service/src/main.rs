use anyhow::{Context, Result};
use echourl::shorten_url_server::{ShortenUrl, ShortenUrlServer};
use echourl::{DeleteResponse, OriginalUrl, ShortenedUrl};
use entity::url;
use rand::distr::Alphanumeric;
use rand::{rng, Rng};
use redis::AsyncCommands;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use shared::connection::{connect_db, connect_redis};
use std::env;
use std::sync::Arc;
use thiserror::Error;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tracing::{error, info, Level};

mod echourl {
    tonic::include_proto!("echourl");
}

#[derive(Debug, Error)]
pub enum UrlShortenerError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("URL not found")]
    NotFound,

    #[error("Failed to generate short code")]
    ShortCodeGenerationFailed,

    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

impl From<UrlShortenerError> for Status {
    fn from(err: UrlShortenerError) -> Self {
        match err {
            UrlShortenerError::DatabaseError(e) => Status::internal(e.to_string()),
            UrlShortenerError::NotFound => Status::not_found("URL not found"),
            UrlShortenerError::ShortCodeGenerationFailed => {
                Status::internal("Failed to generate short code")
            }
            UrlShortenerError::InternalServerError(msg) => Status::internal(msg),
        }
    }
}

fn generate_short_code(length: usize) -> Result<String, UrlShortenerError> {
    let rng = rng();
    let code: String = rng
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();

    if code.is_empty() {
        Err(UrlShortenerError::ShortCodeGenerationFailed)
    } else {
        Ok(code)
    }
}

#[derive(Debug)]
struct ShortenUrlService {
    db: Arc<DatabaseConnection>,
    redis: Arc<redis::Client>,
}

impl ShortenUrlService {
    pub fn new(db: Arc<DatabaseConnection>, redis: Arc<redis::Client>) -> Self {
        Self { db, redis }
    }
}

#[tonic::async_trait]
impl ShortenUrl for ShortenUrlService {
    async fn create_shortened_url(
        &self,
        request: Request<OriginalUrl>,
    ) -> Result<Response<ShortenedUrl>, Status> {
        let original_url = request.into_inner().url;
        let short_code = generate_short_code(5)?;

        let shortened_url = url::ActiveModel {
            id: Default::default(),
            original: Set(original_url.clone()),
            shortened: Set(short_code.clone()),
            clicks: Set(0),
            created_at: Default::default(),
        };

        let saved_url = shortened_url
            .insert(&*self.db)
            .await
            .map_err(UrlShortenerError::from)?;

        info!("Shortened URL: {}", saved_url.id);
        let mut redis_conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                error!("Failed to get Redis connection: {:?}", e);
                UrlShortenerError::InternalServerError("Redis connection error".into())
            })?;

        redis_conn
            .set_ex::<_, _, ()>(format!("slug:{}", short_code), original_url.clone(), 86_400)
            .await
            .map_err(|e| {
                error!("Failed to cache in Redis: {:?}", e);
                UrlShortenerError::InternalServerError("Redis cache error".into())
            })?;

        Ok(Response::new(ShortenedUrl {
            id: saved_url.id,
            original_url: saved_url.original,
            shortened_url: saved_url.shortened,
            clicks: saved_url.clicks,
            created_at: saved_url.created_at.to_string(),
        }))
    }

    async fn delete_shortened_url(
        &self,
        request: Request<OriginalUrl>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let original_url = request.into_inner().url;

        let delete_result = url::Entity::delete_many()
            .filter(url::Column::Original.eq(&original_url))
            .exec(&*self.db)
            .await
            .map_err(UrlShortenerError::from)?;

        if delete_result.rows_affected == 0 {
            return Err(UrlShortenerError::NotFound.into());
        }

        info!("Deleted {} URL(s)", delete_result.rows_affected);
        let mut redis_conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| {
                error!("Failed to get Redis connection: {:?}", e);
                UrlShortenerError::InternalServerError("Redis connection error".into())
            })?;

        redis_conn
            .del::<_, ()>(format!("slug:{}", original_url))
            .await
            .map_err(|e| {
                error!("Failed to delete cache entry from Redis: {:?}", e);
                UrlShortenerError::InternalServerError("Redis deletion error".into())
            })?;

        Ok(Response::new(DeleteResponse {
            message: "URL deleted successfully".to_string(),
            success: true,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    unsafe {
        env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let db = connect_db().await.context("Database connection failed")?;
    let redis = connect_redis().await.context("Redis connection failed")?;

    let addr = "0.0.0.0:50051".parse()?;
    let service = ShortenUrlService::new(db.clone(), redis.clone());

    info!("🚀 gRPC server listening on {}", addr);

    Server::builder()
        .add_service(ShortenUrlServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
