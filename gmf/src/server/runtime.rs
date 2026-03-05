use std::future::Future;
use std::io;
use std::net::SocketAddr;

use crate::server::error::GmfError;

/// Core trait for a thread-per-core async runtime.
pub trait Runtime: Sized + 'static {
    type TcpListener: RuntimeTcpListener;
    type Executor: RuntimeExecutor + Clone;
    type Semaphore: RuntimeSemaphore;

    /// Spawn one event loop per core, each running the provided closure.
    /// The closure receives the core index (0-based).
    fn run_multi_core<F, Fut>(cores: usize, f: F) -> Result<(), GmfError>
    where
        F: Fn(usize) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = Result<(), GmfError>> + 'static;
}

/// Async TCP listener bound to a socket address.
pub trait RuntimeTcpListener: Sized {
    type Stream: RuntimeTcpStream;

    fn bind(addr: SocketAddr) -> impl Future<Output = io::Result<Self>>;
    fn accept(&self) -> impl Future<Output = io::Result<(Self::Stream, SocketAddr)>>;
}

/// A TCP stream that can be converted into a hyper-compatible IO type.
pub trait RuntimeTcpStream: Sized + 'static {
    type HyperIo: hyper::rt::Read + hyper::rt::Write + Unpin + 'static;

    fn into_hyper_io(self) -> Self::HyperIo;
}

/// Executor for spawning futures within the current thread's event loop.
pub trait RuntimeExecutor: Clone + Default + 'static {
    fn spawn<F: Future<Output = ()> + 'static>(&self, fut: F);
}

/// A single-threaded semaphore for connection limiting.
pub trait RuntimeSemaphore: Sized {
    fn new(permits: usize) -> Self;
    fn try_acquire(&self) -> bool;
}
