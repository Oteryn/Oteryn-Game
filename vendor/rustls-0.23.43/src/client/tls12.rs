use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use pki_types::ServerName;
pub(super) use server_hello::CompleteServerHelloHandling;
use subtle::ConstantTimeEq;

use super::client_conn::ClientConnectionData;
use super::hs::ClientContext;
use crate::ConnectionTrafficSecrets;
use crate::check::{inappropriate_handshake_message, inappropriate_message};
use crate::client::common::{ClientAuthDetails, ServerCertDetails};
use crate::client::{ClientConfig, hs};
use crate::common_state::{CommonState, HandshakeKind, KxState, Side, State};
use crate::conn::ConnectionRandoms;
use crate::conn::kernel::{Direction, KernelContext, KernelState};
use crate::crypto::KeyExchangeAlgorithm;
use crate::enums::{AlertDescription, ContentType, HandshakeType, ProtocolVersion};
use crate::error::{Error, InvalidMessage, PeerIncompatible, PeerMisbehaved};
use crate::hash_hs::HandshakeHash;
use crate::log::{debug, trace, warn};
use crate::msgs::base::{Payload, PayloadU8, PayloadU16};
use crate::msgs::ccs::ChangeCipherSpecPayload;
use crate::msgs::handshake::{
    CertificateChain, ClientDhParams, ClientEcdhParams, ClientKeyExchangeParams,
    HandshakeMessagePayload, HandshakePayload, NewSessionTicketPayload,
    NewSessionTicketPayloadTls13, ServerKeyExchangeParams, SessionId,
};
use crate::msgs::message::{Message, MessagePayload};
use crate::msgs::persist;
use crate::sign::Signer;
use crate::suites::{PartiallyExtractedSecrets, SupportedCipherSuite};
use crate::sync::Arc;
use crate::tls12::{self, ConnectionSecrets, Tls12CipherSuite};
use crate::verify::{self, DigitallySignedStruct};

mod server_hello {
    use super::*;
    use crate::client::hs::{ClientHelloInput, ClientSessionValue};
    use crate::msgs::handshake::ServerHelloPayload;

    pub(in crate::client) struct CompleteServerHelloHandling {
        pub(in crate::client) randoms: ConnectionRandoms,
        pub(in crate::client) transcript: HandshakeHash,
        pub(in crate::client) input: ClientHelloInput,
    }

    impl CompleteServerHelloHandling {
        pub(in crate::client) fn handle_server_hello(
            mut self,
            cx: &mut ClientContext<'_>,
            suite: &'static Tls12CipherSuite,
            server_hello: &ServerHelloPayload,
            tls13_supported: bool,
        ) -> hs::NextStateOrError<'static> {
            self.randoms
                .server
                .clone_from_slice(&server_hello.random.0[..]);

            // Look for TLS1.3 downgrade signal in server random
            // both the server random and TLS12_DOWNGRADE_SENTINEL are
            // public values and don't require constant time comparison
            let has_downgrade_marker = self.randoms.server[24..] == tls12::DOWNGRADE_SENTINEL;
            if tls13_supported && has_downgrade_marker {
                return Err({
                    cx.common.send_fatal_alert(
                        AlertDescription::IllegalParameter,
                        PeerMisbehaved::AttemptedDowngradeToTls12WhenTls13IsSupported,
                    )
                });
            }

            // If we didn't have an input session to resume, and we sent a session ID,
            // that implies we sent a TLS 1.3 legacy_session_id for compatibility purposes.
            // In this instance since we're now continuing a TLS 1.2 handshake the server
            // should not have echoed it back: it's a randomly generated session ID it couldn't
            // have known.
            if self.input.resuming.is_none()
                && !self.input.session_id.is_empty()
                && self.input.session_id == server_hello.session_id
            {
                return Err({
                    cx.common.send_fatal_alert(
                        AlertDescription::IllegalParameter,
                        PeerMisbehaved::ServerEchoedCompatibilitySessionId,
                    )
                });
            }

            let ClientHelloInput {
                config,
                server_name,
                ..
            } = self.input;

            let resuming_session = self
                .input
                .resuming
                .and_then(|resuming| match resuming.value {
                    ClientSessionValue::Tls12(inner) => Some(inner),
                    ClientSessionValue::Tls13(_) => None,
                });

            // Doing EMS?
            let using_ems = server_hello
                .extended_master_secret_ack
                .is_some();
            if config.require_ems && !using_ems {
                return Err({
                    cx.common.send_fatal_alert(
                        AlertDescription::HandshakeFailure,
                        PeerIncompatible::ExtendedMasterSecretExtensionRequired,
                    )
                });
            }

            // Might the server send a ticket?
            let must_issue_new_ticket = if server_hello
                .session_ticket_ack
                .is_some()
            {
                debug!("Server supports tickets");
                true
            } else {
                false
            };

            // Might the server send a CertificateStatus between Certificate and
            // ServerKeyExchange?
            let may_send_cert_status = server_hello
                .certificate_status_request_ack
                .is_some();
            if may_send_cert_status {
                debug!("Server may staple OCSP response");
            }

            // See if we're successfully resuming.
            if let Some(resuming) = resuming_session {
                if resuming.session_id == server_hello.session_id {
                    debug!("Server agreed to resume");

                    // Is the server telling lies about the ciphersuite?
                    if resuming.suite() != suite {
                        return Err(PeerMisbehaved::ResumptionOfferedWithVariedCipherSuite.into());
                    }

                    // And about EMS support?
                    if resuming.extended_ms() != using_ems {
                        return Err(PeerMisbehaved::ResumptionOfferedWithVariedEms.into());
                    }

                    let secrets =
                        ConnectionSecrets::new_resume(self.randoms, suite, resuming.secret());
                    config.key_log.log(
                        "CLIENT_RANDOM",
                        &secrets.randoms.client,
                        &secrets.master_secret,
                    );
                    cx.common
                        .start_encryption_tls12(&secrets, Side::Client);

                    // Since we're resuming, we verified the certificate and
                    // proof of possession in the prior session.
                    #[cfg(feature = "std")]
                    {
                        let (peer_certificates, custody) = resuming
                            .copy_server_cert_chain_for_current_owner()
                            .map_err(|_| InvalidMessage::MessageTooLarge)?;
                        cx.common.peer_certificates = Some(peer_certificates);
                        cx.common.peer_certificate_custody = Some(custody.into());
                    }
                    #[cfg(not(feature = "std"))]
                    {
                        cx.common.peer_certificates = Some(
                            resuming
                                .server_cert_chain()
                                .clone()
                                .into_owned(),
                        );
                    }
                    cx.common.handshake_kind = Some(HandshakeKind::Resumed);
                    let cert_verified = verify::ServerCertVerified::assertion();
                    let sig_verified = verify::HandshakeSignatureValid::assertion();

                    return if must_issue_new_ticket {
                        Ok(Box::new(ExpectNewTicket {
                            config,
                            secrets,
                            resuming_session: Some(resuming),
                            session_id: server_hello.session_id,
                            server_name,
                            using_ems,
                            transcript: self.transcript,
                            resuming: true,
                            cert_verified,
                            sig_verified,
                        }))
                    } else {
                        Ok(Box::new(ExpectCcs {
                            config,
                            secrets,
                            resuming_session: Some(resuming),
                            session_id: server_hello.session_id,
                            server_name,
                            using_ems,
                            transcript: self.transcript,
                            ticket: None,
                            resuming: true,
                            cert_verified,
                            sig_verified,
                        }))
                    };
                }
            }

