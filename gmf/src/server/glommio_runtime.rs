use std::cell::Cell;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::rc::Rc;

use glommio::net::{TcpListener as GlommioTcpListener, TcpStream as GlommioTcpStream};
use glommio::{executor, Latency, LocalExecutorBuilder, Placement, Shares};

use crate::server::error::GmfError;
use crate::server::hyper_io::HyperIo;
use crate::server::runtime::{
    Runtime, RuntimeExecutor, RuntimeSemaphore, RuntimeTcpListener, RuntimeTcpStream,
};

/// Thread-per-core runtime using glommio (io_uring, Linux only).
pub struct GlommioRuntime;

impl Runtime for GlommioRuntime {
    type TcpListener = GlommioListener;
    type Executor = GlommioExec;
    type Semaphore = GlommioSemaphore;

    fn run_multi_core<F, Fut>(cores: usize, f: F) -> Result<(), GmfError>
    where
        F: Fn(usize) -> Fut + Send + Clone + 'static,
        Fut: Future<Output = Result<(), GmfError>> + 'static,
    {
        let mut handles = Vec::with_capacity(cores);

        for cpu in 0..cores {
            let f = f.clone();
            let handle = LocalExecutorBuilder::new(Placement::Fixed(cpu))
                .name(&format!("gmf_core_{cpu}"))
                .spawn(move || async move {
                    let tq = executor().create_task_queue(
                        Shares::default(),
                        Latency::NotImportant,
                        &format!("gmf_tq_{cpu}"),
                    );

                    // Store the task queue in thread-local so the executor can use it
                    TASK_QUEUE.with(|cell| cell.set(Some(tq)));

                    f(cpu).await
                })
                .map_err(|e| GmfError::SpawnExecutor {
                    cpu,
                    source: io::Error::other(e.to_string()),
                })?;

            handles.push(handle);
        }

        for handle in handles {
            handle
                .join()
                .map_err(|e| GmfError::Io(io::Error::other(e.to_string())))?;
        }

        Ok(())
    }
}

thread_local! {
    static TASK_QUEUE: Cell<Option<glommio::TaskQueueHandle>> = const { Cell::new(None) };
}

fn current_task_queue() -> glommio::TaskQueueHandle {
    TASK_QUEUE.with(|cell| {
        cell.get()
            .expect("GlommioExec used outside of GlommioRuntime::run_multi_core")
    })
}

// -- TCP Listener --

pub struct GlommioListener(GlommioTcpListener);

impl RuntimeTcpListener for GlommioListener {
    type Stream = GlommioStream;

    async fn bind(addr: SocketAddr) -> io::Result<Self> {
        let listener = GlommioTcpListener::bind(addr)?;
        Ok(GlommioListener(listener))
    }

    async fn accept(&self) -> io::Result<(Self::Stream, SocketAddr)> {
        let stream = self.0.accept().await?;
        let addr = stream.local_addr()?;
        Ok((GlommioStream(stream), addr))
    }
}

// -- TCP Stream --

pub struct GlommioStream(GlommioTcpStream);

impl RuntimeTcpStream for GlommioStream {
    type HyperIo = HyperIo<GlommioTcpStream>;

    fn into_hyper_io(self) -> Self::HyperIo {
        HyperIo(self.0)
    }
}

// -- Executor --

#[derive(Clone, Default)]
pub struct GlommioExec;

impl RuntimeExecutor for GlommioExec {
    fn spawn<F: Future<Output = ()> + 'static>(&self, fut: F) {
        let tq = current_task_queue();
        match glommio::spawn_local_into(fut, tq) {
            Ok(task) => {
                task.detach();
            }
            Err(e) => {
                tracing::error!("failed to spawn task: {e}");
            }
        }
    }
}

impl<F> hyper::rt::Executor<F> for GlommioExec
where
    F: Future + 'static,
    F::Output: 'static,
{
    fn execute(&self, fut: F) {
        let tq = current_task_queue();
        match glommio::spawn_local_into(
            async move {
                fut.await;
            },
            tq,
        ) {
            Ok(task) => {
                task.detach();
            }
            Err(e) => {
                tracing::error!("failed to spawn hyper task: {e}");
            }
        }
    }
}

// -- Semaphore --

pub struct GlommioSemaphore(Rc<Cell<usize>>);

impl RuntimeSemaphore for GlommioSemaphore {
    fn new(permits: usize) -> Self {
        GlommioSemaphore(Rc::new(Cell::new(permits)))
    }

    fn try_acquire(&self) -> bool {
        let current = self.0.get();
        if current > 0 {
            self.0.set(current - 1);
            true
        } else {
            false
        }
    }
}

impl GlommioSemaphore {
    pub fn release(&self) {
        self.0.set(self.0.get() + 1);
    }
}
