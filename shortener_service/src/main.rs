use anyhow::Result;
use echourl::Greet;
use sea_orm::{Database, DatabaseConnection};
use std::env;
use tonic::transport::Server;
use tracing::{error, info};

mod echourl {
    tonic::include_proto!("echourl");
}

#[derive(Default)]
pub struct MyServer {
    connection: DatabaseConnection,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    let connection = Database::connect("postgres://echo:1234@localhost:5432/echodb").await?;
    // Migrator::up(&connection, None).await?;

    // let addr = "0.0.0.0:50051".parse()?;
    // let hello_server = MyServer { connection };
    // let grpc_server = tokio::spawn(async move {
    //     let addr = "0.0.0.0:50051".parse()?;
    //     let service = HelloWorldService::default();
    //
    //     info!("🚀 gRPC server listening on 0.0.0.0:50051");
    //
    //     Server::builder()
    //         .add_service(HelloWorldServer::new(service))
    //         .await?;
    //     Ok::<(), anyhow::Error>(())
    // });
    Ok(())
}
// mod echourl {
//     tonic::include_proto!("echourl");
// }
//
// #[tokio::main]
// async fn main() -> Result<()> {
//     tracing_subscriber::fmt()
//         .with_max_level(tracing::Level::INFO)
//         .init();
//
//     let mut client = HelloWorldClient::connect("http://0.0.0.0:50051").await?;
//     info!("🚀 gRPC Client Requesting 0.0.0.0:50051");
//
//     let request = tonic::Request::new(Greet {
//         greet: "Hello from gRPC Client!".to_string(),
//     });
//
//     match client.greeter(request).await {
//         Ok(response) => info!("gRPC Response: {:?}", response.into_inner().greeted),
//         Err(e) => error!("❌ gRPC Request failed: {}", e),
//     }
//
//     Ok(())
// }
