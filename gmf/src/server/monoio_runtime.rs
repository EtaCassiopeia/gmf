use std::cell::Cell;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::rc::Rc;

use monoio::net::{TcpListener as MonoioTcpListener, TcpStream as MonoioTcpStream};

use crate::server::error::GmfError;
use crate::server::runtime::{
    Runtime, RuntimeExecutor, RuntimeSemaphore, RuntimeTcpListener, RuntimeTcpStream,
};

/// Thread-per-core runtime using monoio (io_uring on Linux, kqueue on macOS).
pub struct MonoioRuntime;

impl Runtime for MonoioRuntime {
    type TcpListener = MonoioListener;
    type Executor = MonoioExec;
    type Semaphore = MonoioSemaphore;

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
                .spawn(move || -> Result<(), GmfError> {
                    // Pin to CPU core on Linux for optimal thread-per-core performance.
                    #[cfg(target_os = "linux")]
                    {
                        unsafe {
                            let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
                            libc::CPU_SET(cpu, &mut cpuset);
                            libc::sched_setaffinity(
                                0,
                                std::mem::size_of::<libc::cpu_set_t>(),
                                &cpuset,
                            );
                        }
                    }

                    let mut rt = monoio::RuntimeBuilder::<monoio::FusionDriver>::new()
                        .enable_timer()
                        .build()
                        .map_err(|e| GmfError::SpawnExecutor { cpu, source: e })?;

                    rt.block_on(f(cpu))
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

// -- TCP Listener --

pub struct MonoioListener(MonoioTcpListener);

impl RuntimeTcpListener for MonoioListener {
    type Stream = MonoioStream;

    async fn bind(addr: SocketAddr) -> io::Result<Self> {
        let listener = MonoioTcpListener::bind(addr)?;
        Ok(MonoioListener(listener))
    }

    async fn accept(&self) -> io::Result<(Self::Stream, SocketAddr)> {
        let (stream, addr) = self.0.accept().await?;
        Ok((MonoioStream(stream), addr))
    }
}

// -- TCP Stream --

pub struct MonoioStream(MonoioTcpStream);

impl RuntimeTcpStream for MonoioStream {
    // monoio TcpStream is completion-based; wrap with StreamWrapper for poll-based compat,
    // then MonoioIo for hyper::rt::{Read, Write}.
    type HyperIo = monoio_compat::hyper::MonoioIo<monoio_compat::StreamWrapper<MonoioTcpStream>>;

    fn into_hyper_io(self) -> Self::HyperIo {
        let compat = monoio_compat::StreamWrapper::new(self.0);
        monoio_compat::hyper::MonoioIo::new(compat)
    }
}

// -- Executor --

#[derive(Clone, Default)]
pub struct MonoioExec;

impl RuntimeExecutor for MonoioExec {
    fn spawn<F: Future<Output = ()> + 'static>(&self, fut: F) {
        monoio::spawn(fut);
    }
}

impl<F> hyper::rt::Executor<F> for MonoioExec
where
    F: Future + 'static,
    F::Output: 'static,
{
    fn execute(&self, fut: F) {
        monoio::spawn(async move {
            fut.await;
        });
    }
}

// -- Semaphore --

pub struct MonoioSemaphore {
    permits: Rc<Cell<usize>>,
}

impl RuntimeSemaphore for MonoioSemaphore {
    fn new(permits: usize) -> Self {
        MonoioSemaphore {
            permits: Rc::new(Cell::new(permits)),
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
