use alloc::vec::Vec;
#[cfg(feature = "std")]
use core::alloc::Layout;
use core::cmp;
#[cfg(feature = "std")]
use core::sync::atomic::AtomicUsize;
#[cfg(feature = "std")]
use core::mem::size_of;

use pki_types::{DnsName, UnixTime};
use zeroize::Zeroizing;

use crate::client::ResolvesClientCert;
use crate::enums::{CipherSuite, ProtocolVersion};
use crate::error::InvalidMessage;
use crate::msgs::base::{MaybeEmpty, PayloadU8, PayloadU16};
use crate::msgs::codec::{Codec, Reader};
#[cfg(feature = "tls12")]
use crate::msgs::handshake::SessionId;
use crate::msgs::handshake::{CertificateChain, ProtocolName, TicketPayload};
use crate::sync::{Arc, Weak};
#[cfg(feature = "tls12")]
use crate::tls12::Tls12CipherSuite;
use crate::tls13::Tls13CipherSuite;
use crate::verify::ServerCertVerifier;

pub(crate) struct Retrieved<T> {
    pub(crate) value: T,
    retrieved_at: UnixTime,
}

impl<T> Retrieved<T> {
    pub(crate) fn new(value: T, retrieved_at: UnixTime) -> Self {
        Self {
            value,
            retrieved_at,
        }
    }

    pub(crate) fn map<M>(&self, f: impl FnOnce(&T) -> Option<&M>) -> Option<Retrieved<&M>> {
        Some(Retrieved {
            value: f(&self.value)?,
            retrieved_at: self.retrieved_at,
        })
    }
}

impl Retrieved<&Tls13ClientSessionValue> {
    pub(crate) fn obfuscated_ticket_age(&self) -> u32 {
        let age_secs = self
            .retrieved_at
            .as_secs()
            .saturating_sub(self.value.common.epoch);
        // nb. tickets have an upper age limit of ~7 days, well short of the 49 days here
        let age_millis = u32::try_from(age_secs)
            .unwrap_or(u32::MAX)
            .saturating_mul(1000);
        age_millis.wrapping_add(self.value.age_add)
    }
}

impl<T: core::ops::Deref<Target = ClientSessionCommon>> Retrieved<T> {
    pub(crate) fn has_expired(&self) -> bool {
        let common = &*self.value;
        common.lifetime_secs != 0
            && common
                .epoch
                .saturating_add(u64::from(common.lifetime_secs))
                < self.retrieved_at.as_secs()
    }
}

impl<T> core::ops::Deref for Retrieved<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Debug)]
pub struct Tls13ClientSessionValue {
    suite: &'static Tls13CipherSuite,
    age_add: u32,
    max_early_data_size: u32,
    pub(crate) common: ClientSessionCommon,
    quic_params: PayloadU16,
}

