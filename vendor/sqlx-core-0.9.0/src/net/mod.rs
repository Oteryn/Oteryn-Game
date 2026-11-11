mod socket;
pub mod tls;

pub use socket::{
    connect_uds, BudgetError, BufferedSocket, ResourceBudget, ResourceReservation, Socket,
    SocketIntoBox, WithSocket, WriteBuffer,
};

use std::future::Future;

/// Oteryn: [`socket::connect_tcp`] with Nagle's algorithm off, as libpq does.
///
/// With Nagle on, the last TLS record of a message larger than one record (16 KiB) waited for the
/// server's delayed ACK, about 40 ms, while the single durability holder stayed checked out (RC5).
pub async fn connect_tcp<Ws: WithSocket>(
    host: &str,
    port: u16,
    with_socket: Ws,
) -> crate::Result<Ws::Output> {
    socket::connect_tcp(host, port, NoDelay(with_socket)).await?
}

struct NoDelay<Ws>(Ws);

impl<Ws: WithSocket> WithSocket for NoDelay<Ws> {
    type Output = crate::Result<Ws::Output>;

    fn with_socket<S: Socket>(self, socket: S) -> impl Future<Output = Self::Output> + Send {
        let nodelay = set_nodelay(&socket);
        let inner = self.0.with_socket(socket);
        async move {
            nodelay?;
            Ok(inner.await)
        }
    }
}

fn set_nodelay<S: Socket>(socket: &S) -> std::io::Result<()> {
    let socket: &dyn std::any::Any = socket;
    #[cfg(feature = "_rt-tokio")]
    if let Some(stream) = socket.downcast_ref::<tokio::net::TcpStream>() {
        return stream.set_nodelay(true);
    }
    #[cfg(feature = "_rt-async-io")]
    if let Some(stream) = socket.downcast_ref::<async_io::Async<std::net::TcpStream>>() {
        return stream.get_ref().set_nodelay(true);
    }
    Ok(())
}

#[cfg(all(test, feature = "_rt-tokio"))]
mod nodelay_tests {
    use super::{Socket, WithSocket};
    use std::future::Future;

    struct NodelayProbe;

    impl WithSocket for NodelayProbe {
        type Output = Option<bool>;

        fn with_socket<S: Socket>(self, socket: S) -> impl Future<Output = Self::Output> + Send {
            let socket: &dyn std::any::Any = &socket;
            let nodelay = socket
                .downcast_ref::<tokio::net::TcpStream>()
                .and_then(|stream| stream.nodelay().ok());
            async move { nodelay }
        }
    }

    // RC5: every TCP connection the driver opens has TCP_NODELAY set.
    #[test]
    fn tcp_connections_send_without_nagle_delay() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()?;
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let port = listener.local_addr()?.port();
            let nodelay = super::connect_tcp("127.0.0.1", port, NodelayProbe).await?;
            assert_eq!(nodelay, Some(true));
            Ok(())
        })
    }
}
