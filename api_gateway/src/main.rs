use anyhow::Result;
use axum::{Extension, Router, routing::get};
use echourl::hello_world_server::{HelloWorld, HelloWorldServer};
use echourl::{Greet, Greeted};
use std::env;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status, transport::Server};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::{self, TraceLayer};
use tracing::{Level, error, info};

mod echourl {
    tonic::include_proto!("echourl");
}

#[derive(Debug, Default)]
struct HelloWorldService {}

#[tonic::async_trait]
impl HelloWorld for HelloWorldService {
    async fn greeter(&self, request: Request<Greet>) -> Result<Response<Greeted>, Status> {
        info!("Received gRPC request: {:?}", request);

        let response = Greeted {
            greeted: "Hello, World from gRPC!".to_string(),
        };
        Ok(Response::new(response))
    }
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

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let http_server = tokio::spawn(async move {
        let app = Router::new().route("/", get(root)).layer(
            ServiceBuilder::new()
                .layer(Extension(State {}))
                .layer(CompressionLayer::new())
                .layer(TraceLayer::new_for_http()),
        );

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
        info!("🚀 HTTP server listening on 0.0.0.0:3000");

        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    });

    let grpc_server = tokio::spawn(async move {
        let addr = "0.0.0.0:50051".parse()?;
        let service = HelloWorldService::default();

        info!("🚀 gRPC server listening on 0.0.0.0:50051");

        Server::builder()
            .add_service(HelloWorldServer::new(service))
            .serve_with_shutdown(addr, async {
                shutdown_rx.await.ok();
            })
            .await?;
        Ok::<(), anyhow::Error>(())
    });

    tokio::select! {
        _ = http_server => {
            error!("HTTP server stopped!");
            let _ = shutdown_tx.send(());
        }
        _ = grpc_server => {
            error!("gRPC server stopped!");
        }
    }

    Ok(())
}

async fn root() -> &'static str {
    "Hello, World from HTTP!"
}
