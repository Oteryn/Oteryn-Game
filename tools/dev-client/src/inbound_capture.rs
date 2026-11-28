//! Bounded capture of the original inbound bytes of a qualification transport.
use std::io;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// A single inbound transcript. Overflow invalidates the entire capture.
#[derive(Clone)]
pub struct InboundCapture(Arc<Mutex<Option<Vec<u8>>>>);

impl std::fmt::Debug for InboundCapture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InboundCapture")
            .finish_non_exhaustive()
    }
}

impl InboundCapture {
    /// Copies all captured bytes, or returns `None` after overflow or a failed capture lock.
    pub fn bytes(&self) -> Option<Vec<u8>> {
        self.0.lock().ok()?.clone()
    }
}

/// A transport that passes through writes and records reads without changing their boundaries.
#[derive(Debug)]
pub struct RecordingTransport<T> {
    inner: T,
    capture: InboundCapture,
    limit: NonZeroUsize,
}

impl<T> RecordingTransport<T> {
    pub fn new(inner: T, limit: NonZeroUsize) -> (Self, InboundCapture) {
        let capture = InboundCapture(Arc::new(Mutex::new(Some(Vec::new()))));
        (
            Self {
                inner,
                capture: capture.clone(),
                limit,
            },
            capture,
        )
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for RecordingTransport<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let previously_filled = buf.filled().len();
        let result = Pin::new(&mut this.inner).poll_read(cx, buf);
        if matches!(result, Poll::Ready(Ok(())))
            && let Ok(mut capture) = this.capture.0.lock()
            && let Some(bytes) = capture.as_mut()
        {
            let received = &buf.filled()[previously_filled..];
            if received.len() <= this.limit.get() - bytes.len() {
                bytes.extend_from_slice(received);
            } else {
                *capture = None;
            }
        }
        result
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for RecordingTransport<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, bytes)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn retains_original_reads_and_invalidates_overflow() -> io::Result<()> {
        tokio::runtime::Builder::new_current_thread()
            .build()?
            .block_on(check_recording())
    }

    async fn check_recording() -> io::Result<()> {
        let (client, mut peer) = tokio::io::duplex(64);
        let (mut transport, capture) =
            RecordingTransport::new(client, NonZeroUsize::MIN.saturating_add(2));
        transport.write_all(b"request").await?;
        let mut request = [0; 7];
        peer.read_exact(&mut request).await?;
        assert_eq!(&request, b"request");

        peer.write_all(&[0, 255, 128, 1]).await?;
        let mut response = [0; 4];
        transport.read_exact(&mut response[..2]).await?;
        transport.read_exact(&mut response[2..3]).await?;
        assert_eq!(capture.bytes().as_deref(), Some(&[0, 255, 128][..]));
        transport.read_exact(&mut response[3..]).await?;
        assert_eq!(response, [0, 255, 128, 1]);
        assert!(capture.bytes().is_none());
        Ok(())
    }
}