impl Tls13ClientSessionValue {
    pub(crate) fn new(
        suite: &'static Tls13CipherSuite,
        ticket: TicketPayload,
        secret: &[u8],
        server_cert_chain: CertificateChain<'static>,
        server_cert_verifier: &Arc<dyn ServerCertVerifier>,
        client_creds: &Arc<dyn ResolvesClientCert>,
        time_now: UnixTime,
        lifetime_secs: u32,
        age_add: u32,
        max_early_data_size: u32,
    ) -> Self {
        Self {
            suite,
            age_add,
            max_early_data_size,
            common: ClientSessionCommon::new(
                ticket,
                secret,
                time_now,
                lifetime_secs,
                server_cert_chain,
                server_cert_verifier,
                client_creds,
            ),
            quic_params: PayloadU16::new(Vec::new()),
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn new_with_resource_owner(
        suite: &'static Tls13CipherSuite,
        ticket: TicketPayload,
        secret: &[u8],
        server_cert_chain: &CertificateChain<'static>,
        server_cert_verifier: &Arc<dyn ServerCertVerifier>,
        client_creds: &Arc<dyn ResolvesClientCert>,
        time_now: UnixTime,
        lifetime_secs: u32,
        age_add: u32,
        max_early_data_size: u32,
    ) -> Result<Self, crate::DeframerBufferError> {
        let owner = ticket.resource_owner().ok_or(crate::DeframerBufferError)?;
        Ok(Self {
            suite,
            age_add,
            max_early_data_size,
            common: ClientSessionCommon::new_with_resource_owner(
                ticket, secret, time_now, lifetime_secs, server_cert_chain,
                server_cert_verifier, client_creds, owner,
            )?,
            quic_params: PayloadU16::new(Vec::new()),
        })
    }

    pub fn max_early_data_size(&self) -> u32 {
        self.max_early_data_size
    }

    pub fn suite(&self) -> &'static Tls13CipherSuite {
        self.suite
    }

    #[doc(hidden)]
    /// Test only: rewind epoch by `delta` seconds.
    pub fn rewind_epoch(&mut self, delta: u32) {
        self.common.epoch -= delta as u64;
    }

    #[doc(hidden)]
    /// Test only: replace `max_early_data_size` with `new`
    pub fn _private_set_max_early_data_size(&mut self, new: u32) {
        self.max_early_data_size = new;
    }

    pub fn set_quic_params(&mut self, quic_params: &[u8]) {
        self.quic_params = PayloadU16::new(quic_params.to_vec());
    }

    pub fn quic_params(&self) -> Vec<u8> {
        self.quic_params.0.clone()
    }
}

impl core::ops::Deref for Tls13ClientSessionValue {
    type Target = ClientSessionCommon;

    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

#[derive(Debug)]
pub struct Tls12ClientSessionValue {
    #[cfg(feature = "tls12")]
    suite: &'static Tls12CipherSuite,
    #[cfg(feature = "tls12")]
    pub(crate) session_id: SessionId,
    #[cfg(feature = "tls12")]
    extended_ms: bool,
    #[doc(hidden)]
    #[cfg(feature = "tls12")]
    pub(crate) common: ClientSessionCommon,
}

#[cfg(feature = "tls12")]
impl Tls12ClientSessionValue {
    pub(crate) fn new(
        suite: &'static Tls12CipherSuite,
        session_id: SessionId,
        ticket: TicketPayload,
        master_secret: &[u8],
        server_cert_chain: CertificateChain<'static>,
        server_cert_verifier: &Arc<dyn ServerCertVerifier>,
        client_creds: &Arc<dyn ResolvesClientCert>,
        time_now: UnixTime,
        lifetime_secs: u32,
        extended_ms: bool,
    ) -> Self {
        Self {
            suite,
            session_id,
            extended_ms,
            common: ClientSessionCommon::new(
                ticket,
                master_secret,
                time_now,
                lifetime_secs,
                server_cert_chain,
                server_cert_verifier,
                client_creds,
            ),
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn new_with_resource_owner(
        suite: &'static Tls12CipherSuite,
        session_id: SessionId,
        ticket: TicketPayload,
        master_secret: &[u8],
        server_cert_chain: &CertificateChain<'static>,
        server_cert_verifier: &Arc<dyn ServerCertVerifier>,
        client_creds: &Arc<dyn ResolvesClientCert>,
        time_now: UnixTime,
        lifetime_secs: u32,
        extended_ms: bool,
        owner: Arc<dyn crate::DeframerBufferOwner>,
    ) -> Result<Self, crate::DeframerBufferError> {
        Ok(Self {
            suite,
            session_id,
            extended_ms,
            common: ClientSessionCommon::new_with_resource_owner(
                ticket, master_secret, time_now, lifetime_secs, server_cert_chain,
                server_cert_verifier, client_creds, owner,
            )?,
        })
    }

    pub(crate) fn ticket(&mut self) -> TicketPayload {
        self.common.ticket.clone()
    }

    pub(crate) fn extended_ms(&self) -> bool {
        self.extended_ms
    }

    pub(crate) fn suite(&self) -> &'static Tls12CipherSuite {
        self.suite
    }

    #[cfg(feature = "std")]
    pub(crate) fn clone_with_resource_owner(
        &self,
        owner: Arc<dyn crate::DeframerBufferOwner>,
    ) -> Result<Self, crate::DeframerBufferError> {
        Ok(Self {
            suite: self.suite,
            session_id: self.session_id,
            extended_ms: self.extended_ms,
            common: self.common.clone_with_resource_owner(owner)?,
        })
    }

    #[doc(hidden)]
    /// Test only: rewind epoch by `delta` seconds.
    pub fn rewind_epoch(&mut self, delta: u32) {
        self.common.epoch -= delta as u64;
    }
}

#[cfg(feature = "tls12")]
impl Clone for Tls12ClientSessionValue {
    fn clone(&self) -> Self {
        Self {
            suite: self.suite,
            session_id: self.session_id,
            extended_ms: self.extended_ms,
            common: self.common.clone(),
        }
    }
}

#[cfg(feature = "tls12")]
impl core::ops::Deref for Tls12ClientSessionValue {
    type Target = ClientSessionCommon;

