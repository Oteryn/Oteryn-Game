//! Production gameplay TCP/TLS seam. Transport-only: Foundation owns protocol
//! semantics and the composed owners decide admission.

pub(crate) mod actor_spell;
mod capabilities;
pub(crate) mod charm;
mod connection;
mod container_view;
pub(crate) mod fresh_evidence;
mod item_view;
mod monk_save;
#[cfg(test)]
mod qualification;
mod resume;
mod tcp_tls;
pub(crate) mod world_object;
pub(crate) mod world_spatial;

use crate::achievement_catalogue::AccountAchievementsRequest;
use crate::content::NativeEntryMovementCells;
use crate::domain;
use crate::durability::DurabilityRoot;
use crate::durability::admission_authority_guards::{
    AdmissionGuardStore, GuardPublicationDisposition,
};
use crate::durability::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority,
};
use crate::durability::fresh_admission::{
    ExpiredLossReleaseV1, FreshAdmissionStore, FreshLossReconciliation, FreshReconciliation,
};
use crate::durability::fresh_admission_composition::FreshAdmissionSubject;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::{AssignmentState, NodeIncarnationProof};
use crate::foundation::FoundationProtocolError;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1, FreshAdmissionClaimTransitionV1,
};
use crate::foundation::fnd04_verifier::{
    FreshDurabilityCurrentAuthorityV1, FreshDurabilityTrustContext, fresh_grant_signing_key_id,
    recovery_source_sealed, verify_fresh_grant_durability_v1,
};
use crate::foundation::fresh_admission_durability::{
    FreshAdmissionCommitAuthorizationV1, FreshAdmissionCommitRequestV1,
    FreshAdmissionDurabilityFlowV1, FreshAdmissionDurabilityPortV1, FreshAdmissionDurableOutcomeV1,
    FreshAdmissionOperationV1, FreshAdmissionSubmissionV1,
};
use crate::foundation::{
    AccountPresenceClaimV1, ControlLossAuthorizationV1, ControlLossCauseV1, ControlLossEpochRefV1,
    ControlLossFlowV1, ControlLossHistoryV1, ControlLossMark, ControlLossObservationV1,
    ControlLossOutcomeV1, ControlLossSourceV1, ReconnectDurabilityErrorV1,
    RecoveryProtectionContinuityV1, RecoveryProtectionRearmV1, RecoveryProtectionUseV1,
};
use crate::foundation::{
    AuthenticatedTransportRefV1, CarrierError, ChannelId, ChannelRuntimeV1,
    CharacterWorldEligibilityClaimV1, ExactActorRef, FirstEntryPosition,
    GameSessionAuthoritySnapshot, GameSessionId, GameSessionState, PlayerActorReservation,
    RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};
use connection::{
    AccountAchievementsReply, AdmissionRefusal, AdmittedSession, ConnectionIdentifiers,
    ControlLossResult, ControllerBinding, FirstEntryOutcome, FreshAdmissionAttempt,
    FreshAdmissionAuthority, GraceExpiryResult, IDLE_LIVENESS, SessionContinuity, StepOutcome,
    UseOutcome, admit_frame, serve_admitted,
};
pub use fresh_evidence::FreshEvidenceSource;
use oteryn_foundation::CancellationToken;
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, Semaphore};
use tokio_rustls::rustls;
use world_spatial::{ActorPosition, StepDirection, StepDisposition, WorldSpatialObservation};

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
    let mut connections: Vec<Pin<Box<dyn Future<Output = Option<LostControl>> + '_>>> = Vec::new();
    // Loss/grace lifecycles of ended admitted connections. They hold no socket
    // and at most one exists per admitted session, so they stay outside the
    // connection budget; shutdown cancels them.
    let mut lifecycles: Vec<Pin<Box<dyn Future<Output = ()> + '_>>> = Vec::new();
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
        connections.retain_mut(|connection| match connection.as_mut().poll(context) {
            Poll::Pending => true,
            Poll::Ready(lost) => {
                if let Some(lost) = lost {
                    let mut lifecycle = Box::pin(control_loss_lifecycle(authority, lost, shutdown));
                    // Registers the lifecycle's first wake-up.
                    if lifecycle.as_mut().poll(context).is_pending() {
                        lifecycles.push(lifecycle);
                    }
                }
                false
            }
        });
        lifecycles.retain_mut(|lifecycle| lifecycle.as_mut().poll(context).is_pending());
        if stopped && connections.is_empty() && lifecycles.is_empty() {
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
) -> Option<LostControl>
where
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
        return None;
    };
    // The owning authority's decision is never cancelled midway.
    let admitted = admit_frame(&mut stream, &frame, authority, identifiers).await;
    drop(unit);
    let (end, served) = match admitted {
        // A durably admitted or resumed session whose acceptance could not be delivered
        // still has a controller that is gone: it enters the loss lifecycle, never limbo.
        Err(end @ ConnectionEnd::AdmittedThenDisconnected(_)) => (end, true),
        Err(end) => (end, false),
        Ok(admitted) => match first(
            serve_admitted(&mut stream, admitted, authority, IDLE_LIVENESS),
            shutdown.cancelled(),
        )
        .await
        {
            Some(end) => (end, true),
            // A shutdown drain is not evidence of lost playable control.
            None => (ConnectionEnd::AdmittedThenDisconnected(admitted), false),
        },
    };
    // Liveness proved the loss: record it now. A closed or failed transport alone is not
    // proof (FND-04B §4): control is lost once the detection window passes unanswered.
    let lost = match (&end, served) {
        (ConnectionEnd::AdmittedThenControlLost(session), true) => Some((*session, Duration::ZERO)),
        (
            ConnectionEnd::AdmittedThenDisconnected(session)
            | ConnectionEnd::AdmittedThenClosed(session, _),
            true,
        ) => Some((
            *session,
            IDLE_LIVENESS
                .interval
                .saturating_mul(IDLE_LIVENESS.missed_limit),
        )),
        _ => None,
    };
    drop(stream);
    observer.ended(end);
    lost.map(|(session, wait)| LostControl { session, wait })
}

/// An ended admitted connection whose control is lost after `wait` without restored control.
struct LostControl {
    session: AdmittedSession,
    wait: Duration,
}

/// Durable loss, then grace-expiry release, of one ended admitted connection.
/// Shutdown cancels it; a durable decision already in flight stays atomic.
async fn control_loss_lifecycle<A: FreshAdmissionAuthority>(
    authority: &A,
    lost: LostControl,
    shutdown: &CancellationToken,
) {
    let lifecycle = async {
        match authority.lose_control(lost.session, lost.wait).await {
            ControlLossResult::Recorded => {
                let _ = authority.expire_control_loss(lost.session).await;
            }
            ControlLossResult::ResumedHistory => {
                let _ = authority.release_abandoned(lost.session).await;
            }
            ControlLossResult::NotApplicable
            | ControlLossResult::Refused
            | ControlLossResult::Unknown => {}
        }
    };
    let _ = first(lifecycle, shutdown.cancelled()).await;
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
    /// First-slice runtime authority remains crate-owned and cannot be supplied
    /// by a transport or client caller.
    pub(crate) runtime: &'a Mutex<ChannelRuntimeV1>,
    /// The active generation's qualified cells the Channel's Movement reads (#935).
    pub(crate) movement_cells: &'a NativeEntryMovementCells,
    /// The one entry-room door's bound runtime (#162 5868482467, M2b): the Channel-owner state
    /// `ComposedFreshAdmission::step` and `::use_object` both lock, alongside `runtime`, to
    /// decide movement blocking and USE_INTENT transitions.
    pub(crate) door: &'a Mutex<crate::world_runtime::LocalObjectRuntime>,
    /// C2: the activated entry-room Content with the one reward chest injected
    /// (`interaction_chest_use::with_entry_chest`), read by `::use_object` for a chest `USE`.
    pub(crate) chest: &'a crate::content::CanonicalReferencePlayableContent,
    /// The V1 spell book the cast intent's index resolves against (spell cast §3, SPELL-D1).
    pub(crate) spells: &'a crate::spell::SpellBook,
    /// The Achievement catalogue the `ACCOUNT_ACHIEVEMENTS_QUERY` display read resolves every
    /// fact against (display contract §2.1, §4), and a chest's achievement resolves in (C2).
    pub(crate) achievements: &'a crate::achievement_catalogue::AchievementCatalogue,
    /// Complete immutable Charm data imported at boot. This reference supplies no current
    /// generation authority or effect availability; gameplay composition is a separate step.
    pub(crate) imported_charms: &'a crate::content::charm_source::CanonicalCharmCatalogue,
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

/// Check that the gameplay certificate chain and key form the served TLS 1.3
/// configuration, before anything is bound or published.
pub(crate) fn validate_gameplay_tls(
    certificates: &[rustls::pki_types::CertificateDer<'static>],
    private_key: &rustls::pki_types::PrivateKeyDer<'static>,
) -> Result<(), rustls::Error> {
    tcp_tls::tls_config(certificates.to_vec(), private_key.clone_key()).map(|_| ())
}

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
        runtime: owners.runtime,
        movement_cells: owners.movement_cells,
        door: owners.door,
        chest: owners.chest,
        spells: owners.spells,
        achievements: owners.achievements,
        imported_charms: owners.imported_charms,
        spell_states: Mutex::default(),
        clock_origin: std::time::Instant::now(),
        lost: std::sync::Mutex::default(),
        revision_sequencer:
            crate::durability::character_revision_sequencer::CharacterRevisionSequencer::new(),
        // QUEST-STATE-1: no quest content is loaded yet (QUEST-LOWER-1 adds the loader), so
        // admission loads each copy and leaves its obligations pending.
        quest_catalogue: None,
        quest_sessions: std::sync::Mutex::default(),
        premium: premium_refresher(owners.root),
        fence_holders: FenceHolders::default(),
    };
    // QUEST-STATE-0 §5.4: the owner cadence requests failed quest obligations again, whether or
    // not the session's connection runs any other cadence. It never ends on its own.
    let quest_retries = async {
        loop {
            if first(
                tokio::time::sleep(QUEST_RETRY_CADENCE),
                shutdown.cancelled(),
            )
            .await
            .is_none()
            {
                std::future::pending::<()>().await;
            }
            authority.refresh_due_quest_sessions().await;
        }
    };
    first(
        serve_listener(
            listener,
            &tls,
            limits,
            &authority,
            &SecureIdentifiers,
            &(),
            shutdown,
        ),
        quest_retries,
    )
    .await;
    Ok(())
}

/// PREM-1b: the node's Premium pulls. Without configuration, or with an invalid one, there is no
/// client and every account reads Free; admission is never affected.
fn premium_refresher(root: &DurabilityRoot) -> crate::premium::refresh::PremiumRefresher {
    use crate::premium::client::{PremiumClientConfig, PremiumSnapshotClient};
    let client = match PremiumClientConfig::from_env()
        .and_then(|config| config.map(|c| PremiumSnapshotClient::new(&c)).transpose())
    {
        Ok(client) => client,
        Err(error) => {
            operator_event(&format!("premium_snapshot_client_disabled reason={error}"));
            None
        }
    };
    crate::premium::refresh::PremiumRefresher::new(Default::default(), root.clone(), client)
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
    pub(crate) runtime: &'a Mutex<ChannelRuntimeV1>,
    pub(crate) movement_cells: &'a NativeEntryMovementCells,
    /// The one entry-room door's bound runtime (#162 5868482467, M2b). Always locked after
    /// `runtime`, never before, so `step` and `use_object` can never deadlock against each
    /// other.
    pub(crate) door: &'a Mutex<crate::world_runtime::LocalObjectRuntime>,
    /// C2: the entry-room Content with the injected reward chest; never locked, read-only.
    pub(crate) chest: &'a crate::content::CanonicalReferencePlayableContent,
    pub(crate) spells: &'a crate::spell::SpellBook,
    pub(crate) achievements: &'a crate::achievement_catalogue::AchievementCatalogue,
    // Keep the same imported catalogue resident for every connection served by this owner.
    #[allow(
        dead_code,
        reason = "data-only import; actual Charm gameplay consumers are separately composed"
    )]
    pub(crate) imported_charms: &'a crate::content::charm_source::CanonicalCharmCatalogue,
    /// The Channel owner's player vitals and cooldowns (spell cast §4). Always locked after
    /// `runtime`, never before, like `door`.
    pub(crate) spell_states: Mutex<actor_spell::ChannelSpellStates>,
    /// Origin of the owner clock cooldowns are measured on: process-local and monotonic, never a
    /// client or wall-clock time (spell cast §2).
    pub(crate) clock_origin: std::time::Instant,
    /// Ended admitted sessions whose loss is durably recorded and whose grace has not ended:
    /// the only sessions a `ClientResume` can name. At most one entry per admitted session.
    pub(crate) lost: std::sync::Mutex<std::collections::HashMap<GameSessionId, AdmittedSession>>,
    /// One revision-advancing write in flight per Character (CHAR-REV-SEQ-1). Never awaited
    /// while `runtime` is locked.
    pub(crate) revision_sequencer:
        crate::durability::character_revision_sequencer::CharacterRevisionSequencer,
    /// The quest catalogue of the served content revision (QUEST-STATE-0 §4); `None` while no
    /// quest content is loaded.
    pub(crate) quest_catalogue:
        Option<std::sync::Arc<crate::durability::quest_state::quest::QuestStateCatalogue>>,
    /// The quest copy of each admitted session (QUEST-STATE-0 §7), loaded at fresh admission
    /// and resume. Never held across an await.
    pub(crate) quest_sessions:
        std::sync::Mutex<std::collections::HashMap<GameSessionId, QuestSession>>,
    /// PREM-1b: the account's Premium pulls, started at fresh admission and reconnect without
    /// waiting on them, and cancelled when the session is released.
    pub(crate) premium: crate::premium::refresh::PremiumRefresher,
    /// The releases holding each write fence of the slot (CHARM-DESC-FENCE-LEASE step c).
    pub(crate) fence_holders: FenceHolders,
}

/// One admitted session's quest state (QUEST-STATE-0 §5.4, §7).
#[derive(Debug)]
pub(crate) struct QuestSession {
    /// `None`: the load failed or exceeded a bound; quest actions fail closed.
    #[allow(
        dead_code,
        reason = "the session copy is read by QUEST-PRED-1's predicates"
    )]
    pub(crate) copy: Option<crate::durability::quest_state::QuestStateCopy>,
    /// When the pending obligations may be requested again after a failed attempt.
    pub(crate) retry_at: Option<std::time::Instant>,
}

