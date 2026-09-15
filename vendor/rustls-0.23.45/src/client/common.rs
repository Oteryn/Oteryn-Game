use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ops::Deref;

use super::ResolvesClientCert;
use crate::log::{debug, trace};
use crate::msgs::enums::ExtensionType;
use crate::msgs::handshake::{CertificateChain, DistinguishedName, ProtocolName, ServerExtensions};
use crate::sync::Arc;
use crate::{CipherSuite, SignatureScheme, compress, sign};

#[derive(Debug)]
pub(super) struct ServerCertDetails<'a> {
    pub(super) cert_chain: ServerCertChain<'a>,
    pub(super) ocsp_response: Vec<u8>,
    #[cfg(feature = "std")]
    pub(super) chain_custody: Option<crate::msgs::codec::DecodedCustody>,
    #[cfg(feature = "std")]
    pub(super) ocsp_custody: Option<crate::msgs::codec::DecodedCustody>,
}

/// Distinguishes ordinary borrowed certificate input from the already-owned,
/// already-charged destination produced by owner-aware certificate decoding.
///
/// In particular, converting `Charged` to `'static` is a move.  Calling the
/// ordinary `CertificateChain::into_owned()` here would allocate a replacement
/// outer `Vec` and detach the existing custody token from its backing.
#[derive(Debug)]
pub(super) enum ServerCertChain<'a> {
    Ordinary(CertificateChain<'a>),
    #[cfg(feature = "std")]
    Charged(CertificateChain<'static>),
}

impl<'a> Deref for ServerCertChain<'a> {
    type Target = CertificateChain<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Ordinary(chain) => chain,
            #[cfg(feature = "std")]
            Self::Charged(chain) => chain,
        }
    }
}

impl<'a> ServerCertChain<'a> {
    fn into_owned(self) -> ServerCertChain<'static> {
        match self {
            Self::Ordinary(chain) => ServerCertChain::Ordinary(chain.into_owned()),
            #[cfg(feature = "std")]
            Self::Charged(chain) => ServerCertChain::Charged(chain),
        }
    }

    fn into_peer_certificates(self) -> CertificateChain<'static> {
        match self.into_owned() {
            ServerCertChain::Ordinary(chain) => chain,
            #[cfg(feature = "std")]
            ServerCertChain::Charged(chain) => chain,
        }
    }
}

impl<'a> ServerCertDetails<'a> {
    pub(super) fn new(cert_chain: CertificateChain<'a>, ocsp_response: Vec<u8>) -> Self {
        Self {
            cert_chain: ServerCertChain::Ordinary(cert_chain),
            ocsp_response,
            #[cfg(feature = "std")]
            chain_custody: None,
            #[cfg(feature = "std")]
            ocsp_custody: None,
        }
    }

    #[cfg(feature = "std")]
    pub(super) fn set_ocsp_with_resource_owner(
        &mut self,
        source: &[u8],
    ) -> Result<(), crate::Error> {
        if source.is_empty() {
            return Ok(());
        }
        let custody = self.chain_custody.as_ref()
            .ok_or(crate::Error::InvalidMessage(crate::error::InvalidMessage::MessageTooLarge))?;
        let (destination, destination_custody) = custody.copy_bytes(source)?;
        self.ocsp_response = destination;
        self.ocsp_custody = Some(destination_custody);
        Ok(())
    }

