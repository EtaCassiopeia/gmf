//! This module provides the `GmfServer` struct, which creates an instance of GlommioServer
//! and runs it in a dedicated thread. The `GmfServer` listens on a specified address,
//! and can be gracefully shutdown using a CTRL-C signal. The server serves incoming requests
//! using the provided hyper service.

use std::net::SocketAddr;
use std::sync::Arc;

use glommio::{executor, GlommioError, Latency, LocalExecutorBuilder, Placement, Shares, Task};
use hyper::body::HttpBody;
use hyper::http;
use log::info;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tower_service::Service;

use crate::server::glommio_server::GlommioServer;
use crate::server::glommio_server::Server;

const THREAD_NAME: &str = "gmf_server";

/// Represents a server that handles incoming GRPC requests.
pub struct GmfServer<S, RespBd, Error> {
    /// The service that handles incoming GRPC requests.
    service: S,
    max_connections: usize,
    /// A channel used to send a termination signal to the server.
    pub signal_tx: Arc<Sender<()>>,
    /// A channel used to receive a termination signal from the server.
    signal_rx: Arc<Mutex<Receiver<()>>>,
    /// Specifies a policy by which Executor selects CPUs for tasks.
    placement: Placement,
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
    ///
    /// # Example
    ///
    /// ```ignore
    /// let greeter: MyGreeter = MyGreeter::default();
    /// let tonic: GreeterServer<MyGreeter> = GreeterServer::new(greeter);
    ///
    /// use glommio::Placement;
    /// use hyper::service::service_fn;
    /// use gmf::server::gmf_server::GmfServer;
    ///
    /// let gmf = GmfServer::new(
    ///     service_fn(move |req| {
    ///         let mut tonic = tonic.clone();
    ///         tonic.call(req)
    ///     }),
    ///     10240,  // max_connections
    ///     Placement::Fixed(0) //Specifies a policy by which Executor selects CPUs to run on.
    /// );
    ///
    /// ```
    pub fn new(service: S, max_connections: usize, placement: Placement) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel::<()>(1);
        Self {
            service,
            max_connections,
            signal_tx: Arc::new(sender),
            signal_rx: Arc::new(Mutex::new(receiver)),
            placement,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Terminates the server.
    pub async fn terminate(&self) {
        let sender = Arc::clone(&self.signal_tx);
        if sender.try_send(()).is_err() {
            println!("Failed to send termination signal.");
        }
    }

    /// Serves the incoming requests using the provided service, in a separate Glommio task.
    /// Listens for incoming connections on the provided `SocketAddr`.
    /// Graceful shutdown is handled by listening for a CTRL-C signal.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::net::SocketAddr;
    /// use gmf::server::gmf_server::GmfServer;
    ///
    /// let gmf = GmfServer::new(...);
    ///
    /// let addr: SocketAddr = "0.0.0.0:50051".parse().unwrap();
    ///
    /// gmf.serve(addr).unwrap_or_else(|e| panic!("failed {}", e));
    /// ```
    pub fn serve(&self, addr: SocketAddr) -> glommio::Result<(), ()> {
        let service = self.service.clone();
        let max_connections = self.max_connections;
        let signal_rx_clone = Arc::clone(&self.signal_rx);
        let placement = self.placement.clone();

        LocalExecutorBuilder::new(placement)
            .name(THREAD_NAME)
            .spawn(move || async move {
                let rpc_server_tq = executor().create_task_queue(
                    Shares::default(),
                    Latency::NotImportant,
                    "rpc_server_tq",
                );

                let server: GlommioServer =
                    GlommioServer::new(max_connections, rpc_server_tq, addr);

                let server_task: Task<Result<(), GlommioError<()>>> =
                    server.serve(service).expect("GMF server failed!");

                let server_join_handle = server_task.detach();

                info!("Listening for GRPC requests on {}", addr);

                let mut signal_rx = signal_rx_clone.lock().await;
                signal_rx.recv().await;

                server_join_handle.cancel();
                server_join_handle.await;
            })
            .expect("unable to spawn connection handler")
            .join()
    }
}