/// SPEED-1: the level a player's effective speed uses until a Character progression owner
/// supplies it (as `character_cast_facts` waits for one).
const PLAYER_LEVEL_UNTIL_PROGRESSION_OWNER: u32 = 1;

/// The backoff before pending obligations are requested again (QUEST-STATE-0 §5.4).
const QUEST_OBLIGATION_RETRY: Duration = Duration::from_secs(60);
/// How often the owner looks for sessions whose quest backoff has passed.
const QUEST_RETRY_CADENCE: Duration = Duration::from_secs(10);

/// The write-fence token of one transition (CHARM-DESC-FENCE-LEASE §3 item 2): the lost epoch
/// for grace expiry and the D449 capability-mismatch release, otherwise a transition id the
/// Channel owner minted.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum TransitionFence {
    GraceExpiry(u64),
    Transition(u64),
}

impl TransitionFence {
    /// Step (a) on the Channel owner, under its runtime lock.
    fn fence(
        self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<(), CarrierError> {
        match self {
            Self::GraceExpiry(epoch) => runtime.fence_player_writes(
                actor,
                session,
                ChannelRuntimeV1::grace_expiry_fence(epoch),
            ),
            Self::Transition(id) => {
                runtime.fence_player_writes(actor, session, ChannelRuntimeV1::transition_fence(id))
            }
        }
        .map(|_| ())
    }

    /// The exact-token lift of step (c); any other token is a no-op.
    fn lift(
        self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<bool, CarrierError> {
        match self {
            Self::GraceExpiry(epoch) => runtime.lift_player_fence(
                actor,
                session,
                ChannelRuntimeV1::grace_expiry_fence(epoch),
            ),
            Self::Transition(id) => {
                runtime.lift_player_fence(actor, session, ChannelRuntimeV1::transition_fence(id))
            }
        }
    }
}

/// Step (a) of one write-fenced transition (CHARM-DESC-FENCE-LEASE §3 item 2).
enum FenceStep {
    Fenced,
    /// Another transition of the session is unsettled: wait for its step (c), then re-evaluate.
    Wait,
    /// The slot does not hold the session (or, for grace expiry, its lost epoch): no start.
    Refused(CarrierError),
}

/// The durable terminal release a connection decides.
#[derive(Clone, Copy)]
enum TerminalRelease {
    /// A resumed session whose recovered connection ended again, on its exact transport.
    Abandoned(AuthenticatedTransportRefV1),
    /// D449: a lost session whose otherwise valid resume lacked a selected capability, at the
    /// exact loss epoch the resume verified.
    CapabilityMismatch(ControlLossEpochRefV1),
}

impl TerminalRelease {
    /// The write-fence token of the release. The mismatch release fences with its lost epoch,
    /// as grace expiry does: its own retry and grace expiry join that fence instead of waiting
    /// on it, a resume of the epoch that wins the race lifts it (`restore_control`), and once
    /// control is restored no mismatch attempt can fence the slot again. An abandoned resumed
    /// session has no loss mark, so its release mints a transition id.
    fn fence(self, runtime: &mut ChannelRuntimeV1) -> Result<TransitionFence, CarrierError> {
        match self {
            Self::Abandoned(_) => runtime
                .mint_transition_fence()
                .map(TransitionFence::Transition),
            Self::CapabilityMismatch(epoch) => Ok(TransitionFence::GraceExpiry(epoch.get())),
        }
    }
}

/// The releases in flight on each write fence of the Channel owner. Grace expiry and the D449
/// mismatch releases of one lost epoch join one `ControlLoss(epoch)` fence, so only the last
/// holder to settle lifts it: a joiner settling early never lifts the fence while another
/// release of the epoch is still saving or committing. Only changed while `runtime` is locked.
#[derive(Default)]
pub(crate) struct FenceHolders(
    std::sync::Mutex<std::collections::HashMap<(GameSessionId, TransitionFence), u32>>,
);

/// One release's hold on its fence token, taken at its first fence and given back when it
/// settles or stops. A release that ends unknown keeps its hold, so the fence stays.
struct FenceHold {
    session: GameSessionId,
    token: TransitionFence,
    held: bool,
}

impl FenceHold {
    const fn new(session: GameSessionId, token: TransitionFence) -> Self {
        Self {
            session,
            token,
            held: false,
        }
    }
}

impl FenceHolders {
    fn counts(
        &self,
    ) -> std::sync::MutexGuard<'_, std::collections::HashMap<(GameSessionId, TransitionFence), u32>>
    {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Step (a) under the runtime lock: fence, or join the fence, and count this release once.
    fn fence(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        hold: &mut FenceHold,
    ) -> Result<(), CarrierError> {
        hold.token.fence(runtime, actor, hold.session)?;
        if !hold.held {
            *self.counts().entry((hold.session, hold.token)).or_default() += 1;
            hold.held = true;
        }
        Ok(())
    }

    /// Step (c) under the runtime lock: give back this release's hold, lifting the exact token
    /// only when no other release holds it. A release holding nothing lifts nothing.
    fn lift(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        hold: &mut FenceHold,
    ) -> Result<(), CarrierError> {
        if !hold.held {
            return Ok(());
        }
        let key = (hold.session, hold.token);
        let mut counts = self.counts();
        let others = counts.get(&key).copied().unwrap_or(1).saturating_sub(1);
        if others == 0 {
            hold.token.lift(runtime, actor, hold.session)?;
            counts.remove(&key);
        } else {
            counts.insert(key, others);
        }
        hold.held = false;
        Ok(())
    }

    /// A release that stops without settling the fence (its slot or epoch moved): its hold
    /// ends and the fence is left to the others.
    fn release(&self, hold: &mut FenceHold) {
        if !hold.held {
            return;
        }
        let key = (hold.session, hold.token);
        let mut counts = self.counts();
        match counts.get(&key).copied() {
            Some(count) if count > 1 => {
                counts.insert(key, count - 1);
            }
            _ => {
                counts.remove(&key);
            }
        }
        hold.held = false;
    }

    /// The session left the slot: its fences went with it.
    fn forget(&self, session: GameSessionId) {
        self.counts().retain(|(held, _), _| *held != session);
    }
}

/// Step (c) of a transition whose durable attempt did not end the hold.
enum UnendedSettle {
    Lifted,
    Terminal,
    Unknown,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// QUEST-STATE-0 §7 and §5.4: load the admitted session's quest copy and request its
    /// pending obligations again, in the Character's revision slot. A failed load fails the
    /// session's quest actions closed, never the login.
    async fn admit_quest_session(&self, admitted: &AdmittedSession) {
        self.refresh_quest_session(admitted.game_session_id, true)
            .await;
    }

    /// Reload the quest copy of `session` from the store and request its pending obligations
    /// again. A failed load or an attempt without an outcome is tried again after
    /// [`QUEST_OBLIGATION_RETRY`] on the owner cadence. `insert` is false for a refresh of a
    /// session that may have ended meanwhile: it then updates only a session still present.
    async fn refresh_quest_session(&self, session: GameSessionId, insert: bool) {
        let fence = self.current_quest_fence(session).await;
        let (copy, retry) = match fence {
            Ok(Some(fence)) => {
                let admission = crate::durability::quest_state::admit_character_quest_state(
                    &self.revision_sequencer,
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    self.quest_catalogue.as_ref(),
                )
                .await;
                let retry = admission.retry || admission.copy.is_none();
                (Some(admission.copy), retry)
            }
            // A proven terminal session: no quest action and nothing to retry.
            Ok(None) => (Some(None), false),
            // A failed read proves nothing: keep the current copy and try again.
            Err(()) => (None, true),
        };
        let retry_at = retry.then(|| std::time::Instant::now() + QUEST_OBLIGATION_RETRY);
        if let Ok(mut sessions) = self.quest_sessions.lock() {
            match (copy, sessions.get_mut(&session)) {
                (Some(copy), Some(entry)) => *entry = QuestSession { copy, retry_at },
                (Some(copy), None) if insert => {
                    sessions.insert(session, QuestSession { copy, retry_at });
                }
                (None, Some(entry)) => entry.retry_at = retry_at,
                (None, None) if insert => {
                    sessions.insert(
                        session,
                        QuestSession {
                            copy: None,
                            retry_at,
                        },
                    );
                }
                _ => {}
            }
        }
    }

    /// The owner cadence: refresh every session whose backoff has passed. The map lock is
    /// never held across an await.
    async fn refresh_due_quest_sessions(&self) {
        let now = std::time::Instant::now();
        let due: Vec<GameSessionId> = match self.quest_sessions.lock() {
            Ok(mut sessions) => sessions
                .iter_mut()
                .filter(|(_, entry)| entry.retry_at.is_some_and(|at| at <= now))
                .map(|(session, entry)| {
                    entry.retry_at = None;
                    *session
                })
                .collect(),
            Err(_) => return,
        };
        for session in due {
            self.refresh_quest_session(session, false).await;
        }
    }

    /// Refresh `session` on the owner cadence after the backoff.
    fn schedule_quest_refresh(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.quest_sessions.lock()
            && let Some(entry) = sessions.get_mut(&session)
        {
            entry.retry_at = Some(std::time::Instant::now() + QUEST_OBLIGATION_RETRY);
        }
    }

    fn forget_quest_session(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.quest_sessions.lock() {
            sessions.remove(&session);
        }
    }

    /// The gameplay fence of `session` from current durable reads; its expected revision is
    /// replaced by the revision slot's cursor at commit. `Ok(None)` when the session is proven
    /// terminal; `Err` when a read failed, which proves nothing.
    async fn current_quest_fence(
        &self,
        session: GameSessionId,
    ) -> Result<Option<crate::durability::character_progression::CurrentCharacterGameplayFence>, ()>
    {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (current, _) = store.current_session_at(session).await.map_err(|_| ())?;
        if current.session_state() == crate::foundation::GameSessionState::Terminal {
            return Ok(None);
        }
        let character_id =
            domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
                .map_err(|_| ())?;
        let record = self
            .root
            .read_current_character(self.character, character_id)
            .await
            .map_err(|_| ())?;
        Ok(Some(
            crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_id,
                game_session_id: session,
                connection_generation: current.current_connection_generation(),
                character_lease_generation: current.current_character_lease().generation(),
                runtime_scope: current.current_runtime_scope(),
                scope_ownership_generation: current.current_scope_generation(),
                expected_character_revision: record.revision,
            },
        ))
    }

    /// Drop the lost entry of `session` only if it is still the one that ended at
    /// `generation`; a later resumed and lost again connection keeps its own entry.
    fn forget_lost(&self, session: GameSessionId, generation: u64) {
        if let Ok(mut lost) = self.lost.lock()
            && lost
                .get(&session)
                .is_some_and(|entry| entry.continuity.connection_generation == generation)
        {
            lost.remove(&session);
        }
    }

    /// CHARM-DESC-FENCE-LEASE: grace expiry fences the slot's damage writes under the runtime
    /// lock (step a), commits the durable release with the lock released (step b), then settles
    /// the slot by the durable outcome (step c). The fence token is the lost epoch, so a retry
    /// joins its own fence and a resume of that epoch lifts it.
    async fn release_after_grace(&self, admitted: AdmittedSession) -> GraceExpiryResult {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return GraceExpiryResult::NotApplicable;
        };
        let session = admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let account_id = canonical_uuid(&controller.account_id);
        // The deadline is fixed, so waiting converges; store failures back off
        // exponentially, and the bounds only guard an owner that never recovers.
        let mut backoff = RECONCILE_BACKOFF;
        let mut next_backoff = || {
            let pause = backoff;
            backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
            pause
        };
        // One hold across the attempts of an epoch, so a retry is counted once.
        let mut held: Option<FenceHold> = None;
        for _ in 0..EXPIRY_ATTEMPTS {
            let mark = match self
                .runtime
                .lock()
                .await
                .player_control_facts(actor, session)
            {
                Ok(facts) => facts.control_loss,
                Err(_) => return GraceExpiryResult::Unknown,
            };
            // The lost epoch was resumed: nothing is this transition's to release.
            let Some(ControlLossMark {
                epoch,
                grace_deadline: deadline,
            }) = mark
            else {
                if let Some(hold) = held.as_mut() {
                    self.fence_holders.release(hold);
                }
                return GraceExpiryResult::NotApplicable;
            };
            let token = TransitionFence::GraceExpiry(epoch);
            if let Some(stale) = held.as_mut().filter(|hold| hold.token != token) {
                self.fence_holders.release(stale);
            }
            let hold = match &mut held {
                Some(hold) if hold.token == token => hold,
                slot => slot.insert(FenceHold::new(session, token)),
            };
            // §3 item 6: fence only once the held deadline has passed.
            if let Some(ahead) = unix_seconds()
                .and_then(|now| u64::try_from(deadline.saturating_sub(now)).ok())
                .filter(|ahead| *ahead > 0)
            {
                tokio::time::sleep(Duration::from_secs(ahead).saturating_add(EXPIRY_SLACK)).await;
                continue;
            }
            match self.fence_transition(actor, hold).await {
                FenceStep::Fenced => {}
                FenceStep::Wait => {
                    tokio::time::sleep(next_backoff()).await;
                    continue;
                }
                // The mark moved: the epoch was resumed before the fence.
                FenceStep::Refused(CarrierError::ControlLossConflict) => {
                    return GraceExpiryResult::NotApplicable;
                }
                FenceStep::Refused(_) => return GraceExpiryResult::Unknown,
            }
            // SPELL-D8 §8.2 save point 1: the actor's monk values are durable, or fenced out,
            // before the release can end the Character lease.
            if self.save_monk_state(&admitted, actor).await == monk_save::MonkSave::Unknown {
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            let pause = match store.release_expired_loss(session, &account_id).await {
                Ok(ExpiredLossReleaseV1::NotApplicable) => {
                    match self.settle_unended(&store, actor, hold).await {
                        UnendedSettle::Lifted => return GraceExpiryResult::NotApplicable,
                        UnendedSettle::Terminal => return self.retire(session, actor).await,
                        UnendedSettle::Unknown => next_backoff(),
                    }
                }
                Ok(ExpiredLossReleaseV1::NotExpired { deadline, now }) => {
                    match self.settle_unended(&store, actor, hold).await {
                        UnendedSettle::Lifted => {
                            Duration::from_secs(u64::try_from(deadline - now).unwrap_or(0))
                                .saturating_add(EXPIRY_SLACK)
                        }
                        UnendedSettle::Terminal => return self.retire(session, actor).await,
                        UnendedSettle::Unknown => next_backoff(),
                    }
                }
                Ok(ExpiredLossReleaseV1::Released { .. } | ExpiredLossReleaseV1::Terminal) => {
                    // PREM-1b: the session is over; its Premium pulls stop.
                    self.premium.release(controller.account_id);
                    return self.retire(session, actor).await;
                }
                // Unknown outcome: keep the fence; the retry reconciles from the durable row.
                Err(_) => next_backoff(),
            };
            tokio::time::sleep(pause).await;
        }
        GraceExpiryResult::Unknown
    }

    /// CHARM-DESC-FENCE-LEASE step (a): fence the slot's damage writes for `session` under the
    /// runtime lock, with no I/O while it is held. Another unsettled transition of the session
    /// is waited for, never replaced.
    async fn fence_transition(&self, actor: ExactActorRef, hold: &mut FenceHold) -> FenceStep {
        let mut runtime = self.runtime.lock().await;
        match self.fence_holders.fence(&mut runtime, actor, hold) {
            Ok(()) => FenceStep::Fenced,
            Err(CarrierError::WriteFenceBusy) => FenceStep::Wait,
            Err(error) => {
                self.fence_holders.release(hold);
                FenceStep::Refused(error)
            }
        }
    }

    /// Step (c) for a durable outcome that did not end the hold in this attempt (`NotApplicable`,
    /// `NotExpired`): lift the fence with its exact token only if a durable read shows the session
    /// still holding the lease. TERMINAL is absorbing, so a non-terminal read after the attempt
    /// proves the session held it at the attempt too. A terminal row settles as terminal, with the
    /// session kept fenced; an unreadable row keeps the fence. A fence another release of the
    /// epoch still holds is left to it (`FenceHolders`); a resume of the epoch may already have
    /// lifted it.
    async fn settle_unended(
        &self,
        store: &FreshAdmissionStore,
        actor: ExactActorRef,
        hold: &mut FenceHold,
    ) -> UnendedSettle {
        let Ok(current) = store.current_session(hold.session).await else {
            return UnendedSettle::Unknown;
        };
        if current.session_state() == GameSessionState::Terminal {
            return UnendedSettle::Terminal;
        }
        match self
            .fence_holders
            .lift(&mut *self.runtime.lock().await, actor, hold)
        {
            Ok(()) => UnendedSettle::Lifted,
            Err(_) => UnendedSettle::Unknown,
        }
    }

    /// Step (c) for a TERMINAL session: the durable fact that allows removing the exact actor.
    /// Its fence is never lifted; the slot goes with it.
    async fn retire(&self, session: GameSessionId, actor: ExactActorRef) -> GraceExpiryResult {
        match self
            .runtime
            .lock()
            .await
            .remove_terminal_session(session, actor)
        {
            Ok(()) => {
                self.fence_holders.forget(session);
                self.forget_quest_session(session);
                GraceExpiryResult::Released
            }
            Err(_) => GraceExpiryResult::Unknown,
        }
    }

    /// The fence, monk save, durable commit and settle steps of a terminal release that a
    /// connection decides (abandoned resume, capability mismatch), under one transition token
    /// minted for it. The actor leaves the Channel only after the durable TERMINAL fact.
    async fn release_terminal(
        &self,
        admitted: AdmittedSession,
        release: TerminalRelease,
    ) -> GraceExpiryResult {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return GraceExpiryResult::NotApplicable;
        };
        let session = admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let account_id = canonical_uuid(&controller.account_id);
        let Ok(token) = release.fence(&mut *self.runtime.lock().await) else {
            return GraceExpiryResult::Unknown;
        };
        let mut hold = FenceHold::new(session, token);
        let mut backoff = RECONCILE_BACKOFF;
        let mut next_backoff = || {
            let pause = backoff;
            backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
            pause
        };
        for _ in 0..EXPIRY_ATTEMPTS {
            match self.fence_transition(actor, &mut hold).await {
                FenceStep::Fenced => {}
                FenceStep::Wait => {
                    tokio::time::sleep(next_backoff()).await;
                    continue;
                }
                FenceStep::Refused(_) => return GraceExpiryResult::Unknown,
            }
            // SPELL-D8 §8.2 save point 1: the actor's monk values are durable, or fenced out,
            // before the release can end the Character lease.
            if self.save_monk_state(&admitted, actor).await == monk_save::MonkSave::Unknown {
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            let outcome = match release {
                TerminalRelease::Abandoned(transport) => {
                    store
                        .release_abandoned_session(session, &account_id, transport)
                        .await
                }
                TerminalRelease::CapabilityMismatch(epoch) => {
                    store
                        .release_capability_mismatch(session, &account_id, epoch)
                        .await
                }
            };
            let pause = match outcome {
                Ok(
                    ExpiredLossReleaseV1::NotApplicable | ExpiredLossReleaseV1::NotExpired { .. },
                ) => match self.settle_unended(&store, actor, &mut hold).await {
                    UnendedSettle::Lifted => return GraceExpiryResult::NotApplicable,
                    UnendedSettle::Terminal => return self.retire(session, actor).await,
                    UnendedSettle::Unknown => next_backoff(),
                },
                Ok(ExpiredLossReleaseV1::Released { .. } | ExpiredLossReleaseV1::Terminal) => {
                    if matches!(release, TerminalRelease::CapabilityMismatch(_)) {
                        // PREM-1b: the lost session is over; its Premium pulls stop, as at
                        // grace expiry.
                        self.premium.release(controller.account_id);
                    }
                    return self.retire(session, actor).await;
                }
                // Unknown outcome: keep the fence; the retry reconciles from the durable row.
                Err(_) => next_backoff(),
            };
            tokio::time::sleep(pause).await;
        }
        if matches!(release, TerminalRelease::CapabilityMismatch(_)) {
            // D449: an unproven mismatch release is reconciled from the durable row. TERMINAL
            // settles as released; a session still holding the lease gives back this release's
            // hold, and the epoch fence is lifted once no other release holds it, so the client's
            // retry can repeat the release.
            return match self.settle_unended(&store, actor, &mut hold).await {
                UnendedSettle::Terminal => {
                    self.premium.release(controller.account_id);
                    self.retire(session, actor).await
                }
                UnendedSettle::Lifted | UnendedSettle::Unknown => GraceExpiryResult::Unknown,
            };
        }
        GraceExpiryResult::Unknown
    }

    /// D449 (ARCH-BATCH-ITEM-EQUIP-PACKETS §1.13): a resume that passed every check but the
    /// capability check ends the lost session in a terminal release with no successor, so the
    /// client's fresh admission finds the character free. Nothing is resumed; the lost entry
    /// is dropped once the release is durable.
    pub(super) async fn release_capability_mismatch(
        &self,
        lost: AdmittedSession,
        epoch: ControlLossEpochRefV1,
    ) -> GraceExpiryResult {
        let result = self
            .release_terminal(lost, TerminalRelease::CapabilityMismatch(epoch))
            .await;
        if result == GraceExpiryResult::Released {
            self.forget_lost(lost.game_session_id, lost.continuity.connection_generation);
        }
        result
    }

    fn observation(
        runtime: &ChannelRuntimeV1,
        position: crate::foundation::MovementLocalPosition,
    ) -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: runtime.content_pin().client_artifact_digest(),
            actor_position: ActorPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        }
    }

