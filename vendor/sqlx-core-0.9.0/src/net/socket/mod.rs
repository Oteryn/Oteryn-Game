use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

pub use buffered::{BufferedSocket, OwnedBytes, WriteBuffer};
use bytes::BufMut;
use cfg_if::cfg_if;

use crate::io::ReadBuf;

mod buffered;

pub trait Socket: Send + Sync + Unpin + 'static {
    fn try_read(&mut self, buf: &mut dyn ReadBuf) -> io::Result<usize>;

    fn try_write(&mut self, buf: &[u8]) -> io::Result<usize>;

    fn poll_read_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>;

    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>;

    fn poll_flush(&mut self, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // `flush()` is a no-op for TCP/UDS
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>;

    fn read<'a, B: ReadBuf>(&'a mut self, buf: &'a mut B) -> Read<'a, Self, B>
    where
        Self: Sized,
    {
        Read { socket: self, buf }
    }

    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Write<'a, Self>
    where
        Self: Sized,
    {
        Write { socket: self, buf }
    }

    fn flush(&mut self) -> Flush<'_, Self>
    where
        Self: Sized,
    {
        Flush { socket: self }
    }

    fn shutdown(&mut self) -> Shutdown<'_, Self>
    where
        Self: Sized,
    {
        Shutdown { socket: self }
    }
}

pub struct Read<'a, S: ?Sized, B> {
    socket: &'a mut S,
    buf: &'a mut B,
}

impl<S: ?Sized, B> Future for Read<'_, S, B>
where
    S: Socket,
    B: ReadBuf,
{
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;

        while this.buf.has_remaining_mut() {
            match this.socket.try_read(&mut *this.buf) {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    ready!(this.socket.poll_read_ready(cx))?;
                }
                ready => return Poll::Ready(ready),
            }
        }

        Poll::Ready(Ok(0))
    }
}

pub struct Write<'a, S: ?Sized> {
    socket: &'a mut S,
    buf: &'a [u8],
}

impl<S: ?Sized> Future for Write<'_, S>
where
    S: Socket,
{
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;

        while !this.buf.is_empty() {
            match this.socket.try_write(this.buf) {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    ready!(this.socket.poll_write_ready(cx))?;
                }
                ready => return Poll::Ready(ready),
            }
        }

        Poll::Ready(Ok(0))
    }
}

pub struct Flush<'a, S: ?Sized> {
    socket: &'a mut S,
}

impl<S: Socket + ?Sized> Future for Flush<'_, S> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.socket.poll_flush(cx)
    }
}

pub struct Shutdown<'a, S: ?Sized> {
    socket: &'a mut S,
}

impl<S: ?Sized> Future for Shutdown<'_, S>
where
    S: Socket,
{
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.socket.poll_shutdown(cx)
    }
}

pub trait WithSocket {
    type Output;

    fn with_socket<S: Socket>(self, socket: S) -> impl Future<Output = Self::Output> + Send;
}

pub struct SocketIntoBox;

impl WithSocket for SocketIntoBox {
    type Output = Box<dyn Socket>;

    async fn with_socket<S: Socket>(self, socket: S) -> Self::Output {
        Box::new(socket)
    }
}

/// A final socket box with its debit outside the allocation it owns.
/// Field order destroys the box before returning its reservation.
pub struct OwnedSocket {
    socket: Box<dyn Socket>,
    _reservation: Option<crate::net::resource_budget::ResourceReservation>,
}

impl OwnedSocket {
    pub fn unowned(socket: Box<dyn Socket>) -> Self {
        Self {
            socket,
            _reservation: None,
        }
    }

    pub fn try_new<S: Socket>(
        socket: S,
        budget: std::sync::Arc<dyn crate::net::resource_budget::ResourceBudget>,
    ) -> Result<Self, crate::net::resource_budget::BudgetError> {
        let reservation = crate::net::resource_budget::ResourceReservation::try_new(
            budget,
            std::mem::size_of::<S>(),
        )?;
        Ok(Self {
            socket: Box::new(socket),
            _reservation: Some(reservation),
        })
    }
}

pub struct SocketIntoOwnedBox(pub std::sync::Arc<dyn crate::net::resource_budget::ResourceBudget>);

impl WithSocket for SocketIntoOwnedBox {
    type Output = crate::Result<OwnedSocket>;

    async fn with_socket<S: Socket>(self, socket: S) -> Self::Output {
        OwnedSocket::try_new(socket, self.0)
            .map_err(|_| crate::Error::Io(io::ErrorKind::OutOfMemory.into()))
    }
}

impl Socket for OwnedSocket {
    fn try_read(&mut self, buf: &mut dyn ReadBuf) -> io::Result<usize> {
        self.socket.try_read(buf)
    }

    fn try_write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.socket.try_write(buf)
    }

    fn poll_read_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.socket.poll_read_ready(cx)
    }

    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.socket.poll_write_ready(cx)
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.socket.poll_flush(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.socket.poll_shutdown(cx)
    }
}

impl<S: Socket + ?Sized> Socket for Box<S> {
    fn try_read(&mut self, buf: &mut dyn ReadBuf) -> io::Result<usize> {
        (**self).try_read(buf)
    }

    fn try_write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (**self).try_write(buf)
    }

    fn poll_read_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_read_ready(cx)
    }

    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_write_ready(cx)
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_flush(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_shutdown(cx)
    }
}