            cx.common.handshake_kind = Some(HandshakeKind::Full);
            Ok(Box::new(ExpectCertificate {
                config,
                resuming_session: None,
                session_id: server_hello.session_id,
                server_name,
                randoms: self.randoms,
                using_ems,
                transcript: self.transcript,
                suite,
                may_send_cert_status,
                must_issue_new_ticket,
            }))
        }
    }
}

struct ExpectCertificate {
    config: Arc<ClientConfig>,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    randoms: ConnectionRandoms,
    using_ems: bool,
    transcript: HandshakeHash,
    pub(super) suite: &'static Tls12CipherSuite,
    may_send_cert_status: bool,
    must_issue_new_ticket: bool,
}

impl State<ClientConnectionData> for ExpectCertificate {
    fn handle<'m>(
        mut self: Box<Self>,
        _cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        self.transcript.add_message(&m);
        #[cfg(feature = "std")]
        let decoded_owner = m.decoded_owner();
        let server_cert_chain = require_handshake_msg_move!(
            m,
            HandshakeType::Certificate,
            HandshakePayload::Certificate
        )?;

        #[cfg(feature = "std")]
        let server_cert = if let Some(owner) = decoded_owner {
            let (chain, custody) = server_cert_chain.into_owned_with_resource_owner(owner)?;
            ServerCertDetails::new_with_resource_custody(chain, vec![], custody)
        } else {
            ServerCertDetails::new(server_cert_chain, vec![])
        };
        #[cfg(not(feature = "std"))]
        let server_cert = ServerCertDetails::new(server_cert_chain, vec![]);

        if self.may_send_cert_status {
            Ok(Box::new(ExpectCertificateStatusOrServerKx {
                config: self.config,
                resuming_session: self.resuming_session,
                session_id: self.session_id,
                server_name: self.server_name,
                randoms: self.randoms,
                using_ems: self.using_ems,
                transcript: self.transcript,
                suite: self.suite,
                server_cert,
                must_issue_new_ticket: self.must_issue_new_ticket,
            }))
        } else {
            Ok(Box::new(ExpectServerKx {
                config: self.config,
                resuming_session: self.resuming_session,
                session_id: self.session_id,
                server_name: self.server_name,
                randoms: self.randoms,
                using_ems: self.using_ems,
                transcript: self.transcript,
                suite: self.suite,
                server_cert,
                must_issue_new_ticket: self.must_issue_new_ticket,
            }))
        }
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        self
    }
}

struct ExpectCertificateStatusOrServerKx<'m> {
    config: Arc<ClientConfig>,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    randoms: ConnectionRandoms,
    using_ems: bool,
    transcript: HandshakeHash,
    suite: &'static Tls12CipherSuite,
    server_cert: ServerCertDetails<'m>,
    must_issue_new_ticket: bool,
}

impl State<ClientConnectionData> for ExpectCertificateStatusOrServerKx<'_> {
    fn handle<'m>(
        self: Box<Self>,
        cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        match m.payload {
            MessagePayload::Handshake {
                parsed: HandshakeMessagePayload(HandshakePayload::ServerKeyExchange(..)),
                ..
            } => Box::new(ExpectServerKx {
                config: self.config,
                resuming_session: self.resuming_session,
                session_id: self.session_id,
                server_name: self.server_name,
                randoms: self.randoms,
                using_ems: self.using_ems,
                transcript: self.transcript,
                suite: self.suite,
                server_cert: self.server_cert,
                must_issue_new_ticket: self.must_issue_new_ticket,
            })
            .handle(cx, m),
            MessagePayload::Handshake {
                parsed: HandshakeMessagePayload(HandshakePayload::CertificateStatus(..)),
                ..
            } => Box::new(ExpectCertificateStatus {
                config: self.config,
                resuming_session: self.resuming_session,
                session_id: self.session_id,
                server_name: self.server_name,
                randoms: self.randoms,
                using_ems: self.using_ems,
                transcript: self.transcript,
                suite: self.suite,
                server_cert: self.server_cert,
                must_issue_new_ticket: self.must_issue_new_ticket,
            })
            .handle(cx, m),
            payload => Err(inappropriate_handshake_message(
                &payload,
                &[ContentType::Handshake],
                &[
                    HandshakeType::ServerKeyExchange,
                    HandshakeType::CertificateStatus,
                ],
            )),
        }
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        Box::new(ExpectCertificateStatusOrServerKx {
            config: self.config,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            randoms: self.randoms,
            using_ems: self.using_ems,
            transcript: self.transcript,
            suite: self.suite,
            server_cert: self.server_cert.into_owned(),
            must_issue_new_ticket: self.must_issue_new_ticket,
        })
    }
}

struct ExpectCertificateStatus<'a> {
    config: Arc<ClientConfig>,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    randoms: ConnectionRandoms,
    using_ems: bool,
    transcript: HandshakeHash,
    suite: &'static Tls12CipherSuite,
    server_cert: ServerCertDetails<'a>,
    must_issue_new_ticket: bool,
}

impl State<ClientConnectionData> for ExpectCertificateStatus<'_> {
    fn handle<'m>(
        mut self: Box<Self>,
        _cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        self.transcript.add_message(&m);
        let server_cert_status = require_handshake_msg_move!(
            m,
            HandshakeType::CertificateStatus,
            HandshakePayload::CertificateStatus
        )?
        ;

        #[cfg(feature = "std")]
        let mut server_cert = self.server_cert;
        #[cfg(feature = "std")]
        server_cert.set_ocsp_with_resource_owner(server_cert_status.ocsp_response.0.bytes())?;
        #[cfg(not(feature = "std"))]
        let server_cert = {
            let chain = self.server_cert.into_peer_certificates();
            ServerCertDetails::new(chain, server_cert_status.into_inner())
        };

        trace!(
            "Server stapled OCSP response is {:?}",
            server_cert.ocsp_response
        );

        Ok(Box::new(ExpectServerKx {
            config: self.config,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            randoms: self.randoms,
            using_ems: self.using_ems,
            transcript: self.transcript,
            suite: self.suite,
            server_cert,
            must_issue_new_ticket: self.must_issue_new_ticket,
        }))
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        Box::new(ExpectCertificateStatus {
            config: self.config,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            randoms: self.randoms,
            using_ems: self.using_ems,
            transcript: self.transcript,
            suite: self.suite,
            server_cert: self.server_cert.into_owned(),
            must_issue_new_ticket: self.must_issue_new_ticket,
        })
    }
}

struct ExpectServerKx<'a> {
    config: Arc<ClientConfig>,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    randoms: ConnectionRandoms,
    using_ems: bool,
    transcript: HandshakeHash,
    suite: &'static Tls12CipherSuite,
    server_cert: ServerCertDetails<'a>,
    must_issue_new_ticket: bool,
}