    /// USE-WIRE-V1 reach (#162 5868482467): same floor, Chebyshev distance <=1 from any of the
    /// target's own collision cells, no line-of-sight check. A pure function so it is directly
    /// unit-testable independent of any Channel runtime: the qualified native entry room is too
    /// small (#935/#937: every accepted walkable cell is already within one step of the door) to
    /// exercise a genuine TOO_FAR case through real movement.
    fn use_object_reachable(
        actor_x: i32,
        actor_y: i32,
        actor_floor: i32,
        target_cells: &std::collections::BTreeSet<crate::content::LogicalCell>,
    ) -> bool {
        target_cells.iter().any(|cell| {
            cell.z == actor_floor
                && (i64::from(cell.x) - i64::from(actor_x)).abs() <= 1
                && (i64::from(cell.y) - i64::from(actor_y)).abs() <= 1
        })
    }

    /// C2 routing: the chest placement of `chest` that `placement` names, if any. The client
    /// names only the placement; the chest, its claim and its reward come from Content.
    fn chest_target<'c>(
        chest: &'c crate::content::CanonicalReferencePlayableContent,
        placement: &[u8],
    ) -> Option<&'c crate::content::PlacementRef> {
        chest
            .placements
            .iter()
            .find(|candidate| candidate.key.as_str().as_bytes() == placement)
    }

    /// C2: one `USE_INTENT` on the entry chest (USE-WIRE-V1 field 1, #162 5914960502). Reach is
    /// the door's rule (same floor, Chebyshev distance <=1 from the chest's cell), read under
    /// the Channel-owner lock, which is released before the durable MINT so the Channel is
    /// never blocked on the database. The MINT itself is DUR-03's
    /// (`interaction_chest_use::use_chest`), fenced by the admitted session's item fence and
    /// keyed by this `USE`'s `CommandRef`: a mint is `COMMITTED` with no overlay delta, a claim
    /// already taken is `NOTHING_TO_USE`, anything else `REJECTED` with nothing written.
    async fn use_chest(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        chest: &crate::content::PlacementRef,
    ) -> UseOutcome {
        use crate::interaction_chest_use::{ChestUseRequest, entry_chest, use_chest};
        // The chest must stand in this Channel's World (D39 review 5906461018 item 7).
        if chest.address.world_id != self.world_id {
            return UseOutcome::rejected();
        }
        {
            let mut runtime = self.runtime.lock().await;
            let Ok(expected) = runtime.borrow_movement_position().read(actor) else {
                return UseOutcome::rejected();
            };
            let position = expected.position();
            let cells = std::collections::BTreeSet::from([chest.address.cell]);
            if !Self::use_object_reachable(
                position.x,
                position.y,
                i32::from(position.floor),
                &cells,
            ) {
                return UseOutcome {
                    disposition: world_object::UseDisposition::TooFar,
                    committed: None,
                    earned: connection::EarnedNotice::NoneEarned,
                };
            }
        }
        let Ok(command_id) = crate::foundation::CommandId::new(command.command_id) else {
            return UseOutcome::rejected();
        };
        let request = ChestUseRequest {
            command: crate::foundation::CommandRef::new(command.game_session_id, command_id),
            chest: chest.key.clone(),
            content_revision: entry_chest::CONTENT_REVISION.to_owned(),
            ruleset_revision: entry_chest::RULESET_REVISION.to_owned(),
            sim_revision: entry_chest::SIM_REVISION.to_owned(),
        };
        let session = crate::combat::DurabilitySession {
            root: self.root,
            authority: self.character,
            node: self.holder,
        };
        let (disposition, _, notice) = use_chest(
            &session,
            self.chest,
            self.achievements,
            command.item_fence,
            request,
        )
        .await;
        // QUEST-STATE-0 §5.4: a chest that advances a quest left a PENDING obligation with its
        // claim; request it in this session. An outcome that is not proven is picked up by the
        // owner cadence after the backoff.
        if crate::interaction_chest_use::resolve_chest(self.chest, &chest.key)
            .is_ok_and(|resolved| resolved.quest_transition.is_some())
        {
            match disposition {
                world_object::UseDisposition::Committed => {
                    self.refresh_quest_session(command.game_session_id, false)
                        .await;
                }
                world_object::UseDisposition::Rejected => {
                    self.schedule_quest_refresh(command.game_session_id);
                }
                _ => {}
            }
        }
        UseOutcome {
            disposition,
            committed: None,
            earned: earned_notice(self.achievements, notice, &mut operator_event),
        }
    }
}

/// Display contract §3.3, §4: the page of `facts`, encoded. A fact under a key the catalogue
/// lacks is an integrity fault: one operator event through `log` and `ACCOUNT_DATA_INTEGRITY`
/// (owner decision 2026-09-30). A malformed row or an overflow is a server fault (`REJECTED`, no
/// rows, no code); a row over its byte bounds fails closed with `PAYLOAD_LIMIT_EXCEEDED`. Nothing
/// is truncated.
fn account_achievements_reply(
    catalogue: &crate::achievement_catalogue::AchievementCatalogue,
    facts: &[crate::durability::account_achievement::EarnedAchievement],
    request: &AccountAchievementsRequest,
    log: &mut dyn FnMut(&str),
) -> AccountAchievementsReply {
    use oteryn_protocol_oteryn::account_achievements::{
        AccountAchievementsError, encode_account_achievements_result,
    };
    let result = match catalogue.answer_account_achievements(facts, request, log) {
        Ok(result) => result,
        Err(error) => {
            return error.protocol_error().map_or(
                AccountAchievementsReply::Rejected,
                AccountAchievementsReply::Terminal,
            );
        }
    };
    match encode_account_achievements_result(&result) {
        Ok(payload) => AccountAchievementsReply::Page(payload),
        Err(AccountAchievementsError::LimitExceeded) => {
            AccountAchievementsReply::Terminal(FoundationProtocolError::PayloadLimitExceeded)
        }
        Err(AccountAchievementsError::Malformed) => AccountAchievementsReply::Rejected,
    }
}

/// ACHIEVEMENT-0 §5: the watermark of the account's facts (the display contract's `fact_count`
/// and `total_points`); `None` for a key the catalogue lacks.
fn achievement_watermark(
    catalogue: &crate::achievement_catalogue::AchievementCatalogue,
    facts: &[crate::durability::account_achievement::EarnedAchievement],
) -> Option<oteryn_protocol_oteryn::achievement_notices::AchievementWatermark> {
    let page = catalogue.account_achievements_page(facts, 0).ok()?;
    Some(
        oteryn_protocol_oteryn::achievement_notices::AchievementWatermark {
            fact_count: page.fact_count,
            total_points: page.total_points,
        },
    )
}

