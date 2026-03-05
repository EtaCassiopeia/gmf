use std::future::Future;
use std::io;
use std::net::SocketAddr;

use crate::server::error::GmfError;
use crate::server::runtime::{
    Runtime, RuntimeExecutor, RuntimeSemaphore, RuntimeTcpListener, RuntimeTcpStream,
};

/// Thread-per-core runtime using tokio (current-thread mode, one per core).
pub struct TokioRuntime;

impl Runtime for TokioRuntime {
    type TcpListener = TokioListener;
    type Executor = TokioExec;
    type Semaphore = TokioSemaphore;

    fn run_multi_core<F, Fut>(cores: usize, f: F) -> Result<(), GmfError>
    where
        F: Fn(usize) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = Result<(), GmfError>> + 'static,
    {
        let mut handles = Vec::with_capacity(cores);

        for cpu in 0..cores {
            let f = f.clone();
            let handle = std::thread::Builder::new()
                .name(format!("gmf_core_{cpu}"))
                .spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|e| GmfError::SpawnExecutor { cpu, source: e })?;

                    // Pin to CPU core on Linux
                    #[cfg(target_os = "linux")]
                    {
                        let cpuset = libc_cpuset(cpu);
                        unsafe {
                            libc::sched_setaffinity(0, std::mem::size_of_val(&cpuset), &cpuset);
                        }
                    }

                    let local_set = tokio::task::LocalSet::new();
                    rt.block_on(local_set.run_until(f(cpu)))
                })
                .map_err(|e| GmfError::SpawnExecutor { cpu, source: e })?;

            handles.push(handle);
        }

        for handle in handles {
            handle
                .join()
                .map_err(|_| GmfError::Io(io::Error::other("thread panicked")))??;
        }

        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn libc_cpuset(cpu: usize) -> libc::cpu_set_t {
    let mut set = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    unsafe { libc::CPU_SET(cpu, &mut set) };
    set
}

// -- TCP Listener --

pub struct TokioListener(tokio::net::TcpListener);

impl RuntimeTcpListener for TokioListener {
    type Stream = TokioStream;

    async fn bind(addr: SocketAddr) -> io::Result<Self> {
        let socket = socket2::Socket::new(
            match addr {
                SocketAddr::V4(_) => socket2::Domain::IPV4,
                SocketAddr::V6(_) => socket2::Domain::IPV6,
            },
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;
        socket.set_reuse_address(true)?;
        #[cfg(target_os = "linux")]
        {
            socket.set_reuse_port(true)?;
        }
        socket.set_nonblocking(true)?;
        socket.bind(&addr.into())?;
        socket.listen(1024)?;
        let std_listener: std::net::TcpListener = socket.into();
        let listener = tokio::net::TcpListener::from_std(std_listener)?;
        Ok(TokioListener(listener))
    }

    async fn accept(&self) -> io::Result<(Self::Stream, SocketAddr)> {
        let (stream, addr) = self.0.accept().await?;
        Ok((TokioStream(stream), addr))
    }
}

// -- TCP Stream --

pub struct TokioStream(tokio::net::TcpStream);

impl RuntimeTcpStream for TokioStream {
    type HyperIo = hyper_util::rt::TokioIo<tokio::net::TcpStream>;

    fn into_hyper_io(self) -> Self::HyperIo {
        hyper_util::rt::TokioIo::new(self.0)
    }
}

// -- Executor --

#[derive(Clone, Default)]
pub struct TokioExec;

impl RuntimeExecutor for TokioExec {
    fn spawn<F: Future<Output = ()> + 'static>(&self, fut: F) {
        tokio::task::spawn_local(fut);
    }
}

impl<F> hyper::rt::Executor<F> for TokioExec
where
    F: Future + 'static,
    F::Output: 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(async move {
            fut.await;
        });
    }
}

// -- Semaphore --

pub struct TokioSemaphore {
    permits: std::cell::Cell<usize>,
}

impl RuntimeSemaphore for TokioSemaphore {
    fn new(permits: usize) -> Self {
        TokioSemaphore {
            permits: std::cell::Cell::new(permits),
        }
    }

    fn try_acquire(&self) -> bool {
        let current = self.permits.get();
        if current > 0 {
            self.permits.set(current - 1);
            true
        } else {
            false
        }
    }
}

impl TokioSemaphore {
    pub fn release(&self) {
        self.permits.set(self.permits.get() + 1);
    }
}
