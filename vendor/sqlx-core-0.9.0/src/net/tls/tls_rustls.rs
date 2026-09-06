use std::future;
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::task::{ready, Context, Poll};

use rustls::{
    client::{
        danger::{ServerCertVerified, ServerCertVerifier},
        WebPkiServerVerifier,
    },
    crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider},
    pki_types::{
        pem::{self, PemObject},
        CertificateDer, PrivateKeyDer, ServerName, UnixTime,
    },
    CertificateError, ClientConfig, ClientConnection, Error as TlsError, RootCertStore,
};

use crate::error::Error;
use crate::io::ReadBuf;
use crate::net::tls::util::StdSocket;
use crate::net::tls::TlsConfig;
use crate::net::Socket;

pub struct RustlsSocket<S: Socket> {
    inner: StdSocket<S>,
    state: ClientConnection,
    close_notify_sent: bool,
}

impl<S: Socket> RustlsSocket<S> {
    fn poll_complete_io(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            match self.state.complete_io(&mut self.inner) {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    ready!(self.inner.poll_ready(cx))?;
                }
                ready => return Poll::Ready(ready.map(|_| ())),
            }
        }
    }

    async fn complete_io(&mut self) -> io::Result<()> {
        future::poll_fn(|cx| self.poll_complete_io(cx)).await
    }
}

impl<S: Socket> Socket for RustlsSocket<S> {
    fn try_read(&mut self, buf: &mut dyn ReadBuf) -> io::Result<usize> {
        self.state.reader().read(buf.init_mut())
    }

    fn try_write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.state.writer().write(buf) {
            // Returns a zero-length write when the buffer is full.
            Ok(0) => Err(io::ErrorKind::WouldBlock.into()),
            other => other,
        }
    }

    fn poll_read_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_complete_io(cx)
    }

    fn poll_write_ready(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_complete_io(cx)
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_complete_io(cx)
    }

    fn poll_shutdown(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if !self.close_notify_sent {
            self.state.send_close_notify();
            self.close_notify_sent = true;
        }

        ready!(self.poll_complete_io(cx))?;

        // Server can close socket as soon as it receives the connection shutdown request.
        // We shouldn't expect it to stick around for the TLS session to close cleanly.
        // https://security.stackexchange.com/a/82034
        let _ = ready!(self.inner.socket.poll_shutdown(cx));

        Poll::Ready(Ok(()))
    }
}

pub async fn handshake<S>(socket: S, tls_config: TlsConfig<'_>) -> Result<RustlsSocket<S>, Error>
where
    S: Socket,
{
    #[cfg(all(
        feature = "_tls-rustls-aws-lc-rs",
        not(feature = "_tls-rustls-ring-webpki"),
        not(feature = "_tls-rustls-ring-native-roots")
    ))]
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    #[cfg(any(
        feature = "_tls-rustls-ring-webpki",
        feature = "_tls-rustls-ring-native-roots"
    ))]
    let provider = Arc::new(rustls::crypto::ring::default_provider());

    // Unwrapping is safe here because we use a default provider.
    let config = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .unwrap();

    // authentication using user's key and its associated certificate
    let user_auth = match (tls_config.client_cert_path, tls_config.client_key_path) {
        (Some(cert_path), Some(key_path)) => {
            let cert_chain = certs_from_pem(cert_path.data().await?)?;
            let key_der = private_key_from_pem(key_path.data().await?)?;
            Some((cert_chain, key_der))
        }
        (None, None) => None,
        (_, _) => {
            return Err(Error::Configuration(
                "user auth key and certs must be given together".into(),
            ))
        }
    };

    let config = if tls_config.accept_invalid_certs {
        if let Some(user_auth) = user_auth {
            config
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(DummyTlsVerifier { provider }))
                .with_client_auth_cert(user_auth.0, user_auth.1)
                .map_err(Error::tls)?
        } else {
            config
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(DummyTlsVerifier { provider }))
                .with_no_client_auth()
        }
    } else {
        let mut cert_store = import_root_certs();

        if let Some(ca) = tls_config.root_cert_path {
            let data = ca.data().await?;

            for result in CertificateDer::pem_slice_iter(&data) {
                let Ok(cert) = result else {
                    return Err(Error::Tls(format!("Invalid certificate {ca}").into()));
                };

                cert_store.add(cert).map_err(|err| Error::Tls(err.into()))?;
            }
        }

        if tls_config.accept_invalid_hostnames {
            let verifier = WebPkiServerVerifier::builder(Arc::new(cert_store))
                .build()
                .map_err(|err| Error::Tls(err.into()))?;

            if let Some(user_auth) = user_auth {
                config
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoHostnameTlsVerifier { verifier }))
                    .with_client_auth_cert(user_auth.0, user_auth.1)
                    .map_err(Error::tls)?
            } else {
                config
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoHostnameTlsVerifier { verifier }))
                    .with_no_client_auth()
            }
        } else if let Some(user_auth) = user_auth {
            config
                .with_root_certificates(cert_store)
                .with_client_auth_cert(user_auth.0, user_auth.1)
                .map_err(Error::tls)?
        } else {
            config
                .with_root_certificates(cert_store)
                .with_no_client_auth()
        }
    };

    let host = ServerName::try_from(tls_config.hostname.to_owned()).map_err(Error::tls)?;

    let mut socket = RustlsSocket {
        inner: StdSocket::new(socket),
        state: ClientConnection::new(Arc::new(config), host).map_err(Error::tls)?,
        close_notify_sent: false,
    };

    // Performs the TLS handshake or bails
    socket.complete_io().await?;

    Ok(socket)
}

