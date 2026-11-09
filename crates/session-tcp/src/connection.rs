//! Routed endpoint resolution, address fallback and one session admission.
use crate::{TcpConnect, TcpTlsStream, connect_with_nodelay};
use oteryn_session::{Admission, AdmissionQueue, Session, SessionError};
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
    admit_detailed(host, port, server_name, admission, root)
        .await
        .map_err(|refusal| refusal.map(|refusal| refusal.code))
}

/// Validated server refusal; queue metadata never grants admission or extends a grant.
#[derive(Debug, Clone, Copy)]
pub struct AdmissionRefusalInfo {
    pub code: u32,
    pub queue: Option<AdmissionQueue>,
}

/// One real TCP/TLS admission preserving authenticated capacity wait metadata.
pub async fn admit_detailed(
    host: &str,
    port: u16,
    server_name: &str,
    admission: Admission<'_>,
    root: &CertificateDer<'static>,
) -> Result<Session<TcpTlsStream>, Option<AdmissionRefusalInfo>> {
    admit_detailed_optimized(host, port, server_name, admission, root, false).await
}

/// Admission with optional low-latency TCP delivery; authorization and retry rules are unchanged.
pub async fn admit_optimized(
    host: &str,
    port: u16,
    server_name: &str,
    admission: Admission<'_>,
    root: &CertificateDer<'static>,
    optimized: bool,
) -> Result<Session<TcpTlsStream>, Option<u32>> {
    admit_detailed_optimized(host, port, server_name, admission, root, optimized)
        .await
        .map_err(|refusal| refusal.map(|refusal| refusal.code))
}

/// Preserves both authenticated queue metadata and the caller's TCP delivery preference.
pub async fn admit_detailed_optimized(
    host: &str,
    port: u16,
    server_name: &str,
    admission: Admission<'_>,
    root: &CertificateDer<'static>,
    optimized: bool,
) -> Result<Session<TcpTlsStream>, Option<AdmissionRefusalInfo>> {
    let addresses = resolve(host, port).await;
    let stream = connect_any_optimized(&addresses, server_name, root, optimized)
        .await
        .ok_or(None)?;
    Session::admit(stream, admission)
        .await
        .map_err(|error| match error {
            SessionError::AdmissionRefused { code } => {
                Some(AdmissionRefusalInfo { code, queue: None })
            }
            SessionError::AdmissionQueued { code, queue } => Some(AdmissionRefusalInfo {
                code,
                queue: Some(queue),
            }),
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
    connect_any_optimized(addresses, server_name, root, false).await
}

async fn connect_any_optimized(
    addresses: &[SocketAddr],
    server_name: &str,
    root: &CertificateDer<'static>,
    optimized: bool,
) -> Option<TcpTlsStream> {
    for &address in addresses {
        if let Ok(stream) = connect_with_nodelay(
            TcpConnect {
                address,
                server_name,
                root_certificate: root,
                deadline: CONNECT_DEADLINE,
            },
            optimized,
        )
        .await
        {
            return Some(stream);
        }
    }
    None
}