impl State<ClientConnectionData> for ExpectServerKx<'_> {
    fn handle<'m>(
        mut self: Box<Self>,
        cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        let opaque_kx = require_handshake_msg!(
            m,
            HandshakeType::ServerKeyExchange,
            HandshakePayload::ServerKeyExchange
        )?;
        #[cfg(feature = "std")]
        let resource_owner = m.decoded_owner().map(|owner| owner.owner());
        self.transcript.add_message(&m);

        let kx = opaque_kx.unwrap_given_kxa(self.suite.kx).ok_or_else(|| {
            cx.common.send_fatal_alert(
                AlertDescription::DecodeError,
                InvalidMessage::MissingKeyExchange,
            )
        })?;

        // Save the signature and signed parameters for later verification.
        #[cfg(feature = "std")]
        let server_kx = if let Some(owner) = resource_owner {
            ServerKxDetails::new_with_resource_owner(&kx.params, kx.dss, owner)?
        } else {
            ServerKxDetails::new(&kx.params, kx.dss)
        };
        #[cfg(not(feature = "std"))]
        let server_kx = ServerKxDetails::new(&kx.params, kx.dss);

        #[cfg_attr(not(feature = "logging"), allow(unused_variables))]
        {
            match &kx.params {
                ServerKeyExchangeParams::Ecdh(ecdhe) => {
                    debug!("ECDHE curve is {:?}", ecdhe.curve_params)
                }
                ServerKeyExchangeParams::Dh(dhe) => {
                    debug!("DHE params are p = {:?}, g = {:?}", dhe.dh_p, dhe.dh_g)
                }
            }
        }

        Ok(Box::new(ExpectServerDoneOrCertReq {
            config: self.config,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            randoms: self.randoms,
            using_ems: self.using_ems,
            transcript: self.transcript,
            suite: self.suite,
            server_cert: self.server_cert,
            server_kx,
            must_issue_new_ticket: self.must_issue_new_ticket,
        }))
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        Box::new(ExpectServerKx {
            config: self.config,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            randoms: self.randoms,
            using_ems: self.using_ems,
            transcript: self.transcript,
            suite: self.suite,
            server_cert: self.server_cert.into_owned(),
            must_issue_new_ticket: self.must_issue_new_ticket,
        })
    }
}

fn emit_certificate(
    transcript: &mut HandshakeHash,
    cert_chain: CertificateChain<'static>,
    common: &mut CommonState,
) {
    let cert = Message::new(ProtocolVersion::TLSv1_2, MessagePayload::handshake(HandshakeMessagePayload(HandshakePayload::Certificate(
            cert_chain,
        ))));

    transcript.add_message(&cert);
    common.send_msg(cert, false);
}

fn emit_client_kx(
    transcript: &mut HandshakeHash,
    kxa: KeyExchangeAlgorithm,
    common: &mut CommonState,
    pub_key: &[u8],
) -> Result<(), Error> {
    #[cfg(feature = "std")]
    if let Some(owner) = common
        .peer_certificate_custody
        .as_ref()
        .map(|custody| custody.resource_owner())
    {
        use crate::msgs::codec::{exact_vec_copy, Codec, DirectDecodedCustody};
        use crate::vecbuf::OutboundTlsCustody;

        // ExpectServerDone has transferred the authenticated certificate's
        // custody to CommonState before this call: reuse that SAME ledger.
        let prefix_len = match kxa {
            KeyExchangeAlgorithm::ECDHE => 1usize,
            KeyExchangeAlgorithm::DHE => 2usize,
        };
        let body_len = prefix_len
            .checked_add(pub_key.len())
            .ok_or(InvalidMessage::MessageTooLarge)?;
        let encoded_len = 4usize
            .checked_add(body_len)
            .ok_or(InvalidMessage::MessageTooLarge)?;

        // Declare each debit before its backing so every error/unwind destroys
        // the backing first. The provider's borrowed key remains with the KX.
        let source_custody = DirectDecodedCustody::reserve(owner.clone(), pub_key.len())?;
        let public = exact_vec_copy(pub_key);
        if public.capacity() != pub_key.len() {
            return Err(InvalidMessage::MessageTooLarge.into());
        }
        let params = match kxa {
            KeyExchangeAlgorithm::ECDHE => ClientKeyExchangeParams::Ecdh(ClientEcdhParams {
                public: PayloadU8::new(public),
            }),
            KeyExchangeAlgorithm::DHE => ClientKeyExchangeParams::Dh(ClientDhParams {
                public: PayloadU16::new(public),
            }),
        };
        let body_custody = DirectDecodedCustody::reserve(owner.clone(), body_len)?;
        let mut body = Vec::with_capacity(body_len);
        if body.capacity() != body_len {
            return Err(InvalidMessage::MessageTooLarge.into());
        }
        params.encode(&mut body);
        if body.len() != body_len || body.capacity() != body_len {
            return Err(InvalidMessage::MessageTooLarge.into());
        }
        drop(params);
        drop(source_custody);

        let parsed =
            HandshakeMessagePayload(HandshakePayload::ClientKeyExchange(Payload::new(body)));
        let encoded_custody = DirectDecodedCustody::reserve(owner.clone(), encoded_len)?;
        let mut encoded = Vec::with_capacity(encoded_len);
        if encoded.capacity() != encoded_len {
            return Err(InvalidMessage::MessageTooLarge.into());
        }
        // Use the existing encoder into pre-funded exact-capacity backing;
        // MessagePayload::handshake would allocate its own unreserved Vec.
        parsed.encode(&mut encoded);
        if encoded.len() != encoded_len || encoded.capacity() != encoded_len {
            return Err(InvalidMessage::MessageTooLarge.into());
        }
        let ckx = Message::new(
            ProtocolVersion::TLSv1_2,
            MessagePayload::Handshake {
                parsed,
                encoded: Payload::new(encoded),
            },
        );
        let outbound = DirectDecodedCustody::reserve(owner, common.client_kx_outbound_len(&ckx)?)?;
        transcript.try_add_message(&ckx)?;
        common.send_msg_with_custody(&ckx, OutboundTlsCustody::from_reserved(outbound))?;
        // The transcript copied/hashed the bytes and the queue owns separate
        // funded records. Release these debits only after both message buffers.
        drop(ckx);
        drop(encoded_custody);
        drop(body_custody);
        return Ok(());
    }

    let mut buf = Vec::new();
    match kxa {
        KeyExchangeAlgorithm::ECDHE => ClientKeyExchangeParams::Ecdh(ClientEcdhParams {
            public: PayloadU8::new(pub_key.to_vec()),
        }),
        KeyExchangeAlgorithm::DHE => ClientKeyExchangeParams::Dh(ClientDhParams {
            public: PayloadU16::new(pub_key.to_vec()),
        }),
    }
    .encode(&mut buf);
    let pubkey = Payload::new(buf);

    let ckx = Message::new(ProtocolVersion::TLSv1_2, MessagePayload::handshake(HandshakeMessagePayload(
            HandshakePayload::ClientKeyExchange(pubkey),
        )));

    transcript.add_message(&ckx);
    common.send_msg(ckx, false);
    Ok(())
}

