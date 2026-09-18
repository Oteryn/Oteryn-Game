mod socket;
pub mod tls;

pub use socket::{
    connect_tcp, connect_uds, BufferedSocket, OwnedBytes, OwnedSocket, Socket, SocketIntoBox,
    SocketIntoOwnedBox, WithSocket, WriteBuffer,
};

pub mod resource_budget;

/// One operation capability established before transport and reused by TLS.
#[derive(Clone)]
pub struct ConnectionOwner {
    budget: std::sync::Arc<dyn resource_budget::ResourceBudget>,
    #[cfg(feature = "_rt-tokio")]
    pub(crate) blocking: crate::rt::resource_owner::BlockingJobOwner,
}

impl ConnectionOwner {
    pub fn try_new(
        budget: std::sync::Arc<dyn resource_budget::ResourceBudget>,
    ) -> crate::Result<Self> {
        #[cfg(feature = "_rt-tokio")]
        {
            let blocking = crate::rt::resource_owner::blocking_job_owner(budget.clone())
                .map_err(|_| crate::Error::Io(std::io::ErrorKind::OutOfMemory.into()))?;
            Ok(Self { budget, blocking })
        }
        #[cfg(not(feature = "_rt-tokio"))]
        {
            drop(budget);
            Err(crate::Error::Io(std::io::ErrorKind::Unsupported.into()))
        }
    }
    pub fn budget(&self) -> std::sync::Arc<dyn resource_budget::ResourceBudget> {
        self.budget.clone()
    }
}

pub async fn connect_tcp_owned<Ws: WithSocket>(
    host: &str,
    port: u16,
    with_socket: Ws,
    owner: &ConnectionOwner,
) -> crate::Result<Ws::Output> {
    #[cfg(feature = "_rt-tokio")]
    if crate::rt::rt_tokio::available() {
        // Preserve ordinary hostname resolution and all returned addresses.
        // DNS allocations are explicitly outside this component's proof (E00).
        let addresses = tokio::net::lookup_host((host, port)).await?;
        let mut last_error = None;
        for address in addresses {
            match tokio::net::TcpStream::connect_addr_oteryn_owned(
                address,
                owner.blocking.runtime_owner(),
            )
            .await
            {
                Ok(socket) => return Ok(with_socket.with_socket(socket).await),
                Err(error) => last_error = Some(error),
            }
        }
        return Err(last_error
            .unwrap_or_else(|| std::io::ErrorKind::AddrNotAvailable.into())
            .into());
    }
    let _ = owner;
    let _ = (host, port, with_socket);
    Err(crate::Error::Io(std::io::ErrorKind::Unsupported.into()))
}

pub async fn connect_uds_owned<P: AsRef<std::path::Path>, Ws: WithSocket>(
    path: P,
    with_socket: Ws,
    owner: &ConnectionOwner,
) -> crate::Result<Ws::Output> {
    #[cfg(all(unix, feature = "_rt-tokio"))]
    if crate::rt::rt_tokio::available() {
        let socket =
            tokio::net::UnixStream::connect_oteryn_owned(path, owner.blocking.runtime_owner())
                .await?;
        return Ok(with_socket.with_socket(socket).await);
    }
    let _ = owner;
    let _ = (path, with_socket);
    Err(crate::Error::Io(std::io::ErrorKind::Unsupported.into()))
}
