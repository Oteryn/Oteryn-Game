//! TLS 1.3 over TCP adapter for the Oteryn session stream boundary (ADR-0020 section 1, lane N1;
//! ADR-0014 layering `protocol-oteryn -> session -> TCP adapter`).
//!
//! [`connect`] opens the TCP connection, completes a rustls TLS 1.3 handshake against one pinned
//! trust root with ALPN `oteryn-game/1`, and rejects any other or absent negotiated ALPN before a
//! single Foundation frame is sent (FND-02: an ALPN mismatch terminates the connection). The
//! returned [`TcpTlsStream`] is the byte stream an `oteryn_session::Session` admits over; this
//! crate holds no admission, envelope or codec logic. A later QUIC adapter sits beside it and
//! produces its own `SessionStream` without changing the session crate.

use oteryn_session::{ALPN_OTERYN_GAME_V1, SessionStream};
use rustls::pki_types::{CertificateDer, ServerName};
use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

/// The established, ALPN-verified TLS 1.3 stream over TCP.
pub type TcpTlsStream = TlsStream<TcpStream>;

// The adapter's stream implements the session crate's stream boundary.
const _: fn() = || {
    fn assert_session_stream<T: SessionStream>() {}
    assert_session_stream::<TcpTlsStream>();
};

/// Everything [`connect`] needs to open one verified connection.
#[derive(Debug, Clone, Copy)]
pub struct TcpConnect<'a> {
    /// The server's real loopback (or routable) TCP address.
    pub address: SocketAddr,
    /// The TLS server name to verify the presented certificate against.
    pub server_name: &'a str,
    /// The single trust root the connection's certificate must chain to.
    pub root_certificate: &'a CertificateDer<'static>,
    /// Bounds the TCP connect and the TLS handshake, each on its own. A stalled peer fails with
    /// `TcpAdapterError::Timeout` naming the stage, rather than hanging.
    pub deadline: Duration,
}

#[derive(Debug)]
pub enum TcpAdapterError {
    Tls(rustls::Error),
    Io(io::Error),
    InvalidServerName,
    /// The negotiated TLS ALPN protocol was not exactly `oteryn-game/1`, including when no ALPN
    /// was negotiated at all (FND-02: an ALPN mismatch terminates the connection).
    AlpnMismatch,
    /// The TCP connect or TLS handshake did not complete within `TcpConnect::deadline`.
    Timeout(&'static str),
}

impl fmt::Display for TcpAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tls(error) => write!(formatter, "TLS setup failed: {error}"),
            Self::Io(error) => write!(formatter, "transport I/O failed: {error}"),
            Self::InvalidServerName => write!(formatter, "invalid TLS server name"),
            Self::AlpnMismatch => write!(
                formatter,
                "TLS ALPN mismatch: server did not negotiate oteryn-game/1"
            ),
            Self::Timeout(stage) => write!(formatter, "timed out waiting for {stage}"),
        }
    }
}

impl StdError for TcpAdapterError {}

impl From<io::Error> for TcpAdapterError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Connects to `request.address` over rustls TLS 1.3 with ALPN `oteryn-game/1`, rejecting any
/// other or absent negotiated ALPN before returning the stream (so before anything is sent). The
/// TCP connect and the TLS handshake are each bounded by `request.deadline`.
pub async fn connect(request: TcpConnect<'_>) -> Result<TcpTlsStream, TcpAdapterError> {
    let connector = tls_connector(request.root_certificate)?;
    let tcp = bounded(
        request.deadline,
        "TCP connect",
        TcpStream::connect(request.address),
    )
    .await?;
    let server_name = ServerName::try_from(request.server_name.to_owned())
        .map_err(|_error| TcpAdapterError::InvalidServerName)?;
    let stream = bounded(
        request.deadline,
        "TLS handshake",
        connector.connect(server_name, tcp),
    )
    .await?;
    // FND-02: an ALPN mismatch (including no ALPN negotiated at all) terminates the connection
    // before any Foundation frame is sent, exactly like the seam qualification's own transport
    // negatives (`wrong_alpn`/`missing_alpn`) expect of the server's own ALPN enforcement.
    if stream.get_ref().1.alpn_protocol() != Some(ALPN_OTERYN_GAME_V1.as_bytes()) {
        return Err(TcpAdapterError::AlpnMismatch);
    }
    Ok(stream)
}

/// Bounds `future` by `deadline`, mapping an elapsed deadline to `TcpAdapterError::Timeout(label)`.
async fn bounded<T, F>(
    deadline: Duration,
    label: &'static str,
    future: F,
) -> Result<T, TcpAdapterError>
where
    F: Future<Output = Result<T, io::Error>>,
{
    match tokio::time::timeout(deadline, future).await {
        Ok(result) => result.map_err(TcpAdapterError::from),
        Err(_elapsed) => Err(TcpAdapterError::Timeout(label)),
    }
}

fn tls_connector(root: &CertificateDer<'static>) -> Result<TlsConnector, TcpAdapterError> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root.clone()).map_err(TcpAdapterError::Tls)?;
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .map_err(TcpAdapterError::Tls)?
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    Ok(TlsConnector::from(Arc::new(config)))
}
