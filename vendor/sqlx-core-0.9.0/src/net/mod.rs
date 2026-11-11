mod socket;
pub mod tls;

pub use socket::{
    connect_tcp, connect_uds, BudgetError, BufferedSocket, ResourceBudget, ResourceReservation,
    Socket, SocketIntoBox, WithSocket, WriteBuffer,
};

/// Oteryn: turn Nagle's algorithm off on a TCP socket, as libpq does; other sockets are left as
/// they are.
///
/// With Nagle on, the last TLS record of a message larger than one record (16 KiB) waited for the
/// server's delayed ACK, about 40 ms, while the single durability holder stayed checked out (RC5).
/// A plain synchronous call, so a driver can make it without adding a future to the connect path.
pub fn set_tcp_nodelay<S: Socket>(socket: &S) -> std::io::Result<()> {
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
    // RC5: a TCP socket the driver connects is switched to TCP_NODELAY.
    #[test]
    fn tcp_sockets_send_without_nagle_delay() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()?;
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let stream = tokio::net::TcpStream::connect(listener.local_addr()?).await?;
            assert!(!stream.nodelay()?);
            super::set_tcp_nodelay(&stream)?;
            assert!(stream.nodelay()?);
            Ok(())
        })
    }
}