/// ACH-NOTIFY-1: the earned delta of a committed `Granted` grant, its name from the catalogue
/// and its watermark from the fact keys read in the granting transaction.
fn achievement_earned(
    catalogue: &crate::achievement_catalogue::AchievementCatalogue,
    notice: &crate::durability::reward_claim_mint::GrantedAchievementNotice,
) -> Option<oteryn_protocol_oteryn::achievement_notices::AchievementEarned> {
    use crate::durability::account_achievement::EarnedAchievement;
    let fact = |key: &String| EarnedAchievement {
        achievement_key: key.clone(),
        earned_at_unix_ms: 0,
    };
    let facts: Vec<_> = notice.account_fact_keys.iter().map(fact).collect();
    let granted = catalogue
        .account_achievements_page(&[fact(&notice.achievement_key)], 0)
        .ok()?;
    Some(
        oteryn_protocol_oteryn::achievement_notices::AchievementEarned {
            key: notice.achievement_key.clone(),
            name: granted.rows.into_iter().next()?.name,
            watermark: achievement_watermark(catalogue, &facts)?,
        },
    )
}

/// ACH-NOTIFY-2: the notice of a `USE`'s grant. A `Granted` grant whose notice cannot be derived
/// (a fact key the catalogue lacks) and an `Unknown` read are both `Unknown`, never
/// `NoneEarned`, and each is one operator defect event through `log`.
fn earned_notice(
    catalogue: &crate::achievement_catalogue::AchievementCatalogue,
    notice: crate::durability::reward_claim_mint::GrantNotice,
    log: &mut dyn FnMut(&str),
) -> connection::EarnedNotice {
    use crate::durability::reward_claim_mint::GrantNotice;
    let earned = match notice {
        GrantNotice::None => return connection::EarnedNotice::NoneEarned,
        GrantNotice::Granted(notice) => achievement_earned(catalogue, &notice),
        GrantNotice::Unknown => None,
    };
    earned.map_or_else(
        || {
            log("event=achievement_notice_unknown level=error");
            connection::EarnedNotice::Unknown
        },
        connection::EarnedNotice::Earned,
    )
}

/// One structured stderr event line for the operator, as the node's own events are written
/// (OPS-NODE-BOOT-01 D6).
fn operator_event(line: &str) {
    eprintln!("oteryn-game-server {line}");
}

impl FreshAdmissionAuthority for ComposedFreshAdmission<'_, '_, '_> {
    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        let mut runtime = self.runtime.lock().await;
        let snapshot = runtime.borrow_movement_position().read(actor).ok()?;
        if snapshot.context() != runtime.pinned_movement_context() {
            return None;
        }
        Some(Self::observation(&runtime, snapshot.position()))
    }

    /// The Channel's current door `WORLD_OBJECT_OVERLAY` (USE-WIRE-V1, #162 5868482467), for
    /// the join/resync snapshot. Channel-global, unlike `observe`: no actor is involved.
    async fn observe_world_object_overlay(&self) -> Option<world_object::WorldObjectOverlayEntry> {
        let runtime = self.runtime.lock().await;
        let door = self.door.lock().await;
        Some(world_object::WorldObjectOverlayEntry {
            content_generation: runtime.content_pin().client_artifact_digest(),
            placement: door.placement_key().as_str().as_bytes().to_vec(),
            state: door.state_key().as_str().as_bytes().to_vec(),
            revision: door.revision(),
        })
    }

    /// One Channel-owner work item for one actor (`MOVE-RL-02` = 1): the pinned context, one
    /// direct lookup in the active generation's qualified cells (`MOVE-RL-03` = 1) and the
    /// owner's compare-commit. A blocked or out-of-room destination is `Blocked`; any stale,
    /// unpositioned or mismatched binding is `Rejected`. Nothing moves unless the step commits.
    ///
    /// SPEED-1: the step is paced by the step duration onto its destination (CONDITIONS-0 §4.2):
    /// the player's effective speed, with the actor's active `SPEED` condition delta for
    /// `session` at the owner's time, and the destination's ground speed from the §1.11 seam,
    /// which on the engineering map is 150 for every tile. A destination whose duration cannot
    /// be computed (ground speed 0, or no readable condition delta) is refused before anything
    /// moves.
    async fn paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<Duration>) {
        use crate::movement::speed::{
            EngineeringGroundSpeed, player_step_duration, runtime_player_speed,
        };
        use crate::movement::{
            CardinalStep, MovementEngineeringSelection, MovementError, MovementOwnerTurn,
            MovementTurnOutcome,
        };
        use std::num::NonZeroUsize;
        let mut runtime = self.runtime.lock().await;
        // The cells must be the pinned generation's own: same World and server artifact.
        let scope = self.movement_cells.scope();
        if scope.world_id != self.world_id
            || scope.generation_digest != runtime.content_pin().server_artifact_digest()
        {
            return (StepOutcome::rejected(), None);
        }
        let owner_context = runtime.pinned_movement_context();
        let cardinal = match direction {
            StepDirection::North => CardinalStep::North,
            StepDirection::East => CardinalStep::East,
            StepDirection::South => CardinalStep::South,
            StepDirection::West => CardinalStep::West,
        };
        // M2b (#162 5868482467): the door cell is now ordinary `Walkable` terrain in
        // `self.movement_cells.index()` (native_entry.rs), so a closed door must be refused
        // here, before the terrain lookup, from the door `LocalObjectRuntime`'s own current
        // `blocking_cells()` — the exact same Channel-owner turn, so no path can observe a
        // closed door as walkable. An open door (or any other destination) falls through to the
        // unchanged terrain lookup below.
        let mut duration = None;
        if let Ok(expected) = runtime.borrow_movement_position().read(actor) {
            let position = expected.position();
            let delta = match direction {
                StepDirection::North => (0, -1),
                StepDirection::East => (1, 0),
                StepDirection::South => (0, 1),
                StepDirection::West => (-1, 0),
            };
            if let (Some(x), Some(y)) = (
                position.x.checked_add(delta.0),
                position.y.checked_add(delta.1),
            ) {
                let target = crate::content::LogicalCell {
                    x,
                    y,
                    z: i32::from(position.floor),
                };
                if self.door.lock().await.blocking_cells().contains(&target) {
                    return (
                        StepOutcome {
                            disposition: StepDisposition::Blocked,
                            moved_to: None,
                        },
                        None,
                    );
                }
                // No Character progression owner is composed yet, so the speed is the level 1
                // base with the actor's active `SPEED` delta and no equipment term.
                duration = runtime_player_speed(
                    &runtime,
                    actor,
                    session,
                    PLAYER_LEVEL_UNTIL_PROGRESSION_OWNER,
                    self.owner_now(),
                )
                .and_then(|speed| player_step_duration(&EngineeringGroundSpeed, target, speed));
            }
        }
        // A step whose destination or duration cannot be computed never runs unpaced.
        let Some(duration) = duration else {
            return (StepOutcome::rejected(), None);
        };
        let outcome = {
            let mut turn = MovementOwnerTurn::begin(&mut runtime, NonZeroUsize::MIN);
            let Ok(expected) = turn.read(actor) else {
                return (StepOutcome::rejected(), None);
            };
            let selection = MovementEngineeringSelection {
                owner_context,
                content_scope: scope,
            };
            turn.try_step(
                actor,
                expected,
                &selection,
                self.movement_cells.index(),
                cardinal,
            )
        };
        match outcome {
            Ok(MovementTurnOutcome::Applied(snapshot)) => (
                StepOutcome {
                    disposition: StepDisposition::Moved,
                    moved_to: Some(Self::observation(&runtime, snapshot.position())),
                },
                Some(duration),
            ),
            Err(
                MovementError::Blocked
                | MovementError::Cell(
                    crate::content::static_cell_engine::StaticCellEngineError::Absent,
                ),
            ) => (
                StepOutcome {
                    disposition: StepDisposition::Blocked,
                    moved_to: None,
                },
                None,
            ),
            Ok(MovementTurnOutcome::Deferred) | Err(_) => (StepOutcome::rejected(), None),
        }
    }

    /// One `USE_INTENT` for the admitted actor against the one entry-room door (#162
    /// 5868482467, USE-WIRE-V1). The client names only the target placement and its expected
    /// revision; the server selects the unique bound transition
    /// (`LocalObjectRuntime::attempt_use`). Reach is same floor, Chebyshev distance <=1 from the
    /// door's own collision cell, no line-of-sight check. `runtime` is locked first, then
    /// `door`, the same fixed order `step` uses, so the two can never deadlock against each
    /// other; both stay locked for this whole decision so nothing else can move the acting actor
    /// or the door state in between.
    ///
    /// C2: a `USE` naming the injected entry chest is routed to [`Self::use_chest`] instead;
    /// every other placement takes the unchanged door path.
    async fn use_object(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        target: world_object::WorldObjectTarget,
    ) -> UseOutcome {
        if let Some(chest) = Self::chest_target(self.chest, &target.placement) {
            return self.use_chest(actor, command, chest).await;
        }
        let mut runtime = self.runtime.lock().await;
        let Ok(expected) = runtime.borrow_movement_position().read(actor) else {
            return UseOutcome::rejected();
        };
        let position = expected.position();
        let mut door = self.door.lock().await;
        if target.placement.as_slice() != door.placement_key().as_str().as_bytes() {
            return UseOutcome {
                disposition: world_object::UseDisposition::NothingToUse,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
            };
        }
        let actor_floor = i32::from(position.floor);
        if !Self::use_object_reachable(position.x, position.y, actor_floor, door.collision_cells())
        {
            return UseOutcome {
                disposition: world_object::UseDisposition::TooFar,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
            };
        }
        // #162 5868482467 (shared-lease P1 repair r4121956127): occupancy is every currently
        // positioned committed actor in the Channel, not only the issuer — including one
        // retained during disconnect grace, which stays positioned (only its `control_loss`
        // mark changes, never its slot's `committed`/`position`). Read under the same `runtime`
        // lock and the same work item as `attempt_use` below, so this is TOCTOU-free: nothing
        // else can move an actor or change the door's state between this read and that call.
        let occupied: std::collections::BTreeSet<crate::content::LogicalCell> = runtime
            .committed_player_positions()
            .into_iter()
            .map(|other| crate::content::LogicalCell {
                x: other.x,
                y: other.y,
                z: i32::from(other.floor),
            })
            .collect();
        let content_generation = runtime.content_pin().client_artifact_digest();
        match door.attempt_use(target.expected_revision, &occupied) {
            Ok(crate::world_runtime::LocalObjectUseOutcome::Committed { state, revision }) => {
                UseOutcome {
                    disposition: world_object::UseDisposition::Committed,
                    committed: Some(world_object::WorldObjectOverlayEntry {
                        content_generation,
                        placement: door.placement_key().as_str().as_bytes().to_vec(),
                        state: state.as_str().as_bytes().to_vec(),
                        revision,
                    }),
                    earned: connection::EarnedNotice::NoneEarned,
                }
            }
            Ok(crate::world_runtime::LocalObjectUseOutcome::NothingToUse) => UseOutcome {
                disposition: world_object::UseDisposition::NothingToUse,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
            },
            Ok(crate::world_runtime::LocalObjectUseOutcome::Occupied) => UseOutcome {
                disposition: world_object::UseDisposition::Occupied,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
            },
            Ok(crate::world_runtime::LocalObjectUseOutcome::StaleState) => UseOutcome {
                disposition: world_object::UseDisposition::StaleState,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
            },
            Ok(crate::world_runtime::LocalObjectUseOutcome::Rejected) | Err(_) => {
                UseOutcome::rejected()
            }
        }
    }

    async fn observe_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        let runtime = self.runtime.lock().await;
        let states = self.spell_states.lock().await;
        actor_spell::observe_vitals(&runtime, &states, actor, game_session_id)
    }

    /// One cast as one Channel-owner work item (`SPELL-RL-01` = 1), under the same runtime lock
    /// `step` and `use_object` take.
    async fn cast_spell(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: actor_spell::SpellCastIntent,
    ) -> actor_spell::SpellCastOutcome {
        let now = self.owner_now();
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        actor_spell::cast_in_channel(
            &runtime,
            &mut states,
            self.spells,
            actor,
            game_session_id,
            command_id,
            &intent,
            now,
        )
    }

    /// Display contract §4: a read-only query of the facts, no Character fence and no Channel
    /// owner lock. More facts than catalogue keys prove an unknown key, so one more than the
    /// catalogue's size is read and the page build refuses it.
    async fn observe_achievement_notices(
        &self,
        account_id: [u8; 16],
    ) -> Option<oteryn_protocol_oteryn::achievement_notices::AchievementWatermark> {
        let limit = self.achievements.len().saturating_add(1);
        let facts = self
            .root
            .read_account_achievements(self.character, account_id, limit)
            .await
            .ok()?;
        achievement_watermark(self.achievements, &facts)
    }

    async fn account_achievements(
        &self,
        request: AccountAchievementsRequest,
    ) -> AccountAchievementsReply {
        let limit = self.achievements.len().saturating_add(1);
        match self
            .root
            .read_account_achievements(self.character, request.account_id, limit)
            .await
        {
            Ok(facts) => {
                account_achievements_reply(self.achievements, &facts, &request, &mut operator_event)
            }
            Err(_) => AccountAchievementsReply::Rejected,
        }
    }

    /// The periodic 1000 ms Serene evaluation of the admitted actor (SPELL-D8 §8.2), under the
    /// same runtime lock as a cast.
    async fn tick_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        let now = self.owner_now();
        let runtime = self.runtime.lock().await;
        let mut states = self.spell_states.lock().await;
        states.tick(&runtime, actor, game_session_id, now)
    }

    async fn lose_control(&self, admitted: AdmittedSession, wait: Duration) -> ControlLossResult {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return ControlLossResult::NotApplicable;
        };
        tokio::time::sleep(wait).await;
        let result = self
            .commit_control_loss(admitted.game_session_id, actor, controller)
            .await;
        if result == ControlLossResult::Recorded {
            // §8.2: no command of the actor is accepted until a recovery initializes it again.
            let runtime = self.runtime.lock().await;
            self.spell_states
                .lock()
                .await
                .detach(&runtime, actor, admitted.game_session_id);
        }
        if result == ControlLossResult::Recorded
            && let Ok(mut lost) = self.lost.lock()
        {
            lost.insert(admitted.game_session_id, admitted);
        }
        result
    }

    async fn resume(
        &self,
        attempt: connection::ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        let admitted = self.resume_lost(attempt).await?;
        // PREM-1b: a reconnect pulls Premium again before any Premium read, without waiting.
        if let Some(controller) = admitted.controller {
            self.premium.admit(controller.account_id);
        }
        self.admit_quest_session(&admitted).await;
        Ok(admitted)
    }

    /// The abandoned-session release follows the same fence, commit and settle steps as grace
    /// expiry (CHARM-DESC-FENCE-LEASE §3 item 1), under one transition token minted for it.
    async fn release_abandoned(&self, admitted: AdmittedSession) -> GraceExpiryResult {
        let Some(controller) = admitted.controller else {
            return GraceExpiryResult::NotApplicable;
        };
        self.release_terminal(admitted, TerminalRelease::Abandoned(controller.transport))
            .await
    }

    async fn expire_control_loss(&self, admitted: AdmittedSession) -> GraceExpiryResult {
        let result = self.release_after_grace(admitted).await;
        // Resumed, released or unprovable: this lost connection can no longer be resumed.
        self.forget_lost(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        );
        result
    }

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
        // A concurrent attempt for the same account (for example the same grant on a second
        // socket) may refresh the owner evidence between this attempt's publication,
        // composition and commit, leaving it stale. Such a round is retried from fresh
        // evidence, bounded; the grant replay key still admits at most one GameSession.
        let mut round = 0;
        let (request, reservation) = loop {
            round += 1;
            let last = round >= FRESH_ADMISSION_ROUNDS;
            let retry = || async {
                tokio::time::sleep(RECONCILE_BACKOFF).await;
            };
            // D4: the five-second source-age bound requires evidence fetched for
            // this attempt; S2 custody retains it and the composition decides.
            if self
                .evidence
                .refresh_fresh_admission(
                    self.root,
                    self.holder,
                    &subject.account_id,
                    &subject.signing_key_id,
                )
                .await
                .is_err()
            {
                if last {
                    return Err(Unavailable);
                }
                retry().await;
                continue;
            }
            let now = unix_seconds().ok_or(Unavailable)?;
            match self
                .root
                .publish_fresh_admission_sources(self.character, self.holder, &subject, now)
                .await
            {
                Ok(
                    GuardPublicationDisposition::Applied | GuardPublicationDisposition::Existing,
                ) => {}
                Ok(GuardPublicationDisposition::Stale | GuardPublicationDisposition::Conflict)
                | Err(_) => {
                    if last {
                        return Err(Unavailable);
                    }
                    retry().await;
                    continue;
                }
            }
            let Ok(composition) = self
                .root
                .compose_fresh_admission(self.character, self.holder, &subject)
                .await
            else {
                if last {
                    return Err(Unavailable);
                }
                retry().await;
                continue;
            };
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
            // Capacity is reserved only after every semantic/authentication check,
            // but before the durable GameSession mutation. M+1 therefore rejects
            // without creating a playable/committed session.
            let reservation = self.reserve_runtime_player(attempt.game_session_id).await?;
            match self
                .root
                .commit_composed_fresh_admission(
                    self.character,
                    self.holder,
                    &composition,
                    &request,
                )
                .await
            {
                Ok(
                    FreshAdmissionDurableOutcomeV1::Committed(_)
                    | FreshAdmissionDurableOutcomeV1::ExistingCommitted(_),
                ) => {}
                // The owner evidence moved after composition: proven noncommit.
                Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority) if !last => {
                    self.rollback_runtime_player(reservation).await?;
                    retry().await;
                    continue;
                }
                // The commit may have landed with its acknowledgement lost: keep
                // the exact operation and reconcile it until the outcome is proven.
                Ok(FreshAdmissionDurableOutcomeV1::AmbiguousOrUnavailable) | Err(_) => {
                    match self
                        .reconcile(&request, attempt.game_session_id, attempt.transport)
                        .await
                    {
                        ReconciliationDisposition::Committed => {}
                        ReconciliationDisposition::DefinitelyNotCurrent(refusal) => {
                            self.rollback_runtime_player(reservation).await?;
                            return Err(refusal);
                        }
                        ReconciliationDisposition::DurableNotOwned => {
                            // The GameSession is durable but no longer owned by
                            // this socket; that does not prove it terminal. Keep
                            // its slot reserved rather than free durable capacity.
                            return Err(Rejected);
                        }
                        ReconciliationDisposition::Unknown => {
                            // Fail closed. Do not fabricate actor authority and do not
                            // free a slot whose GameSession may already be durable.
                            return Err(Unavailable);
                        }
                    }
                }
                Ok(_) => {
                    self.rollback_runtime_player(reservation).await?;
                    return Err(Rejected);
                }
            }
            break (request, reservation);
        };
        let actor = self
            .runtime
            .lock()
            .await
            .commit_fresh_session(reservation)
            .map_err(|_| Unavailable)?;
        // #935: only after both COMMITs, revalidate current authority and let
        // the Channel owner write the first-entry position. A failure here
        // writes nothing and fabricates no rollback: the committed actor stays
        // unpositioned and is not input-eligible.
        let (first_entry, item_fence) = self
            .initialize_first_entry(&request, attempt.game_session_id, attempt.transport, actor)
            .await;
        // PREM-1b: pull Premium before any Premium read; admission does not wait on it.
        self.premium.admit(*record.account_id.as_bytes());
        let admitted = AdmittedSession {
            game_session_id: attempt.game_session_id,
            world_id: self.world_id,
            channel_id: self.channel_id,
            runtime_actor: Some(actor),
            first_entry,
            controller: Some(ControllerBinding {
                transport: attempt.transport,
                account_id: *record.account_id.as_bytes(),
            }),
            continuity: SessionContinuity::FRESH,
            item_fence,
        };
        self.admit_quest_session(&admitted).await;
        Ok(admitted)
    }
}