    fn deref(&self) -> &Self::Target {
        &self.common
    }
}

#[derive(Debug)]
pub struct ClientSessionCommon {
    ticket: TicketPayload,
    secret: Zeroizing<PayloadU8>,
    epoch: u64,
    lifetime_secs: u32,
    server_cert_chain: RetainedCertificateChain,
    server_cert_verifier: Weak<dyn ServerCertVerifier>,
    client_creds: Weak<dyn ResolvesClientCert>,
    #[cfg(feature = "std")]
    _resource_custody: Option<RetainedSessionCustody>,
}

#[cfg(feature = "std")]
#[derive(Debug)]
struct RetainedSessionCustody {
    owner: Arc<dyn crate::DeframerBufferOwner>,
    bytes: usize,
}


#[derive(Debug)]
struct RetainedCertificateChain {
    chain: Option<Arc<CertificateChain<'static>>>,
    #[cfg(feature = "std")]
    owner: Option<Arc<dyn crate::DeframerBufferOwner>>,
    #[cfg(feature = "std")]
    bytes: usize,
}

impl RetainedCertificateChain {
    fn unowned(chain: CertificateChain<'static>) -> Self {
        Self {
            chain: Some(Arc::new(chain)),
            #[cfg(feature = "std")]
            owner: None,
            #[cfg(feature = "std")]
            bytes: 0,
        }
    }

    #[cfg(feature = "std")]
    fn arc_layout() -> Result<usize, crate::DeframerBufferError> {
        Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<CertificateChain<'static>>())
            .map(|(layout, _)| layout.pad_to_align().size())
            .map_err(|_| crate::DeframerBufferError)
    }

