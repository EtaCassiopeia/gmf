use std::future::Future;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use http_body::Body as HttpBody;
use hyper::body::Incoming;
use hyper::rt::bounds::Http2ServerConnExec;

use crate::server::config::ServerConfig;
use crate::server::error::GmfError;
use crate::server::runtime::{
    Runtime, RuntimeExecutor, RuntimeSemaphore, RuntimeTcpListener, RuntimeTcpStream,
};

/// A runtime-agnostic, thread-per-core gRPC server.
pub struct GmfServer<R: Runtime> {
    config: ServerConfig,
    _runtime: PhantomData<R>,
}

/// Builder for constructing a `GmfServer`.
pub struct GmfServerBuilder<R: Runtime> {
    addr: SocketAddr,
    max_connections: usize,
    num_cores: Option<usize>,
    _runtime: PhantomData<R>,
}

impl<R: Runtime> GmfServer<R> {
    pub fn builder() -> GmfServerBuilder<R> {
        GmfServerBuilder {
            addr: ([0, 0, 0, 0], 50051).into(),
            max_connections: 10240,
            num_cores: None,
            _runtime: PhantomData,
        }
    }

    /// Serve a tower `Service` (e.g. a tonic gRPC service) using the configured runtime.
    ///
    /// Accepts `tower_service::Service` (as produced by tonic) and adapts it to hyper's
    /// service interface internally.
    pub fn serve<S, RespBd>(self, service: S) -> Result<(), GmfError>
    where
        S: tower_service::Service<hyper::Request<Incoming>, Response = hyper::Response<RespBd>>
            + Clone
            + Send
            + 'static,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        S::Future: 'static,
        RespBd: HttpBody<Data = Bytes> + 'static,
        RespBd::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        R::Executor: Http2ServerConnExec<
            <TowerToHyperService<S> as hyper::service::Service<hyper::Request<Incoming>>>::Future,
            RespBd,
        >,
    {
        let addr = self.config.addr;
        let max_conns = self.config.max_connections;
        let cores = self.config.effective_cores();
        let shutdown = Arc::new(AtomicBool::new(false));

        tracing::info!(
            addr = %addr,
            cores = cores,
            max_connections = max_conns,
            "starting gmf server"
        );

        let hyper_svc = TowerToHyperService(service);

        R::run_multi_core(cores, move |cpu| {
            let service = hyper_svc.clone();
            let shutdown = shutdown.clone();
            async move { accept_loop::<R, _, RespBd>(addr, max_conns, cpu, service, shutdown).await }
        })
    }

    /// Serve with a shutdown signal.
    pub fn serve_with_shutdown<S, RespBd, Sig>(
        self,
        service: S,
        signal: Sig,
    ) -> Result<(), GmfError>
    where
        S: tower_service::Service<hyper::Request<Incoming>, Response = hyper::Response<RespBd>>
            + Clone
            + Send
            + 'static,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        S::Future: 'static,
        RespBd: HttpBody<Data = Bytes> + 'static,
        RespBd::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        R::Executor: Http2ServerConnExec<
            <TowerToHyperService<S> as hyper::service::Service<hyper::Request<Incoming>>>::Future,
            RespBd,
        >,
        Sig: Future<Output = ()> + Send + 'static,
    {
        let addr = self.config.addr;
        let max_conns = self.config.max_connections;
        let cores = self.config.effective_cores();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_for_signal = shutdown.clone();

        std::thread::spawn(move || {
            block_on_simple(async move {
                signal.await;
                shutdown_for_signal.store(true, Ordering::SeqCst);
                tracing::info!("shutdown signal received");
            });
        });

        tracing::info!(
            addr = %addr,
            cores = cores,
            max_connections = max_conns,
            "starting gmf server"
        );

        let hyper_svc = TowerToHyperService(service);

        R::run_multi_core(cores, move |cpu| {
            let service = hyper_svc.clone();
            let shutdown = shutdown.clone();
            async move { accept_loop::<R, _, RespBd>(addr, max_conns, cpu, service, shutdown).await }
        })
    }
}

