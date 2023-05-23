//! This module contains the GlommioExecutor, which allows you to run futures
//! on Glommio task queues. It implements the Executor trait from hyper, allowing
//! it to be used in any place a hyper Executor is required.

use futures_lite::Future;
use glommio::TaskQueueHandle as TaskQ;
use hyper::rt::Executor;
use log::error;

/// A GlommioExecutor holds a TaskQueueHandle which it uses to spawn futures
/// onto a Glommio task queue.
#[derive(Clone)]
pub(crate) struct GlommioExecutor {
    /// The TaskQueueHandle this executor will spawn tasks onto.
    pub(crate) task_q: TaskQ,
}

impl GlommioExecutor {
    /// Spawn a future onto this executor's task queue.
    ///
    /// This method is intended to be used with `hyper::rt::Executor`, which
    /// requires the `Executor::execute` method, but this method provides more
    /// control over the returned `Task` and any errors that might occur.
    pub(crate) fn spawn<F>(&self, f: F) -> Result<glommio::Task<F::Output>, ()>
    where
        F: Future + 'static,
        F::Output: 'static,
    {
        glommio::spawn_local_into(f, self.task_q).map_err(|spawn_error| {
            error!("Error spawning future: {:?}", spawn_error);
        })
    }
}

impl<F> Executor<F> for GlommioExecutor
where
    F: Future + 'static,
    F::Output: 'static,
{
    /// Executes a future on the executor's task queue.
    ///
    /// This method consumes the future, spawning it on the task queue and
    /// detaching it, meaning that the task will execute to completion
    /// regardless of whether the returned handle is awaited.
    fn execute(&self, f: F) {
        self.spawn(f).map(|task| task.detach()).ok();
    }
}