fn emit_certverify(
    transcript: &mut HandshakeHash,
    signer: &dyn Signer,
    common: &mut CommonState,
) -> Result<(), Error> {
    let message = transcript
        .take_handshake_buf()
        .ok_or_else(|| Error::General("Expected transcript".to_owned()))?;

    let scheme = signer.scheme();
    let sig = signer.sign(&message)?;
    let body = DigitallySignedStruct::new(scheme, sig);

    let m = Message::new(ProtocolVersion::TLSv1_2, MessagePayload::handshake(HandshakeMessagePayload(
            HandshakePayload::CertificateVerify(body),
        )));

    transcript.add_message(&m);
    common.send_msg(m, false);
    Ok(())
}

fn emit_ccs(common: &mut CommonState) {
    let ccs = Message::new(ProtocolVersion::TLSv1_2, MessagePayload::ChangeCipherSpec(ChangeCipherSpecPayload {}));

    common.send_msg(ccs, false);
}

fn emit_finished(
    secrets: &ConnectionSecrets,
    transcript: &mut HandshakeHash,
    common: &mut CommonState,
) {
    let vh = transcript.current_hash();
    let verify_data = secrets.client_verify_data(&vh);
    let verify_data_payload = Payload::new(verify_data);

    let f = Message::new(ProtocolVersion::TLSv1_2, MessagePayload::handshake(HandshakeMessagePayload(HandshakePayload::Finished(
            verify_data_payload,
        ))));

    transcript.add_message(&f);
    common.send_msg(f, true);
}

struct ServerKxDetails {
    kx_params: Vec<u8>,
    kx_sig: DigitallySignedStruct,
    #[cfg(feature = "std")]
    _kx_params_custody: Option<ServerKxParamsCustody>,
}

impl ServerKxDetails {
    fn new(params: &ServerKeyExchangeParams, sig: DigitallySignedStruct) -> Self {
        let mut kx_params = Vec::new();
        params.encode(&mut kx_params);
        Self {
            kx_params,
            kx_sig: sig,
            #[cfg(feature = "std")]
            _kx_params_custody: None,
        }
    }

    #[cfg(feature = "std")]
    fn new_with_resource_owner(
        params: &ServerKeyExchangeParams,
        sig: DigitallySignedStruct,
        owner: Arc<dyn crate::DeframerBufferOwner>,
    ) -> Result<Self, InvalidMessage> {
        let capacity = server_kx_params_encoded_len(params)?;
        let custody = ServerKxParamsCustody::reserve(owner, capacity)?;
        let mut kx_params = Vec::with_capacity(capacity);
        params.encode(&mut kx_params);
        if kx_params.capacity() != capacity || kx_params.len() != capacity {
            drop(kx_params);
            return Err(InvalidMessage::MessageTooLarge);
        }

        Ok(Self {
            kx_params,
            kx_sig: sig,
            _kx_params_custody: Some(custody),
        })
    }
}

#[cfg(feature = "std")]
fn server_kx_params_encoded_len(params: &ServerKeyExchangeParams) -> Result<usize, InvalidMessage> {
    match params {
        ServerKeyExchangeParams::Ecdh(ecdh) => 4usize
            .checked_add(ecdh.public.0.len())
            .ok_or(InvalidMessage::MessageTooLarge),
        ServerKeyExchangeParams::Dh(dh) => [dh.dh_p.0.len(), dh.dh_g.0.len(), dh.dh_Ys.0.len()]
            .into_iter()
            .try_fold(0usize, |total, len| {
                total
                    .checked_add(2)
                    .and_then(|total| total.checked_add(len))
                    .ok_or(InvalidMessage::MessageTooLarge)
            }),
    }
}

#[cfg(feature = "std")]
#[derive(Debug)]
struct ServerKxParamsCustody {
    owner: Arc<dyn crate::DeframerBufferOwner>,
    bytes: usize,
}

#[cfg(feature = "std")]
impl ServerKxParamsCustody {
    fn reserve(
        owner: Arc<dyn crate::DeframerBufferOwner>,
        bytes: usize,
    ) -> Result<Self, InvalidMessage> {
        owner
            .try_reserve(bytes)
            .map_err(|_| InvalidMessage::MessageTooLarge)?;
        Ok(Self { owner, bytes })
    }
}

#[cfg(feature = "std")]
impl Drop for ServerKxParamsCustody {
    fn drop(&mut self) {
        self.owner.release(self.bytes);
    }
}

// --- Either a CertificateRequest, or a ServerHelloDone. ---
// Existence of the CertificateRequest tells us the server is asking for
// client auth.  Otherwise we go straight to ServerHelloDone.
struct ExpectServerDoneOrCertReq<'a> {
    config: Arc<ClientConfig>,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    randoms: ConnectionRandoms,
    using_ems: bool,
    transcript: HandshakeHash,
    suite: &'static Tls12CipherSuite,
    server_cert: ServerCertDetails<'a>,
    server_kx: ServerKxDetails,
    must_issue_new_ticket: bool,
}

impl State<ClientConnectionData> for ExpectServerDoneOrCertReq<'_> {
    fn handle<'m>(
        mut self: Box<Self>,
        cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        if matches!(
            m.payload,
            MessagePayload::Handshake {
                parsed: HandshakeMessagePayload(HandshakePayload::CertificateRequest(_)),
                ..
            }
        ) {
            Box::new(ExpectCertificateRequest {
                config: self.config,
                resuming_session: self.resuming_session,
                session_id: self.session_id,
                server_name: self.server_name,
                randoms: self.randoms,
                using_ems: self.using_ems,
                transcript: self.transcript,
                suite: self.suite,
                server_cert: self.server_cert,
                server_kx: self.server_kx,
                must_issue_new_ticket: self.must_issue_new_ticket,
            })
            .handle(cx, m)
        } else {
            self.transcript.abandon_client_auth();

            Box::new(ExpectServerDone {
                config: self.config,
                resuming_session: self.resuming_session,
                session_id: self.session_id,
                server_name: self.server_name,
                randoms: self.randoms,
                using_ems: self.using_ems,
                transcript: self.transcript,
                suite: self.suite,
                server_cert: self.server_cert,
                server_kx: self.server_kx,
                client_auth: None,
                must_issue_new_ticket: self.must_issue_new_ticket,
            })
            .handle(cx, m)
        }
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        Box::new(ExpectServerDoneOrCertReq {
            config: self.config,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            randoms: self.randoms,
            using_ems: self.using_ems,
            transcript: self.transcript,
            suite: self.suite,
            server_cert: self.server_cert.into_owned(),
            server_kx: self.server_kx,
            must_issue_new_ticket: self.must_issue_new_ticket,
        })
    }
}

struct ExpectCertificateRequest<'a> {
    config: Arc<ClientConfig>,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    randoms: ConnectionRandoms,
    using_ems: bool,
    transcript: HandshakeHash,
    suite: &'static Tls12CipherSuite,
    server_cert: ServerCertDetails<'a>,
    server_kx: ServerKxDetails,
    must_issue_new_ticket: bool,
}

