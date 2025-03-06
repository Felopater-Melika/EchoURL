use anyhow::Result;
use echourl::shorten_url_server::{ShortenUrl, ShortenUrlServer};
use echourl::{DeleteResponse, OriginalUrl, ShortenedUrl};
use entity::url;
use rand::distr::Alphanumeric;
use rand::{Rng, rng};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use shared::connection::connect_db;
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tracing::{error, info};

mod echourl {
    tonic::include_proto!("echourl");
}

fn generate_short_code(length: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

#[derive(Debug)]
struct ShortenUrlService {
    db: Arc<DatabaseConnection>,
}

impl ShortenUrlService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[tonic::async_trait]
impl ShortenUrl for ShortenUrlService {
    async fn create_shortened_url(
        &self,
        request: Request<OriginalUrl>,
    ) -> std::result::Result<Response<ShortenedUrl>, Status> {
        let shortened_url = url::ActiveModel {
            id: Default::default(),
            original: Set(request.into_inner().url),
            shortened: Set(generate_short_code(5)),
            clicks: Set(0),
            created_at: Default::default(),
        };

        let shortened_url: url::Model = shortened_url
            .insert(&*self.db)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        info!("Shortened URL: {}", shortened_url.id);

        Ok(Response::new(ShortenedUrl {
            id: shortened_url.id,
            original_url: shortened_url.original.clone(),
            shortened_url: shortened_url.shortened.clone(),
            clicks: shortened_url.clicks,
            created_at: shortened_url.created_at.to_string(),
        }))
    }

    async fn delete_shortened_url(
        &self,
        request: Request<OriginalUrl>,
    ) -> std::result::Result<Response<DeleteResponse>, Status> {
        let original_url = request.into_inner().url;

        let delete_result = url::Entity::delete_many()
            .filter(url::Column::Original.eq(original_url))
            .exec(&*self.db)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        info!("Delete result: {:?}", delete_result.rows_affected);

        Ok(Response::new(DeleteResponse {
            message: "URL deleted successfully".to_string(),
            success: delete_result.rows_affected > 0,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let db = connect_db().await;

    let addr = "0.0.0.0:50051".parse()?;
    let service = ShortenUrlService::new(db.clone());

    info!("🚀 gRPC server listening on {}", addr);

    Server::builder()
        .add_service(ShortenUrlServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
