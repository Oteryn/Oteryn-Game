//! Opt-in recording of inbound bytes for qualification. Outbound admission data is never recorded.
use std::cell::RefCell;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use oteryn_session_tcp::TcpTlsStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const MAX_CAPTURE_BYTES: usize = 256 * 1024;

tokio::task_local! {
    static INBOUND: RefCell<Vec<u8>>;
}

/// Record the bytes read by one qualification operation on its current task.
pub async fn capture_qualification_inbound<T>(operation: impl Future<Output = T>) -> (T, Vec<u8>) {
    INBOUND
        .scope(RefCell::new(Vec::new()), async {
            let result = operation.await;
            (result, INBOUND.with(|bytes| bytes.take()))
        })
        .await
}

/// The dev-client TLS stream with optional qualification capture.
#[derive(Debug)]
pub struct QualificationTransport(pub(super) TcpTlsStream);

impl AsyncRead for QualificationTransport {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let start = buffer.filled().len();
        let result = Pin::new(&mut self.0).poll_read(cx, buffer);
        if matches!(result, Poll::Ready(Ok(())))
            && let Ok(false) = INBOUND.try_with(|recorded| {
                let mut recorded = recorded.borrow_mut();
                let received = &buffer.filled()[start..];
                if received.len() > MAX_CAPTURE_BYTES - recorded.len() {
                    return false;
                }
                recorded.extend_from_slice(received);
                true
            })
        {
            return Poll::Ready(Err(io::Error::other(
                "qualification capture exceeded its byte limit",
            )));
        }
        result
    }
}

impl AsyncWrite for QualificationTransport {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, bytes)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}