pub async fn connect_tcp<Ws: WithSocket>(
    host: &str,
    port: u16,
    with_socket: Ws,
) -> crate::Result<Ws::Output> {
    #[cfg(feature = "_rt-tokio")]
    if crate::rt::rt_tokio::available() {
        return Ok(with_socket
            .with_socket(tokio::net::TcpStream::connect((host, port)).await?)
            .await);
    }

    cfg_if! {
        if #[cfg(feature = "_rt-async-io")] {
            Ok(with_socket.with_socket(connect_tcp_async_io(host, port).await?).await)
        } else {
            crate::rt::missing_rt((host, port, with_socket))
        }
    }
}

/// Open a TCP socket to `host` and `port`.
///
/// If `host` is a hostname, attempt to connect to each address it resolves to.
///
/// This implements the same behavior as [`tokio::net::TcpStream::connect()`].
#[cfg(feature = "_rt-async-io")]
async fn connect_tcp_async_io(host: &str, port: u16) -> crate::Result<impl Socket> {
    use async_io::Async;
    use std::net::{IpAddr, TcpStream, ToSocketAddrs};

    // IPv6 addresses in URLs will be wrapped in brackets and the `url` crate doesn't trim those.
    let host = host.trim_matches(&['[', ']'][..]);

    if let Ok(addr) = host.parse::<IpAddr>() {
        return Ok(Async::<TcpStream>::connect((addr, port)).await?);
    }

    let host = host.to_string();

    let addresses = crate::rt::spawn_blocking(move || {
        let addr = (host.as_str(), port);
        ToSocketAddrs::to_socket_addrs(&addr)
    })
    .await?;

    let mut last_err = None;

    // Loop through all the Socket Addresses that the hostname resolves to
    for socket_addr in addresses {
        match Async::<TcpStream>::connect(socket_addr).await {
            Ok(stream) => return Ok(stream),
            Err(e) => last_err = Some(e),
        }
    }

    // If we reach this point, it means we failed to connect to any of the addresses.
    // Return the last error we encountered, or a custom error if the hostname didn't resolve to any address.
    Err(last_err
        .unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "Hostname did not resolve to any addresses",
            )
        })
        .into())
}

/// Connect a Unix Domain Socket at the given path.
///
/// Returns an error if Unix Domain Sockets are not supported on this platform.
pub async fn connect_uds<P: AsRef<Path>, Ws: WithSocket>(
    path: P,
    with_socket: Ws,
) -> crate::Result<Ws::Output> {
    #[cfg(unix)]
    {
        #[cfg(feature = "_rt-tokio")]
        if crate::rt::rt_tokio::available() {
            use tokio::net::UnixStream;

            let stream = UnixStream::connect(path).await?;

            return Ok(with_socket.with_socket(stream).await);
        }

        cfg_if! {
            if #[cfg(feature = "_rt-async-io")] {
                use async_io::Async;
                use std::os::unix::net::UnixStream;

                let stream = Async::<UnixStream>::connect(path).await?;

                Ok(with_socket.with_socket(stream).await)
            } else {
                crate::rt::missing_rt((path, with_socket))
            }
        }
    }

    #[cfg(not(unix))]
    {
        drop((path, with_socket));

        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Unix domain sockets are not supported on this platform",
        )
        .into())
    }
}

#[cfg(test)]
mod custody_tests {
    use super::*;
    use crate::net::resource_budget::{BudgetError, ResourceBudget};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct Ledger {
        held: AtomicUsize,
        limit: usize,
    }
    impl ResourceBudget for Ledger {
        fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
            if bytes > self.limit {
                return Err(BudgetError::Unavailable);
            }
            self.held.fetch_add(bytes, Ordering::SeqCst);
            Ok(())
        }
        fn release(&self, bytes: usize) {
            assert_eq!(self.held.fetch_sub(bytes, Ordering::SeqCst), bytes);
        }
    }
    struct DropSocket {
        budget: Arc<Ledger>,
    }
    impl Drop for DropSocket {
        fn drop(&mut self) {
            let held = self.budget.held.load(Ordering::SeqCst);
            assert!(held == 0 || held == std::mem::size_of::<Self>());
        }
    }
    impl Socket for DropSocket {
        fn try_read(&mut self, _: &mut dyn ReadBuf) -> io::Result<usize> {
            Ok(0)
        }
        fn try_write(&mut self, _: &[u8]) -> io::Result<usize> {
            Ok(0)
        }
        fn poll_read_ready(&mut self, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_write_ready(&mut self, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(&mut self, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }
    }
    #[test]
    fn final_box_is_precharged_and_stalled_close_keeps_custody() {
        use futures_util::FutureExt;
        let size = std::mem::size_of::<DropSocket>();
        let denied = Arc::new(Ledger {
            held: AtomicUsize::new(0),
            limit: size - 1,
        });
        assert!(OwnedSocket::try_new(
            DropSocket {
                budget: denied.clone()
            },
            denied.clone()
        )
        .is_err());
        assert_eq!(denied.held.load(Ordering::SeqCst), 0);
        let budget = Arc::new(Ledger {
            held: AtomicUsize::new(0),
            limit: size,
        });
        let mut socket = OwnedSocket::try_new(
            DropSocket {
                budget: budget.clone(),
            },
            budget.clone(),
        )
        .unwrap();
        assert!(socket.shutdown().now_or_never().is_none());
        assert_eq!(budget.held.load(Ordering::SeqCst), size);
        drop(socket);
        assert_eq!(budget.held.load(Ordering::SeqCst), 0);
    }
}