impl State<ClientConnectionData> for ExpectCertificateRequest<'_> {
    fn handle<'m>(
        mut self: Box<Self>,
        _cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        let certreq = require_handshake_msg!(
            m,
            HandshakeType::CertificateRequest,
            HandshakePayload::CertificateRequest
        )?;
        self.transcript.add_message(&m);
        debug!("Got CertificateRequest {certreq:?}");

        // The RFC jovially describes the design here as 'somewhat complicated'
        // and 'somewhat underspecified'.  So thanks for that.
        //
        // We ignore certreq.certtypes as a result, since the information it contains
        // is entirely duplicated in certreq.sigschemes.

        const NO_CONTEXT: Option<Vec<u8>> = None; // TLS 1.2 doesn't use a context.
        let no_compression = None; // or compression
        let client_auth = ClientAuthDetails::resolve(
            self.config
                .client_auth_cert_resolver
                .as_ref(),
            Some(&certreq.canames),
            &certreq.sigschemes,
            NO_CONTEXT,
            no_compression,
        );

        Ok(Box::new(ExpectServerDone {
            config: self.config,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            randoms: self.randoms,
            using_ems: self.using_ems,
            transcript: self.transcript,
            suite: self.suite,
            server_cert: self.server_cert,
            server_kx: self.server_kx,
            client_auth: Some(client_auth),
            must_issue_new_ticket: self.must_issue_new_ticket,
        }))
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        Box::new(ExpectCertificateRequest {
            config: self.config,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            randoms: self.randoms,
            using_ems: self.using_ems,
            transcript: self.transcript,
            suite: self.suite,
            server_cert: self.server_cert.into_owned(),
            server_kx: self.server_kx,
            must_issue_new_ticket: self.must_issue_new_ticket,
        })
    }
}

struct ExpectServerDone<'a> {
    config: Arc<ClientConfig>,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    randoms: ConnectionRandoms,
    using_ems: bool,
    transcript: HandshakeHash,
    suite: &'static Tls12CipherSuite,
    server_cert: ServerCertDetails<'a>,
    server_kx: ServerKxDetails,
    client_auth: Option<ClientAuthDetails>,
    must_issue_new_ticket: bool,
}

impl State<ClientConnectionData> for ExpectServerDone<'_> {
    fn handle<'m>(
        self: Box<Self>,
        cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        match m.payload {
            MessagePayload::Handshake {
                parsed: HandshakeMessagePayload(HandshakePayload::ServerHelloDone),
                ..
            } => {}
            payload => {
                return Err(inappropriate_handshake_message(
                    &payload,
                    &[ContentType::Handshake],
                    &[HandshakeType::ServerHelloDone],
                ));
            }
        }

        let mut st = *self;
        st.transcript.add_message(&m);

        cx.common.check_aligned_handshake()?;

        trace!("Server cert is {:?}", st.server_cert.cert_chain);
        debug!("Server DNS name is {:?}", st.server_name);

        let suite = st.suite;

        // 1. Verify the cert chain.
        // 2. Verify that the top certificate signed their kx.
        // 3. If doing client auth, send our Certificate.
        // 4. Complete the key exchange:
        //    a) generate our kx pair
        //    b) emit a ClientKeyExchange containing it
        //    c) if doing client auth, emit a CertificateVerify
        //    d) derive the shared keys
        //    e) emit a CCS
        //    f) use the derived keys to start encryption
        // 5. emit a Finished, our first encrypted message under the new keys.

        // 1.
        let (end_entity, intermediates) = st
            .server_cert
            .cert_chain
            .split_first()
            .ok_or(Error::NoCertificatesPresented)?;

        let now = st.config.current_time()?;

        let cert_verified = st
            .config
            .verifier
            .verify_server_cert(
                end_entity,
                intermediates,
                &st.server_name,
                &st.server_cert.ocsp_response,
                now,
            )
            .map_err(|err| {
                cx.common
                    .send_cert_verify_error_alert(err)
            })?;

        // 2.
        // Build up the contents of the signed message.
        // It's ClientHello.random || ServerHello.random || ServerKeyExchange.params
        let sig_verified = {
            let mut message = Vec::new();
            message.extend_from_slice(&st.randoms.client);
            message.extend_from_slice(&st.randoms.server);
            message.extend_from_slice(&st.server_kx.kx_params);

            // Check the signature is compatible with the ciphersuite.
            let sig = &st.server_kx.kx_sig;
            if !SupportedCipherSuite::from(suite)
                .usable_for_signature_algorithm(sig.scheme.algorithm())
            {
                warn!(
                    "peer signed kx with wrong algorithm (got {:?} expect {:?})",
                    sig.scheme.algorithm(),
                    suite.sign
                );
                return Err(PeerMisbehaved::SignedKxWithWrongAlgorithm.into());
            }

            st.config
                .verifier
                .verify_tls12_signature(&message, end_entity, sig)
                .map_err(|err| {
                    cx.common
                        .send_cert_verify_error_alert(err)
                })?
        };
        #[cfg(feature = "std")]
        {
            let (peer_certificates, custody) = st.server_cert.take_peer_certificates();
            cx.common.peer_certificates = Some(peer_certificates);
            cx.common.peer_certificate_custody = custody.map(Into::into);
        }
        #[cfg(not(feature = "std"))]
        {
            cx.common.peer_certificates = Some(st.server_cert.into_peer_certificates());
        }

        // 3.
        if let Some(client_auth) = &st.client_auth {
            let certs = match client_auth {
                ClientAuthDetails::Empty { .. } => CertificateChain::default(),
                ClientAuthDetails::Verify { certkey, .. } => CertificateChain(certkey.cert.clone()),
            };
            emit_certificate(&mut st.transcript, certs, cx.common);
        }

        // 4a.
        let kx_params = tls12::decode_kx_params::<ServerKeyExchangeParams>(
            st.suite.kx,
            cx.common,
            &st.server_kx.kx_params,
        )?;
        let maybe_skxg = match &kx_params {
            ServerKeyExchangeParams::Ecdh(ecdh) => st
                .config
                .find_kx_group(ecdh.curve_params.named_group, ProtocolVersion::TLSv1_2),
            ServerKeyExchangeParams::Dh(dh) => {
                let ffdhe_group = dh.as_ffdhe_group();

                st.config
                    .provider
                    .kx_groups
                    .iter()
                    .find(|kxg| kxg.ffdhe_group() == Some(ffdhe_group))
                    .copied()
            }
        };
        let Some(skxg) = maybe_skxg else {
            return Err(cx.common.send_fatal_alert(
                AlertDescription::IllegalParameter,
                PeerMisbehaved::SelectedUnofferedKxGroup,
            ));
        };
        cx.common.kx_state = KxState::Start(skxg);
        let kx = skxg.start()?;

        // 4b.
        let mut transcript = st.transcript;
        emit_client_kx(&mut transcript, st.suite.kx, cx.common, kx.pub_key())?;
        // Note: EMS handshake hash only runs up to ClientKeyExchange.
        let ems_seed = st
            .using_ems
            .then(|| transcript.current_hash());

        // 4c.
        if let Some(ClientAuthDetails::Verify { signer, .. }) = &st.client_auth {
            emit_certverify(&mut transcript, signer.as_ref(), cx.common)?;
        }

        // 4d. Derive secrets.
        // An alert at this point will be sent in plaintext.  That must happen
        // prior to the CCS, or else the peer will try to decrypt it.
        let secrets = ConnectionSecrets::from_key_exchange(
            kx,
            kx_params.pub_key(),
            ems_seed,
            st.randoms,
            suite,
        )
        .map_err(|err| {
            cx.common
                .send_fatal_alert(AlertDescription::IllegalParameter, err)
        })?;
        cx.common.kx_state.complete();

        // 4e. CCS. We are definitely going to switch on encryption.
        emit_ccs(cx.common);

        // 4f. Now commit secrets.
        st.config.key_log.log(
            "CLIENT_RANDOM",
            &secrets.randoms.client,
            &secrets.master_secret,
        );
        cx.common
            .start_encryption_tls12(&secrets, Side::Client);
        cx.common
            .record_layer
            .start_encrypting();

        // 5.
        emit_finished(&secrets, &mut transcript, cx.common);

        if st.must_issue_new_ticket {
            Ok(Box::new(ExpectNewTicket {
                config: st.config,
                secrets,
                resuming_session: st.resuming_session,
                session_id: st.session_id,
                server_name: st.server_name,
                using_ems: st.using_ems,
                transcript,
                resuming: false,
                cert_verified,
                sig_verified,
            }))
        } else {
            Ok(Box::new(ExpectCcs {
                config: st.config,
                secrets,
                resuming_session: st.resuming_session,
                session_id: st.session_id,
                server_name: st.server_name,
                using_ems: st.using_ems,
                transcript,
                ticket: None,
                resuming: false,
                cert_verified,
                sig_verified,
            }))
        }
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        Box::new(ExpectServerDone {
            config: self.config,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            randoms: self.randoms,
            using_ems: self.using_ems,
            transcript: self.transcript,
            suite: self.suite,
            server_cert: self.server_cert.into_owned(),
            server_kx: self.server_kx,
            client_auth: self.client_auth,
            must_issue_new_ticket: self.must_issue_new_ticket,
        })
    }
}

