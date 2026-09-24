//! Production gameplay TCP/TLS seam. Transport-only: Foundation owns protocol
//! semantics and the composed owners decide admission.

mod connection;
pub(crate) mod fresh_evidence;
#[cfg(test)]
mod qualification;
mod tcp_tls;

use crate::domain;
use crate::durability::DurabilityRoot;
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority,
};
use crate::durability::fresh_admission::{FreshAdmissionStore, FreshReconciliation};
use crate::durability::fresh_admission_composition::FreshAdmissionSubject;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::admission_authority_publication::FreshAdmissionClaimTransitionV1;
use crate::foundation::fnd04_verifier::{
    FreshDurabilityCurrentAuthorityV1, FreshDurabilityTrustContext, fresh_grant_signing_key_id,
    verify_fresh_grant_durability_v1,
};
use crate::foundation::fresh_admission_durability::{
    FreshAdmissionCommitAuthorizationV1, FreshAdmissionCommitRequestV1,
    FreshAdmissionDurabilityFlowV1, FreshAdmissionDurabilityPortV1, FreshAdmissionDurableOutcomeV1,
    FreshAdmissionOperationV1, FreshAdmissionSubmissionV1,
};
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelId, GameSessionAuthoritySnapshot, GameSessionId,
    GameSessionState, WorldId,
};
use connection::{
    AdmissionRefusal, AdmittedSession, ConnectionIdentifiers, FreshAdmissionAttempt,
    FreshAdmissionAuthority, admit_frame, hold_admitted,
};
pub use fresh_evidence::FreshEvidenceSource;
use oteryn_foundation::CancellationToken;
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio_rustls::rustls;

/// Registered Server Seam hard maximum of pre-admission connections. Admitted
/// connections stay inside the same budget: no separate maximum is registered.
pub(crate) const MAX_CONNECTIONS: usize = 256;
/// Registered Server Seam hard maximum of concurrent handshake/auth units.
pub(crate) const MAX_HANDSHAKE_UNITS: usize = 64;

/// Pause after a listener accept error before accepting again.
const ACCEPT_ERROR_PAUSE: Duration = Duration::from_millis(100);

/// Caller-supplied listener budgets. Values may reduce, never exceed, the
/// registered maxima; the entry deadline has no default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ListenerLimits {
    connections: usize,
    handshake_units: usize,
    entry_deadline: Duration,
}

impl ListenerLimits {
    pub(crate) fn new(
        connections: usize,
        handshake_units: usize,
        entry_deadline: Duration,
    ) -> Option<Self> {
        ((1..=MAX_CONNECTIONS).contains(&connections)
            && (1..=MAX_HANDSHAKE_UNITS).contains(&handshake_units)
            && !entry_deadline.is_zero())
        .then_some(Self {
            connections,
            handshake_units,
            entry_deadline,
        })
    }
}

/// How one accepted connection ended; production ignores it.
pub(crate) trait ConnectionObserver {
    fn ended(&self, end: connection::ConnectionEnd);
}

impl ConnectionObserver for () {
    fn ended(&self, _: connection::ConnectionEnd) {}
}