fn certs_from_pem(pem: Vec<u8>) -> Result<Vec<CertificateDer<'static>>, Error> {
    CertificateDer::pem_slice_iter(&pem)
        .map(|result| result.map_err(|err| Error::Tls(err.into())))
        .collect()
}

fn private_key_from_pem(pem: Vec<u8>) -> Result<PrivateKeyDer<'static>, Error> {
    match PrivateKeyDer::from_pem_slice(&pem) {
        Ok(key) => Ok(key),
        Err(pem::Error::NoItemsFound) => Err(Error::Configuration("no keys found pem file".into())),
        Err(e) => Err(Error::Configuration(e.to_string().into())),
    }
}

#[cfg(all(feature = "webpki-roots", not(feature = "rustls-native-certs")))]
fn import_root_certs() -> RootCertStore {
    RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned())
}

#[cfg(feature = "rustls-native-certs")]
fn import_root_certs() -> RootCertStore {
    let mut root_cert_store = RootCertStore::empty();

    let load_results = rustls_native_certs::load_native_certs();
    for e in load_results.errors {
        log::warn!("Error loading native certificates: {e:?}");
    }
    for cert in load_results.certs {
        if let Err(e) = root_cert_store.add(cert) {
            log::warn!("rustls failed to parse native certificate: {e:?}");
        }
    }

    root_cert_store
}

// Not currently used but allows for a "tls-rustls-no-roots" feature.
#[cfg(not(any(feature = "rustls-native-certs", feature = "webpki-roots")))]
fn import_root_certs() -> RootCertStore {
    RootCertStore::empty()
}

#[derive(Debug)]
struct DummyTlsVerifier {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for DummyTlsVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, TlsError> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, TlsError> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[derive(Debug)]
pub struct NoHostnameTlsVerifier {
    verifier: Arc<WebPkiServerVerifier>,
}

impl ServerCertVerifier for NoHostnameTlsVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        match self.verifier.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        ) {
            Err(TlsError::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. },
            )) => Ok(ServerCertVerified::assertion()),
            res => res,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, TlsError> {
        self.verifier.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, TlsError> {
        self.verifier.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.verifier.supported_verify_schemes()
    }
}