/// Same-session grace from the authoritative `ControlLossEpoch` boundary: registry row
/// `FND04B-SAME-SESSION-GRACE-S` (`DISCONNECT-PROTECTION-V1` §4, provisional until measured).
pub(crate) const SAME_SESSION_GRACE_SECONDS: i64 = 60;

/// The owning loss decision, resolved once from the current owners at decision time. It is
/// never built from a request or receipt; the durable adapter re-checks every durable fact.
struct ChannelOwnedLossSource(ControlLossObservationV1);

impl recovery_source_sealed::Sealed for ChannelOwnedLossSource {}

impl ControlLossSourceV1 for ChannelOwnedLossSource {
    fn resolve_loss(
        &self,
        session: GameSessionId,
        _now: i64,
    ) -> Result<ControlLossObservationV1, ReconnectDurabilityErrorV1> {
        if self.0.session.current_game_session_id() != session {
            return Err(ReconnectDurabilityErrorV1::StaleAuthority);
        }
        Ok(self.0.clone())
    }
}

/// Bounded fresh-admission rounds when concurrent owner evidence refreshes leave a
/// round stale (the first round plus two retries).
const FRESH_ADMISSION_ROUNDS: u32 = 3;
/// Bounded reconciliation of one possibly committed admission.
const RECONCILE_ATTEMPTS: u32 = 5;
const RECONCILE_BACKOFF: Duration = Duration::from_millis(200);
/// Bound on grace-expiry release attempts (waits for the deadline and store
/// retries): with the capped backoff, about two minutes of owner unavailability.
const EXPIRY_ATTEMPTS: u32 = 32;
/// Cap of the grace-expiry store retry backoff.
const EXPIRY_MAX_BACKOFF: Duration = Duration::from_secs(5);
/// Whole-second durable clock: wait just past the deadline second.
const EXPIRY_SLACK: Duration = Duration::from_millis(1100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReconciliationDisposition {
    Committed,
    /// Proven noncommit: the reservation may be rolled back.
    DefinitelyNotCurrent(AdmissionRefusal),
    /// Committed, but this socket no longer owns the session.
    DurableNotOwned,
    Unknown,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Record authoritative unexpected control loss, fresh-origin or after a resume. The owning
    /// source is composed at decision time from the Channel owner (actor present, placement,
    /// assignment revision), the current durable GameSession and the admitted account; the
    /// durable commit revalidates session, claims and runtime guard under its relation locks.
    async fn commit_control_loss(
        &self,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
        controller: ControllerBinding,
    ) -> ControlLossResult {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        // Loss is timed on the durable owner's clock, the same clock that
        // samples the final decision time.
        let Ok((session, now)) = store.current_session_at(game_session_id).await else {
            return ControlLossResult::Unknown;
        };
        // Only this controller's still-ACTIVE, never-lost session: a replaced, released or
        // already reconnectable session is not this connection's to lose.
        if session.session_state() != GameSessionState::Active
            || session.current_transport() != Some(controller.transport)
            || session.current_runtime_scope()
                != RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
        {
            return ControlLossResult::NotApplicable;
        }
        // This controller may have resumed a lost session: its loss opens the next epoch
        // and retains the resumed one's history, read from the durable receipts. A resumed
        // session whose history cannot be proven is released instead (`ResumedHistory`).
        let (history, protection) = match session.current_control_loss_epoch() {
            None => (
                ControlLossHistoryV1::FreshOrigin,
                RecoveryProtectionContinuityV1 {
                    usage: RecoveryProtectionUseV1::Unused {
                        entitlement_generation: 1,
                    },
                    rearm: RecoveryProtectionRearmV1::NotRearmed {
                        generation: 1,
                        stable_control_started_at: None,
                        accepted_deadline: None,
                    },
                },
            ),
            Some(_) => match store.resumed_history(game_session_id).await {
                Ok(Some(
                    history @ ControlLossHistoryV1::Resumed {
                        protection: retained,
                        ..
                    },
                )) => (history, retained),
                _ => return ControlLossResult::ResumedHistory,
            },
        };
        // A refused loss must not strand a resumed session ACTIVE on its dead transport.
        let refused = if matches!(history, ControlLossHistoryV1::Resumed { .. }) {
            ControlLossResult::ResumedHistory
        } else {
            ControlLossResult::Refused
        };
        let next_epoch = match &history {
            ControlLossHistoryV1::FreshOrigin => Some(1),
            ControlLossHistoryV1::Resumed { budget, .. } => budget.epoch().get().checked_add(1),
        };
        let (Some(grace_deadline), Some(next_epoch)) =
            (now.checked_add(SAME_SESSION_GRACE_SECONDS), next_epoch)
        else {
            return ControlLossResult::Unknown;
        };
        let (Ok(epoch), Ok(account_presence)) = (
            ControlLossEpochRefV1::new(next_epoch),
            AccountPresenceClaimV1::new(
                &canonical_uuid(&controller.account_id),
                session.commit().character_id(),
            ),
        ) else {
            return refused;
        };
        let observation = {
            let runtime = self.runtime.lock().await;
            let Ok(facts) = runtime.player_control_facts(actor, game_session_id) else {
                return refused;
            };
            if facts.control_loss.is_some() {
                return ControlLossResult::NotApplicable;
            }
            let source_revision = runtime.binding().source_revision();
            ControlLossObservationV1 {
                source_authority: session.current_runtime_scope(),
                source_revision,
                accepted_source_revision: source_revision,
                decision_identity: epoch,
                accepted_decision_identity: epoch,
                observed_at: now,
                session,
                account_presence,
                placement_identity: facts.placement_identity,
                placement_revision: facts.placement_revision,
                actor_present: true,
                runtime_ready: true,
                cause: ControlLossCauseV1::AuthoritativeUnexpectedLoss,
                loss_epoch: epoch,
                loss_origin: now,
                original_grace_deadline: grace_deadline,
                history,
                protection,
            }
        };
        let source = std::sync::Arc::new(ChannelOwnedLossSource(observation));
        let Ok(authorization) =
            ControlLossAuthorizationV1::authorize(source.as_ref(), game_session_id, now)
        else {
            return refused;
        };
        let mut flow = ControlLossFlowV1::begin(authorization);
        let Ok(request) = flow.take_request() else {
            return refused;
        };
        let request = std::sync::Arc::new(request);
        match store
            .commit_fresh_loss(request.clone(), source.clone())
            .await
        {
            Ok(ControlLossOutcomeV1::Committed { .. }) => {}
            Ok(ControlLossOutcomeV1::Rejected) => return refused,
            // The commit may have landed with its acknowledgement lost: reconcile the exact
            // immutable operation; never re-decide with a new observation.
            Ok(ControlLossOutcomeV1::Ambiguous) | Err(_) => {
                let mut proven = None;
                for _ in 0..RECONCILE_ATTEMPTS {
                    match store.reconcile_fresh_loss(request.operation()).await {
                        Ok(FreshLossReconciliation::Committed { .. }) => {
                            proven = Some(true);
                            break;
                        }
                        Ok(FreshLossReconciliation::Absent | FreshLossReconciliation::Conflict) => {
                            proven = Some(false);
                            break;
                        }
                        Err(_) => tokio::time::sleep(RECONCILE_BACKOFF).await,
                    }
                }
                match proven {
                    Some(true) => {}
                    Some(false) => return refused,
                    None => return ControlLossResult::Unknown,
                }
            }
        }
        let mark = ControlLossMark {
            epoch: epoch.get(),
            grace_deadline,
        };
        match self
            .runtime
            .lock()
            .await
            .record_control_loss(actor, game_session_id, mark)
        {
            Ok(()) => ControlLossResult::Recorded,
            Err(_) => ControlLossResult::Unknown,
        }
    }

    async fn reserve_runtime_player(
        &self,
        game_session_id: GameSessionId,
    ) -> Result<PlayerActorReservation, AdmissionRefusal> {
        use AdmissionRefusal::{Rejected, Unavailable};
        self.reserve_precondition().await?;
        self.runtime
            .lock()
            .await
            .reserve_fresh_session(game_session_id)
            .map_err(|error| match error {
                CarrierError::CapacityExceeded => Rejected,
                _ => Unavailable,
            })
    }

    /// The current assignment still names this node incarnation and matches
    /// the exact committed assignment this runtime was composed from.
    async fn reserve_precondition(&self) -> Result<(), AdmissionRefusal> {
        use AdmissionRefusal::{Rejected, Unavailable};
        let scope = RuntimeScopeRefV1::channel(self.world_id, self.channel_id);
        let assignment = self
            .root
            .read_runtime_scope_assignment(scope)
            .await
            .map_err(|_| Unavailable)?
            .ok_or(Rejected)?;
        let fact = self.holder.fact();
        let binding = self.runtime.lock().await.binding();
        if assignment.state != AssignmentState::Assigned
            || assignment.holder != Some(fact)
            || !binding.matches_committed_assignment(
                self.world_id,
                self.channel_id,
                fact.node_id(),
                fact.registration_revision(),
                assignment.ownership_generation,
                assignment.source_revision,
                &assignment.decision_identity,
            )
        {
            return Err(Rejected);
        }
        Ok(())
    }

    /// Immediately before the position write, resolve independently current
    /// authority (#935): the current GameSession, the current Character guard
    /// (owner, World, eligibility, lease generation and holder) and the current
    /// assignment of this runtime. The immutable request supplies only the
    /// expected values those current reads are compared against. A positioned
    /// actor also gets its session's item fence (C2), from that current read.
    async fn initialize_first_entry(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        game_session_id: GameSessionId,
        transport: AuthenticatedTransportRefV1,
        actor: ExactActorRef,
    ) -> (FirstEntryOutcome, Option<CurrentCharacterItemFence>) {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let current = match store.reconcile(request.operation()).await {
            Ok(FreshReconciliation::Committed(snapshot)) => snapshot.current_session,
            Ok(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None),
        };
        let key = AdmissionAuthorityGuardKeyV1::Character(current.commit().character_id());
        let character = match AdmissionGuardStore::from_root(self.root.clone())
            .load(&[key])
            .await
        {
            Ok(rows) => rows.into_iter().next().flatten().map(|row| row.state),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None),
        };
        match self.reserve_precondition().await {
            Ok(()) => {}
            Err(AdmissionRefusal::Unavailable) => {
                return (FirstEntryOutcome::RefusedUnavailable, None);
            }
            Err(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None),
        }
        // SPELL-D8 §8.2: a new runtime actor loads the durable Harmony and remaining forced
        // Serene time, read before the Channel-owner lock; a failed or corrupt load fails closed.
        let facts = character_cast_facts(current.commit().character_id());
        let monk = match facts {
            Some(_) => {
                let Ok(character_id) =
                    domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
                else {
                    return (FirstEntryOutcome::RefusedUnavailable, None);
                };
                match self.load_monk_state(character_id).await {
                    Some(values) => values,
                    None => return (FirstEntryOutcome::RefusedUnavailable, None),
                }
            }
            None => (0, 0),
        };
        // One Channel-owner lock covers the binding comparison and the write.
        let mut runtime = self.runtime.lock().await;
        let expected = FirstEntryExpectation {
            game_session_id,
            transport,
            world_id: self.world_id,
            channel_id: self.channel_id,
            account_id: &request.binding().account_id,
            scope_generation: runtime.binding().scope_generation(),
        };
        if !first_entry_authority_is_current(&expected, current, character.as_ref()) {
            return (FirstEntryOutcome::RefusedStaleAuthority, None);
        }
        // A2 (D295 item 4): bind the admitted session's lease to its slot in this owner step,
        // from the read that just proved it current. No damage write is admitted before.
        if runtime
            .bind_attacker_lease(actor, game_session_id, current.current_character_lease())
            .is_err()
        {
            return (FirstEntryOutcome::RefusedByChannel, None);
        }
        let outcome = match runtime.initialize_first_entry_position(actor) {
            Ok(FirstEntryPosition::Initialized(_)) => FirstEntryOutcome::Positioned,
            Ok(FirstEntryPosition::Reconciled(_)) => FirstEntryOutcome::Reconciled,
            Err(_) => return (FirstEntryOutcome::RefusedByChannel, None),
        };
        // C2: the item fence of the admitted session, from the same current session read that
        // just proved it is this admission's own.
        let item_fence = item_fence_of(current);
        // Spell cast §4: the vitals and cooldowns are created in this same owner step, before the
        // actor's first command, and only from Character-owned facts; a monk's Serene is
        // evaluated here too (§8.2).
        if let Some(facts) = facts
            && self
                .spell_states
                .lock()
                .await
                .initialize(
                    &runtime,
                    actor,
                    game_session_id,
                    facts,
                    monk,
                    self.owner_now(),
                )
                .is_none()
        {
            return (FirstEntryOutcome::RefusedByChannel, None);
        }
        (outcome, item_fence)
    }

    async fn rollback_runtime_player(
        &self,
        reservation: PlayerActorReservation,
    ) -> Result<(), AdmissionRefusal> {
        self.runtime
            .lock()
            .await
            .rollback_definitely_uncommitted(reservation)
            .map_err(|_| AdmissionRefusal::Unavailable)
    }

    /// Committed proves the exact current session belongs to this socket.
    /// Only absent/conflict prove noncommit and allow reservation rollback. A
    /// committed receipt whose session this socket no longer owns is durable,
    /// so its reservation is kept. Repeated read failures remain unknown.
    async fn reconcile(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        game_session_id: GameSessionId,
        transport: AuthenticatedTransportRefV1,
    ) -> ReconciliationDisposition {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        for attempt in 0..RECONCILE_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(RECONCILE_BACKOFF).await;
            }
            match store.reconcile(request.operation()).await {
                Ok(FreshReconciliation::Committed(snapshot)) => {
                    return if owns_fresh_session(
                        snapshot.current_session,
                        game_session_id,
                        transport,
                    ) {
                        ReconciliationDisposition::Committed
                    } else {
                        ReconciliationDisposition::DurableNotOwned
                    };
                }
                Ok(FreshReconciliation::Absent) => {
                    return ReconciliationDisposition::DefinitelyNotCurrent(
                        AdmissionRefusal::Unavailable,
                    );
                }
                Ok(FreshReconciliation::Conflict) => {
                    return ReconciliationDisposition::DefinitelyNotCurrent(
                        AdmissionRefusal::Rejected,
                    );
                }
                Err(_) => {}
            }
        }
        ReconciliationDisposition::Unknown
    }
}

