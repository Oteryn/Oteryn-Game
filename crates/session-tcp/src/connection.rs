//! Routed endpoint resolution, address fallback and one session admission.
use crate::{TcpConnect, TcpTlsStream, connect};
use oteryn_session::{Admission, Session, SessionError};
use rustls::pki_types::CertificateDer;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

/// Bounds endpoint lookup, TCP connect, TLS handshake and each admission read.
pub const CONNECT_DEADLINE: Duration = Duration::from_secs(10);

/// One connect and admission. `Err(Some(code))` is an admission refusal; `Err(None)` is any
/// other failure (lookup, connect, TLS, I/O, close, timeout), which is never retried.
pub async fn admit(
    host: &str,
    port: u16,
    server_name: &str,
    admission: Admission<'_>,
    root: &CertificateDer<'static>,
) -> Result<Session<TcpTlsStream>, Option<u32>> {
    let addresses = resolve(host, port).await;
    let stream = connect_any(&addresses, server_name, root)
        .await
        .ok_or(None)?;
    Session::admit(stream, admission)
        .await
        .map_err(|error| match error {
            SessionError::AdmissionRefused { code } => Some(code),
            _ => None,
        })
}

/// Every address of the routed endpoint, in resolver order; empty when the lookup fails.
async fn resolve(host: &str, port: u16) -> Vec<SocketAddr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return vec![SocketAddr::new(ip, port)];
    }
    match tokio::time::timeout(CONNECT_DEADLINE, tokio::net::lookup_host((host, port))).await {
        Ok(Ok(addresses)) => addresses.collect(),
        _ => Vec::new(),
    }
}

/// The first address that completes TCP connect and the TLS handshake; later addresses are
/// tried when an earlier one fails.
pub async fn connect_any(
    addresses: &[SocketAddr],
    server_name: &str,
    root: &CertificateDer<'static>,
) -> Option<TcpTlsStream> {
    for &address in addresses {
        if let Ok(stream) = connect(TcpConnect {
            address,
            server_name,
            root_certificate: root,
            deadline: CONNECT_DEADLINE,
        })
        .await
        {
            return Some(stream);
        }
    }
    None
}
