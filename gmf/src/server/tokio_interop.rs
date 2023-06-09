//! This module provides interoperability with the Tokio async runtime.
//! It contains utilities to bridge between futures_lite and Tokio.

use std::io::{self};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_lite::{AsyncRead, AsyncWrite};
use tokio::io::ReadBuf;

/// A wrapper type for AsyncRead + AsyncWrite + Unpin types, providing
/// interoperability with Tokio's AsyncRead and AsyncWrite traits.
#[pin_project::pin_project] // This generates a projection for the inner type.
pub struct TokioIO<T>(#[pin] pub T)
where
    T: AsyncRead + AsyncWrite + Unpin;

impl<T> tokio::io::AsyncWrite for TokioIO<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    /// Write some data into the inner type, returning how many bytes were written.
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        // This is the same as  Pin::new(&mut self.0).poll_write(cx, buf) with the source type of `mut self`
        // using projection makes it easier to read.
        let this = self.project();
        this.0.poll_write(cx, buf)
    }

    /// Flushes the inner type.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        self.project().0.poll_flush(cx)
    }

    /// Shuts down the inner type, flushing any buffered data.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        self.project().0.poll_close(cx)
    }
}

impl<T> tokio::io::AsyncRead for TokioIO<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    /// Reads some data from the inner type, returning how many bytes were read.
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.project()
            .0
            .poll_read(cx, buf.initialize_unfilled())
            .map(|n| {
                if let Ok(n) = n {
                    buf.advance(n);
                }

                Ok(())
            })
    }
}
