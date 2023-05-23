use helloworld::greeter_client::GreeterClient;
use helloworld::HelloRequest;
use tonic::{transport::Channel, Request};

pub mod helloworld {
    tonic::include_proto!("helloworld");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let channel = Channel::from_static("http://[::1]:50051").connect().await?;
    let channel = Channel::from_static("http://0.0.0.0:50051")
        .connect()
        .await?;
    let mut client = GreeterClient::new(channel);

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });

    let response = client.say_hello(request).await?.into_inner();

    println!("RESPONSE={:?}", response);

    Ok(())
}
