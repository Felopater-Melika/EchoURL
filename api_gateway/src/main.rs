use crate::echourl::shorten_url_client::ShortenUrlClient;
use crate::echourl::{DeleteResponse, OriginalUrl, ShortenedUrl};
use anyhow::{Context, Result};
use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use std::env;
use thiserror::Error;
use tonic::transport::Channel;
use tonic::Request;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, Level};

mod echourl {
    tonic::include_proto!("echourl");
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("gRPC request failed: {0}")]
    GrpcError(#[from] tonic::Status),

    #[error("Internal server error: {0}")]
    InternalServerError(String),
}

impl From<ApiError> for StatusCode {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::GrpcError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    unsafe {
        env::set_var("RUST_LOG", "info");
    }

    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let grpc_channel = Channel::from_static("http://127.0.0.1:50051/")
        .connect()
        .await
        .context("Failed to connect to gRPC server")?;

    let app = Router::new()
        .route("/createurl", post(create_url))
        .route("/deleteurl", delete(delete_url))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(grpc_channel))
                .layer(CompressionLayer::new())
                .layer(TraceLayer::new_for_http()),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .context("Failed to bind HTTP server to port 3000")?;
    info!("🚀 HTTP server listening on 0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .context("HTTP server error")?;
    Ok(())
}

async fn create_url(
    Extension(grpc_channel): Extension<Channel>,
    Json(payload): Json<CreateUrlRequest>,
) -> Result<(StatusCode, Json<UrlCreated>), StatusCode> {
    let mut client = ShortenUrlClient::new(grpc_channel.clone());
    let grpc_request = Request::new(OriginalUrl {
        url: payload.url.clone(),
    });

    let response = client
        .create_shortened_url(grpc_request)
        .await
        .map_err(ApiError::from)?;
    let ShortenedUrl {
        id,
        original_url,
        shortened_url,
        clicks,
        created_at,
    } = response.into_inner();

    Ok((
        StatusCode::CREATED,
        Json(UrlCreated {
            id,
            original_url,
            shortened_url,
            clicks,
            created_at,
        }),
    ))
}

async fn delete_url(
    Extension(grpc_channel): Extension<Channel>,
    Json(payload): Json<DeleteUrlRequest>,
) -> Result<(StatusCode, Json<UrlDeleted>), StatusCode> {
    let mut client = ShortenUrlClient::new(grpc_channel);
    let grpc_request = Request::new(OriginalUrl {
        url: payload.url.clone(),
    });

    match client.delete_shortened_url(grpc_request).await {
        Ok(response) => {
            let DeleteResponse { message, success } = response.into_inner();
            Ok((StatusCode::OK, Json(UrlDeleted { message, success })))
        }
        Err(err) => {
            error!("gRPC call failed: {:?}", err);
            let status_code: StatusCode = ApiError::GrpcError(err).into();
            Err(status_code)
        }
    }
}

#[derive(Deserialize)]
struct CreateUrlRequest {
    url: String,
}

#[derive(Serialize)]
struct UrlCreated {
    id: i32,
    original_url: String,
    shortened_url: String,
    clicks: i32,
    created_at: String,
}

#[derive(Deserialize)]
struct DeleteUrlRequest {
    url: String,
}

#[derive(Serialize)]
struct UrlDeleted {
    message: String,
    success: bool,
}
