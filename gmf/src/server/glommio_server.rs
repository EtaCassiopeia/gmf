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

/// GlommioServer is a server that listens for connections on a specified address,
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
            addr: addr,
        }
    }
}

pub struct ConnResult(SocketAddr, Result<(), hyper::Error>);

impl From<ConnResult> for io::Result<()> {
    fn from(value: ConnResult) -> Self {
        match value.1 {
            Err(err) if !err.is_incomplete_message() => {
                error!("Stream from {:?} failed with error {:?}", value.0, err);
                Err(())
            }
            Err(_) => Err(()),
            _ => Ok(()),
        }
        .map_err(|_| io::Error::from(ConnectionReset))
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
        let max_connections = self.max_connections.clone();
        let task_q = self.task_q;

        debug!("Binding to address {:?}.", self.addr);
        let listener = TcpListener::bind(self.addr)?;

        let conn_control = Rc::new(Semaphore::new(max_connections as _));

        let spawn_result = GlommioExecutor { task_q }.spawn(async move {
            if max_connections == 0 {
                error!("Max connections is 0, no connections will be accepted.");
                return Err::<(), GlommioError<()>>(io::Error::from(ConnectionRefused).into());
            }

            debug!("Listening for connections.");

            loop {
                let stream = listener.accept().await?;
                let addr = stream.local_addr()?;

                debug!("Accepted connection from {:?}.", addr);

                let scoped_conn_control = conn_control.clone();
                let captured_service = service.clone();

                GlommioExecutor { task_q }.execute(async move {
                    let _semaphore_permit = scoped_conn_control.acquire_permit(1).await?;

                    let http = Http::new()
                        .with_executor(GlommioExecutor { task_q })
                        .serve_connection(TokioIO(stream), captured_service);

                    debug!("Serving connection from {:?}.", addr);

                    let conn_res: io::Result<()> = ConnResult(addr, http.await).into();

                    debug!("Connection from {:?} closed.", addr);
                    conn_res
                });
            }
        });

        spawn_result.map_err(|_| io::Error::new(Other, "Failed to spawn server."))
    }
}