/// Serve gameplay connections until `shutdown`. Accepting stops at the
/// connection budget (backpressure through the listen backlog). On shutdown the
/// listener stops accepting and entry work is cancelled, while an admission
/// already handed to the owning authority completes before this returns.
pub(crate) async fn serve_listener<A, I, O>(
    listener: &TcpListener,
    tls: &Arc<rustls::ServerConfig>,
    limits: ListenerLimits,
    authority: &A,
    identifiers: &I,
    observer: &O,
    shutdown: &CancellationToken,
) where
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
    O: ConnectionObserver,
{
    let units = Semaphore::new(limits.handshake_units);
    let mut connections: Vec<Pin<Box<dyn Future<Output = ()> + '_>>> = Vec::new();
    let mut stopping = pin!(shutdown.cancelled());
    let mut stopped = false;
    // After an accept error (e.g. EMFILE) accepting pauses instead of spinning.
    let mut accept_pause: Option<Pin<Box<tokio::time::Sleep>>> = None;
    poll_fn(|context| {
        if !stopped && stopping.as_mut().poll(context).is_ready() {
            stopped = true;
        }
        if let Some(pause) = accept_pause.as_mut()
            && pause.as_mut().poll(context).is_ready()
        {
            accept_pause = None;
        }
        while !stopped && accept_pause.is_none() && connections.len() < limits.connections {
            match listener.poll_accept(context) {
                Poll::Ready(Ok((stream, _))) => connections.push(Box::pin(serve_accepted(
                    stream,
                    tls,
                    &units,
                    limits.entry_deadline,
                    authority,
                    identifiers,
                    observer,
                    shutdown,
                ))),
                Poll::Ready(Err(_)) => {
                    let mut pause = Box::pin(tokio::time::sleep(ACCEPT_ERROR_PAUSE));
                    // Registers the wake-up that resumes accepting.
                    let _ = pause.as_mut().poll(context);
                    accept_pause = Some(pause);
                }
                Poll::Pending => break,
            }
        }
        connections.retain_mut(|connection| connection.as_mut().poll(context).is_pending());
        if stopped && connections.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn serve_accepted<A, I, O>(
    stream: TcpStream,
    tls: &Arc<rustls::ServerConfig>,
    units: &Semaphore,
    entry_deadline: Duration,
    authority: &A,
    identifiers: &I,
    observer: &O,
    shutdown: &CancellationToken,
) where
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
    O: ConnectionObserver,
{
    use connection::ConnectionEnd;
    // Entry work (unit wait, TLS, entry frame) is bounded by the deadline and
    // cancelled by shutdown; nothing authoritative has started yet.
    let entry = async {
        let unit = units.acquire().await.ok()?;
        let mut stream = tcp_tls::accept_tls(stream, tls.clone()).await.ok()?;
        let frame = tcp_tls::read_frame(&mut stream).await.ok()?;
        Some((unit, stream, frame))
    };
    let entry = first(
        tokio::time::timeout(entry_deadline, entry),
        shutdown.cancelled(),
    )
    .await;
    let Some(Ok(Some((unit, mut stream, frame)))) = entry else {
        observer.ended(ConnectionEnd::TransportFailed);
        return;
    };
    // The owning authority's decision is never cancelled midway.
    let admitted = admit_frame(&mut stream, &frame, authority, identifiers).await;
    drop(unit);
    let end = match admitted {
        Err(end) => end,
        Ok(admitted) => first(hold_admitted(&mut stream, admitted), shutdown.cancelled())
            .await
            .unwrap_or(ConnectionEnd::AdmittedThenDisconnected(admitted)),
    };
    observer.ended(end);
}

/// Output of `primary`, or `None` when `stop` completes first.
async fn first<T>(primary: impl Future<Output = T>, stop: impl Future<Output = ()>) -> Option<T> {
    let mut primary = pin!(primary);
    let mut stop = pin!(stop);
    poll_fn(|context| {
        if let Poll::Ready(value) = primary.as_mut().poll(context) {
            return Poll::Ready(Some(value));
        }
        stop.as_mut().poll(context).map(|()| None)
    })
    .await
}

/// Owners already composed by the caller: the durability root, the reconciled
/// #414 Character authority, and this GameNode's #415 incarnation holding the
/// served Channel.
pub struct GameplaySeamOwners<'a, 'f, 's> {
    pub root: &'a DurabilityRoot,
    pub character: &'a ReconciledCharacterAuthority<'f, 's>,
    pub holder: &'a NodeIncarnationProof,
    /// The Platform admission-evidence route, fetched on demand per attempt.
    pub evidence: &'a FreshEvidenceSource,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
}

/// Explicit listener configuration; nothing has a production default.
pub struct GameplayListenerConfig {
    pub certificates: Vec<rustls::pki_types::CertificateDer<'static>>,
    pub private_key: rustls::pki_types::PrivateKeyDer<'static>,
    /// At most the registered 256.
    pub max_connections: usize,
    /// At most the registered 64.
    pub max_handshake_units: usize,
    /// Bound on TLS plus the entry frame for one connection.
    pub entry_deadline: Duration,
}

#[derive(Debug)]
pub enum GameplayServeError {
    /// A budget is zero or above its registered maximum, or the deadline is zero.
    InvalidLimits,
    /// The TLS material cannot form a TLS 1.3 server configuration.
    InvalidTls(rustls::Error),
}

impl std::fmt::Display for GameplayServeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLimits => formatter.write_str("invalid gameplay listener limits"),
            Self::InvalidTls(error) => write!(formatter, "invalid gameplay TLS material: {error}"),
        }
    }
}

