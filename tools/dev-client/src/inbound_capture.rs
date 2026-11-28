//! Opt-in recording of received TLS plaintext for qualification replay.
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use oteryn_session_tcp::TcpTlsStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Default)]
pub struct InboundCapture(Arc<Mutex<Vec<u8>>>);

impl std::fmt::Debug for InboundCapture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InboundCapture")
            .finish_non_exhaustive()
    }
}

tokio::task_local! {
    static CAPTURE: InboundCapture;
}

impl InboundCapture {
    /// Attach this recorder to a connection opened by the supplied future.
    pub async fn connect<T>(&self, future: impl Future<Output = T>) -> T {
        CAPTURE.scope(self.clone(), future).await
    }

    pub fn bytes(&self) -> io::Result<Vec<u8>> {
        Ok(self
            .0
            .lock()
            .map_err(|_| io::Error::other("qualification inbound capture lock poisoned"))?
            .clone())
    }
}

#[derive(Debug)]
pub struct DevClientStream {
    inner: TcpTlsStream,
    capture: Option<InboundCapture>,
}

impl DevClientStream {
    pub(super) fn new(inner: TcpTlsStream) -> Self {
        Self {
            inner,
            capture: CAPTURE.try_with(Clone::clone).ok(),
        }
    }
}

impl AsyncRead for DevClientStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let start = buf.filled().len();
        let result = Pin::new(&mut this.inner).poll_read(cx, buf);
        if matches!(result, Poll::Ready(Ok(())))
            && let Some(capture) = &this.capture
        {
            let mut bytes = capture
                .0
                .lock()
                .map_err(|_| io::Error::other("qualification inbound capture lock poisoned"))?;
            let added = &buf.filled()[start..];
            if added.len() > MAX_BYTES.saturating_sub(bytes.len()) {
                return Poll::Ready(Err(io::Error::other(
                    "qualification inbound capture exceeded limit",
                )));
            }
            bytes.extend_from_slice(added);
        }
        result
    }
}

impl AsyncWrite for DevClientStream {
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