    #[cfg(feature = "std")]
    fn deep_copy(
        source: &CertificateChain<'static>,
        owner: Arc<dyn crate::DeframerBufferOwner>,
    ) -> Result<Self, crate::DeframerBufferError> {
        let outer = source.0.len().checked_mul(size_of::<pki_types::CertificateDer<'static>>())
            .ok_or(crate::DeframerBufferError)?;
        owner.try_reserve(outer)?;
        let mut reserved = outer;
        let mut certificates = Vec::with_capacity(source.0.len());
        if certificates.capacity() != source.0.len() {
            drop(certificates);
            owner.release(reserved);
            return Err(crate::DeframerBufferError);
        }
        for certificate in &source.0 {
            let bytes = certificate.as_ref().len();
            if owner.try_reserve(bytes).is_err() {
                drop(certificates);
                owner.release(reserved);
                return Err(crate::DeframerBufferError);
            }
            reserved = match reserved.checked_add(bytes) {
                Some(total) => total,
                None => {
                    owner.release(bytes);
                    drop(certificates);
                    owner.release(reserved);
                    return Err(crate::DeframerBufferError);
                }
            };
            let mut owned = Vec::with_capacity(bytes);
            owned.extend_from_slice(certificate.as_ref());
            if owned.capacity() != bytes {
                drop(owned);
                drop(certificates);
                owner.release(reserved);
                return Err(crate::DeframerBufferError);
            }
            certificates.push(pki_types::CertificateDer::from(owned));
        }
        let control = match Self::arc_layout() {
            Ok(control) => control,
            Err(error) => {
                drop(certificates);
                owner.release(reserved);
                return Err(error);
            }
        };
        if owner.try_reserve(control).is_err() {
            drop(certificates);
            owner.release(reserved);
            return Err(crate::DeframerBufferError);
        }
        reserved = match reserved.checked_add(control) {
            Some(total) => total,
            None => {
                owner.release(control);
                drop(certificates);
                owner.release(reserved);
                return Err(crate::DeframerBufferError);
            }
        };
        Ok(Self { chain: Some(Arc::new(CertificateChain(certificates))), owner: Some(owner), bytes: reserved })
    }
}

impl Clone for RetainedCertificateChain {
    fn clone(&self) -> Self {
        Self {
            chain: self.chain.clone(),
            #[cfg(feature = "std")]
            owner: self.owner.clone(),
            #[cfg(feature = "std")]
            bytes: self.bytes,
        }
    }
}

impl core::ops::Deref for RetainedCertificateChain {
    type Target = CertificateChain<'static>;
    fn deref(&self) -> &Self::Target { self.chain.as_deref().unwrap() }
}

impl Drop for RetainedCertificateChain {
    fn drop(&mut self) {
        #[cfg(feature = "std")]
        let final_control = self.owner.is_some()
            && self.chain.as_ref().is_some_and(|chain| Arc::strong_count(chain) == 1);
        drop(self.chain.take());
        #[cfg(feature = "std")]
        if final_control {
            if let Some(owner) = self.owner.take() { owner.release(self.bytes); }
        }
    }
}

#[cfg(feature = "std")]
fn copy_certificate_chain_for_peer(
    source: &CertificateChain<'static>,
    owner: Arc<dyn crate::DeframerBufferOwner>,
) -> Result<
    (
        CertificateChain<'static>,
        crate::msgs::codec::DirectDecodedCustody,
    ),
    crate::DeframerBufferError,
> {
    let outer = source
        .0
        .len()
        .checked_mul(size_of::<pki_types::CertificateDer<'static>>())
        .ok_or(crate::DeframerBufferError)?;
    let bytes = source.0.iter().try_fold(outer, |total, certificate| {
        total
            .checked_add(certificate.as_ref().len())
            .ok_or(crate::DeframerBufferError)
    })?;
    let custody = crate::msgs::codec::DirectDecodedCustody::reserve(owner, bytes)
        .map_err(|_| crate::DeframerBufferError)?;

    let mut certificates = Vec::with_capacity(source.0.len());
    if certificates.capacity() != source.0.len() {
        drop(certificates);
        drop(custody);
        return Err(crate::DeframerBufferError);
    }
    for certificate in &source.0 {
        let expected = certificate.as_ref().len();
        let mut copied = Vec::with_capacity(expected);
        copied.extend_from_slice(certificate.as_ref());
        if copied.capacity() != expected {
            drop(copied);
            drop(certificates);
            drop(custody);
            return Err(crate::DeframerBufferError);
        }
        certificates.push(pki_types::CertificateDer::from(copied));
    }

    Ok((CertificateChain(certificates), custody))
}

#[cfg(feature = "std")]
impl Drop for RetainedSessionCustody {
    fn drop(&mut self) {
        self.owner.release(self.bytes);
    }
}

impl ClientSessionCommon {
    fn new(
        ticket: TicketPayload,
        secret: &[u8],
        time_now: UnixTime,
        lifetime_secs: u32,
        server_cert_chain: CertificateChain<'static>,
        server_cert_verifier: &Arc<dyn ServerCertVerifier>,
        client_creds: &Arc<dyn ResolvesClientCert>,
    ) -> Self {
        Self {
            ticket,
            secret: Zeroizing::new(PayloadU8::new(secret.to_vec())),
            epoch: time_now.as_secs(),
            lifetime_secs: cmp::min(lifetime_secs, MAX_TICKET_LIFETIME),
            server_cert_chain: RetainedCertificateChain::unowned(server_cert_chain),
            server_cert_verifier: Arc::downgrade(server_cert_verifier),
            client_creds: Arc::downgrade(client_creds),
            #[cfg(feature = "std")]
            _resource_custody: None,
        }
    }


    #[cfg(feature = "std")]
    fn new_with_resource_owner(
        ticket: TicketPayload,
        secret: &[u8],
        time_now: UnixTime,
        lifetime_secs: u32,
        server_cert_chain: &CertificateChain<'static>,
        server_cert_verifier: &Arc<dyn ServerCertVerifier>,
        client_creds: &Arc<dyn ResolvesClientCert>,
        owner: Arc<dyn crate::DeframerBufferOwner>,
    ) -> Result<Self, crate::DeframerBufferError> {
        let secret_bytes = secret.len();
        owner.try_reserve(secret_bytes)?;
        let mut secret_copy = Vec::with_capacity(secret_bytes);
        secret_copy.extend_from_slice(secret);
        if secret_copy.capacity() != secret_bytes {
            drop(secret_copy);
            owner.release(secret_bytes);
            return Err(crate::DeframerBufferError);
        }
        let secret_copy = Zeroizing::new(PayloadU8::new(secret_copy));
        let server_cert_chain = match RetainedCertificateChain::deep_copy(server_cert_chain, owner.clone()) {
            Ok(chain) => chain,
            Err(_error) => {
                drop(secret_copy);
                owner.release(secret_bytes);
                return Err(crate::DeframerBufferError);
            }
        };
        Ok(Self {
            ticket,
            secret: secret_copy,
            epoch: time_now.as_secs(),
            lifetime_secs: cmp::min(lifetime_secs, MAX_TICKET_LIFETIME),
            server_cert_chain,
            server_cert_verifier: Arc::downgrade(server_cert_verifier),
            client_creds: Arc::downgrade(client_creds),
            _resource_custody: Some(RetainedSessionCustody { owner, bytes: secret_bytes }),
        })
    }

    #[cfg(feature = "std")]
    fn clone_with_resource_owner(
        &self,
        owner: Arc<dyn crate::DeframerBufferOwner>,
    ) -> Result<Self, crate::DeframerBufferError> {
        let bytes = self.secret.0.len();
        owner.try_reserve(bytes)?;
        let mut secret = alloc::vec![0; bytes];
        secret.copy_from_slice(self.secret.0.as_ref());
        debug_assert_eq!(secret.capacity(), bytes);
        Ok(Self {
            ticket: self.ticket.clone(),
            secret: Zeroizing::new(PayloadU8::new(secret)),
            epoch: self.epoch,
            lifetime_secs: self.lifetime_secs,
            server_cert_chain: self.server_cert_chain.clone(),
            server_cert_verifier: self.server_cert_verifier.clone(),
            client_creds: self.client_creds.clone(),
            _resource_custody: Some(RetainedSessionCustody { owner, bytes }),
        })
    }

    pub(crate) fn compatible_config(
        &self,
        server_cert_verifier: &Arc<dyn ServerCertVerifier>,
        client_creds: &Arc<dyn ResolvesClientCert>,
    ) -> bool {
        let same_verifier = Weak::ptr_eq(
            &Arc::downgrade(server_cert_verifier),
            &self.server_cert_verifier,
        );
        let same_creds = Weak::ptr_eq(&Arc::downgrade(client_creds), &self.client_creds);

        match (same_verifier, same_creds) {
            (true, true) => true,
            (false, _) => {
                crate::log::trace!("resumption not allowed between different ServerCertVerifiers");
                false
            }
            (_, _) => {
                crate::log::trace!(
                    "resumption not allowed between different ResolvesClientCert values"
                );
                false
            }
        }
    }

    pub(crate) fn server_cert_chain(&self) -> &CertificateChain<'static> {
        &self.server_cert_chain
    }

    #[cfg(feature = "std")]
    pub(crate) fn copy_server_cert_chain_for_current_owner(
        &self,
    ) -> Result<
        (
            CertificateChain<'static>,
            crate::msgs::codec::DirectDecodedCustody,
        ),
        crate::DeframerBufferError,
    > {
        let owner = self
            ._resource_custody
            .as_ref()
            .ok_or(crate::DeframerBufferError)?
            .owner
            .clone();
        copy_certificate_chain_for_peer(&self.server_cert_chain, owner)
    }

    pub(crate) fn secret(&self) -> &[u8] {
        self.secret.0.as_ref()
    }

    pub(crate) fn ticket(&self) -> &[u8] {
        self.ticket.0.as_ref()
    }
}

impl Clone for ClientSessionCommon {
    fn clone(&self) -> Self {
        Self {
            ticket: self.ticket.clone(),
            secret: self.secret.clone(),
            epoch: self.epoch,
            lifetime_secs: self.lifetime_secs,
            server_cert_chain: self.server_cert_chain.clone(),
            server_cert_verifier: self.server_cert_verifier.clone(),
            client_creds: self.client_creds.clone(),
            #[cfg(feature = "std")]
            _resource_custody: None,
        }
    }
}

static MAX_TICKET_LIFETIME: u32 = 7 * 24 * 60 * 60;

/// This is the maximum allowed skew between server and client clocks, over
/// the maximum ticket lifetime period.  This encompasses TCP retransmission
/// times in case packet loss occurs when the client sends the ClientHello
/// or receives the NewSessionTicket, _and_ actual clock skew over this period.
static MAX_FRESHNESS_SKEW_MS: u32 = 60 * 1000;

#[cfg(all(test, feature = "std"))]
mod owner_tests {
    use alloc::vec;
    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::client::danger::{HandshakeSignatureValid, ServerCertVerified};
    use crate::sign;
    use crate::enums::SignatureScheme;
    use crate::msgs::codec::{DecodedCustody, DecodedOwner};
    use crate::pki_types::{CertificateDer, ServerName};
    use crate::{DigitallySignedStruct, Error};