/// Requested heap capacity during one pre-state handshake decode, including
/// moving Vec growth. This is only a phase bound: configuration, input buffers,
/// fragment spans, crypto, retained certificates and returned errors are separate
/// owners. It must never be used as a complete TLS allowance.
///
/// Proof profile: pinned rustls 0.23.43, Rust 1.94, 64-bit selected ring graph.
/// `OTERYN_PROVENANCE.md` records the exhaustive grammar and independent review.
/// An unqualified layout fails closed rather than borrowing a guessed allowance.
#[cfg(feature = "_tls-rustls-ring-webpki")]
pub(super) fn handshake_decode_heap_bound(
) -> Result<usize, crate::net::resource_budget::BudgetError> {
    use crate::net::resource_budget::BudgetError;
    use rustls::internal::msgs::handshake::{EchConfigPayload, HpkeSymmetricCipherSuite};
    if size_of::<usize>() != 8
        || size_of::<EchConfigPayload>() != 112
        || size_of::<HpkeSymmetricCipherSuite>() != 8
    {
        return Err(BudgetError::Unavailable);
    }

    // Existing rustls handshake limit; this does not introduce a wire cap.
    let wire = 0xffffusize;
    // Across non-ECH grammar, disjoint wire partitions fund vector elements,
    // moving old allocations, payload copies and unknown-tag BTree nodes.
    // 744 pays minimum-capacity overhead of all 14 ClientHello vector families;
    // 560 is the pinned largest extension Box (ClientExtensions).
    let non_ech = 39usize
        .checked_mul(wire)
        .and_then(|bytes| bytes.checked_add(744 + 560))
        .ok_or(BudgetError::Overflow)?;

    // Parent ECH Vec's maximal moving growth is 8192 -> 16384 entries.
    // Remaining child wire costs <=16 bytes of stable heap per wire byte.
    // A known configuration pays at least174 bytes of grammar-derived slack;
    // all-unknown configurations have the smaller byte-payload-only bound.
    // Six bytes belong to the containing extension header and ECH list prefix.
    // Unknown-tag BTree nodes fit the same16*remaining-wire envelope. The
    // enclosing ServerExtensions Box adds200. Duplicate known ECH is rejected
    // before decoding, so two ECH ASTs cannot coexist here.
    let old_capacity = 8192usize;
    let remaining = wire
        .checked_sub(4 * (old_capacity + 1))
        .ok_or(BudgetError::Overflow)?;
    let ech = 3usize
        .checked_mul(old_capacity)
        .and_then(|n| n.checked_mul(size_of::<EchConfigPayload>()))
        .and_then(|bytes| bytes.checked_add(16 * remaining))
        .and_then(|bytes| bytes.checked_sub(174 + 16 * 6))
        .and_then(|bytes| bytes.checked_add(200))
        .ok_or(BudgetError::Overflow)?;
    Ok(non_ech.max(ech))
}

/// Finite input-derived bound for the private, not-yet-verified peer chain.
///
/// A budgeted I/O loop must add bytes returned by `read_tls` and reserve before
/// every `process_new_packets` call. Stock `complete_io` may process repeatedly
/// before returning and is not a sufficient preallocation hook. After handshake,
/// use the public actual peer chain instead; this is not a lifetime read quota.
#[cfg(feature = "_tls-rustls-ring-webpki")]
#[derive(Default)]
pub(super) struct HandshakeInputBound {
    bytes: usize,
}

#[cfg(feature = "_tls-rustls-ring-webpki")]
impl HandshakeInputBound {
    pub(super) fn add_read(&mut self, bytes: usize) {
        // The existing rustls message maximum bounds one peer chain even if
        // more wire bytes were consumed. Saturation cannot reject a read.
        const HANDSHAKE_MAX: usize = 0xffff;
        self.bytes = (self.bytes + bytes.min(HANDSHAKE_MAX)).min(HANDSHAKE_MAX);
    }

    pub(super) fn peer_chain_heap_bound(
        &self,
    ) -> Result<usize, crate::net::resource_budget::BudgetError> {
        use crate::net::resource_budget::BudgetError;
        fn capacity(count: usize) -> Result<usize, BudgetError> {
            if count == 0 {
                Ok(0)
            } else {
                count
                    .checked_next_power_of_two()
                    .map(|n| n.max(4))
                    .ok_or(BudgetError::Overflow)
            }
        }
        // TLS1.2: each entry needs its three-byte length prefix, slot24.
        // TLS1.3: each entry also needs its two-byte extension prefix, slot48.
        // Into-owned conversion can retain the original48-byte allocation
        // while reusing it as24-byte slots; do not substitute len*24.
        let tls12 = capacity(self.bytes / 3)?
            .checked_mul(24)
            .ok_or(BudgetError::Overflow)?;
        let tls13 = capacity(self.bytes / 5)?
            .checked_mul(48)
            .ok_or(BudgetError::Overflow)?;
        // DER and retained OCSP payloads occupy disjoint source wire bytes.
        tls12
            .max(tls13)
            .checked_add(self.bytes)
            .ok_or(BudgetError::Overflow)
    }
}
