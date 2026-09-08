use pki_types::ServerName;

use crate::enums::SignatureScheme;
use crate::msgs::persist;
use crate::sync::Arc;
use crate::{NamedGroup, client, sign};

/// An implementer of `ClientSessionStore` which does nothing.
#[derive(Debug)]
pub(super) struct NoClientSessionStorage;

impl client::ClientSessionStore for NoClientSessionStorage {
    fn set_kx_hint(&self, _: ServerName<'static>, _: NamedGroup) {}

    fn kx_hint(&self, _: &ServerName<'_>) -> Option<NamedGroup> {
        None
    }

    fn set_tls12_session(&self, _: ServerName<'static>, _: persist::Tls12ClientSessionValue) {}

    fn tls12_session(&self, _: &ServerName<'_>) -> Option<persist::Tls12ClientSessionValue> {
        None
    }

    fn remove_tls12_session(&self, _: &ServerName<'_>) {}

    fn insert_tls13_ticket(&self, _: ServerName<'static>, _: persist::Tls13ClientSessionValue) {}

    fn take_tls13_ticket(&self, _: &ServerName<'_>) -> Option<persist::Tls13ClientSessionValue> {
        None
    }
}

#[cfg(any(feature = "std", feature = "hashbrown"))]
mod cache {
    use alloc::collections::VecDeque;
    use core::fmt;

    use pki_types::ServerName;

    use crate::lock::Mutex;
    use crate::msgs::persist;
    #[cfg(feature = "std")]
    use crate::sync::Arc;
    use crate::{NamedGroup, limited_cache};

    const MAX_TLS13_TICKETS_PER_SERVER: usize = 8;

    struct ServerData {
        kx_hint: Option<NamedGroup>,

        // Zero or one TLS1.2 sessions.
        #[cfg(feature = "tls12")]
        tls12: Option<persist::Tls12ClientSessionValue>,

        // Up to MAX_TLS13_TICKETS_PER_SERVER TLS1.3 tickets, oldest first.
        tls13: VecDeque<persist::Tls13ClientSessionValue>,
    }

    impl Default for ServerData {
        fn default() -> Self {
            Self {
                kx_hint: None,
                #[cfg(feature = "tls12")]
                tls12: None,
                tls13: VecDeque::with_capacity(MAX_TLS13_TICKETS_PER_SERVER),
            }
        }
    }

    /// An implementer of `ClientSessionStore` that stores everything
    /// in memory.
    ///
    /// It enforces a limit on the number of entries to bound memory usage.
    pub struct ClientSessionMemoryCache {
        servers: Mutex<limited_cache::LimitedCache<ServerName<'static>, ServerData>>,
    }

    impl ClientSessionMemoryCache {
        /// Make a new ClientSessionMemoryCache.  `size` is the
        /// maximum number of stored sessions.
        #[cfg(feature = "std")]
        pub fn new(size: usize) -> Self {
            let max_servers = size.saturating_add(MAX_TLS13_TICKETS_PER_SERVER - 1)
                / MAX_TLS13_TICKETS_PER_SERVER;
            Self {
                servers: Mutex::new(limited_cache::LimitedCache::new(max_servers)),
            }
        }

        /// Make a new ClientSessionMemoryCache.  `size` is the
        /// maximum number of stored sessions.
        #[cfg(not(feature = "std"))]
        pub fn new<M: crate::lock::MakeMutex>(size: usize) -> Self {
            let max_servers = size.saturating_add(MAX_TLS13_TICKETS_PER_SERVER - 1)
                / MAX_TLS13_TICKETS_PER_SERVER;
            Self {
                servers: Mutex::new::<M>(limited_cache::LimitedCache::new(max_servers)),
            }
        }
    }

    impl super::client::ClientSessionStore for ClientSessionMemoryCache {
        fn set_kx_hint(&self, server_name: ServerName<'static>, group: NamedGroup) {
            self.servers
                .lock()
                .unwrap()
                .get_or_insert_default_and_edit(server_name, |data| data.kx_hint = Some(group));
        }

        fn kx_hint(&self, server_name: &ServerName<'_>) -> Option<NamedGroup> {
            self.servers
                .lock()
                .unwrap()
                .get(server_name)
                .and_then(|sd| sd.kx_hint)
        }

        fn set_tls12_session(
            &self,
            _server_name: ServerName<'static>,
            _value: persist::Tls12ClientSessionValue,
        ) {
            #[cfg(feature = "tls12")]
            self.servers
                .lock()
                .unwrap()
                .get_or_insert_default_and_edit(_server_name.clone(), |data| {
                    data.tls12 = Some(_value)
                });
        }

        fn tls12_session(
            &self,
            _server_name: &ServerName<'_>,
        ) -> Option<persist::Tls12ClientSessionValue> {
            #[cfg(not(feature = "tls12"))]
            return None;

            #[cfg(feature = "tls12")]
            self.servers
                .lock()
                .unwrap()
                .get(_server_name)
                .and_then(|sd| sd.tls12.as_ref().cloned())
        }

        #[cfg(feature = "std")]
        fn tls12_session_with_resource_owner(
            &self,
            _server_name: &ServerName<'_>,
            owner: Arc<dyn crate::DeframerBufferOwner>,
        ) -> Result<Option<persist::Tls12ClientSessionValue>, crate::DeframerBufferError> {
            #[cfg(not(feature = "tls12"))]
            return Ok(None);

            #[cfg(feature = "tls12")]
            self.servers
                .lock()
                .unwrap()
                .get(_server_name)
                .and_then(|sd| sd.tls12.as_ref())
                .map(|value| value.clone_with_resource_owner(owner))
                .transpose()
        }

        fn remove_tls12_session(&self, _server_name: &ServerName<'static>) {
            #[cfg(feature = "tls12")]
            self.servers
                .lock()
                .unwrap()
                .get_mut(_server_name)
                .and_then(|data| data.tls12.take());
        }

        fn insert_tls13_ticket(
            &self,
            server_name: ServerName<'static>,
            value: persist::Tls13ClientSessionValue,
        ) {
            self.servers
                .lock()
                .unwrap()
                .get_or_insert_default_and_edit(server_name.clone(), |data| {
                    if data.tls13.len() == data.tls13.capacity() {
                        data.tls13.pop_front();
                    }
                    data.tls13.push_back(value);
                });
        }

        fn take_tls13_ticket(
            &self,
            server_name: &ServerName<'static>,
        ) -> Option<persist::Tls13ClientSessionValue> {
            self.servers
                .lock()
                .unwrap()
                .get_mut(server_name)
                .and_then(|data| data.tls13.pop_back())
        }
    }

    impl fmt::Debug for ClientSessionMemoryCache {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // Note: we omit self.servers as it may contain sensitive data.
            f.debug_struct("ClientSessionMemoryCache")
                .finish()
        }
    }
}

#[cfg(any(feature = "std", feature = "hashbrown"))]
pub use cache::ClientSessionMemoryCache;

#[derive(Debug)]
pub(super) struct FailResolveClientCert {}

impl client::ResolvesClientCert for FailResolveClientCert {
    fn resolve(
        &self,
        _root_hint_subjects: &[&[u8]],
        _sigschemes: &[SignatureScheme],
    ) -> Option<Arc<sign::CertifiedKey>> {
        None
    }

    fn has_certs(&self) -> bool {
        false
    }
}

/// An exemplar `ResolvesClientCert` implementation that always resolves to a single
/// [RFC 7250] raw public key.
///
/// [RFC 7250]: https://tools.ietf.org/html/rfc7250
#[derive(Clone, Debug)]
pub struct AlwaysResolvesClientRawPublicKeys(Arc<sign::CertifiedKey>);
impl AlwaysResolvesClientRawPublicKeys {
    /// Create a new `AlwaysResolvesClientRawPublicKeys` instance.
    pub fn new(certified_key: Arc<sign::CertifiedKey>) -> Self {
        Self(certified_key)
    }
}

impl client::ResolvesClientCert for AlwaysResolvesClientRawPublicKeys {
    fn resolve(
        &self,
        _root_hint_subjects: &[&[u8]],
        _sigschemes: &[SignatureScheme],
    ) -> Option<Arc<sign::CertifiedKey>> {
        Some(self.0.clone())
    }

    fn only_raw_public_keys(&self) -> bool {
        true
    }

    /// Returns true if the resolver is ready to present an identity.
    ///
    /// Even though the function is called `has_certs`, it returns true
    /// although only an RPK (Raw Public Key) is available, not an actual certificate.
    fn has_certs(&self) -> bool {
        true
    }
}

#[cfg(test)]
#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    use std::prelude::v1::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use pki_types::{ServerName, UnixTime};

    use super::{ClientSessionMemoryCache, NoClientSessionStorage};
    use super::provider::cipher_suite;
    use crate::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
    use crate::client::{ClientSessionStore, ResolvesClientCert};
    use crate::msgs::base::PayloadU16;
    use crate::msgs::enums::NamedGroup;
    use crate::msgs::handshake::CertificateChain;
    #[cfg(feature = "tls12")]
    use crate::msgs::handshake::SessionId;
    use crate::msgs::persist::Tls13ClientSessionValue;
    use crate::pki_types::CertificateDer;
    use crate::suites::SupportedCipherSuite;
    use crate::sync::Arc;
    use crate::{DigitallySignedStruct, Error, SignatureScheme, sign};

    #[derive(Debug)]
    struct SessionOwner {
        available: usize,
        charged: AtomicUsize,
    }

    impl crate::DeframerBufferOwner for SessionOwner {
        fn try_reserve(&self, bytes: usize) -> Result<(), crate::DeframerBufferError> {
            self.charged
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |charged| {
                    charged.checked_add(bytes).filter(|next| *next <= self.available)
                })
                .map(|_| ())
                .map_err(|_| crate::DeframerBufferError)
        }

        fn release(&self, bytes: usize) {
            self.charged.fetch_sub(bytes, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_noclientsessionstorage_does_nothing() {
        let c = NoClientSessionStorage {};
        let name = ServerName::try_from("example.com").unwrap();
        let now = UnixTime::now();
        let server_cert_verifier: Arc<dyn ServerCertVerifier> = Arc::new(DummyServerCertVerifier);
        let resolves_client_cert: Arc<dyn ResolvesClientCert> = Arc::new(DummyResolvesClientCert);

        c.set_kx_hint(name.clone(), NamedGroup::X25519);
        assert_eq!(None, c.kx_hint(&name));

        #[cfg(feature = "tls12")]
        {
            use crate::msgs::persist::Tls12ClientSessionValue;
            let SupportedCipherSuite::Tls12(tls12_suite) =
                cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
            else {
                unreachable!()
            };

            c.set_tls12_session(
                name.clone(),
                Tls12ClientSessionValue::new(
                    tls12_suite,
                    SessionId::empty(),
                    Arc::new(PayloadU16::empty()),
                    &[],
                    CertificateChain::default(),
                    &server_cert_verifier,
                    &resolves_client_cert,
                    now,
                    0,
                    true,
                ),
            );
            assert!(c.tls12_session(&name).is_none());
            c.remove_tls12_session(&name);
        }

        let SupportedCipherSuite::Tls13(tls13_suite) = cipher_suite::TLS13_AES_256_GCM_SHA384
        else {
            unreachable!();
        };
        c.insert_tls13_ticket(
            name.clone(),
            Tls13ClientSessionValue::new(
                tls13_suite,
                Arc::new(PayloadU16::empty()),
                &[],
                CertificateChain::default(),
                &server_cert_verifier,
                &resolves_client_cert,
                now,
                0,
                0,
                0,
            ),
        );
        assert!(c.take_tls13_ticket(&name).is_none());
    }

    #[cfg(feature = "tls12")]
    #[test]
    fn tls12_owner_retrieval_reserves_before_clone_and_releases_on_drop() {
        use crate::msgs::persist::Tls12ClientSessionValue;

        let SupportedCipherSuite::Tls12(suite) =
            cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
        else {
            unreachable!()
        };
        let name = ServerName::try_from("example.com").unwrap();
        let verifier: Arc<dyn ServerCertVerifier> = Arc::new(DummyServerCertVerifier);
        let resolver: Arc<dyn ResolvesClientCert> = Arc::new(DummyResolvesClientCert);
        let session = Tls12ClientSessionValue::new(
            suite,
            SessionId::empty(),
            Arc::new(PayloadU16::empty()),
            &[7; 48],
            CertificateChain::default(),
            &verifier,
            &resolver,
            UnixTime::now(),
            60,
            true,
        );
        let owner = Arc::new(SessionOwner {
            available: 96,
            charged: AtomicUsize::new(0),
        });
        let cache = ClientSessionMemoryCache::new(16);
        cache.set_tls12_session(
            name.clone(),
            session.clone_with_resource_owner(owner.clone()).unwrap(),
        );
        assert_eq!(owner.charged.load(Ordering::SeqCst), 48);

        let retrieved = cache
            .tls12_session_with_resource_owner(&name, owner.clone())
            .unwrap()
            .unwrap();
        assert_eq!(owner.charged.load(Ordering::SeqCst), 96);
        drop(retrieved);
        assert_eq!(owner.charged.load(Ordering::SeqCst), 48);

        let denied = Arc::new(SessionOwner {
            available: 95,
            charged: AtomicUsize::new(0),
        });
        let denied_cache = ClientSessionMemoryCache::new(16);
        denied_cache.set_tls12_session(
            name.clone(),
            session.clone_with_resource_owner(denied.clone()).unwrap(),
        );
        assert!(denied_cache
            .tls12_session_with_resource_owner(&name, denied.clone())
            .is_err());
        assert_eq!(denied.charged.load(Ordering::SeqCst), 48);
        assert!(denied_cache.tls12_session(&name).is_some());
        drop(denied_cache);
        assert_eq!(denied.charged.load(Ordering::SeqCst), 0);
    }

    #[cfg(feature = "tls12")]
    #[test]
    fn unsupported_store_fails_closed_on_owner_retrieval() {
        let store = NoClientSessionStorage;
        let name = ServerName::try_from("example.com").unwrap();
        let owner = Arc::new(SessionOwner {
            available: usize::MAX,
            charged: AtomicUsize::new(0),
        });
        assert!(store
            .tls12_session_with_resource_owner(&name, owner)
            .is_err());
        assert!(store.tls12_session(&name).is_none());
    }

    #[derive(Debug)]
    struct DummyServerCertVerifier;

    impl ServerCertVerifier for DummyServerCertVerifier {
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, Error> {
            unreachable!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            unreachable!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            unreachable!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            unreachable!()
        }
    }

    #[derive(Debug)]
    struct DummyResolvesClientCert;

    impl ResolvesClientCert for DummyResolvesClientCert {
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn resolve(
            &self,
            _root_hint_subjects: &[&[u8]],
            _sigschemes: &[SignatureScheme],
        ) -> Option<Arc<sign::CertifiedKey>> {
            unreachable!()
        }

        #[cfg_attr(coverage_nightly, coverage(off))]
        fn has_certs(&self) -> bool {
            unreachable!()
        }
    }
}
