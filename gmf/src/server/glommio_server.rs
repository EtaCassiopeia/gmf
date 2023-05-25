//! This module contains the implementation of a Glommio-based server adapted to handle
//! Hyper services. It includes the GlommioServer struct which accepts connections
//! and serves them.
//!
//! In order to use this server, you will need to provide an implementation of the
//! tower_service::Service trait, which is used to handle incoming requests.
use std::{
    error::Error as StdError,
    io::{
        self,
        ErrorKind::{ConnectionRefused, ConnectionReset, Other},
    },
    net::SocketAddr,
    rc::Rc,
};

use glommio::{net::TcpListener, sync::Semaphore, GlommioError, TaskQueueHandle as TaskQ};
use hyper::{rt::Executor, server::conn::Http, Request, Response};
use log::{debug, error};
use tower_service::Service;

use crate::server::executor::GlommioExecutor;
use crate::server::tokio_interop::TokioIO;

/// An abstraction for a server, this trait is implemented by GlommioServer.
pub trait Server<Service> {
    type Result;

    /// Serves the incoming requests using the provided service.
    fn serve(&self, service: Service) -> Self::Result;
}

/// GlommioServer listens for connections on a specified address,
/// and serves them using a provided service.
#[derive(Debug)]
pub struct GlommioServer {
    /// Maximum number of concurrent connections this server will accept.
    pub max_connections: usize,
    /// The Glommio task queue handle for this server.
    pub task_q: TaskQ,
    /// The address this server will listen on.
    pub addr: SocketAddr,
}

impl GlommioServer {
    /// Creates a new instance of GlommioServer.
    pub fn new(max_connections: usize, task_q: TaskQ, addr: SocketAddr) -> Self {
        Self {
            max_connections,
            task_q,
            addr,
        }
    }
}

/// Implementation of Server trait for GlommioServer.
impl<S, RespBd, Error> Server<S> for GlommioServer
where
    S: Service<Request<hyper::Body>, Response = Response<RespBd>, Error = Error> + Clone + 'static,
    Error: StdError + 'static + Send + Sync,
    RespBd: hyper::body::HttpBody + 'static,
    RespBd::Error: StdError + Send + Sync,
{
    type Result = io::Result<glommio::Task<Result<(), GlommioError<()>>>>;

    /// Serves an instance of Service to incoming connections.
    ///
    /// This method will block until the server is shut down. It continuously accepts
    /// incoming connections and serves them using the provided service.
    /// Each connection is served in its own Glommio task.
    fn serve(&self, service: S) -> Self::Result {
        let max_connections = self.max_connections;
        let task_q = self.task_q;

        debug!("Binding to address {:?}.", self.addr);
        let listener = TcpListener::bind(self.addr)?;

        let conn_control = Rc::new(Semaphore::new(max_connections as _));

        let spawn_result = GlommioExecutor { task_q }.spawn(async move {
            debug!("Listening for connections.");

            loop {
                let stream = listener.accept().await?;
                let addr = stream.local_addr()?;

                debug!("Accepted connection from {:?}.", addr);

                let scoped_conn_control = conn_control.clone();
                let captured_service = service.clone();

                GlommioExecutor { task_q }.execute(async move {
                    // Acquire a permit from the connection semaphore. It will reject the connection if the max number of connections is reached.
                    // To avoid leaking permits, we use a scoped permit which will release the permit when it goes out of scope.
                    // scoped_conn_control.acquire_permit(1).await?; can be used instead of scoped_conn_control.try_acquire_permit(1) if we want to block the task until a permit is available.
                     match scoped_conn_control.try_acquire_permit(1) {
                         Ok(_) => {
                             debug!("Acquired connection semaphore, number of available connection permits : {}.",  scoped_conn_control.available());
                             let http = Http::new()
                                 .with_executor(GlommioExecutor { task_q })
                                 .serve_connection(TokioIO(stream), captured_service);

                              http.await.map_err(|e| {
                                 error!("Stream failed with error {:?}", e);
                                 io::Error::from(ConnectionReset).into()
                             })
                         },
                         Err(GlommioError::Closed(_)) => {
                             error!("Failed to acquire connection semaphore.");
                             Err::<(), GlommioError<()>>(io::Error::from(ConnectionRefused).into())
                         },
                         Err(_) => {
                             error!(
                                 "Max connections reached, refusing connection from {:?}.",
                                 addr
                             );
                             Err::<(), GlommioError<()>>(io::Error::from(ConnectionRefused).into())
                         }
                     }
                });
            }
        });

        spawn_result.map_err(|_| io::Error::new(Other, "Failed to spawn server."))
    }
}