    pub(super) fn into_owned(self) -> ServerCertDetails<'static> {
        let Self {
            cert_chain,
            ocsp_response,
            #[cfg(feature = "std")]
            chain_custody,
            #[cfg(feature = "std")]
            ocsp_custody,
        } = self;
        ServerCertDetails {
            cert_chain: cert_chain.into_owned(),
            ocsp_response,
            #[cfg(feature = "std")]
            chain_custody,
            #[cfg(feature = "std")]
            ocsp_custody,
        }
    }


    #[cfg(feature = "std")]
    pub(super) fn into_peer_certificates(
        self,
    ) -> (CertificateChain<'static>, Option<crate::msgs::codec::DecodedCustody>) {
        let Self {
            cert_chain,
            ocsp_response,
            chain_custody,
            ocsp_custody,
        } = self;
        drop(ocsp_response);
        drop(ocsp_custody);
        (cert_chain.into_peer_certificates(), chain_custody)
    }

    #[cfg(not(feature = "std"))]
    pub(super) fn into_peer_certificates(self) -> CertificateChain<'static> {
        self.cert_chain.into_peer_certificates()
    }

    #[cfg(feature = "std")]
    pub(super) fn take_peer_certificates(
        &mut self,
    ) -> (CertificateChain<'static>, Option<crate::msgs::codec::DecodedCustody>) {
        let chain = core::mem::replace(
            &mut self.cert_chain,
            ServerCertChain::Ordinary(CertificateChain::default()),
        )
        .into_peer_certificates();
        let ocsp = core::mem::take(&mut self.ocsp_response);
        drop(ocsp);
        drop(self.ocsp_custody.take());
        (chain, self.chain_custody.take())
    }
}

#[cfg(all(test, feature = "std"))]
mod resource_owner_tests {
    use alloc::vec;
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::msgs::codec::{DecodedCustody, DecodedOwner};
    use crate::pki_types::CertificateDer;
    use crate::{DeframerBufferError, DeframerBufferOwner};

    #[derive(Debug)]
    struct Owner {
        used: AtomicUsize,
    }

    impl DeframerBufferOwner for Owner {
        fn try_reserve(&self, bytes: usize) -> Result<(), DeframerBufferError> {
            self.used.fetch_add(bytes, Ordering::SeqCst);
            Ok(())
        }

        fn release(&self, bytes: usize) {
            self.used.fetch_sub(bytes, Ordering::SeqCst);
        }
    }

    #[test]
    fn charged_certificate_chain_keeps_outer_backing_through_transfers() {
        let owner = Arc::new(Owner {
            used: AtomicUsize::new(0),
        });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let bytes = size_of::<CertificateDer<'static>>() + 3 + 2;
        decoded.reserve(bytes).unwrap();
        let custody = DecodedCustody::exact(decoded, bytes);

        let mut outer = Vec::with_capacity(1);
        outer.push(CertificateDer::from(vec![1, 2, 3]));
        let chain = CertificateChain(outer);
        let chain_ptr = chain.0.as_ptr();
        let chain_capacity = chain.0.capacity();
        let details = ServerCertDetails::new_with_resource_custody(chain, vec![4, 5], custody);
        let used = owner.used.load(Ordering::SeqCst);

        let details = details.into_owned();
        assert_eq!(details.cert_chain.0.as_ptr(), chain_ptr);
        assert_eq!(details.cert_chain.0.capacity(), chain_capacity);
        assert_eq!(owner.used.load(Ordering::SeqCst), used);

        let (peer, peer_custody) = details.into_peer_certificates();
        assert_eq!(peer.0.as_ptr(), chain_ptr);
        assert_eq!(peer.0.capacity(), chain_capacity);
        assert_eq!(owner.used.load(Ordering::SeqCst), used - 2);

        drop(peer);
        assert_eq!(owner.used.load(Ordering::SeqCst), used - 2);
        drop(peer_custody);
        assert_eq!(owner.used.load(Ordering::SeqCst), DecodedOwner::arc_layout().unwrap());
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn tls12_ocsp_custody_releases_at_peer_chain_handoff() {
        let owner = Arc::new(Owner {
            used: AtomicUsize::new(0),
        });
        let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
        let chain_bytes = size_of::<CertificateDer<'static>>() + 3;
        decoded.reserve(chain_bytes).unwrap();
        let custody = DecodedCustody::exact(decoded, chain_bytes);

        let mut outer = Vec::with_capacity(1);
        outer.push(CertificateDer::from(vec![1, 2, 3]));
        let mut details =
            ServerCertDetails::new_with_resource_custody(CertificateChain(outer), vec![], custody);
        details.set_ocsp_with_resource_owner(&[4, 5]).unwrap();
        let with_ocsp = owner.used.load(Ordering::SeqCst);

        let (peer, peer_custody) = details.take_peer_certificates();
        assert_eq!(owner.used.load(Ordering::SeqCst), with_ocsp - 2);
        drop(peer);
        assert_eq!(owner.used.load(Ordering::SeqCst), with_ocsp - 2);
        drop(peer_custody);
        assert_eq!(owner.used.load(Ordering::SeqCst), DecodedOwner::arc_layout().unwrap());
        drop(arc_charge);
        assert_eq!(owner.used.load(Ordering::SeqCst), 0);
    }
}

#[cfg(feature = "std")]
impl ServerCertDetails<'static> {
    pub(super) fn new_with_resource_custody(
        cert_chain: CertificateChain<'static>,
        ocsp_response: Vec<u8>,
        mut resource_custody: crate::msgs::codec::DecodedCustody,
    ) -> Self {
        let ocsp_custody = resource_custody
            .split_off(ocsp_response.capacity())
            .expect("certificate destination custody matches its backing");
        Self {
            cert_chain: ServerCertChain::Charged(cert_chain),
            ocsp_response,
            chain_custody: Some(resource_custody),
            ocsp_custody,
        }
    }
}

