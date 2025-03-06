use anyhow::Result;
use echourl::Greet;
use echourl::hello_world_client::HelloWorldClient;
use tonic::Request;
use tracing::{error, info};

mod echourl {
    tonic::include_proto!("echourl");
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut client = HelloWorldClient::connect("http://0.0.0.0:50051").await?;
    info!("🚀 gRPC Client Requesting 0.0.0.0:50051");

    let request = tonic::Request::new(Greet {
        greet: "Hello from gRPC Client!".to_string(),
    });

    match client.greeter(request).await {
        Ok(response) => info!("✅ gRPC Response: {:?}", response.into_inner().greeted),
        Err(e) => error!("❌ gRPC Request failed: {}", e),
    }

    Ok(())
}
