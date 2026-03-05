use std::net::SocketAddr;

use tonic::{Request, Response, Status};

use gmf::server::TokioServer;
use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Default)]
pub struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let port = std::env::var("GRPC_PORT").unwrap_or_else(|_| "50051".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse().expect("valid address");

    TokioServer::builder()
        .addr(addr)
        .max_connections(10240)
        .build()
        .serve(GreeterServer::new(MyGreeter))
        .expect("server failed");
}