/// The values a first-entry write expects; each is compared with a current read.
#[derive(Clone, Copy)]
struct FirstEntryExpectation<'a> {
    game_session_id: GameSessionId,
    transport: AuthenticatedTransportRefV1,
    world_id: WorldId,
    channel_id: ChannelId,
    account_id: &'a str,
    scope_generation: ScopeOwnershipGeneration,
}

/// #935 current-authority test for the first-entry position write. Every
/// changed, stale or missing binding refuses: session ownership, the
/// committed World/Channel, current Character/World eligibility (absent means
/// no longer eligible), the session's Character lease, the runtime scope and
/// ownership generation, and the current Character guard's owner account,
/// World, eligibility, lease generation and holder.
fn first_entry_authority_is_current(
    expected: &FirstEntryExpectation<'_>,
    current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    character: Option<&AdmissionAuthorityGuardStateV1>,
) -> bool {
    let commit = current.commit();
    let lease = current.current_character_lease();
    let session = owns_fresh_session(current, expected.game_session_id, expected.transport)
        && commit.world_id() == expected.world_id
        && commit.channel_id() == expected.channel_id
        && current.current_character_world_eligibility()
            == Some(CharacterWorldEligibilityClaimV1::new(
                commit.character_id(),
                expected.world_id,
            ))
        && lease.character_id() == commit.character_id()
        && lease.generation() == commit.character_lease_generation()
        && current.current_runtime_scope()
            == RuntimeScopeRefV1::channel(expected.world_id, expected.channel_id)
        && current.current_scope_generation() == expected.scope_generation;
    let guard = matches!(
        character,
        Some(AdmissionAuthorityGuardStateV1::Character {
            account_id,
            world_id,
            eligible: true,
            lease_generation,
            holder,
        }) if account_id == expected.account_id
            && *world_id == expected.world_id
            && *lease_generation == commit.character_lease_generation()
            && *holder == Some(expected.game_session_id)
    );
    session && guard
}

