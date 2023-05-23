//! This module provides a server implementation for the `Greeter` service
//! defined in the `hello_world` proto file. The `MyGreeter` struct is the
//! actual implementation of the service, and the `main` function is responsible
//! for creating an instance of `GmfServer` and serving the `MyGreeter` service on it.

use std::sync::Arc;

use log::{error, info};
use tonic::{Request, Response, Status};
use tower::Service;

use gmf::server::gmf_server::GmfServer;
use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

/// Greeter Service Implementation.
#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    /// Handler for the `say_hello` gRPC method.
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

// #[cfg(target_os = "linux")]
fn main() {
    env_logger::init();
    use std::net::SocketAddr;

    // let addr: SocketAddr = "[::1]:50051".parse().unwrap();
    let addr: SocketAddr = "0.0.0.0:50051".parse().unwrap();
    let greeter: MyGreeter = MyGreeter::default();

    let tonic: GreeterServer<MyGreeter> = GreeterServer::new(greeter);
    use hyper::service::service_fn;
    let gmf = GmfServer::new(
        service_fn(move |req| {
            let mut tonic = tonic.clone();
            tonic.call(req)
        }),
        1024,
    );

    let sender = Arc::clone(&gmf.signal_tx);

    ctrlc_async::set_async_handler(async move {
        info!("Received Ctrl-C, shutting down");
        sender.try_send(()).unwrap_or_else(|_| {
            error!("Failed to send termination signal.");
        });
    })
    .expect("Error setting Ctrl-C handler");

    // Run the gRPC server on the provided address
    gmf.serve(addr).unwrap_or_else(|e| panic!("failed {}", e));
}