    #[derive(Debug)]
    struct Owner {
        limit: usize,
        used: AtomicUsize,
    }

    impl crate::DeframerBufferOwner for Owner {
        fn try_reserve(&self, bytes: usize) -> Result<(), crate::DeframerBufferError> {
            self.used
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                    used.checked_add(bytes).filter(|next| *next <= self.limit)
                })
                .map(|_| ())
                .map_err(|_| crate::DeframerBufferError)
        }

        fn release(&self, bytes: usize) {
            self.used.fetch_sub(bytes, Ordering::Relaxed);
        }
    }

    #[derive(Debug)]
    struct DummyServerCertVerifier;

    impl ServerCertVerifier for DummyServerCertVerifier {
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

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            unreachable!()
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, Error> {
            unreachable!()
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            unreachable!()
        }
    }

    #[derive(Debug)]
    struct DummyResolvesClientCert;

    impl ResolvesClientCert for DummyResolvesClientCert {
        fn resolve(
            &self,
            _root_hint_subjects: &[&[u8]],
            _sigschemes: &[SignatureScheme],
        ) -> Option<Arc<sign::CertifiedKey>> {
            unreachable!()
        }

        fn has_certs(&self) -> bool {
            unreachable!()
        }
    }

    #[test]
    fn tls12_session_id_retention_uses_underlying_owner_after_decoded_owner_drop() {
        let source = CertificateChain(vec![CertificateDer::from(vec![1, 2, 3])]);
        let secret = [4, 5, 6, 7];
        let retained_bytes = secret.len()
            + size_of::<CertificateDer<'static>>()
            + source[0].as_ref().len()
            + RetainedCertificateChain::arc_layout().unwrap();
        let decoded_arc_bytes = DecodedOwner::arc_layout().unwrap();
        let base = Arc::new(Owner {
            limit: decoded_arc_bytes + 1 + retained_bytes,
            used: AtomicUsize::new(0),
        });
        let (decoded, decoded_arc_charge) = DecodedOwner::new(base.clone()).unwrap();
        decoded.reserve(1).unwrap();
        let peer_custody = DecodedCustody::exact(decoded.clone(), 1);
        let decoded_strong_count = Arc::strong_count(&decoded);
        let resource_owner = peer_custody.resource_owner();
        assert_eq!(Arc::strong_count(&decoded), decoded_strong_count);

        let verifier: Arc<dyn ServerCertVerifier> = Arc::new(DummyServerCertVerifier);
        let resolver: Arc<dyn ResolvesClientCert> = Arc::new(DummyResolvesClientCert);
        let retained = ClientSessionCommon::new_with_resource_owner(
            TicketPayload::from_unowned(PayloadU16::empty()),
            &secret,
            UnixTime::since_unix_epoch(core::time::Duration::from_secs(1)),
            0,
            &source,
            &verifier,
            &resolver,
            resource_owner,
        )
        .unwrap();

        drop(peer_custody);
        drop(decoded);
        drop(decoded_arc_charge);
        assert_eq!(base.used.load(Ordering::Relaxed), retained_bytes);
        drop(retained);
        assert_eq!(base.used.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn tls12_session_id_retention_denies_before_secret_or_chain_backing_survives() {
        let source = CertificateChain(vec![CertificateDer::from(vec![1, 2, 3])]);
        let secret = [4, 5, 6, 7];
        let verifier: Arc<dyn ServerCertVerifier> = Arc::new(DummyServerCertVerifier);
        let resolver: Arc<dyn ResolvesClientCert> = Arc::new(DummyResolvesClientCert);
        let construct = |owner: Arc<Owner>| {
            ClientSessionCommon::new_with_resource_owner(
                TicketPayload::from_unowned(PayloadU16::empty()),
                &secret,
                UnixTime::since_unix_epoch(core::time::Duration::from_secs(1)),
                0,
                &source,
                &verifier,
                &resolver,
                owner,
            )
        };

        let before_secret = Arc::new(Owner {
            limit: secret.len() - 1,
            used: AtomicUsize::new(0),
        });
        assert!(construct(before_secret.clone()).is_err());
        assert_eq!(before_secret.used.load(Ordering::Relaxed), 0);

        let complete = secret.len()
            + size_of::<CertificateDer<'static>>()
            + source[0].as_ref().len()
            + RetainedCertificateChain::arc_layout().unwrap();
        let before_chain_control = Arc::new(Owner {
            limit: complete - 1,
            used: AtomicUsize::new(0),
        });
        assert!(construct(before_chain_control.clone()).is_err());
        assert_eq!(before_chain_control.used.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn retained_certificate_chain_deep_copy_and_arc_custody() {
        let source = CertificateChain(vec![
            pki_types::CertificateDer::from(vec![1, 2, 3]),
            pki_types::CertificateDer::from(vec![4, 5]),
        ]);
        let outer = 2 * size_of::<pki_types::CertificateDer<'static>>();
        let expected = outer + 5 + RetainedCertificateChain::arc_layout().unwrap();
        let owner = Arc::new(Owner { limit: expected, used: AtomicUsize::new(0) });

        let retained = RetainedCertificateChain::deep_copy(&source, owner.clone()).unwrap();
        assert_eq!(owner.used.load(Ordering::Relaxed), expected);
        assert_ne!(retained[0].as_ref().as_ptr(), source[0].as_ref().as_ptr());
        let clone = retained.clone();
        assert_eq!(owner.used.load(Ordering::Relaxed), expected);
        drop(source);
        drop(retained);
        assert_eq!(owner.used.load(Ordering::Relaxed), expected);
        drop(clone);
        assert_eq!(owner.used.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn resumed_tls12_rebinds_peer_chain_to_current_owner() {
        let source = CertificateChain(vec![
            CertificateDer::from(vec![1, 2, 3]),
            CertificateDer::from(vec![4, 5]),
        ]);
        let secret = [6, 7, 8, 9];
        let verifier: Arc<dyn ServerCertVerifier> = Arc::new(DummyServerCertVerifier);
        let resolver: Arc<dyn ResolvesClientCert> = Arc::new(DummyResolvesClientCert);
        let retained_chain_bytes = 2 * size_of::<CertificateDer<'static>>()
            + 5
            + RetainedCertificateChain::arc_layout().unwrap();
        let old_owner = Arc::new(Owner {
            limit: secret.len() + retained_chain_bytes,
            used: AtomicUsize::new(0),
        });
        let stored = ClientSessionCommon::new_with_resource_owner(
            TicketPayload::from_unowned(PayloadU16::empty()),
            &secret,
            UnixTime::since_unix_epoch(core::time::Duration::from_secs(1)),
            0,
            &source,
            &verifier,
            &resolver,
            old_owner.clone(),
        )
        .unwrap();
        assert_eq!(old_owner.used.load(Ordering::Relaxed), secret.len() + retained_chain_bytes);

        let peer_chain_bytes = 2 * size_of::<CertificateDer<'static>>() + 5;
        let current_owner = Arc::new(Owner {
            limit: secret.len() + peer_chain_bytes,
            used: AtomicUsize::new(0),
        });
        let retrieved = stored
            .clone_with_resource_owner(current_owner.clone())
            .unwrap();
        let (peer_chain, peer_custody) = retrieved
            .copy_server_cert_chain_for_current_owner()
            .unwrap();
        assert_eq!(current_owner.used.load(Ordering::Relaxed), secret.len() + peer_chain_bytes);
        assert_eq!(old_owner.used.load(Ordering::Relaxed), secret.len() + retained_chain_bytes);
        assert_ne!(peer_chain[0].as_ref().as_ptr(), stored.server_cert_chain()[0].as_ref().as_ptr());

        drop(peer_chain);
        drop(peer_custody);
        assert_eq!(current_owner.used.load(Ordering::Relaxed), secret.len());
        drop(retrieved);
        assert_eq!(current_owner.used.load(Ordering::Relaxed), 0);
        assert_eq!(old_owner.used.load(Ordering::Relaxed), secret.len() + retained_chain_bytes);
        drop(stored);
        assert_eq!(old_owner.used.load(Ordering::Relaxed), 0);

        let denied_owner = Arc::new(Owner {
            limit: secret.len() + peer_chain_bytes - 1,
            used: AtomicUsize::new(0),
        });
        let stored = ClientSessionCommon::new_with_resource_owner(
            TicketPayload::from_unowned(PayloadU16::empty()),
            &secret,
            UnixTime::since_unix_epoch(core::time::Duration::from_secs(1)),
            0,
            &source,
            &verifier,
            &resolver,
            old_owner.clone(),
        )
        .unwrap();
        let retrieved = stored
            .clone_with_resource_owner(denied_owner.clone())
            .unwrap();
        assert!(retrieved.copy_server_cert_chain_for_current_owner().is_err());
        assert_eq!(denied_owner.used.load(Ordering::Relaxed), secret.len());
        drop(retrieved);
        assert_eq!(denied_owner.used.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn retained_certificate_chain_denies_before_arc_allocation() {
        let source = CertificateChain(vec![pki_types::CertificateDer::from(vec![1, 2, 3])]);
        let before_control = size_of::<pki_types::CertificateDer<'static>>() + 3;
        let owner = Arc::new(Owner {
            limit: before_control + RetainedCertificateChain::arc_layout().unwrap() - 1,
            used: AtomicUsize::new(0),
        });

        assert!(RetainedCertificateChain::deep_copy(&source, owner.clone()).is_err());
        assert_eq!(owner.used.load(Ordering::Relaxed), 0);
        assert_eq!(source[0].as_ref(), [1, 2, 3]);
    }
}

// --- Server types ---
#[derive(Debug)]
pub struct ServerSessionValue {
    pub(crate) sni: Option<DnsName<'static>>,
    pub(crate) version: ProtocolVersion,
    pub(crate) cipher_suite: CipherSuite,
    pub(crate) master_secret: Zeroizing<PayloadU8>,
    pub(crate) extended_ms: bool,
    pub(crate) client_cert_chain: Option<CertificateChain<'static>>,
    pub(crate) alpn: Option<PayloadU8>,
    pub(crate) application_data: PayloadU16,
    pub creation_time_sec: u64,
    pub(crate) age_obfuscation_offset: u32,
    freshness: Option<bool>,
}

impl Codec<'_> for ServerSessionValue {
    fn encode(&self, bytes: &mut Vec<u8>) {
        if let Some(sni) = &self.sni {
            1u8.encode(bytes);
            let sni_bytes: &str = sni.as_ref();
            PayloadU8::<MaybeEmpty>::encode_slice(sni_bytes.as_bytes(), bytes);
        } else {
            0u8.encode(bytes);
        }
        self.version.encode(bytes);
        self.cipher_suite.encode(bytes);
        self.master_secret.encode(bytes);
        (u8::from(self.extended_ms)).encode(bytes);
        if let Some(chain) = &self.client_cert_chain {
            1u8.encode(bytes);
            chain.encode(bytes);
        } else {
            0u8.encode(bytes);
        }
        if let Some(alpn) = &self.alpn {
            1u8.encode(bytes);
            alpn.encode(bytes);
        } else {
            0u8.encode(bytes);
        }
        self.application_data.encode(bytes);
        self.creation_time_sec.encode(bytes);
        self.age_obfuscation_offset
            .encode(bytes);
    }

    fn read(r: &mut Reader<'_>) -> Result<Self, InvalidMessage> {
        let has_sni = u8::read(r)?;
        let sni = if has_sni == 1 {
            let dns_name = PayloadU8::<MaybeEmpty>::read(r)?;
            let dns_name = match DnsName::try_from(dns_name.0.as_slice()) {
                Ok(dns_name) => dns_name.to_owned(),
                Err(_) => return Err(InvalidMessage::InvalidServerName),
            };

            Some(dns_name)
        } else {
            None
        };

        let v = ProtocolVersion::read(r)?;
        let cs = CipherSuite::read(r)?;
        let ms = Zeroizing::new(PayloadU8::read(r)?);
        let ems = u8::read(r)?;
        let has_ccert = u8::read(r)? == 1;
        let ccert = if has_ccert {
            Some(CertificateChain::read(r)?.into_owned())
        } else {
            None
        };
        let has_alpn = u8::read(r)? == 1;
        let alpn = if has_alpn {
            Some(PayloadU8::read(r)?)
        } else {
            None
        };
        let application_data = PayloadU16::read(r)?;
        let creation_time_sec = u64::read(r)?;
        let age_obfuscation_offset = u32::read(r)?;

        Ok(Self {
            sni,
            version: v,
            cipher_suite: cs,
            master_secret: ms,
            extended_ms: ems == 1u8,
            client_cert_chain: ccert,
            alpn,
            application_data,
            creation_time_sec,
            age_obfuscation_offset,
            freshness: None,
        })
    }
}

impl ServerSessionValue {
    pub(crate) fn new(
        sni: Option<&DnsName<'_>>,
        v: ProtocolVersion,
        cs: CipherSuite,
        ms: &[u8],
        client_cert_chain: Option<CertificateChain<'static>>,
        alpn: Option<ProtocolName>,
        application_data: Vec<u8>,
        creation_time: UnixTime,
        age_obfuscation_offset: u32,
    ) -> Self {
        Self {
            sni: sni.map(|dns| dns.to_owned()),
            version: v,
            cipher_suite: cs,
            master_secret: Zeroizing::new(PayloadU8::new(ms.to_vec())),
            extended_ms: false,
            client_cert_chain,
            alpn: alpn.map(|p| PayloadU8::new(p.as_ref().to_vec())),
            application_data: PayloadU16::new(application_data),
            creation_time_sec: creation_time.as_secs(),
            age_obfuscation_offset,
            freshness: None,
        }
    }

    #[cfg(feature = "tls12")]
    pub(crate) fn set_extended_ms_used(&mut self) {
        self.extended_ms = true;
    }

    pub(crate) fn set_freshness(
        mut self,
        obfuscated_client_age_ms: u32,
        time_now: UnixTime,
    ) -> Self {
        let client_age_ms = obfuscated_client_age_ms.wrapping_sub(self.age_obfuscation_offset);
        let server_age_ms = (time_now
            .as_secs()
            .saturating_sub(self.creation_time_sec) as u32)
            .saturating_mul(1000);

        let age_difference = server_age_ms.abs_diff(client_age_ms);

        self.freshness = Some(age_difference <= MAX_FRESHNESS_SKEW_MS);
        self
    }

    pub(crate) fn is_fresh(&self) -> bool {
        self.freshness.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "std")] // for UnixTime::now
    #[test]
    fn serversessionvalue_is_debug() {
        use std::{println, vec};
        let ssv = ServerSessionValue::new(
            None,
            ProtocolVersion::TLSv1_3,
            CipherSuite::TLS13_AES_128_GCM_SHA256,
            &[1, 2, 3],
            None,
            None,
            vec![4, 5, 6],
            UnixTime::now(),
            0x12345678,
        );
        println!("{ssv:?}");
    }

    #[test]
    fn serversessionvalue_no_sni() {
        let bytes = [
            0x00, 0x03, 0x03, 0xc0, 0x23, 0x03, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x12, 0x23, 0x34, 0x45, 0x56, 0x67, 0x78, 0x89, 0xfe, 0xed, 0xf0, 0x0d,
        ];
        let mut rd = Reader::init(&bytes);
        let ssv = ServerSessionValue::read(&mut rd).unwrap();
        assert_eq!(ssv.get_encoding(), bytes);
    }

    #[test]
    fn serversessionvalue_with_cert() {
        let bytes = [
            0x00, 0x03, 0x03, 0xc0, 0x23, 0x03, 0x01, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x12, 0x23, 0x34, 0x45, 0x56, 0x67, 0x78, 0x89, 0xfe, 0xed, 0xf0, 0x0d,
        ];
        let mut rd = Reader::init(&bytes);
        let ssv = ServerSessionValue::read(&mut rd).unwrap();
        assert_eq!(ssv.get_encoding(), bytes);
    }
}