pub(super) struct ClientHelloDetails {
    pub(super) alpn_protocols: Vec<ProtocolName>,
    pub(super) sent_extensions: Vec<ExtensionType>,
    pub(super) extension_order_seed: u16,
    pub(super) offered_cert_compression: bool,
    pub(super) offered_cipher_suites: Vec<CipherSuite>,
}

impl ClientHelloDetails {
    pub(super) fn new(alpn_protocols: Vec<ProtocolName>, extension_order_seed: u16) -> Self {
        Self {
            alpn_protocols,
            sent_extensions: Vec::new(),
            extension_order_seed,
            offered_cert_compression: false,
            offered_cipher_suites: Vec::new(),
        }
    }

    pub(super) fn server_sent_unsolicited_extensions(
        &self,
        received_exts: &ServerExtensions<'_>,
        allowed_unsolicited: &[ExtensionType],
    ) -> bool {
        let mut extensions = received_exts.collect_used();
        extensions.extend(
            received_exts
                .unknown_extensions
                .iter()
                .map(|ext| ExtensionType::from(*ext)),
        );
        for ext_type in extensions {
            if !self.sent_extensions.contains(&ext_type) && !allowed_unsolicited.contains(&ext_type)
            {
                trace!("Unsolicited extension {ext_type:?}");
                return true;
            }
        }

        false
    }
}

pub(super) enum ClientAuthDetails {
    /// Send an empty `Certificate` and no `CertificateVerify`.
    Empty { auth_context_tls13: Option<Vec<u8>> },
    /// Send a non-empty `Certificate` and a `CertificateVerify`.
    Verify {
        certkey: Arc<sign::CertifiedKey>,
        signer: Box<dyn sign::Signer>,
        auth_context_tls13: Option<Vec<u8>>,
        compressor: Option<&'static dyn compress::CertCompressor>,
    },
}

impl ClientAuthDetails {
    pub(super) fn resolve(
        resolver: &dyn ResolvesClientCert,
        canames: Option<&[DistinguishedName]>,
        sigschemes: &[SignatureScheme],
        auth_context_tls13: Option<Vec<u8>>,
        compressor: Option<&'static dyn compress::CertCompressor>,
    ) -> Self {
        let acceptable_issuers = canames
            .unwrap_or_default()
            .iter()
            .map(|p| p.as_ref())
            .collect::<Vec<&[u8]>>();

        if let Some(certkey) = resolver.resolve(&acceptable_issuers, sigschemes) {
            if let Some(signer) = certkey.key.choose_scheme(sigschemes) {
                debug!("Attempting client auth");
                return Self::Verify {
                    certkey,
                    signer,
                    auth_context_tls13,
                    compressor,
                };
            }
        }

        debug!("Client auth requested but no cert/sigscheme available");
        Self::Empty { auth_context_tls13 }
    }
}
