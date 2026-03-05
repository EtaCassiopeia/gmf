use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Bridges `futures_lite::{AsyncRead, AsyncWrite}` to `hyper::rt::{Read, Write}`.
///
/// Used by the glommio runtime where streams implement futures_lite traits
/// but hyper 1.x requires its own Read/Write traits.
#[cfg(feature = "glommio-runtime")]
#[pin_project::pin_project]
pub struct HyperIo<T>(#[pin] pub T);

#[cfg(feature = "glommio-runtime")]
impl<T> hyper::rt::Write for HyperIo<T>
where
    T: futures_lite::AsyncWrite + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.project().0.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().0.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().0.poll_close(cx)
    }
}

#[cfg(feature = "glommio-runtime")]
impl<T> hyper::rt::Read for HyperIo<T>
where
    T: futures_lite::AsyncRead + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        // SAFETY: We write into the uninitialized portion and only advance by bytes written.
        let slice = unsafe {
            let unfilled = buf.as_mut();
            std::slice::from_raw_parts_mut(unfilled.as_mut_ptr() as *mut u8, unfilled.len())
        };
        match self.project().0.poll_read(cx, slice) {
            Poll::Ready(Ok(n)) => {
                unsafe { buf.advance(n) };
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}
