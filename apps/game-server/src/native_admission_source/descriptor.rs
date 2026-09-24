use super::SourceError;
use rustls::{
    DigitallySignedStruct, RootCertStore, SignatureScheme,
    client::{
        WebPkiServerVerifier,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
    pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime},
};
use std::sync::Arc;
pub const TRUST_ROOTS_MAX: usize = 4;
pub const TRUST_ROOT_DER_MAX: usize = 4096;
pub const TRUST_ROOT_DER_AGGREGATE_MAX: usize = 16384;
pub const PEER_CERTIFICATES_MAX: usize = 4;
pub const PEER_CERT_DER_MAX: usize = 4096;
pub const PEER_CERT_DER_AGGREGATE_MAX: usize = 16384;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    ReadAccountSecurityV1,
    ReadFreshSigningTrustV1,
    ReadRecoveryAccountSecurityV2,
    ReadRecoverySigningTrustV2,
    /// Separate private Character bootstrap-intent reconciliation read.
    ReadCharacterBootstrapIntentV1,
}
impl Operation {
    pub const fn path(self) -> &'static str {
        match self {
            // The four closed evidence tags share the accepted Platform endpoint.
            // Operation identity remains bound by the existing request/response codec.
            Self::ReadAccountSecurityV1
            | Self::ReadFreshSigningTrustV1
            | Self::ReadRecoveryAccountSecurityV2
            | Self::ReadRecoverySigningTrustV2 => "/internal/v1/game-auth/native-evidence",
            Self::ReadCharacterBootstrapIntentV1 => {
                "/internal/v1/game-auth/character-bootstrap-intents/read"
            }
        }
    }
}
pub struct ProducerDescriptor {
    pub(super) source_authority: String,
    pub(super) connect_addr: std::net::SocketAddr,
    pub(super) server_name: ServerName<'static>,
    pub(super) host_header: String,
    pub(super) tls: Arc<rustls::ClientConfig>,
}
impl ProducerDescriptor {
    /// The configured source authority this descriptor authenticates.
    #[must_use]
    pub fn source_authority(&self) -> &str {
        &self.source_authority
    }

    pub fn new(
        source_authority: String,
        connect_endpoint: (String, u16),
        server_name: String,
        host_header: String,
        roots: Vec<CertificateDer<'static>>,
        client_chain: Vec<CertificateDer<'static>>,
        client_key: PrivateKeyDer<'static>,
    ) -> Result<Self, SourceError> {
        let (connect_host, port) = connect_endpoint;
        if source_authority.is_empty()
            || source_authority.len() > 128
            || !source_authority
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
            || server_name.len() > 128
            || format!("https://{connect_host}:{port}").len() > 256
            || connect_host.is_empty()
            || connect_host.len() > 253
            || host_header != server_name
            || port == 0
            || roots.is_empty()
            || roots.len() > TRUST_ROOTS_MAX
        {
            return Err(SourceError::InvalidDescriptor);
        }
        let connect_addr = std::net::SocketAddr::new(
            connect_host
                .parse::<std::net::IpAddr>()
                .map_err(|_| SourceError::InvalidDescriptor)?,
            port,
        );
        let total = roots
            .iter()
            .try_fold(0usize, |n, c| n.checked_add(c.as_ref().len()))
            .ok_or(SourceError::InvalidDescriptor)?;
        if total > TRUST_ROOT_DER_AGGREGATE_MAX
            || roots.iter().any(|c| c.as_ref().len() > TRUST_ROOT_DER_MAX)
        {
            return Err(SourceError::InvalidDescriptor);
        }
        let mut store = RootCertStore::empty();
        for root in roots {
            store
                .add(root)
                .map_err(|_| SourceError::InvalidDescriptor)?
        }
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let inner = WebPkiServerVerifier::builder_with_provider(Arc::new(store), provider.clone())
            .build()
            .map_err(|_| SourceError::InvalidDescriptor)?;
        let verifier = Arc::new(BoundedServerVerifier { inner });
        let mut tls = rustls::ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| SourceError::InvalidDescriptor)?
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(client_chain, client_key)
            .map_err(|_| SourceError::InvalidDescriptor)?;
        tls.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(Self {
            source_authority,
            connect_addr,
            server_name: ServerName::try_from(server_name)
                .map_err(|_| SourceError::InvalidDescriptor)?,
            host_header,
            tls: Arc::new(tls),
        })
    }
}
#[derive(Debug)]
pub struct BoundedServerVerifier {
    inner: Arc<WebPkiServerVerifier>,
}
impl ServerCertVerifier for BoundedServerVerifier {
    fn verify_server_cert(
        &self,
        end: &CertificateDer<'_>,
        inter: &[CertificateDer<'_>],
        name: &ServerName<'_>,
        ocsp: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        validate_peer_lengths(std::iter::once(end).chain(inter).map(|c| c.as_ref().len()))?;
        self.inner.verify_server_cert(end, inter, name, ocsp, now)
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(m, c, d)
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(m, c, d)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}
fn limit_error() -> rustls::Error {
    rustls::Error::InvalidCertificate(rustls::CertificateError::ApplicationVerificationFailure)
}

fn validate_peer_lengths(lengths: impl IntoIterator<Item = usize>) -> Result<(), rustls::Error> {
    let mut count = 0usize;
    let mut total = 0usize;
    for length in lengths {
        count = count.checked_add(1).ok_or_else(limit_error)?;
        total = total.checked_add(length).ok_or_else(limit_error)?;
        if count > PEER_CERTIFICATES_MAX
            || length > PEER_CERT_DER_MAX
            || total > PEER_CERT_DER_AGGREGATE_MAX
        {
            return Err(limit_error());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn peer_numeric_caps_remain_coupled() {
        assert!(validate_peer_lengths([4096; 4]).is_ok());
        assert!(validate_peer_lengths([4097]).is_err());
        assert!(validate_peer_lengths([4096, 4096, 4096, 4097]).is_err());
        assert!(validate_peer_lengths([1; 5]).is_err());
        assert!(validate_peer_lengths([1, usize::MAX]).is_err());
    }
}
