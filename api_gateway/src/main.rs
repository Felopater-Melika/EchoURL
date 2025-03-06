use crate::echourl::shorten_url_client::ShortenUrlClient;
use crate::echourl::{DeleteResponse, OriginalUrl, ShortenedUrl};
use anyhow::Result;
use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use std::env;
use tonic::transport::Channel;
use tonic::Request;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};

mod echourl {
    tonic::include_proto!("echourl");
}

#[derive(Clone)]
struct State {}

#[tokio::main]
async fn main() -> Result<()> {
    unsafe {
        env::set_var("RUST_LOG", "debug");
    }

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let grpc_channel = Channel::from_static("http://127.0.0.1:50051/")
        .connect()
        .await?;

    let app = Router::new()
        .route("/createurl", post(create_url))
        .route("/deleteurl", delete(delete_url))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(grpc_channel))
                .layer(CompressionLayer::new())
                .layer(TraceLayer::new_for_http()),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("🚀 HTTP server listening on 0.0.0.0:3000");

    axum::serve(listener, app).await?;
    Ok::<(), anyhow::Error>(())
}

async fn create_url(
    Extension(grpc_channel): Extension<Channel>,
    Json(payload): Json<CreateUrlRequest>,
) -> Result<(StatusCode, Json<UrlCreated>), StatusCode> {
    let mut client = ShortenUrlClient::new(grpc_channel.clone());

    let grpc_request = Request::new(OriginalUrl {
        url: payload.url.clone(),
    });

    match client.create_shortened_url(grpc_request).await {
        Ok(response) => {
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
        Err(err) => {
            tracing::error!("gRPC call failed: {:?}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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
            tracing::error!("gRPC call failed: {:?}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
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