impl std::error::Error for GameplayServeError {}

/// The production gameplay seam: TCP + TLS 1.3 (ALPN `oteryn-game/1`),
/// bounded FND-02 framing and fresh admission through the composed owners,
/// served on `listener` until `shutdown`.
pub async fn serve_gameplay(
    listener: &TcpListener,
    config: GameplayListenerConfig,
    owners: GameplaySeamOwners<'_, '_, '_>,
    shutdown: &CancellationToken,
) -> Result<(), GameplayServeError> {
    let limits = ListenerLimits::new(
        config.max_connections,
        config.max_handshake_units,
        config.entry_deadline,
    )
    .ok_or(GameplayServeError::InvalidLimits)?;
    let tls = tcp_tls::tls_config(config.certificates, config.private_key)
        .map_err(GameplayServeError::InvalidTls)?;
    let authority = ComposedFreshAdmission {
        root: owners.root,
        character: owners.character,
        holder: owners.holder,
        evidence: owners.evidence,
        world_id: owners.world_id,
        channel_id: owners.channel_id,
    };
    serve_listener(
        listener,
        &tls,
        limits,
        &authority,
        &SecureIdentifiers,
        &(),
        shutdown,
    )
    .await;
    Ok(())
}

/// Fresh, unpredictable identifiers from the TLS provider's secure random
/// source.
pub(crate) struct SecureIdentifiers;

impl SecureIdentifiers {
    fn random<const N: usize>() -> Option<[u8; N]> {
        let mut bytes = [0u8; N];
        rustls::crypto::aws_lc_rs::default_provider()
            .secure_random
            .fill(&mut bytes)
            .ok()?;
        Some(bytes)
    }
}

impl ConnectionIdentifiers for SecureIdentifiers {
    fn game_session_id(&self) -> Option<crate::foundation::GameSessionId> {
        let mut bytes: [u8; 16] = Self::random()?;
        let millis = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_millis(),
        )
        .ok()?;
        bytes[..6].copy_from_slice(&millis.to_be_bytes()[2..]);
        bytes[6] = 0x70 | (bytes[6] & 0x0f);
        bytes[8] = 0x80 | (bytes[8] & 0x3f);
        crate::foundation::GameSessionId::decode(&bytes).ok()
    }

    fn transport_ref(&self) -> Option<crate::foundation::AuthenticatedTransportRefV1> {
        crate::foundation::AuthenticatedTransportRefV1::decode(&Self::random::<16>()?).ok()
    }
}

/// Fresh admission composed from the real owners: the #414 Character decides
/// account and world, this holder's #415 assignment decides the Channel, S2
/// supplies account security and signing trust, and the durable commit
/// revalidates all of them.
pub(crate) struct ComposedFreshAdmission<'a, 'f, 's> {
    pub(crate) root: &'a DurabilityRoot,
    pub(crate) character: &'a ReconciledCharacterAuthority<'f, 's>,
    pub(crate) holder: &'a NodeIncarnationProof,
    pub(crate) evidence: &'a FreshEvidenceSource,
    pub(crate) world_id: WorldId,
    pub(crate) channel_id: ChannelId,
}

