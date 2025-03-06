use echourl::hello_world_client::HelloWorldClient;
use echourl::{Greet, Greeted};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

mod echourl {
    tonic::include_proto!("echourl");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = HelloWorldClient::connect("http://0.0.0.0:50051").await?;

    println!("🚀 gRPC Client Requesting 0.0.0.0:50051");

    let request = tonic::Request::new(Greet {
        greet: "Hello from gRPC Client!".to_string(),
    });

    let response = client.greeter(request).await?;
    println!("gRPC Response: {:?}", response.into_inner().greeted);

    Ok(())
}
