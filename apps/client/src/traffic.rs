//! Bytes the game session sent and received, counted on the client's own stream wrapper for
//! the diagnostics overlay. The server and the session crate count nothing.
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Shared by the counting stream and whoever displays it.
#[derive(Debug, Default)]
pub struct Traffic {
    sent: AtomicU64,
    received: AtomicU64,
}
impl Traffic {
    /// `(sent, received)` bytes, TLS records excluded: what the session wrote and read.
    #[must_use]
    pub fn bytes(&self) -> (u64, u64) {
        (self.sent.load(Relaxed), self.received.load(Relaxed))
    }
}

/// A stream that adds every byte it reads or writes to its [`Traffic`].
#[derive(Debug)]
pub struct Counting<S> {
    inner: S,
    traffic: Arc<Traffic>,
}
impl<S> Counting<S> {
    pub fn new(inner: S, traffic: Arc<Traffic>) -> Self {
        Self { inner, traffic }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for Counting<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = result {
            let read = buf.filled().len().saturating_sub(before);
            self.traffic.received.fetch_add(read as u64, Relaxed);
        }
        result
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for Counting<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(written)) = result {
            self.traffic.sent.fetch_add(written as u64, Relaxed);
        }
        result
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn counts_the_exact_bytes_written_and_read_each_way() -> Result<(), Box<dyn std::error::Error>>
    {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let traffic = Arc::new(Traffic::default());
            let (client, mut server) = tokio::io::duplex(64);
            let mut client = Counting::new(client, Arc::clone(&traffic));
            client.write_all(b"hello").await?;
            client.flush().await?;
            let mut received = [0_u8; 5];
            server.read_exact(&mut received).await?;
            assert_eq!(&received, b"hello");
            assert_eq!(traffic.bytes(), (5, 0));
            server.write_all(b"ok!").await?;
            let mut reply = [0_u8; 3];
            client.read_exact(&mut reply).await?;
            assert_eq!(&reply, b"ok!");
            assert_eq!(traffic.bytes(), (5, 3));
            Ok(())
        })
    }
}