impl FreshAdmissionAuthority for ComposedFreshAdmission<'_, '_, '_> {
    async fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        use AdmissionRefusal::{Rejected, Unavailable};
        let token = std::str::from_utf8(attempt.admission_material).map_err(|_| Rejected)?;
        // Untrusted selector only; verification below checks the same token.
        let signing_key_id = fresh_grant_signing_key_id(token).ok_or(Rejected)?;
        let character_id = domain::CharacterId::from_bytes(*attempt.character_id.as_bytes())
            .map_err(|_| Rejected)?;
        let record = self
            .root
            .read_current_character(self.character, character_id)
            .await
            .map_err(|error| match error {
                CharacterAuthorityError::Unavailable(_) => Unavailable,
                _ => Rejected,
            })?;
        if record.world_id.as_bytes() != self.world_id.as_bytes() {
            return Err(Rejected);
        }
        let subject = FreshAdmissionSubject {
            account_id: canonical_uuid(record.account_id.as_bytes()),
            character_id: attempt.character_id,
            world_id: self.world_id,
            channel_id: self.channel_id,
            signing_key_id,
        };
        // D4: the five-second source-age bound requires evidence fetched for
        // this attempt; S2 custody retains it and the composition decides.
        self.evidence
            .refresh_fresh_admission(
                self.root,
                self.holder,
                &subject.account_id,
                &subject.signing_key_id,
            )
            .await
            .map_err(|_| Unavailable)?;
        let now = unix_seconds().ok_or(Unavailable)?;
        match self
            .root
            .publish_fresh_admission_sources(self.character, self.holder, &subject, now)
            .await
        {
            Ok(GuardPublicationDisposition::Applied | GuardPublicationDisposition::Existing) => {}
            Ok(GuardPublicationDisposition::Stale | GuardPublicationDisposition::Conflict)
            | Err(_) => return Err(Unavailable),
        }
        let composition = self
            .root
            .compose_fresh_admission(self.character, self.holder, &subject)
            .await
            .map_err(|_| Unavailable)?;
        let facts = verify_fresh_grant_durability_v1(
            token,
            now,
            &FreshDurabilityTrustContext::from_owning_source(&composition),
            &FreshDurabilityCurrentAuthorityV1::from_owning_source(&composition),
        )
        .map_err(|_| Rejected)?;
        let authorization = FreshAdmissionCommitAuthorizationV1::new(
            &facts,
            attempt.game_session_id,
            attempt.transport,
            &composition,
            now,
        )
        .map_err(|_| Rejected)?;
        let transition =
            FreshAdmissionClaimTransitionV1::prepare(&composition, &authorization, now)
                .map_err(|_| Rejected)?;
        let mut flow = FreshAdmissionDurabilityFlowV1::begin(authorization, transition)
            .map_err(|_| Rejected)?;
        let mut prepared = PreparedRequest(None);
        flow.submit(&mut prepared).map_err(|_| Rejected)?;
        let request = prepared.0.ok_or(Unavailable)?;
        match self
            .root
            .commit_composed_fresh_admission(self.character, self.holder, &composition, &request)
            .await
        {
            Ok(FreshAdmissionDurableOutcomeV1::Committed(_)) => {}
            // The commit may have landed with its acknowledgement lost: keep
            // the exact operation and reconcile it until the outcome is proven.
            Ok(FreshAdmissionDurableOutcomeV1::AmbiguousOrUnavailable) | Err(_) => {
                self.reconcile(&request, attempt.game_session_id, attempt.transport)
                    .await?;
            }
            Ok(_) => return Err(Rejected),
        }
        Ok(AdmittedSession {
            game_session_id: attempt.game_session_id,
            world_id: self.world_id,
            channel_id: self.channel_id,
        })
    }
}

/// Bounded reconciliation of one possibly committed admission.
const RECONCILE_ATTEMPTS: u32 = 5;
const RECONCILE_BACKOFF: Duration = Duration::from_millis(200);

impl ComposedFreshAdmission<'_, '_, '_> {
    /// `Ok` only when the exact operation is durably committed and this socket
    /// still owns the session it created. The receipt is read under the
    /// admission relation locks, so `Absent` proves noncommit.
    async fn reconcile(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        game_session_id: GameSessionId,
        transport: AuthenticatedTransportRefV1,
    ) -> Result<(), AdmissionRefusal> {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        for attempt in 0..RECONCILE_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(RECONCILE_BACKOFF).await;
            }
            match store.reconcile(request.operation()).await {
                // The receipt is immutable; the session may since have lost
                // control, been rebound or terminated.
                Ok(FreshReconciliation::Committed(snapshot)) => {
                    return if owns_fresh_session(
                        snapshot.current_session,
                        game_session_id,
                        transport,
                    ) {
                        Ok(())
                    } else {
                        Err(AdmissionRefusal::Rejected)
                    };
                }
                Ok(FreshReconciliation::Absent) => return Err(AdmissionRefusal::Unavailable),
                Ok(FreshReconciliation::Conflict) => return Err(AdmissionRefusal::Rejected),
                Err(_) => {}
            }
        }
        Err(AdmissionRefusal::Unavailable)
    }
}

