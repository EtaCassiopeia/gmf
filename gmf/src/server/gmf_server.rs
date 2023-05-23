//! This module provides the `GmfServer` struct, which creates an instance of GlommioServer
//! and runs it in a dedicated thread. The `GmfServer` listens on a specified address,
//! and can be gracefully shutdown using a CTRL-C signal. The server serves incoming requests
//! using the provided hyper service.

use std::net::SocketAddr;

use glommio::{executor, GlommioError, Latency, LocalExecutorBuilder, Placement, Shares, Task};
use hyper::body::HttpBody;
use hyper::http;
use log::info;
use tower_service::Service;

use crate::server::glommio_server::GlommioServer;
use crate::server::glommio_server::Server;

const THREAD_NAME: &str = "gmf_server";

/// Represents a server that handles incoming GRPC requests.
pub struct GmfServer<S, RespBd, Error> {
    /// The service that handles incoming GRPC requests.
    service: S,
    _phantom: std::marker::PhantomData<(RespBd, Error)>,
}

impl<S, RespBd, Error> GmfServer<S, RespBd, Error>
where
    S: Service<
            http::request::Request<hyper::Body>,
            Response = http::response::Response<RespBd>,
            Error = Error,
        > + Clone
        + Send
        + 'static,
    Error: std::error::Error + 'static + Send + Sync,
    RespBd: HttpBody + 'static,
    RespBd::Error: std::error::Error + Send + Sync,
{
    /// Creates a new instance of `GmfServer`.
    pub fn new(service: S) -> Self {
        Self {
            service,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Serves the incoming requests using the provided service, in a separate Glommio task.
    /// Listens for incoming connections on the provided `SocketAddr`.
    /// Graceful shutdown is handled by listening for a CTRL-C signal.
    pub fn serve(&self, addr: SocketAddr) -> glommio::Result<(), ()> {
        let (signal_tx, mut signal_rx) = tokio::sync::mpsc::channel::<()>(1);

        ctrlc_async::set_async_handler(async move {
            info!("Received Ctrl-C, shutting down");
            signal_tx
                .send(())
                .await
                .expect("Error sending signal to server");
        })
        .expect("Error setting Ctrl-C handler");

        let service = self.service.clone();

        LocalExecutorBuilder::new(Placement::Unbound)
            .name(THREAD_NAME)
            .spawn(move || async move {
                let rpc_server_tq = executor().create_task_queue(
                    Shares::default(),
                    Latency::NotImportant,
                    "rpc_server_tq",
                );

                let server: GlommioServer = GlommioServer::new(1024, rpc_server_tq, addr);

                let server_task: Task<Result<(), GlommioError<()>>> =
                    server.serve(service).expect("GMF server failed!");

                let server_join_handle = server_task.detach();

                info!("Listening for GRPC requests on {}", addr);

                signal_rx.recv().await;

                server_join_handle.cancel();
                server_join_handle.await;
            })
            .expect("unable to spawn connection handler")
            .join()
    }
}