struct ExpectNewTicket {
    config: Arc<ClientConfig>,
    secrets: ConnectionSecrets,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    using_ems: bool,
    transcript: HandshakeHash,
    resuming: bool,
    cert_verified: verify::ServerCertVerified,
    sig_verified: verify::HandshakeSignatureValid,
}

impl State<ClientConnectionData> for ExpectNewTicket {
    fn handle<'m>(
        mut self: Box<Self>,
        _cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        self.transcript.add_message(&m);

        let nst = require_handshake_msg_move!(
            m,
            HandshakeType::NewSessionTicket,
            HandshakePayload::NewSessionTicket
        )?;

        Ok(Box::new(ExpectCcs {
            config: self.config,
            secrets: self.secrets,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            using_ems: self.using_ems,
            transcript: self.transcript,
            ticket: Some(nst),
            resuming: self.resuming,
            cert_verified: self.cert_verified,
            sig_verified: self.sig_verified,
        }))
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        self
    }
}

// -- Waiting for their CCS --
struct ExpectCcs {
    config: Arc<ClientConfig>,
    secrets: ConnectionSecrets,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    using_ems: bool,
    transcript: HandshakeHash,
    ticket: Option<NewSessionTicketPayload>,
    resuming: bool,
    cert_verified: verify::ServerCertVerified,
    sig_verified: verify::HandshakeSignatureValid,
}

impl State<ClientConnectionData> for ExpectCcs {
    fn handle<'m>(
        self: Box<Self>,
        cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        match m.payload {
            MessagePayload::ChangeCipherSpec(..) => {}
            payload => {
                return Err(inappropriate_message(
                    &payload,
                    &[ContentType::ChangeCipherSpec],
                ));
            }
        }
        // CCS should not be received interleaved with fragmented handshake-level
        // message.
        cx.common.check_aligned_handshake()?;

        // Note: msgs layer validates trivial contents of CCS.
        cx.common
            .record_layer
            .start_decrypting();

        Ok(Box::new(ExpectFinished {
            config: self.config,
            secrets: self.secrets,
            resuming_session: self.resuming_session,
            session_id: self.session_id,
            server_name: self.server_name,
            using_ems: self.using_ems,
            transcript: self.transcript,
            ticket: self.ticket,
            resuming: self.resuming,
            cert_verified: self.cert_verified,
            sig_verified: self.sig_verified,
        }))
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        self
    }
}

struct ExpectFinished {
    config: Arc<ClientConfig>,
    resuming_session: Option<persist::Tls12ClientSessionValue>,
    session_id: SessionId,
    server_name: ServerName<'static>,
    using_ems: bool,
    transcript: HandshakeHash,
    ticket: Option<NewSessionTicketPayload>,
    secrets: ConnectionSecrets,
    resuming: bool,
    cert_verified: verify::ServerCertVerified,
    sig_verified: verify::HandshakeSignatureValid,
}

impl ExpectFinished {
    // -- Waiting for their finished --
    fn save_session(&mut self, cx: &ClientContext<'_>) {
        // Save a ticket.  If we got a new ticket, save that.  Otherwise, save the
        // original ticket again.
        let (mut ticket, lifetime) = match self.ticket.take() {
            Some(nst) => (nst.ticket, nst.lifetime_hint),
            None => (crate::msgs::handshake::TicketPayload::from_unowned(PayloadU16::empty()), 0),
        };

        if ticket.0.is_empty() {
            if let Some(resuming_session) = &mut self.resuming_session {
                ticket = resuming_session.ticket();
            }
        }

        if self.session_id.is_empty() && ticket.0.is_empty() {
            debug!("Session not saved: server didn't allocate id or ticket");
            return;
        }

        let Ok(now) = self.config.current_time() else {
            debug!("Could not get current time");
            return;
        };

        #[cfg(feature = "std")]
        let chain = cx.common.peer_certificates.as_ref();
        #[cfg(feature = "std")]
        let peer_owner: Option<Arc<dyn crate::DeframerBufferOwner>> = cx
            .common
            .peer_certificate_custody
            .as_ref()
            .map(|custody| custody.resource_owner());
        #[cfg(feature = "std")]
        let session_owner = peer_owner.or_else(|| ticket.resource_owner());
        #[cfg(feature = "std")]
        let session_value = if let Some(owner) = session_owner {
            let Some(chain) = chain else { return; };
            let Ok(value) = persist::Tls12ClientSessionValue::new_with_resource_owner(
                self.secrets.suite(), self.session_id, ticket,
                self.secrets.master_secret(), chain, &self.config.verifier,
                &self.config.client_auth_cert_resolver, now, lifetime, self.using_ems, owner,
            ) else { return; };
            value
        } else { persist::Tls12ClientSessionValue::new(
            self.secrets.suite(),
            self.session_id,
            ticket,
            self.secrets.master_secret(),
            cx.common
                .peer_certificates
                .clone()
                .unwrap_or_default(),
            &self.config.verifier,
            &self.config.client_auth_cert_resolver,
            now,
            lifetime,
            self.using_ems,
        ) };
        #[cfg(not(feature = "std"))]
        let session_value = persist::Tls12ClientSessionValue::new(
            self.secrets.suite(), self.session_id, ticket, self.secrets.master_secret(),
            cx.common.peer_certificates.clone().unwrap_or_default(),
            &self.config.verifier, &self.config.client_auth_cert_resolver, now, lifetime,
            self.using_ems,
        );

        self.config
            .resumption
            .store
            .set_tls12_session(self.server_name.clone(), session_value);
    }
}