/// The reconciled current session is still the untouched fresh admission bound
/// to this transport: active, at its admitted generation, never lost or replaced.
fn owns_fresh_session<T: Copy + Eq>(
    current: GameSessionAuthoritySnapshot<T>,
    game_session_id: GameSessionId,
    transport: T,
) -> bool {
    let commit = current.commit();
    commit.game_session_id() == game_session_id
        && commit.initial_transport() == transport
        && current.current_game_session_id() == game_session_id
        && current.session_state() == GameSessionState::Active
        && current.current_connection_generation() == commit.connection_generation()
        && current.current_transport() == Some(transport)
        && current.current_control_loss_epoch().is_none()
}

/// Captures the flow's prepared request; the durable commit is the composed
/// owner-revalidating route, never this port.
struct PreparedRequest(Option<FreshAdmissionCommitRequestV1>);

impl FreshAdmissionDurabilityPortV1 for PreparedRequest {
    fn submit(&mut self, request: &FreshAdmissionCommitRequestV1) -> FreshAdmissionSubmissionV1 {
        if self.0.is_some() {
            return FreshAdmissionSubmissionV1::Unavailable;
        }
        self.0 = Some(request.clone());
        FreshAdmissionSubmissionV1::Accepted
    }

    fn reconcile(&mut self, _: &FreshAdmissionOperationV1) -> FreshAdmissionSubmissionV1 {
        FreshAdmissionSubmissionV1::Unavailable
    }
}

fn unix_seconds() -> Option<i64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(elapsed.as_secs()).ok()
}