/// C2: the Character item fence of `current`, the durable GameSession a connection serves: its
/// Character, current GameSession and connection generation, current Character lease
/// generation, runtime scope and scope ownership generation. Only ever built from a current
/// durable session read; each fenced write rechecks every field against current authority.
pub(super) fn item_fence_of<T: Copy + Eq>(
    current: GameSessionAuthoritySnapshot<T>,
) -> Option<CurrentCharacterItemFence> {
    Some(CurrentCharacterItemFence {
        character_id: domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
            .ok()?,
        game_session_id: current.current_game_session_id(),
        connection_generation: current.current_connection_generation(),
        character_lease_generation: current.current_character_lease().generation(),
        runtime_scope: current.current_runtime_scope(),
        scope_ownership_generation: current.current_scope_generation(),
    })
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

/// Spell cast §4 and SPELL-D4: the Character-owned cast facts of an admitted Character. Level is
/// Character progression (R7 P03, D88), but vocation and magic level have no GAME-CHAR owner yet,
/// so no Character has cast facts: casting stays gated (`REJECTED`, no `ACTOR_VITALS`) until that
/// owner supplies them here. The client is never a source.
const fn character_cast_facts(
    _character: crate::foundation::CharacterId,
) -> Option<crate::spell::cast::CharacterCastFacts> {
    None
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

    /// ACH-NOTIFY-1: the earned delta takes its name from the catalogue and its watermark from
    /// the fact keys read in the granting transaction; a key the catalogue lacks sends none.
    #[test]
    fn achievement_earned_names_the_grant_and_sums_the_account_facts() -> Result<(), Box<dyn Error>>
    {
        use crate::achievement_catalogue::AchievementCatalogue;
        use crate::durability::reward_claim_mint::GrantedAchievementNotice;
        use oteryn_protocol_oteryn::achievement_notices::AchievementWatermark;
        let record = |slug: &str, points: u32| {
            format!(
                r#"{{"identity":{{"family":"Achievement","key":"oteryn:achievement/{slug}","revision":"1"}},"name":"N {slug}","description":"D","grade":1,"points":{points},"secret":false}}"#
            )
        };
        let shard = format!(
            r#"{{"family":"Achievement","records":[{},{}]}}"#,
            record("first", 3),
            record("second", 5),
        );
        let catalogue =
            AchievementCatalogue::from_shards(&[&shard]).map_err(|error| format!("{error:?}"))?;
        let notice = |key: &str, keys: &[&str]| GrantedAchievementNotice {
            achievement_key: format!("oteryn:achievement/{key}"),
            account_fact_keys: keys
                .iter()
                .map(|key| format!("oteryn:achievement/{key}"))
                .collect(),
        };
        let earned = achievement_earned(&catalogue, &notice("second", &["first", "second"]))
            .ok_or("earned")?;
        assert_eq!(earned.key, "oteryn:achievement/second");
        assert_eq!(earned.name, "N second");
        assert_eq!(
            earned.watermark,
            AchievementWatermark {
                fact_count: 2,
                total_points: 8,
            }
        );
        assert!(achievement_earned(&catalogue, &notice("absent", &["absent"])).is_none());
        assert!(achievement_earned(&catalogue, &notice("first", &["first", "absent"])).is_none());

        // ACH-NOTIFY-2: the three states. An underivable `Granted` and an `Unknown` read are both
        // `Unknown` with one operator event each; nothing granted is `NoneEarned` and logs nothing.
        use crate::durability::reward_claim_mint::GrantNotice;
        let mut events = Vec::new();
        let mut log = |line: &str| events.push(line.to_owned());
        assert_eq!(
            earned_notice(&catalogue, GrantNotice::None, &mut log),
            connection::EarnedNotice::NoneEarned
        );
        assert_eq!(
            earned_notice(
                &catalogue,
                GrantNotice::Granted(notice("second", &["first", "second"])),
                &mut log
            ),
            connection::EarnedNotice::Earned(earned)
        );
        for unknown in [
            GrantNotice::Granted(notice("first", &["first", "absent"])),
            GrantNotice::Unknown,
        ] {
            assert_eq!(
                earned_notice(&catalogue, unknown, &mut log),
                connection::EarnedNotice::Unknown
            );
        }
        assert_eq!(events.len(), 2);
        assert!(
            events
                .iter()
                .all(|line| line.starts_with("event=achievement_notice_unknown level=error"))
        );
        Ok(())
    }

    /// Display contract §3.3, §4.3: a valid page encodes; a row over its byte bounds fails closed
    /// with `PAYLOAD_LIMIT_EXCEEDED`; an unknown key with `ACCOUNT_DATA_INTEGRITY` and one operator
    /// event (owner decision 2026-09-30); a malformed row is a plain `REJECTED`. Nothing is
    /// truncated or skipped, and only the integrity fault logs.
    #[test]
    fn account_achievements_reply_encodes_or_fails_closed() -> Result<(), Box<dyn Error>> {
        use crate::achievement_catalogue::AchievementCatalogue;
        use crate::durability::account_achievement::EarnedAchievement;
        use oteryn_protocol_oteryn::account_achievements::decode_account_achievements_result;
        let record = |slug: &str, name: &str, grade: u32| {
            format!(
                r#"{{"identity":{{"family":"Achievement","key":"oteryn:achievement/{slug}","revision":"1"}},"name":"{name}","description":"D","grade":{grade},"points":2,"secret":false}}"#
            )
        };
        let shard = format!(
            r#"{{"family":"Achievement","records":[{},{},{}]}}"#,
            record("fits", "Fits", 1),
            record("long", &"n".repeat(65), 1),
            record("gradeless", "Gradeless", 0),
        );
        let catalogue =
            AchievementCatalogue::from_shards(&[&shard]).map_err(|error| format!("{error:?}"))?;
        let facts = |slugs: &[&str]| -> Vec<EarnedAchievement> {
            slugs
                .iter()
                .map(|slug| EarnedAchievement {
                    achievement_key: format!("oteryn:achievement/{slug}"),
                    earned_at_unix_ms: 7,
                })
                .collect()
        };
        let request = |page| AccountAchievementsRequest {
            account_id: [0x11; 16],
            page,
            game_session_id: [0x22; 16],
            command_id: 9,
        };
        let reply = |catalogue: &AchievementCatalogue, slugs: &[&str], page| {
            let mut events = Vec::new();
            let reply =
                account_achievements_reply(catalogue, &facts(slugs), &request(page), &mut |line| {
                    events.push(line.to_owned())
                });
            (reply, events)
        };
        let (AccountAchievementsReply::Page(payload), events) = reply(&catalogue, &["fits"], 0)
        else {
            return Err("a valid page encodes".into());
        };
        let page =
            decode_account_achievements_result(&payload).map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            (page.total_points, page.fact_count, page.rows.len()),
            (2, 1, 1)
        );
        assert!(events.is_empty());
        let integrity_event = crate::achievement_catalogue::integrity_fault_event(
            &request(0),
            "oteryn:achievement/absent",
        );
        for (slugs, expected, logged) in [
            (
                &["fits", "long"][..],
                AccountAchievementsReply::Terminal(FoundationProtocolError::PayloadLimitExceeded),
                &[][..],
            ),
            (
                &["fits", "gradeless"][..],
                AccountAchievementsReply::Rejected,
                &[][..],
            ),
            (
                &["fits", "absent"][..],
                AccountAchievementsReply::Terminal(FoundationProtocolError::AccountDataIntegrity),
                &[integrity_event.as_str()][..],
            ),
        ] {
            assert_eq!(
                reply(&catalogue, slugs, 0),
                (
                    expected,
                    logged.iter().map(|line| (*line).to_owned()).collect()
                ),
                "{slugs:?}"
            );
        }
        // Only the reply that would carry the over-bound row fails; its grade puts it on page 1.
        let many: Vec<String> = (0..70).map(|index| format!("k{index:02}")).collect();
        let records: Vec<String> = many.iter().map(|slug| record(slug, "A", 1)).collect();
        let shard = format!(
            r#"{{"family":"Achievement","records":[{},{}]}}"#,
            records.join(","),
            record("long", &"n".repeat(65), 4),
        );
        let catalogue =
            AchievementCatalogue::from_shards(&[&shard]).map_err(|error| format!("{error:?}"))?;
        let mut slugs: Vec<&str> = many.iter().map(String::as_str).collect();
        slugs.push("long");
        assert!(matches!(
            reply(&catalogue, &slugs, 0).0,
            AccountAchievementsReply::Page(_)
        ));
        assert_eq!(
            reply(&catalogue, &slugs, 1).0,
            AccountAchievementsReply::Terminal(FoundationProtocolError::PayloadLimitExceeded)
        );
        Ok(())
    }

    // USE-WIRE-V1 reach (#162 5868482467). SEAM_EVIDENCE: gameplay_transport/mod.rs
    // use_object_reachable unit coverage (a genuine TOO_FAR case cannot be reached through real
    // movement in the qualified native entry room; see `use_object_reachable`'s own doc comment).
    #[test]
    fn use_object_reachable_is_chebyshev_one_same_floor_only() {
        let door =
            std::collections::BTreeSet::from([crate::content::LogicalCell { x: 1, y: -1, z: 0 }]);
        // Same cell, every adjacent (including diagonal) cell, and up to distance 1 inclusive.
        for (x, y) in [
            (1, -1),
            (0, -1),
            (2, -1),
            (1, 0),
            (1, -2),
            (0, 0),
            (2, 0),
            (0, -2),
            (2, -2),
        ] {
            assert!(
                ComposedFreshAdmission::use_object_reachable(x, y, 0, &door),
                "({x},{y}) expected reachable"
            );
        }
        // Distance 2 on either axis: TOO_FAR.
        for (x, y) in [(3, -1), (1, 1), (-1, -1), (1, -3)] {
            assert!(
                !ComposedFreshAdmission::use_object_reachable(x, y, 0, &door),
                "({x},{y}) expected too far"
            );
        }
        // The same (x, y) on a different floor is never reachable.
        assert!(!ComposedFreshAdmission::use_object_reachable(
            1, -1, 1, &door
        ));
        // No collision cell at all (an unbound/empty door) is never reachable.
        assert!(!ComposedFreshAdmission::use_object_reachable(
            1,
            -1,
            0,
            &std::collections::BTreeSet::new()
        ));
    }

    /// #162 5868482467 shared-lease P1 repair (r4121956127, Codex): the occupancy set
    /// `use_object` builds must contain every committed actor's cell, not only the issuing
    /// actor's own. Two real actors in one real `ChannelRuntimeV1`: actor A stands in the door
    /// cell, actor B stands adjacent (the accepted `east` cell) and is the one attempting to
    /// close the door. `ChannelRuntimeV1::committed_player_positions` (the new read this repair
    /// adds, reusing the existing per-slot position store) must report both, and the real door
    /// `LocalObjectRuntime` (bound exactly as `node/serve.rs` binds it) must then report
    /// OCCUPIED for the close attempt, with no transition and no revision change.
    #[test]
    fn use_object_occupancy_includes_every_committed_actor_not_only_the_issuer()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::content::LogicalCell;
        use crate::content::accepted;
        use crate::foundation::{ChannelContentPin, MovementLocalPosition, NodeId};
        use crate::world_runtime::{
            LocalObjectUseOutcome, ReferenceContentGeneration, ScopeContentGenerationFence,
            bind_native_entry_door,
        };

        let world_id = WorldId::decode(&uuid_v7(0x50)).expect("world");
        let channel_id = ChannelId::decode(&uuid_v7(0x51)).expect("channel");
        let node_id = NodeId::decode(&uuid_v7(0x52)).expect("node");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world_id,
            channel_id,
            node_id,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world_id),
        )
        .expect("channel runtime");

        let session_a = GameSessionId::decode(&uuid_v7(0x53)).expect("session a");
        let session_b = GameSessionId::decode(&uuid_v7(0x54)).expect("session b");
        let reservation_a = runtime.reserve_fresh_session(session_a).expect("reserve a");
        let actor_a = runtime
            .commit_fresh_session(reservation_a)
            .expect("commit a");
        let reservation_b = runtime.reserve_fresh_session(session_b).expect("reserve b");
        let actor_b = runtime
            .commit_fresh_session(reservation_b)
            .expect("commit b");

        // Actor A stands in the door cell; actor B stands adjacent (accepted `east`). Both
        // start at the pinned first-entry cell (0,0,0) under the runtime's real pinned
        // context, then step by real cardinal commits — `initialize_movement_test_position`
        // deliberately uses a synthetic context that would not match the pinned one
        // `committed_player_positions` filters by.
        runtime
            .initialize_first_entry_position(actor_a)
            .expect("first entry a");
        runtime
            .initialize_first_entry_position(actor_b)
            .expect("first entry b");
        let door_cell = accepted::DOOR_CELL;
        {
            let mut position = runtime.borrow_movement_position();
            let snapshot = position.read(actor_a).expect("read a at start");
            let snapshot = position
                .commit_cardinal(
                    snapshot,
                    MovementLocalPosition {
                        x: 1,
                        y: 0,
                        floor: 0,
                    },
                )
                .expect("step a east");
            position
                .commit_cardinal(
                    snapshot,
                    MovementLocalPosition {
                        x: door_cell.1,
                        y: door_cell.2,
                        floor: door_cell.3,
                    },
                )
                .expect("step a north into the door cell");
        }
        let east = accepted::CELLS[1];
        {
            let mut position = runtime.borrow_movement_position();
            let snapshot = position.read(actor_b).expect("read b at start");
            position
                .commit_cardinal(
                    snapshot,
                    MovementLocalPosition {
                        x: east.1,
                        y: east.2,
                        floor: east.3,
                    },
                )
                .expect("step b east");
        }

        // The exact read `use_object` performs already includes both actors, not only
        // whichever one happens to be the issuer.
        let occupied: std::collections::BTreeSet<LogicalCell> = runtime
            .committed_player_positions()
            .into_iter()
            .map(|position| LogicalCell {
                x: position.x,
                y: position.y,
                z: i32::from(position.floor),
            })
            .collect();
        assert_eq!(occupied.len(), 2);
        let door_logical_cell = LogicalCell {
            x: door_cell.1,
            y: door_cell.2,
            z: i32::from(door_cell.3),
        };
        assert!(occupied.contains(&door_logical_cell));
        assert!(occupied.contains(&LogicalCell {
            x: east.1,
            y: east.2,
            z: i32::from(east.3),
        }));

        // Bind the real door runtime from the real qualified native entry room content, exactly
        // as `node/serve.rs` does at activation, then open it (opening's target collision is
        // Absent, so it is never occupancy-checked) so a subsequent close can be.
        let room = crate::content::qualify_native_entry_room(world_id).expect("qualified room");
        let door_content = room.door().clone();
        let scope = RuntimeScopeRefV1::channel(world_id, channel_id);
        let scope_generation = ScopeOwnershipGeneration::new(1).expect("scope generation");
        let content_generation = ReferenceContentGeneration::from_content(&door_content)
            .map_err(|error| format!("{error:?}"))?;
        let fence = ScopeContentGenerationFence::for_activation(
            scope,
            scope_generation,
            content_generation,
        );
        let mut door = bind_native_entry_door(&door_content, &fence, scope, scope_generation)
            .map_err(|error| format!("{error:?}"))?;
        let empty = std::collections::BTreeSet::new();
        let opened = door
            .attempt_use(0, &empty)
            .map_err(|error| format!("{error:?}"))?;
        assert!(matches!(
            opened,
            LocalObjectUseOutcome::Committed { revision: 1, .. }
        ));

        // Actor B (adjacent, the issuer) attempts to close the door while actor A is standing
        // in the doorway: OCCUPIED, no transition, no revision change.
        assert_eq!(
            door.attempt_use(1, &occupied)
                .map_err(|error| format!("{error:?}"))?,
            LocalObjectUseOutcome::Occupied
        );
        assert_eq!(door.revision(), 1);
        assert_eq!(door.state_key().as_str(), accepted::DOOR_OPEN_STATE);
        Ok(())
    }

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
                runtime_actor: None,
                first_entry: FirstEntryOutcome::NotApplicable,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
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

    /// Admits at once; every ended connection's loss is recorded and its
    /// grace expiry then waits until shutdown.
    struct LifecycleAuthority {
        inner: GatedAuthority,
        losses: AtomicUsize,
        expiring: AtomicUsize,
    }

    impl FreshAdmissionAuthority for LifecycleAuthority {
        async fn admit(
            &self,
            attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            self.inner.admit(attempt).await
        }
        async fn lose_control(&self, _: AdmittedSession, _: Duration) -> ControlLossResult {
            self.losses.fetch_add(1, Ordering::SeqCst);
            ControlLossResult::Recorded
        }
        async fn expire_control_loss(&self, _: AdmittedSession) -> GraceExpiryResult {
            self.expiring.fetch_add(1, Ordering::SeqCst);
            std::future::pending().await
        }
    }

    #[test]
    fn resource_loss_lifecycle_holds_no_connection_slot() -> Result<(), Box<dyn Error>> {
        runtime()?.block_on(async {
            let material = material()?;
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let limits = ListenerLimits::new(1, 1, Duration::from_millis(600)).ok_or("limits")?;
            let authority = LifecycleAuthority {
                inner: GatedAuthority::new(true),
                losses: AtomicUsize::new(0),
                expiring: AtomicUsize::new(0),
            };
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
                // The first admitted client leaves; its grace lifecycle stays pending.
                let first = admit_client(material.client.clone(), address).await;
                settle(200).await;
                let pending = authority.expiring.load(Ordering::SeqCst);
                // With a budget of one connection, a second admission still fits.
                let second = tokio::time::timeout(
                    Duration::from_secs(2),
                    admit_client(material.client.clone(), address),
                )
                .await;
                // The second client has left too; wait (bounded) until its end is served.
                for _ in 0..40 {
                    if authority.losses.load(Ordering::SeqCst) == 2 {
                        break;
                    }
                    settle(50).await;
                }
                shutdown.cancel();
                (first.is_ok(), pending, matches!(second, Ok(Ok(_))))
            };
            let ((), (first, pending, second)) = join(serve, check).await;
            assert!(first);
            assert_eq!(pending, 1, "first loss must be in its grace lifecycle");
            assert!(
                second,
                "a pending grace lifecycle must not hold the only slot"
            );
            assert_eq!(authority.losses.load(Ordering::SeqCst), 2);
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

    /// C2: the item fence is exactly the current session's Character, GameSession, connection
    /// generation, current Character lease generation, runtime scope and scope generation, so
    /// a resume (a new connection generation) or a new lease yields a different fence.
    #[test]
    fn item_fence_is_the_current_session_binding() {
        use crate::foundation::{
            CharacterLease, ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts,
        };
        let session = GameSessionId::decode(&uuid_v7(0x41)).expect("session");
        let character = CharacterId::decode(&CHARACTER).expect("character");
        let world = WorldId::decode(&uuid_v7(0x43)).expect("world");
        let channel = ChannelId::decode(&uuid_v7(0x44)).expect("channel");
        let transport = AuthenticatedTransportRefV1::decode(&[9; 16]).expect("transport");
        let facts =
            FreshAdmissionFacts::new([7; 32], character, world, channel, 3, 5).expect("facts");
        let commit = FreshAdmissionCommit::from_facts(session, facts, transport).expect("commit");
        let scope = RuntimeScopeRefV1::channel(world, channel);
        let snapshot = |connection, lease| {
            GameSessionAuthoritySnapshot::from_current_facts(
                commit,
                GameSessionState::Active,
                ConnectionGeneration::new(connection).expect("generation"),
                Some(transport),
                CharacterLease::new(character, lease).expect("lease"),
                Some(CharacterWorldEligibilityClaimV1::new(character, world)),
                scope,
                ScopeOwnershipGeneration::new(5).expect("scope"),
            )
            .expect("snapshot")
        };
        let fence = item_fence_of(snapshot(1, 3)).expect("fence");
        assert_eq!(
            fence,
            CurrentCharacterItemFence {
                character_id: domain::CharacterId::from_bytes(CHARACTER).expect("character"),
                game_session_id: session,
                connection_generation: ConnectionGeneration::new(1).expect("generation"),
                character_lease_generation: 3,
                runtime_scope: scope,
                scope_ownership_generation: ScopeOwnershipGeneration::new(5).expect("scope"),
            }
        );
        let resumed = item_fence_of(snapshot(2, 3)).expect("fence");
        assert_eq!(resumed.connection_generation.get(), 2);
        assert_ne!(resumed, fence);
        assert_eq!(
            item_fence_of(snapshot(1, 4))
                .expect("fence")
                .character_lease_generation,
            4
        );
    }

    /// C2 routing over the real activated entry room: only the injected chest placement is
    /// routed to the chest `USE`; the door and any other placement keep the door path. The
    /// injection leaves the activated Content (and so its digests) unchanged, the chest stands
    /// on the non-walkable `entry-north` cell in reach of the start cell, its reward and backpack
    /// resolve with known stack classes (D82) and its claim names no achievement.
    #[test]
    fn use_routes_only_the_injected_entry_chest_away_from_the_door_path()
    -> Result<(), Box<dyn Error>> {
        use crate::combat_pickup::resolve_item_definition_facts;
        use crate::content::{LogicalCell, accepted};
        use crate::durability::item_transfer::ItemStackClass;
        use crate::interaction_chest_use::{entry_chest, resolve_chest, with_entry_chest};
        use crate::world_runtime::ReferenceContentGeneration;

        let world = WorldId::decode(&uuid_v7(0x45))?;
        let room = crate::content::qualify_native_entry_room(world)
            .map_err(|error| format!("{error:?}"))?;
        let activated = room.door().clone();
        let chest = with_entry_chest(&activated).map_err(|error| format!("{error:?}"))?;
        assert_eq!(&activated, room.door());
        assert_eq!(
            ReferenceContentGeneration::from_content(&activated)?,
            ReferenceContentGeneration::from_content(room.door())?
        );

        let placement =
            ComposedFreshAdmission::chest_target(&chest, entry_chest::PLACEMENT.as_bytes())
                .ok_or("chest not routed")?;
        for other in [accepted::DOOR_CELL.0, "oteryn:cell/unknown", ""] {
            assert!(
                ComposedFreshAdmission::chest_target(&chest, other.as_bytes()).is_none(),
                "{other}"
            );
            assert!(ComposedFreshAdmission::chest_target(&activated, other.as_bytes()).is_none());
        }

        let north = accepted::CELLS[2];
        assert!(!north.4, "the chest cell must not be walkable");
        assert_eq!(
            placement.address.cell,
            LogicalCell {
                x: north.1,
                y: north.2,
                z: i32::from(north.3)
            }
        );
        let cells = std::collections::BTreeSet::from([placement.address.cell]);
        let start = accepted::CELLS[0];
        assert!(ComposedFreshAdmission::use_object_reachable(
            start.1,
            start.2,
            i32::from(start.3),
            &cells
        ));
        assert_eq!(entry_chest::CONTENT_REVISION, accepted::REVISIONS[0]);
        assert_eq!(entry_chest::MAP_REVISION, accepted::REVISIONS[1]);
        assert_eq!(entry_chest::RULESET_REVISION, accepted::REVISIONS[2]);
        assert_eq!(entry_chest::SIM_REVISION, accepted::REVISIONS[6]);

        let resolved = resolve_chest(&chest, &placement.key).map_err(|error| format!("{error}"))?;
        assert_eq!(resolved.claim.production_key, entry_chest::CLAIM);
        assert_eq!(resolved.quantity, entry_chest::REWARD_COUNT);
        assert_eq!(resolved.achievement, None);
        let reward = resolve_item_definition_facts(&chest, &resolved.reward_item)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            reward.stack,
            ItemStackClass::Stackable {
                proven_maximum: Some(100)
            }
        );
        let backpack = resolve_item_definition_facts(
            &chest,
            &crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: entry_chest::BACKPACK_ITEM.into(),
                revision_ref: entry_chest::DEFINITION_REVISION.into(),
            },
        )
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(backpack.stack, ItemStackClass::NonStackable);
        assert_eq!(
            backpack.container_capacity,
            Some(u32::from(entry_chest::BACKPACK_CAPACITY))
        );
        assert!(backpack.container_slot_equip_pattern);
        let achievements = crate::achievement_catalogue::AchievementCatalogue::embedded()
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            achievements
                .unbound_reward_claim_achievements(&chest)
                .is_empty()
        );
        Ok(())
    }

    /// #935 no-write matrix: each independently changed binding alone refuses
    /// the first-entry write while every other binding stays valid.
    #[test]
    fn first_entry_refuses_every_single_changed_current_binding() {
        use crate::foundation::{
            CharacterLease, ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts,
        };
        let session = GameSessionId::decode(&uuid_v7(0x31)).expect("session");
        let other_session = GameSessionId::decode(&uuid_v7(0x32)).expect("session");
        let character = CharacterId::decode(&CHARACTER).expect("character");
        let world = WorldId::decode(&uuid_v7(0x33)).expect("world");
        let other_world = WorldId::decode(&uuid_v7(0x35)).expect("world");
        let channel = ChannelId::decode(&uuid_v7(0x34)).expect("channel");
        let transport = AuthenticatedTransportRefV1::decode(&[9; 16]).expect("transport");
        let facts =
            FreshAdmissionFacts::new([7; 32], character, world, channel, 3, 5).expect("facts");
        let commit = FreshAdmissionCommit::from_facts(session, facts, transport).expect("commit");
        let scope = RuntimeScopeRefV1::channel(world, channel);
        let generation = |value| ScopeOwnershipGeneration::new(value).expect("scope");
        let snapshot = |state, lease, eligibility, scope_generation| {
            GameSessionAuthoritySnapshot::from_current_facts(
                commit,
                state,
                ConnectionGeneration::new(1).expect("generation"),
                Some(transport),
                CharacterLease::new(character, lease).expect("lease"),
                eligibility,
                scope,
                generation(scope_generation),
            )
            .expect("snapshot")
        };
        let eligible = Some(CharacterWorldEligibilityClaimV1::new(character, world));
        let valid = snapshot(GameSessionState::Active, 3, eligible, 5);
        let guard = |account: &str, guard_world, eligible, lease, holder| {
            AdmissionAuthorityGuardStateV1::Character {
                account_id: account.to_owned(),
                world_id: guard_world,
                eligible,
                lease_generation: lease,
                holder,
            }
        };
        let valid_guard = guard("account-1", world, true, 3, Some(session));
        let expected = FirstEntryExpectation {
            game_session_id: session,
            transport,
            world_id: world,
            channel_id: channel,
            account_id: "account-1",
            scope_generation: generation(5),
        };
        assert!(first_entry_authority_is_current(
            &expected,
            valid,
            Some(&valid_guard)
        ));

        // Session-side changes.
        for (label, current) in [
            (
                "not active",
                snapshot(GameSessionState::Reconnectable, 3, eligible, 5),
            ),
            ("ineligible", snapshot(GameSessionState::Active, 3, None, 5)),
            ("lease", snapshot(GameSessionState::Active, 4, eligible, 5)),
            (
                "scope generation",
                snapshot(GameSessionState::Active, 3, eligible, 6),
            ),
        ] {
            assert!(
                !first_entry_authority_is_current(&expected, current, Some(&valid_guard)),
                "{label}"
            );
        }
        // Current Character guard changes.
        for (label, changed) in [
            ("owner", guard("account-2", world, true, 3, Some(session))),
            (
                "guard world",
                guard("account-1", other_world, true, 3, Some(session)),
            ),
            (
                "guard ineligible",
                guard("account-1", world, false, 3, Some(session)),
            ),
            (
                "guard lease",
                guard("account-1", world, true, 4, Some(session)),
            ),
            (
                "holder",
                guard("account-1", world, true, 3, Some(other_session)),
            ),
            ("no holder", guard("account-1", world, true, 3, None)),
        ] {
            assert!(
                !first_entry_authority_is_current(&expected, valid, Some(&changed)),
                "{label}"
            );
        }
        assert!(!first_entry_authority_is_current(&expected, valid, None));
        // Expectation-side changes (another socket, World, Channel or runtime).
        let other_channel = ChannelId::decode(&uuid_v7(0x36)).expect("channel");
        let other_transport = AuthenticatedTransportRefV1::decode(&[8; 16]).expect("transport");
        for (label, changed) in [
            (
                "session",
                FirstEntryExpectation {
                    game_session_id: other_session,
                    ..expected
                },
            ),
            (
                "transport",
                FirstEntryExpectation {
                    transport: other_transport,
                    ..expected
                },
            ),
            (
                "world",
                FirstEntryExpectation {
                    world_id: other_world,
                    ..expected
                },
            ),
            (
                "channel",
                FirstEntryExpectation {
                    channel_id: other_channel,
                    ..expected
                },
            ),
            (
                "runtime generation",
                FirstEntryExpectation {
                    scope_generation: generation(4),
                    ..expected
                },
            ),
        ] {
            assert!(
                !first_entry_authority_is_current(&changed, valid, Some(&valid_guard)),
                "{label}"
            );
        }
    }

    /// D449 (#1708 Codex P1 4175882774): the capability-mismatch release fences with the lost
    /// epoch. Its retry and grace expiry join that fence, a compatible resume that wins the race
    /// lifts it when it restores control, and no mismatch attempt can fence the slot again
    /// afterwards, so an unproven mismatch release never leaves a fence behind a resumed session.
    /// A slot whose bound session lost control at epoch 1.
    fn lost_slot() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        use crate::foundation::{ChannelContentPin, CharacterLease, NodeId};
        let world_id = WorldId::decode(&uuid_v7(0x60)).expect("world");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world_id,
            ChannelId::decode(&uuid_v7(0x61)).expect("channel"),
            NodeId::decode(&uuid_v7(0x62)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            ChannelContentPin::test(world_id),
        )
        .expect("channel runtime");
        let session = GameSessionId::decode(&uuid_v7(0x63)).expect("session");
        let reservation = runtime.reserve_fresh_session(session).expect("reserve");
        let actor = runtime.commit_fresh_session(reservation).expect("commit");
        let character = CharacterId::decode(&CHARACTER).expect("character");
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(character, 1).expect("lease"),
            )
            .expect("bind lease");
        runtime
            .record_control_loss(
                actor,
                session,
                ControlLossMark {
                    epoch: 1,
                    grace_deadline: 160,
                },
            )
            .expect("loss");
        (runtime, actor, session)
    }

    #[test]
    fn capability_mismatch_fence_is_lifted_by_a_winning_resume() {
        let (mut runtime, actor, session) = lost_slot();
        let epoch = ControlLossEpochRefV1::new(1).expect("epoch");
        let mismatch = TerminalRelease::CapabilityMismatch(epoch)
            .fence(&mut runtime)
            .expect("mismatch token");
        mismatch
            .fence(&mut runtime, actor, session)
            .expect("mismatch fences");
        // A retry of the mismatch release and grace expiry of the same epoch join the fence.
        let retry = TerminalRelease::CapabilityMismatch(epoch)
            .fence(&mut runtime)
            .expect("retry token");
        assert_eq!(retry.fence(&mut runtime, actor, session), Ok(()));
        assert_eq!(
            TransitionFence::GraceExpiry(1).fence(&mut runtime, actor, session),
            Ok(())
        );
        // Any other transition waits on it.
        let other = TerminalRelease::Abandoned(
            AuthenticatedTransportRefV1::decode(&[7; 16]).expect("transport"),
        )
        .fence(&mut runtime)
        .expect("other token");
        assert_eq!(
            other.fence(&mut runtime, actor, session),
            Err(CarrierError::WriteFenceBusy)
        );
        // The compatible resume wins: restoring control of the epoch lifts the mismatch fence,
        // with no durable read and no lost entry needed.
        assert_eq!(runtime.restore_control(actor, session, 1), Ok(()));
        assert_eq!(mismatch.lift(&mut runtime, actor, session), Ok(false));
        assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
        assert_eq!(other.lift(&mut runtime, actor, session), Ok(true));
        // A late mismatch attempt can no longer fence the resumed slot.
        assert_eq!(
            retry.fence(&mut runtime, actor, session),
            Err(CarrierError::ControlLossConflict)
        );
        assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
    }

    /// D449 (#1708 Codex P1 4175947583): grace expiry and a mismatch release of one epoch join
    /// its `ControlLoss` fence. A release that settles first gives back only its own hold, so the
    /// fence stays while the other is still saving, and the last holder lifts it. A retry is
    /// counted once, a refused release leaves the fence to the others, and a release holding
    /// nothing lifts nothing.
    #[test]
    fn a_joined_epoch_fence_is_lifted_only_by_its_last_holder() {
        let (mut runtime, actor, session) = lost_slot();
        let holders = FenceHolders::default();
        let token = TransitionFence::GraceExpiry(1);
        let other = TransitionFence::Transition(9);
        let mut grace = FenceHold::new(session, token);
        let mut mismatch = FenceHold::new(session, token);
        assert_eq!(holders.fence(&mut runtime, actor, &mut grace), Ok(()));
        assert_eq!(holders.fence(&mut runtime, actor, &mut mismatch), Ok(()));
        // The mismatch release retries its fence: still one hold.
        assert_eq!(holders.fence(&mut runtime, actor, &mut mismatch), Ok(()));
        // The mismatch release settles NotApplicable while grace expiry is still saving.
        assert_eq!(holders.lift(&mut runtime, actor, &mut mismatch), Ok(()));
        assert_eq!(
            other.fence(&mut runtime, actor, session),
            Err(CarrierError::WriteFenceBusy)
        );
        // A second release holding nothing lifts nothing.
        assert_eq!(holders.lift(&mut runtime, actor, &mut mismatch), Ok(()));
        assert_eq!(
            other.fence(&mut runtime, actor, session),
            Err(CarrierError::WriteFenceBusy)
        );
        // A mismatch retry joins and is refused later: the fence stays with grace expiry.
        let mut retry = FenceHold::new(session, token);
        assert_eq!(holders.fence(&mut runtime, actor, &mut retry), Ok(()));
        holders.release(&mut retry);
        assert_eq!(
            other.fence(&mut runtime, actor, session),
            Err(CarrierError::WriteFenceBusy)
        );
        // Grace expiry, the last holder, settles: the fence is lifted and no count is left.
        assert_eq!(holders.lift(&mut runtime, actor, &mut grace), Ok(()));
        assert!(holders.counts().is_empty());
        assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
        assert_eq!(other.lift(&mut runtime, actor, session), Ok(true));
        // A retired session leaves no hold behind.
        assert_eq!(holders.fence(&mut runtime, actor, &mut grace), Ok(()));
        holders.forget(session);
        assert!(holders.counts().is_empty());
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