impl State<ClientConnectionData> for ExpectFinished {
    fn handle<'m>(
        self: Box<Self>,
        cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        let mut st = *self;
        let finished =
            require_handshake_msg!(m, HandshakeType::Finished, HandshakePayload::Finished)?;

        cx.common.check_aligned_handshake()?;

        // Work out what verify_data we expect.
        let vh = st.transcript.current_hash();
        let expect_verify_data = st.secrets.server_verify_data(&vh);

        // Constant-time verification of this is relatively unimportant: they only
        // get one chance.  But it can't hurt.
        let _fin_verified =
            match ConstantTimeEq::ct_eq(&expect_verify_data[..], finished.bytes()).into() {
                true => verify::FinishedMessageVerified::assertion(),
                false => {
                    return Err(cx
                        .common
                        .send_fatal_alert(AlertDescription::DecryptError, Error::DecryptError));
                }
            };

        // Hash this message too.
        st.transcript.add_message(&m);

        st.save_session(cx);

        if st.resuming {
            emit_ccs(cx.common);
            cx.common
                .record_layer
                .start_encrypting();
            emit_finished(&st.secrets, &mut st.transcript, cx.common);
        }

        cx.common
            .start_traffic(&mut cx.sendable_plaintext);
        Ok(Box::new(ExpectTraffic {
            secrets: st.secrets,
            _cert_verified: st.cert_verified,
            _sig_verified: st.sig_verified,
            _fin_verified,
        }))
    }

    // we could not decrypt the encrypted handshake message with session resumption
    // this might mean that the ticket was invalid for some reason, so we remove it
    // from the store to restart a session from scratch
    fn handle_decrypt_error(&self) {
        if self.resuming {
            self.config
                .resumption
                .store
                .remove_tls12_session(&self.server_name);
        }
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        self
    }
}

// -- Traffic transit state --
struct ExpectTraffic {
    secrets: ConnectionSecrets,
    _cert_verified: verify::ServerCertVerified,
    _sig_verified: verify::HandshakeSignatureValid,
    _fin_verified: verify::FinishedMessageVerified,
}

impl State<ClientConnectionData> for ExpectTraffic {
    fn handle<'m>(
        self: Box<Self>,
        cx: &mut ClientContext<'_>,
        m: Message<'m>,
    ) -> hs::NextStateOrError<'m>
    where
        Self: 'm,
    {
        match m.payload {
            MessagePayload::ApplicationData(payload) => cx
                .common
                .take_received_plaintext(payload),
            payload => {
                return Err(inappropriate_message(
                    &payload,
                    &[ContentType::ApplicationData],
                ));
            }
        }
        Ok(self)
    }

    fn export_keying_material(
        &self,
        output: &mut [u8],
        label: &[u8],
        context: Option<&[u8]>,
    ) -> Result<(), Error> {
        self.secrets
            .export_keying_material(output, label, context);
        Ok(())
    }

    fn extract_secrets(&self) -> Result<PartiallyExtractedSecrets, Error> {
        self.secrets
            .extract_secrets(Side::Client)
    }

    fn into_external_state(self: Box<Self>) -> Result<Box<dyn KernelState + 'static>, Error> {
        Ok(self)
    }

    fn into_owned(self: Box<Self>) -> hs::NextState<'static> {
        self
    }
}

impl KernelState for ExpectTraffic {
    fn update_secrets(&mut self, _: Direction) -> Result<ConnectionTrafficSecrets, Error> {
        Err(Error::General(
            "TLS 1.2 connections do not support traffic secret updates".into(),
        ))
    }

    fn handle_new_session_ticket(
        &mut self,
        _cx: &mut KernelContext<'_>,
        _message: &NewSessionTicketPayloadTls13,
    ) -> Result<(), Error> {
        Err(Error::General(
            "TLS 1.2 session tickets may not be sent once the handshake has completed".into(),
        ))
    }
}

#[cfg(all(test, feature = "std", feature = "ring"))]
pub(crate) mod client_kx_custody_tests {
    use super::*;
    use crate::crypto::ring::hash::SHA256;
    use crate::hash_hs::HandshakeHashBuffer;
    use crate::msgs::codec::DirectDecodedCustody;
    use crate::vecbuf::outbound_custody_tests::{control_bytes, Owner};
    use core::sync::atomic::Ordering;

    fn common_with_owner(owner: &Arc<Owner>) -> CommonState {
        let mut common = CommonState::new(Side::Client);
        common.peer_certificate_custody = Some(
            DirectDecodedCustody::reserve(owner.clone(), 0)
                .unwrap()
                .into(),
        );
        common
    }

    #[test]
    fn client_kx_custody_wire_transcript_ems_and_drain() {
        for (kxa, key_len, prefix) in [
            (KeyExchangeAlgorithm::ECDHE, 32, 1),
            (KeyExchangeAlgorithm::DHE, 256, 2),
        ] {
            let key = vec![0x42; key_len];
            let owner = Owner::new(usize::MAX);
            let mut common = common_with_owner(&owner);
            let mut ordinary = CommonState::new(Side::Client);
            common.set_max_fragment_size(Some(32)).unwrap();
            ordinary.set_max_fragment_size(Some(32)).unwrap();
            let mut transcript = HandshakeHashBuffer::new().start_hash(&SHA256);
            let mut expected_hash = HandshakeHashBuffer::new().start_hash(&SHA256);
            emit_client_kx(&mut transcript, kxa, &mut common, &key).unwrap();
            emit_client_kx(&mut expected_hash, kxa, &mut ordinary, &key).unwrap();
            // EMS observes precisely the transcript through ClientKeyExchange.
            assert_eq!(
                transcript.current_hash().as_ref(),
                expected_hash.current_hash().as_ref()
            );
            let mut expected = Vec::new();
            ordinary.sendable_tls.write_to(&mut expected).unwrap();
            let records =
                (expected.len() - (4 + prefix + key_len)) / crate::msgs::message::HEADER_SIZE;
            let control = control_bytes(records);
            assert_eq!(owner.used(), expected.len() + control);
            assert_eq!(
                owner.peak.load(Ordering::SeqCst),
                key_len + prefix + 4 + key_len + prefix + expected.len() + control
            );
            let attempts = owner.attempts.load(Ordering::SeqCst);
            let mut actual = Vec::new();
            assert_eq!(
                common.sendable_tls.write_to(&mut actual).unwrap(),
                expected.len()
            );
            assert_eq!(actual, expected);
            assert_eq!(owner.used(), control);
            assert_eq!(owner.attempts.load(Ordering::SeqCst), attempts);
            drop(common);
            assert_eq!(owner.used(), 0);
        }
    }

