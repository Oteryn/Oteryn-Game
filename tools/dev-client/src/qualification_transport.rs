//! Opt-in recording of a single admitted qualification exchange.
use std::cell::RefCell;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use oteryn_session_tcp::TcpTlsStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const MAX_CAPTURE_BYTES: usize = 256 * 1024;

tokio::task_local! {
    static EXCHANGE: RefCell<QualificationExchange>;
}

/// TLS plaintext read and written within the capture scope.
#[derive(Default)]
pub struct QualificationExchange {
    pub inbound: Vec<u8>,
    pub outbound: Vec<u8>,
}

/// Record one operation after admission on the current task.
pub async fn capture_qualification_exchange<T>(
    operation: impl Future<Output = T>,
) -> (T, QualificationExchange) {
    EXCHANGE
        .scope(RefCell::default(), async {
            let result = operation.await;
            (result, EXCHANGE.with(RefCell::take))
        })
        .await
}

fn record(buffer: &mut Vec<u8>, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() > MAX_CAPTURE_BYTES - buffer.len() {
        return Err(io::Error::other(
            "qualification capture exceeded its byte limit",
        ));
    }
    buffer.extend_from_slice(bytes);
    Ok(())
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
            && let Ok(Err(error)) = EXCHANGE.try_with(|exchange| {
                record(
                    &mut exchange.borrow_mut().inbound,
                    &buffer.filled()[start..],
                )
            })
        {
            return Poll::Ready(Err(error));
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
        let result = Pin::new(&mut self.0).poll_write(cx, bytes);
        if let Poll::Ready(Ok(written)) = result
            && let Ok(Err(error)) = EXCHANGE
                .try_with(|exchange| record(&mut exchange.borrow_mut().outbound, &bytes[..written]))
        {
            return Poll::Ready(Err(error));
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}