fn canonical_uuid(value: &[u8; 16]) -> String {
    let hex: String = value.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::connection::{ConnectionEnd, ConnectionIdentifiers};
    use super::*;
    use crate::foundation::{CharacterId, MessageType, decode_wire_envelope};
    use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName};
    use std::error::Error;
    use std::net::SocketAddr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::Notify;
    use tokio_rustls::TlsConnector;

    const CHARACTER: [u8; 16] = [
        0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x71, 0x11, 0x91, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
        0x11,
    ];

    /// Admits after `gate` opens; counts calls.
    struct GatedAuthority {
        calls: AtomicUsize,
        gate: Notify,
        open: std::sync::atomic::AtomicBool,
    }

    impl GatedAuthority {
        fn new(open: bool) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                gate: Notify::new(),
                open: std::sync::atomic::AtomicBool::new(open),
            }
        }
        fn release(&self) {
            self.open.store(true, Ordering::SeqCst);
            self.gate.notify_waiters();
        }
    }

    impl FreshAdmissionAuthority for GatedAuthority {
        async fn admit(
            &self,
            attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            while !self.open.load(Ordering::SeqCst) {
                let notified = self.gate.notified();
                if self.open.load(Ordering::SeqCst) {
                    break;
                }
                notified.await;
            }
            Ok(AdmittedSession {
                game_session_id: attempt.game_session_id,
                world_id: crate::foundation::WorldId::decode(&CHARACTER)
                    .map_err(|_| AdmissionRefusal::Unavailable)?,
                channel_id: crate::foundation::ChannelId::decode(&CHARACTER)
                    .map_err(|_| AdmissionRefusal::Unavailable)?,
            })
        }
    }

    #[derive(Default)]
    struct Ends(Mutex<Vec<ConnectionEnd>>);
    impl ConnectionObserver for Ends {
        fn ended(&self, end: ConnectionEnd) {
            self.0.lock().expect("ends").push(end);
        }
    }
    impl Ends {
        fn snapshot(&self) -> Vec<ConnectionEnd> {
            self.0.lock().expect("ends").clone()
        }
    }

    struct Material {
        server: Arc<rustls::ServerConfig>,
        client: TlsConnector,
        certificate: CertificateDer<'static>,
    }

    fn material() -> Result<Material, Box<dyn Error>> {
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let cert: CertificateDer<'static> = generated.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
        let server = tcp_tls::tls_config(vec![cert.clone()], key.into())?;
        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert.clone())?;
        let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_no_client_auth();
        config.alpn_protocols = vec![b"oteryn-game/1".to_vec()];
        Ok(Material {
            server,
            client: TlsConnector::from(Arc::new(config)),
            certificate: cert,
        })
    }

    fn bootstrap_frame() -> Vec<u8> {
        fn varint(output: &mut Vec<u8>, mut value: u64) {
            while value >= 0x80 {
                output.push((value as u8 & 0x7f) | 0x80);
                value >>= 7;
            }
            output.push(value as u8);
        }
        fn field(output: &mut Vec<u8>, key: u64, value: &[u8]) {
            varint(output, key);
            varint(output, value.len() as u64);
            output.extend_from_slice(value);
        }
        let mut payload = vec![0x08, 1, 0x10, 1, 0x18, 1];
        field(&mut payload, 0x2a, b"grant");
        field(&mut payload, 0x32, &CHARACTER);
        field(&mut payload, 0x3a, b"seam-test");
        let mut envelope = vec![0x08, 1];
        field(&mut envelope, 0x22, &payload);
        let mut framed = (envelope.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(&envelope);
        framed
    }

    /// Complete TLS, send a bootstrap, return the first server frame.
    async fn admit_client(
        connector: TlsConnector,
        address: SocketAddr,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let mut stream = connector
            .connect(
                ServerName::try_from("localhost")?,
                TcpStream::connect(address).await?,
            )
            .await?;
        stream.write_all(&bootstrap_frame()).await?;
        let mut prefix = [0u8; 4];
        stream.read_exact(&mut prefix).await?;
        let mut body = vec![0; u32::from_be_bytes(prefix) as usize];
        stream.read_exact(&mut body).await?;
        Ok(body)
    }

    fn runtime() -> Result<tokio::runtime::Runtime, Box<dyn Error>> {
        Ok(tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?)
    }

    /// Both outputs; the workspace Tokio has no macros.
    async fn join<A, B>(a: impl Future<Output = A>, b: impl Future<Output = B>) -> (A, B) {
        let mut a = pin!(a);
        let mut b = pin!(b);
        let (mut left, mut right) = (None, None);
        poll_fn(|context| {
            if left.is_none()
                && let Poll::Ready(value) = a.as_mut().poll(context)
            {
                left = Some(value);
            }
            if right.is_none()
                && let Poll::Ready(value) = b.as_mut().poll(context)
            {
                right = Some(value);
            }
            if left.is_some() && right.is_some() {
                Poll::Ready((left.take().expect("left"), right.take().expect("right")))
            } else {
                Poll::Pending
            }
        })
        .await
    }

    async fn settle(ms: u64) {
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }

    #[test]
    fn resource_limits_reject_values_above_registered_maxima() {
        let deadline = Duration::from_secs(1);
        assert!(ListenerLimits::new(MAX_CONNECTIONS, MAX_HANDSHAKE_UNITS, deadline).is_some());
        assert!(ListenerLimits::new(MAX_CONNECTIONS + 1, MAX_HANDSHAKE_UNITS, deadline).is_none());
        assert!(ListenerLimits::new(MAX_CONNECTIONS, MAX_HANDSHAKE_UNITS + 1, deadline).is_none());
        assert!(ListenerLimits::new(0, 1, deadline).is_none());
        assert!(ListenerLimits::new(1, 0, deadline).is_none());
        assert!(ListenerLimits::new(1, 1, Duration::ZERO).is_none());
    }

    #[test]
    fn resource_connection_budget_backpressures_at_limit() -> Result<(), Box<dyn Error>> {
        runtime()?.block_on(async {
            let material = material()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let limits = ListenerLimits::new(2, 2, Duration::from_millis(600)).ok_or("limits")?;
            let authority = GatedAuthority::new(true);
            let ends = Ends::default();
            let shutdown = CancellationToken::new();
            // Three silent clients: only two fit the budget; the third is not
            // accepted until an entry deadline frees a slot.
            let mut clients = Vec::new();
            for _ in 0..3 {
                clients.push(TcpStream::connect(address).await?);
            }
            let serve = serve_listener(
                &listener,
                &material.server,
                limits,
                &authority,
                &SecureIdentifiers,
                &ends,
                &shutdown,
            );
            let check = async {
                settle(900).await;
                let midway = ends.snapshot().len();
                settle(900).await;
                let finished = ends.snapshot().len();
                shutdown.cancel();
                (midway, finished)
            };
            let ((), (midway, finished)) = join(serve, check).await;
            assert_eq!(midway, 2, "third client must wait for a free slot");
            assert_eq!(finished, 3);
            assert!(
                ends.snapshot()
                    .iter()
                    .all(|end| *end == ConnectionEnd::TransportFailed)
            );
            assert_eq!(authority.calls.load(Ordering::SeqCst), 0);
            drop(clients);
            Ok(())
        })
    }

    #[test]
    fn resource_handshake_units_bound_concurrent_admission() -> Result<(), Box<dyn Error>> {
        runtime()?.block_on(async {
            let material = material()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let limits = ListenerLimits::new(4, 1, Duration::from_millis(700)).ok_or("limits")?;
            let authority = GatedAuthority::new(false);
            let ends = Ends::default();
            let shutdown = CancellationToken::new();
            let first = tokio::spawn(admit_client(material.client.clone(), address));
            let serve = serve_listener(
                &listener,
                &material.server,
                limits,
                &authority,
                &SecureIdentifiers,
                &ends,
                &shutdown,
            );
            let check = async {
                settle(300).await;
                // The first admission holds the only unit; the second client
                // cannot even start TLS and reaches its entry deadline.
                let second = admit_client(material.client.clone(), address).await;
                let calls_while_held = authority.calls.load(Ordering::SeqCst);
                authority.release();
                let first = first.await;
                shutdown.cancel();
                (second.is_err(), calls_while_held, first)
            };
            let ((), (second_failed, calls_while_held, first)) = join(serve, check).await;
            assert!(second_failed);
            assert_eq!(calls_while_held, 1);
            let accepted: Vec<u8> = first?.map_err(|error| error.to_string())?;
            assert_eq!(
                decode_wire_envelope(&accepted)?.message_type(),
                MessageType::ServerAccepted
            );
            assert_eq!(authority.calls.load(Ordering::SeqCst), 1);
            Ok(())
        })
    }

    #[test]
    fn shutdown_completes_admission_in_flight_and_cancels_entry_work() -> Result<(), Box<dyn Error>>
    {
        runtime()?.block_on(async {
            let material = material()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let limits = ListenerLimits::new(4, 4, Duration::from_secs(30)).ok_or("limits")?;
            let authority = GatedAuthority::new(false);
            let ends = Ends::default();
            let shutdown = CancellationToken::new();
            let admitting = tokio::spawn(admit_client(material.client.clone(), address));
            let silent = TcpStream::connect(address).await?;
            let serve = serve_listener(
                &listener,
                &material.server,
                limits,
                &authority,
                &SecureIdentifiers,
                &ends,
                &shutdown,
            );
            let check = async {
                settle(300).await;
                shutdown.cancel();
                settle(300).await;
                // Entry work was cancelled; the admission is still in flight.
                let before_release = ends.snapshot();
                authority.release();
                (before_release, admitting.await)
            };
            let ((), (before_release, admitted)) = join(serve, check).await;
            assert_eq!(before_release, vec![ConnectionEnd::TransportFailed]);
            let accepted: Vec<u8> = admitted?.map_err(|error| error.to_string())?;
            assert_eq!(
                decode_wire_envelope(&accepted)?.message_type(),
                MessageType::ServerAccepted
            );
            let ends = ends.snapshot();
            assert_eq!(ends.len(), 2);
            assert!(matches!(
                ends[1],
                ConnectionEnd::AdmittedThenDisconnected(_)
            ));
            drop(silent);
            Ok(())
        })
    }

    #[test]
    fn transport_negatives_close_without_admission() -> Result<(), Box<dyn Error>> {
        runtime()?.block_on(async {
            let material = material()?;
            let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
            let _ = generated;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let limits = ListenerLimits::new(8, 8, Duration::from_secs(5)).ok_or("limits")?;
            let authority = GatedAuthority::new(true);
            let ends = Ends::default();
            let shutdown = CancellationToken::new();
            let serve = serve_listener(
                &listener,
                &material.server,
                limits,
                &authority,
                &SecureIdentifiers,
                &ends,
                &shutdown,
            );
            let check = async {
                let mut outcomes = Vec::new();
                for (tls12, alpn) in [
                    (true, Some(b"oteryn-game/1".as_slice())),
                    (false, Some(b"wrong".as_slice())),
                    (false, None),
                ] {
                    let started = std::time::Instant::now();
                    let outcome = tokio::time::timeout(
                        Duration::from_secs(10),
                        negative_client(&material, address, tls12, alpn),
                    )
                    .await;
                    outcomes.push((outcome.is_ok(), started.elapsed()));
                }
                shutdown.cancel();
                outcomes
            };
            let ((), outcomes) = join(serve, check).await;
            for (finished, elapsed) in &outcomes {
                assert!(*finished, "client hung after {elapsed:?}");
            }
            assert_eq!(authority.calls.load(Ordering::SeqCst), 0);
            Ok(())
        })
    }

    async fn negative_client(
        material: &Material,
        address: SocketAddr,
        tls12: bool,
        alpn: Option<&[u8]>,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let server_cert = material.certificate.clone();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(server_cert)?;
        let version = if tls12 {
            &rustls::version::TLS12
        } else {
            &rustls::version::TLS13
        };
        let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[version])?
        .with_root_certificates(roots)
        .with_no_client_auth();
        config.alpn_protocols = alpn.map(<[u8]>::to_vec).into_iter().collect();
        let tcp = TcpStream::connect(address).await?;
        let Ok(mut stream) = TlsConnector::from(Arc::new(config))
            .connect(ServerName::try_from("localhost")?, tcp)
            .await
        else {
            return Ok(Vec::new());
        };
        let _ = stream.write_all(&bootstrap_frame()).await;
        let mut output = Vec::new();
        let _ = stream.read_to_end(&mut output).await;
        Ok(output)
    }

    fn uuid_v7(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70 | (seed & 0x0f);
        bytes[8] = 0x80 | (seed & 0x3f);
        bytes
    }

    #[test]
    fn reconciled_session_must_still_be_the_untouched_fresh_admission() {
        use crate::foundation::{
            CharacterLease, ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts,
            ScopeOwnershipGeneration,
        };
        let session = GameSessionId::decode(&uuid_v7(0x21)).expect("session");
        let other = GameSessionId::decode(&uuid_v7(0x22)).expect("session");
        let character = CharacterId::decode(&CHARACTER).expect("character");
        let world = WorldId::decode(&uuid_v7(0x23)).expect("world");
        let channel = ChannelId::decode(&uuid_v7(0x24)).expect("channel");
        let facts =
            FreshAdmissionFacts::new([7; 32], character, world, channel, 1, 1).expect("facts");
        let commit = FreshAdmissionCommit::from_facts(session, facts, 9_u64).expect("commit");
        let snapshot = |state, generation, transport| {
            GameSessionAuthoritySnapshot::new(
                commit,
                state,
                ConnectionGeneration::new(generation).expect("generation"),
                transport,
                CharacterLease::new(character, 1).expect("lease"),
                ScopeOwnershipGeneration::new(1).expect("scope"),
            )
        };
        let admitted = snapshot(GameSessionState::Active, 1, Some(9));
        assert!(owns_fresh_session(admitted, session, 9));
        assert!(!owns_fresh_session(admitted, session, 8));
        assert!(!owns_fresh_session(admitted, other, 9));
        let rebound = snapshot(GameSessionState::Active, 2, Some(8));
        assert!(!owns_fresh_session(rebound, session, 9));
        let lost = snapshot(GameSessionState::Reconnectable, 1, None);
        assert!(!owns_fresh_session(lost, session, 9));
        let terminal = snapshot(GameSessionState::Terminal, 1, None);
        assert!(!owns_fresh_session(terminal, session, 9));
    }

    #[test]
    fn identifiers_are_fresh_uuid_v7_and_nonzero_transport() {
        let identifiers = SecureIdentifiers;
        let first = identifiers.game_session_id().expect("session");
        let second = identifiers.game_session_id().expect("session");
        assert_ne!(first, second);
        assert_eq!(first.as_bytes()[6] >> 4, 7);
        assert_ne!(identifiers.transport_ref(), identifiers.transport_ref());
        assert!(identifiers.transport_ref().is_some());
        let _ = (
            CharacterId::decode(&CHARACTER),
            AuthenticatedTransportRefV1::decode(&[1; 16]),
            GameSessionId::decode(&CHARACTER),
        );
    }
}