    #[test]
    fn client_kx_custody_each_admission_max_minus_one() {
        for (kxa, key_len, prefix) in [
            (KeyExchangeAlgorithm::ECDHE, 32, 1),
            (KeyExchangeAlgorithm::DHE, 256, 2),
        ] {
            let key = vec![0x42; key_len];
            let body = key_len + prefix;
            let encoded = 4 + body;
            let outbound = 5 + encoded;
            let thresholds = [
                key_len,
                key_len + body,
                body + encoded,
                body + encoded + outbound,
                body + encoded + outbound + control_bytes(1),
            ];
            for (stage, threshold) in thresholds.into_iter().enumerate() {
                let owner = Owner::new(threshold - 1);
                let mut common = common_with_owner(&owner);
                let mut transcript = HandshakeHashBuffer::new().start_hash(&SHA256);
                let before = transcript.current_hash();
                assert!(matches!(
                    emit_client_kx(&mut transcript, kxa, &mut common, &key),
                    Err(Error::InvalidMessage(InvalidMessage::MessageTooLarge))
                ));
                assert!(common.sendable_tls.is_empty());
                assert_eq!(owner.used(), 0);
                assert_eq!(owner.attempts.load(Ordering::SeqCst), stage + 2);
                if stage < 4 {
                    assert_eq!(transcript.current_hash().as_ref(), before.as_ref());
                }
                drop(common);
                assert_eq!(owner.used(), 0);
            }
            let owner = Owner::new(*thresholds.last().unwrap());
            let mut common = common_with_owner(&owner);
            let mut transcript = HandshakeHashBuffer::new().start_hash(&SHA256);
            emit_client_kx(&mut transcript, kxa, &mut common, &key).unwrap();
            assert_eq!(owner.used(), outbound + control_bytes(1));
            drop(common);
            assert_eq!(owner.used(), 0);
        }
    }

    // This state-level fixture isolates admission after certificate/signature
    // verification. It is not a certificate-verification or handshake E2E test.
    #[derive(Debug)]
    struct FixtureVerifier;

    impl verify::ServerCertVerifier for FixtureVerifier {
        fn verify_server_cert(
            &self,
            _: &pki_types::CertificateDer<'_>,
            _: &[pki_types::CertificateDer<'_>],
            _: &ServerName<'_>,
            _: &[u8],
            _: pki_types::UnixTime,
        ) -> Result<verify::ServerCertVerified, Error> {
            Ok(verify::ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _: &[u8],
            _: &pki_types::CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> Result<verify::HandshakeSignatureValid, Error> {
            Ok(verify::HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _: &[u8],
            _: &pki_types::CertificateDer<'_>,
            _: &DigitallySignedStruct,
        ) -> Result<verify::HandshakeSignatureValid, Error> {
            panic!("TLS1.3 is outside this fixture")
        }

        fn supported_verify_schemes(&self) -> Vec<crate::SignatureScheme> {
            vec![crate::SignatureScheme::ECDSA_NISTP256_SHA256]
        }
    }

    struct BeforeEmsHash;
    struct BeforeEmsContext(Box<dyn crate::crypto::hash::Context>);

    impl crate::crypto::hash::Hash for BeforeEmsHash {
        fn start(&self) -> Box<dyn crate::crypto::hash::Context> {
            Box::new(BeforeEmsContext(SHA256.start()))
        }
        fn hash(&self, data: &[u8]) -> crate::crypto::hash::Output {
            SHA256.hash(data)
        }
        fn output_len(&self) -> usize {
            32
        }
        fn algorithm(&self) -> crate::crypto::hash::HashAlgorithm {
            crate::crypto::hash::HashAlgorithm::SHA256
        }
    }

    impl crate::crypto::hash::Context for BeforeEmsContext {
        fn fork_finish(&self) -> crate::crypto::hash::Output {
            panic!("denied KX must return before EMS seed capture")
        }
        fn fork(&self) -> Box<dyn crate::crypto::hash::Context> {
            panic!("denied KX must not fork the transcript")
        }
        fn finish(self: Box<Self>) -> crate::crypto::hash::Output {
            panic!("denied KX must not finish the transcript")
        }
        fn update(&mut self, bytes: &[u8]) {
            self.0.update(bytes);
        }
    }

    #[test]
    fn client_kx_custody_server_done_denial_precedes_ems_and_later_flight() {
        use crate::msgs::codec::DecodedOwner;
        use crate::msgs::handshake::ServerEcdhParams;

        let config = Arc::new(
            ClientConfig::builder_with_provider(Arc::new(crate::crypto::ring::default_provider()))
                .with_protocol_versions(&[&crate::version::TLS12])
                .unwrap()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(FixtureVerifier))
                .with_no_client_auth(),
        );
        let SupportedCipherSuite::Tls12(suite) =
            crate::crypto::ring::cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
        else {
            unreachable!();
        };
        // X25519 has a 32-byte public key. Fail each KX reservation in turn,
        // including the lower-layer queue/control admission after hashing KX.
        for (stage, allowance) in [31, 64, 69, 110, 110 + control_bytes(1)]
            .into_iter()
            .enumerate()
        {
            let owner = Owner::new(usize::MAX);
            let (decoded, arc_charge) = DecodedOwner::new(owner.clone()).unwrap();
            let (certs, custody) =
                CertificateChain(vec![pki_types::CertificateDer::from(&[1][..])])
                    .into_owned_with_resource_owner(decoded.clone())
                    .unwrap();
            let retained = owner.used();
            let server_kx = crate::crypto::ring::kx_group::X25519.start().unwrap();
            let st = ExpectServerDone {
                config: config.clone(),
                resuming_session: None,
                session_id: SessionId::empty(),
                server_name: ServerName::try_from("localhost").unwrap(),
                randoms: ConnectionRandoms {
                    client: [0; 32],
                    server: [1; 32],
                },
                using_ems: true,
                transcript: HandshakeHashBuffer::new().start_hash(&BeforeEmsHash),
                suite,
                server_cert: ServerCertDetails::new_with_resource_custody(certs, vec![], custody),
                server_kx: ServerKxDetails::new(
                    &ServerKeyExchangeParams::Ecdh(ServerEcdhParams::new(server_kx.as_ref())),
                    DigitallySignedStruct::new(
                        crate::SignatureScheme::ECDSA_NISTP256_SHA256,
                        vec![],
                    ),
                ),
                client_auth: None,
                must_issue_new_ticket: false,
            };
            let mut connection = crate::ClientConnection::new(
                config.clone(),
                ServerName::try_from("localhost").unwrap(),
            )
            .unwrap();
            let mut common = CommonState::new(Side::Client);
            let mut cx = ClientContext {
                common: &mut common,
                data: &mut connection.core.data,
                sendable_plaintext: None,
            };
            owner.limit.store(retained + allowance, Ordering::SeqCst);
            let attempts = owner.attempts.load(Ordering::SeqCst);
            assert!(matches!(
                Box::new(st).handle(
                    &mut cx,
                    Message::new(
                        ProtocolVersion::TLSv1_2,
                        MessagePayload::handshake(HandshakeMessagePayload(
                            HandshakePayload::ServerHelloDone
                        )),
                    )
                ),
                Err(Error::InvalidMessage(InvalidMessage::MessageTooLarge))
            ));
            assert_eq!(owner.attempts.load(Ordering::SeqCst) - attempts, stage + 1);
            assert!(common.sendable_tls.is_empty());
            assert!(matches!(common.kx_state, KxState::Start(_)));
            assert!(!common.may_send_application_data);
            assert_eq!(owner.used(), retained);
            drop(common);
            drop(decoded);
            drop(arc_charge);
            assert_eq!(owner.used(), 0);
        }
    }
}
