use axum::{Extension, Router, routing::get};
use echourl::hello_world_server::{HelloWorld, HelloWorldServer};
use echourl::{Greet, Greeted};
use tokio::sync::oneshot;
use tonic::{Request, Response, Status, transport::Server};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::{self, TraceLayer};
use tracing::Level;

mod echourl {
    tonic::include_proto!("echourl");
}

#[derive(Debug, Default)]
struct HelloWorldService {}

#[tonic::async_trait]
impl HelloWorld for HelloWorldService {
    async fn greeter(&self, request: Request<Greet>) -> Result<Response<Greeted>, Status> {
        println!("Got a request: {:?}", request);

        let response = Greeted {
            greeted: "Hello, World from gRPC!".to_string(),
        };
        Ok(Response::new(response))
    }
}

#[derive(Clone)]
struct State {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let http_server = tokio::spawn(async move {
        let app = Router::new().route("/", get(root)).layer(
            ServiceBuilder::new()
                .layer(Extension(State {}))
                .layer(CompressionLayer::new())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                        .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
                ),
        );

        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        println!("🚀 HTTP server listening on 0.0.0.0:3000");

        axum::serve(listener, app).await.unwrap();
    });

    let grpc_server = tokio::spawn(async move {
        let addr = "0.0.0.0:50051".parse().unwrap();
        let service = HelloWorldService::default();

        println!("🚀 gRPC server listening on 0.0.0.0:50051");

        Server::builder()
            .add_service(HelloWorldServer::new(service))
            .serve_with_shutdown(addr, async {
                shutdown_rx.await.ok();
            })
            .await
            .unwrap();
    });

    tokio::select! {
        _ = http_server => {
            println!("HTTP server stopped!");
            let _ = shutdown_tx.send(());
        }
        _ = grpc_server => {
            println!("gRPC server stopped!");
        }
    }

    Ok(())
}

async fn root() -> &'static str {
    "Hello, World from HTTP!"
}