/// Adapter from `tower_service::Service` (takes `&mut self`, has `poll_ready`)
/// to `hyper::service::Service` (takes `&self`, no `poll_ready`).
///
/// This clones the inner service on each call, which is the standard pattern
/// for tonic services (they're typically `Arc`-wrapped internally).
#[derive(Clone)]
pub struct TowerToHyperService<S>(S);

impl<S, ReqBody, RespBd> hyper::service::Service<hyper::Request<ReqBody>> for TowerToHyperService<S>
where
    S: tower_service::Service<hyper::Request<ReqBody>, Response = hyper::Response<RespBd>> + Clone,
{
    type Response = hyper::Response<RespBd>;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, req: hyper::Request<ReqBody>) -> Self::Future {
        // Clone and use &mut on the clone, since tower::Service::call takes &mut self
        let mut svc = self.0.clone();
        tower_service::Service::call(&mut svc, req)
    }
}

/// Minimal block_on for the shutdown signal thread.
fn block_on_simple<F: Future>(fut: F) -> F::Output {
    use std::pin::pin;
    use std::task::{Context, Poll, Wake, Waker};

    struct ThreadWaker(std::thread::Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker: Waker = Arc::new(ThreadWaker(std::thread::current())).into();
    let mut cx = Context::from_waker(&waker);
    let mut fut = pin!(fut);

    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => std::thread::park(),
        }
    }
}

impl<R: Runtime> GmfServerBuilder<R> {
    pub fn addr(mut self, addr: SocketAddr) -> Self {
        self.addr = addr;
        self
    }

    pub fn max_connections(mut self, n: usize) -> Self {
        self.max_connections = n;
        self
    }

    pub fn num_cores(mut self, n: usize) -> Self {
        self.num_cores = Some(n);
        self
    }

    pub fn build(self) -> GmfServer<R> {
        GmfServer {
            config: ServerConfig {
                addr: self.addr,
                max_connections: self.max_connections,
                num_cores: self.num_cores,
            },
            _runtime: PhantomData,
        }
    }
}

/// The core accept loop, shared across all runtimes.
async fn accept_loop<R, S, RespBd>(
    addr: SocketAddr,
    max_connections: usize,
    cpu: usize,
    service: S,
    shutdown: Arc<AtomicBool>,
) -> Result<(), GmfError>
where
    R: Runtime,
    S: hyper::service::Service<hyper::Request<Incoming>, Response = hyper::Response<RespBd>>
        + Clone
        + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    S::Future: 'static,
    RespBd: HttpBody<Data = Bytes> + 'static,
    RespBd::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    R::Executor: Http2ServerConnExec<S::Future, RespBd>,
{
    let listener = R::TcpListener::bind(addr)
        .await
        .map_err(|e| GmfError::Bind { addr, source: e })?;

    let semaphore = R::Semaphore::new(max_connections);
    let executor = R::Executor::default();

    tracing::info!(cpu = cpu, addr = %addr, "accepting connections");

    loop {
        if shutdown.load(Ordering::Relaxed) {
            tracing::info!(cpu = cpu, "shutting down accept loop");
            break;
        }

        let (stream, peer_addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                tracing::warn!(cpu = cpu, error = %e, "accept error, continuing");
                continue;
            }
        };

        if !semaphore.try_acquire() {
            tracing::warn!(cpu = cpu, peer = %peer_addr, "max connections reached, dropping");
            drop(stream);
            continue;
        }

        tracing::debug!(cpu = cpu, peer = %peer_addr, "accepted connection");

        let io = stream.into_hyper_io();
        let svc = service.clone();
        let exec = executor.clone();

        executor.spawn(async move {
            let conn = hyper::server::conn::http2::Builder::new(exec).serve_connection(io, svc);

            if let Err(e) = conn.await {
                tracing::debug!(peer = %peer_addr, error = %e, "connection closed");
            }
        });
    }

    Ok(())
}
