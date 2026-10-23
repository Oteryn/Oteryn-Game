//! Production gameplay TCP/TLS seam. Transport-only: Foundation owns protocol
//! semantics and the composed owners decide admission.

pub mod actor_spell;
pub mod appearance_change;
mod attack;
mod bank_npc;
mod bed_regen;
mod bed_sleep;
mod bundle_map_position;
mod depot;
use bundle_map_position::bundle_map_position;
#[cfg(test)]
use bundle_map_position::bundle_world_map;
#[cfg(test)]
mod bundle_gates_tests;
mod capabilities;
mod channel_states;
mod character_gameplay_fence;
mod character_progression_binding;
#[cfg(test)]
mod character_progression_binding_tests;
pub mod charm;
mod chat_intent;
mod chat_relay;
mod combat_lock_fact;
#[cfg(test)]
mod composed_test_owner;
mod connection;
mod container_view;
mod equipment_move;
pub mod fresh_evidence;
mod guild_chat;
mod guild_intent;
mod highscores;
mod input_gate;
mod item_look;
mod item_move;
mod item_ref_admission;
mod item_use;
mod item_view;
mod kill_reward;
mod last_position;
mod logout;
#[cfg(test)]
mod logout_tests;
mod loop_move;
mod mining;
pub mod spectate_session;
use crate::map::boot::BundleCollisionIndexExt;
pub use last_position::LastPositionFlush;
mod control_loss_reconciliation;
mod map_door;
mod monk_save;
mod monster_ai_cycle;
mod monster_profile_cycle;
#[allow(
    dead_code,
    reason = "Private source-qualified monster owner ABI is native-tested; shipping gameplay loop activation remains a separate integration gate"
)]
mod monster_quest_death;
mod newhaven_arrival;
mod newhaven_travel;
mod npc_actor;
mod npc_command;
mod npc_talk;
mod npc_trade_command;
mod npc_travel;
mod npc_voice;
mod paced_movement;
mod pvp_death;
mod tutorial_corruptor;
mod tutorial_movement;
mod tutorial_session;
mod world_respawns;
pub use npc_talk::NpcTalkBook;
mod quest_admission;
mod quest_log_source;
pub use connection::quest_log;
mod resume;
mod rune_use_bridge;
mod seam_rooms;
pub use seam_rooms::SeamRooms;
mod promotion_death;
mod promotion_login;
mod source_item_cycle;
mod source_pvp_expiry;
mod spell_access_facts;
mod spell_character_facts;
mod spell_magnitude_facts;
pub mod spell_premium_coordinator;
pub use promotion_login::PremiumSessions;
mod spell_presentation_publisher;
#[allow(
    unused_imports,
    reason = "Retain the original presentation type path during the ADR-0019 carve"
)]
pub use spell_presentations::PreparedPresentation as PreparedSpellPresentation;
mod tcp_tls;
mod world_map;

/// ADMIT0: the boot-bound Channel required gameplay capabilities (D1081-D1084).
pub mod world_required_capabilities;
pub mod world_spatial;

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
    Fnd04ConsumerError, FreshDurabilityCurrentAuthorityV1, FreshDurabilityTrustContext,
    fresh_grant_signing_key_id, recovery_source_sealed, verify_fresh_grant_durability_v1,
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
use connection::{
    AccountAchievementsReply, AdmissionRefusal, AdmittedSession, ConnectionIdentifiers,
    ControlLossResult, ControllerBinding, FirstEntryOutcome, FreshAdmissionAttempt,
    FreshAdmissionAuthority, GraceExpiryResult, IDLE_LIVENESS, SessionContinuity, StepOutcome,
    UseOutcome, admit_frame, serve_admitted,
};
pub use fresh_evidence::FreshEvidenceSource;
use kill_reward::{KillRelease, KillReleaseMark, ReleaseEnd};
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::item_look::ItemLookResult;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use {
    crate::foundation::{
        AuthenticatedTransportRefV1, ChannelId, ChannelRuntimeV1, CharacterWorldEligibilityClaimV1,
        ExactActorRef, FirstEntryPosition, GameSessionAuthoritySnapshot, GameSessionId,
        GameSessionState, PlayerActorReservation, RuntimeScopeRefV1, ScopeOwnershipGeneration,
        WorldId,
    },
    oteryn_gameplay_domain::domain::carrier_values::CarrierError,
};
mod condition_snapshots;
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
pub const MAX_CONNECTIONS: usize = 256;
/// Registered Server Seam hard maximum of concurrent handshake/auth units.
pub const MAX_HANDSHAKE_UNITS: usize = 64;

/// Pause after a listener accept error before accepting again.
const ACCEPT_ERROR_PAUSE: Duration = Duration::from_millis(100);

/// Caller-supplied listener budgets. Values may reduce, never exceed, the
/// registered maxima; the entry deadline has no default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerLimits {
    connections: usize,
    handshake_units: usize,
    entry_deadline: Duration,
}

impl ListenerLimits {
    pub fn new(
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
pub trait ConnectionObserver {
    fn ended(&self, end: connection::ConnectionEnd);
}

impl ConnectionObserver for () {
    fn ended(&self, _: connection::ConnectionEnd) {}
}

mod control_loss;
mod listener;
pub use listener::serve_listener;
#[cfg(test)]
use listener::{LostControl, control_loss_lifecycle};
pub mod gameplay_drain;
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
    /// The composition root's leveled diagnostic writer.
    pub emit: fn(&str, &oteryn_error_codes::Line<'_>),
    pub root: &'a DurabilityRoot,
    pub character: &'a ReconciledCharacterAuthority<'f, 's>,
    pub holder: &'a NodeIncarnationProof,
    /// The Platform admission-evidence route, fetched on demand per attempt.
    pub evidence: &'a FreshEvidenceSource,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    /// ADMIT0 §3.2: the requirements boot validated against this generation's real offer
    /// before readiness. Immutable data; it never stands in for current scope authority.
    pub required_capabilities:
        std::sync::Arc<world_required_capabilities::BoundRequiredCapabilities>,
    /// First-slice runtime authority remains crate-owned and cannot be supplied
    /// by a transport or client caller.
    pub runtime: &'a Mutex<ChannelRuntimeV1>,
    /// The active generation's qualified cells the Channel's Movement reads (#935).
    pub movement_cells: &'a NativeEntryMovementCells,
    /// The one entry-room door's bound runtime (#162 5868482467, M2b): the Channel-owner state
    /// `ComposedFreshAdmission::step` and `::use_object` both lock, alongside `runtime`, to
    /// decide movement blocking and USE_INTENT transitions.
    pub door: &'a Mutex<crate::world_runtime::LocalObjectRuntime>,
    /// C2: the activated entry-room Content with the one reward chest injected
    /// (`interaction_chest_use::with_entry_chest`), read by `::use_object` for a chest `USE`.
    pub chest: &'a crate::content::CanonicalReferencePlayableContent,
    /// The V1 spell book the cast intent's index resolves against (spell cast §3, SPELL-D1).
    pub spells: &'a crate::spell::SpellBook,
    pub active_generation: Option<&'a crate::content::ActiveGeneration>,
    pub premium_coordinator: Option<&'a spell_premium_coordinator::SpellPremiumCoordinator>,
    /// BUNDLE-GATES-1: the qualified entry room and the combat room (BUNDLE-COMBAT-ENABLE-1),
    /// read only through their gates.
    pub rooms: SeamRooms<'a>,
    /// The Achievement catalogue the `ACCOUNT_ACHIEVEMENTS_QUERY` display read resolves every
    /// fact against (display contract §2.1, §4), and a chest's achievement resolves in (C2).
    pub achievements: &'a crate::achievement_catalogue::AchievementCatalogue,
    /// NPC-CMD-1a: what the NPCs of the Channel say; `None` while no NPC catalogue is loaded.
    pub npc_talk: Option<std::sync::Arc<NpcTalkBook>>,
    /// Complete immutable Charm data imported at boot. This reference supplies no current
    /// generation authority or effect availability; gameplay composition is a separate step.
    pub imported_charms: &'a crate::content::charm_source::CanonicalCharmCatalogue,
    /// The quest catalogue loaded at boot for the served content revision (QUEST-CAT-BOOT-1).
    pub quest_catalogue: &'a std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>,
    pub quest_log_content: &'a quest_log::QuestLogLines,
    /// CHAR-POSITION-1: the node's shutdown write of the tracked last positions; `None` when the
    /// caller never withdraws readiness through it.
    pub last_position_flush: Option<&'a LastPositionFlush>,
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
pub fn validate_gameplay_tls(
    certificates: &[rustls::pki_types::CertificateDer<'static>],
    private_key: &rustls::pki_types::PrivateKeyDer<'static>,
) -> Result<(), rustls::Error> {
    tcp_tls::tls_config(certificates.to_vec(), private_key.clone_key()).map(|_| ())
}

/// ATTACK0-RL-03: the longest wait of one in-fight hold step, the D115 think interval, so a held
/// actor's monster melee pass keeps the cadence a connection would give it.
const HOLD_STEP: Duration = Duration::from_millis(crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS);

/// The wait of one in-fight hold step with `ahead_micros` left on the deadline.
fn hold_step(ahead_micros: u64) -> Duration {
    Duration::from_micros(ahead_micros).min(HOLD_STEP)
}

/// ATTACK0-RL-03: the next hold step of a closed client, at most `limit`, while its in-fight
/// deadline holds it in the world; `None` when nothing holds it. A dead actor is not held: it
/// takes no hit, and its release settles the death.
fn held_step(
    states: &actor_spell::ChannelSpellStates,
    attack: &attack::ChannelAttackStates,
    actor: ExactActorRef,
    session: GameSessionId,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
    limit: Duration,
) -> Option<Duration> {
    if states.is_dead(actor) {
        return None;
    }
    let until = attack.in_fight_until(actor, session, now)?;
    Some(hold_step(until.get().saturating_sub(now.get())).min(limit))
}

mod serving;
pub use serving::serve_gameplay;
/// PREM-1b: the node's Premium pulls. Without configuration, or with an invalid one, there is no
/// client and every account reads Free; admission is never affected.
fn premium_refresher(
    root: &DurabilityRoot,
    consumer: std::sync::Arc<crate::premium::PremiumConsumer>,
) -> crate::premium::refresh::PremiumRefresher {
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
    crate::premium::refresh::PremiumRefresher::new(consumer, root.clone(), client)
}

/// Fresh, unpredictable identifiers from the TLS provider's secure random
/// source.
pub struct SecureIdentifiers;

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
pub struct ComposedFreshAdmission<'a, 'f, 's> {
    /// The diagnostic writer supplied with the composed gameplay owners.
    pub emit: fn(&str, &oteryn_error_codes::Line<'_>),
    pub root: &'a DurabilityRoot,
    pub character: &'a ReconciledCharacterAuthority<'f, 's>,
    pub holder: &'a NodeIncarnationProof,
    pub evidence: &'a FreshEvidenceSource,
    pub reset_epoch: u64,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    /// See [`GameplaySeamOwners::required_capabilities`].
    pub required_capabilities:
        std::sync::Arc<world_required_capabilities::BoundRequiredCapabilities>,
    pub runtime: &'a Mutex<ChannelRuntimeV1>,
    pub movement_cells: &'a NativeEntryMovementCells,
    /// The one entry-room door's bound runtime (#162 5868482467, M2b). Always locked after
    /// `runtime`, never before, so `step` and `use_object` can never deadlock against each
    /// other.
    pub door: &'a Mutex<crate::world_runtime::LocalObjectRuntime>,
    /// C2: the entry-room Content with the injected reward chest; never locked, read-only.
    pub chest: &'a crate::content::CanonicalReferencePlayableContent,
    pub spells: &'a crate::spell::SpellBook,
    pub active_generation: Option<&'a crate::content::ActiveGeneration>,
    pub premium_coordinator: Option<&'a spell_premium_coordinator::SpellPremiumCoordinator>,
    /// See [`GameplaySeamOwners::rooms`].
    pub rooms: SeamRooms<'a>,
    pub achievements: &'a crate::achievement_catalogue::AchievementCatalogue,
    // Keep the same imported catalogue resident for every connection served by this owner.
    pub imported_charms: &'a crate::content::charm_source::CanonicalCharmCatalogue,
    /// Origin of the owner clock cooldowns are measured on: process-local and monotonic, never a
    /// client or wall-clock time (spell cast §2).
    pub clock_origin: std::time::Instant,
    pub states: channel_states::ChannelSessionStates,
    /// One revision-advancing write in flight per Character (CHAR-REV-SEQ-1). Never awaited
    /// while `runtime` is locked.
    pub revision_sequencer:
        crate::durability::character_revision_sequencer::CharacterRevisionSequencer,
    /// The quest catalogue of the served content revision (QUEST-STATE-0 §4); `None` while no
    /// quest content is loaded.
    pub quest_catalogue: Option<std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>>,
    pub quest_log_content: Option<quest_log::QuestLogLines>,
    /// The active Wheel ruleset (WHEEL-0 §4, §5.1); `None` when the embedded catalogue does not
    /// read, and every Wheel load then fails closed.
    pub wheel_ruleset: Option<std::sync::Arc<crate::durability::character_wheel::WheelRuleset>>,
    /// PREM-1b: the account's Premium pulls, started at fresh admission and reconnect without
    /// waiting on them, and cancelled when the session is released.
    pub premium: crate::premium::refresh::PremiumRefresher,
    /// The Channel's spell lane (ARCH-SPELL-LOCK-2 §1.2): taken before `runtime` by every
    /// key-33 caller and never while a Channel guard is held.
    pub spell_lane: crate::durability::spell_owner_commit::SpellLane,
}

/// The progression binding one admitted session holds (ARCH-PROGRESSION-SOURCE-0 §1.4): the
/// pinned table length on both the death and the kill reward paths.
pub type SessionProgressionBinding = crate::combat::RewardProgressionBinding<
    { crate::content::character_progression_content::CHARACTER_EXPERIENCE_TABLE_LEVELS },
>;

/// The seam's map of bound sessions to their progression bindings.
pub type ProgressionSessions = std::sync::Mutex<
    std::collections::HashMap<GameSessionId, std::sync::Arc<SessionProgressionBinding>>,
>;

/// What one durable death attempt under a session's progression binding decided (DEATH-2
/// §4.3-§4.4, ARCH-PROGRESSION-SOURCE-0 §1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeathCommit {
    /// The death is committed or replayed; its pending respawn stays recorded until the actor
    /// is placed ([`ComposedFreshAdmission::place_and_consume_settled_respawn`]).
    Settled,
    /// The durable writer refused the binding's progression context: a newer Character
    /// interpretation was configured after the session was bound, so the gameplay fence refuses
    /// every durable progression write under this binding and no retry can commit it.
    ContextMismatch,
    /// No outcome: the actor stays dead and the next cadence retries the same occurrence.
    Unavailable,
}

/// DEATH-2: the decision of a refused durable death. Only `ProgressionContextMismatch` is a
/// refusal of the binding itself; every other refusal keeps the actor dead and retries, as before.
const fn death_commit_refusal(
    error: &crate::durability::character_progression::CharacterProgressionError,
) -> DeathCommit {
    use crate::durability::character_progression::CharacterProgressionError as Refusal;
    match error {
        Refusal::ProgressionContextMismatch => DeathCommit::ContextMismatch,
        _ => DeathCommit::Unavailable,
    }
}

/// The progression binding of `session` in `sessions`; `None` for an unbound or unknown session.
/// The lock is released before the caller awaits.
fn session_progression_binding(
    sessions: &ProgressionSessions,
    session: GameSessionId,
) -> Option<std::sync::Arc<SessionProgressionBinding>> {
    sessions
        .lock()
        .ok()?
        .get(&session)
        .map(std::sync::Arc::clone)
}

/// What settling one runtime death of a session decided (DEATH-2 §4.3-§4.5,
/// DEATH-RESTART-SAFETY-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionDeath {
    /// The durable death is committed or replayed: the actor may be placed, its pending respawn
    /// consumed and only then refilled.
    Durable,
    /// A durable attempt without an outcome: the actor stays dead and the next cadence retries
    /// the same occurrence.
    Retry,
    /// The session holds no progression binding, so no durable death can commit under it. The
    /// actor is never respawned without a durable death (first player death decision §4.5): it
    /// stays dead and input-ineligible, nothing is written and nothing retries. The session's
    /// end releases it; the generation ended before a commit, so no durable death happened and
    /// the next admission recovers the Character normally.
    NoDurableDeath,
}

/// ARCH-PROGRESSION-SOURCE-0 §1.5 (as amended by DEATH-RESTART-SAFETY-1) and DEATH-2
/// §4.3-§4.5: settle one death of `session` under its progression binding. An unbound session
/// commits nothing and never respawns ([`SessionDeath::NoDurableDeath`]). A bound one commits
/// through `commit`; an attempt without an outcome keeps the actor dead for the next cadence.
/// When the durable writer refuses the binding's context (a newer Character interpretation
/// configured after the session was bound), the session is downgraded to unbound
/// (`progression_unbound reason=progression_context_mismatch`) and this death is decided
/// `NoDurableDeath`: no XP, no durable write and no further attempt, so the actor is not left
/// retrying a write that can never commit, and it is not respawned without a durable death
/// either. Only the binding the refused attempt used is removed, and the gameplay fence is not
/// widened.
async fn settle_session_death<F, Fut>(
    sessions: &ProgressionSessions,
    session: GameSessionId,
    commit: F,
) -> SessionDeath
where
    F: FnOnce(std::sync::Arc<SessionProgressionBinding>) -> Fut,
    Fut: std::future::Future<Output = DeathCommit>,
{
    let Some(binding) = session_progression_binding(sessions, session) else {
        return SessionDeath::NoDurableDeath;
    };
    match commit(std::sync::Arc::clone(&binding)).await {
        DeathCommit::Settled => SessionDeath::Durable,
        DeathCommit::Unavailable => SessionDeath::Retry,
        DeathCommit::ContextMismatch => {
            if let Ok(mut bound) = sessions.lock()
                && bound
                    .get(&session)
                    .is_some_and(|current| std::sync::Arc::ptr_eq(current, &binding))
            {
                bound.remove(&session);
            }
            operator_event(&format!(
                "progression_unbound reason={}",
                character_progression_binding::ProgressionUnbound::ContextMismatch.reason()
            ));
            SessionDeath::NoDurableDeath
        }
    }
}

/// DEATH-RESTART-SAFETY-1 (first player death decision §4.5, DEATH-0 §3.4): the order of one
/// in-session respawn. The durable death is settled first; only a durable one is respawned: the
/// actor is placed at its respawn position while the session's current durable fence is held, and
/// its pending respawn is consumed in the same transaction after the placement
/// ([`DurabilityRoot::place_and_consume_pending_respawn`], a settle may replay a retained receipt
/// without checking the fence, so this is the first current check); and only a placed, consumed
/// respawn refills the vitals and ends the death. A step that does not succeed stops the chain
/// with the actor still dead, and the next cadence starts again from the settle, which replays the
/// first receipt. So a restart at any point either finds the pending respawn recorded, and the
/// next admission places the new actor at the recorded position before consuming it, or finds it
/// consumed after the placement, which happens only when a fresh admission writes the recorded
/// cell itself ([`settled_respawn_disposition`]): the recorded respawn is never lost, and the
/// death never commits twice.
///
/// [`DurabilityRoot::place_and_consume_pending_respawn`]: crate::durability::DurabilityRoot::place_and_consume_pending_respawn
async fn respawn_in_order<T>(
    settle: impl std::future::Future<Output = SessionDeath>,
    place_and_consume: impl std::future::Future<Output = bool>,
    refill: impl std::future::Future<Output = Option<T>>,
) -> Option<T> {
    match settle.await {
        SessionDeath::Durable => {}
        SessionDeath::Retry | SessionDeath::NoDurableDeath => return None,
    }
    if !place_and_consume.await {
        return None;
    }
    refill.await
}

/// DEATH-RESTART-SAFETY-1: the durable authority a fenced in-session placement holds, which the
/// placement also checks against the Channel owner's own binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AuthorizedRespawn {
    /// The Character the fence proves the session's lease holder.
    character_id: domain::CharacterId,
    /// The session's Character lease generation the fence proves current.
    lease_generation: u64,
    /// The runtime scope ownership generation the fence proves current.
    scope_generation: ScopeOwnershipGeneration,
}

/// DEATH-RESTART-SAFETY-1 (third and fourth Codex reviews of #2004): the placement of an in-session
/// respawn, in one Channel owner step under the runtime lock and with no I/O. It runs inside the
/// transaction that holds the session's durable fence ([`ComposedFreshAdmission::place_and_consume_settled_respawn`]),
/// so no termination, supersession, scope move or node revoke commits in PostgreSQL before it
/// returns. Before anything moves, the owner's own binding of that authority is checked too: the
/// actor's slot is still bound to `session` (a successor's rebind detaches it), carries no write
/// fence (every transition that may end the session fences the slot under this lock before its
/// durable write, CHARM-DESC-FENCE-LEASE step (a), and a terminal outcome keeps the fence), holds
/// exactly the Character lease the fence proves current, and the runtime still holds the scope
/// ownership generation it proves. Any mismatch moves nothing and clears no respawn condition: the
/// actor stays dead and its pending respawn stays recorded.
fn place_authorized_respawn(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    authorized: &AuthorizedRespawn,
) -> bool {
    let current = runtime
        .bound_attacker_lease(actor, session)
        .is_ok_and(|lease| {
            lease.character_id().as_bytes() == authorized.character_id.as_bytes()
                && lease.generation() == authorized.lease_generation
        })
        && runtime.binding().scope_generation() == authorized.scope_generation;
    if !current {
        operator_event("settled_respawn_placement_refused reason=authority_not_current");
        return false;
    }
    let respawn = runtime.respawn_position();
    runtime
        .place_respawned_player(actor, session, respawn)
        .is_ok()
}

/// What the pending respawn row of an in-session respawn allows (DEATH-RESTART-SAFETY-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettledRespawn {
    /// No row of this occurrence is left: an earlier attempt consumed it.
    Gone,
    /// The row records the cell a fresh admission writes, so a restart after its consumption
    /// still places the actor at the recorded respawn: it may be consumed.
    Reproduced,
    /// The row records another (or an undecodable) cell. Only the row can bring the actor there
    /// after a restart, so it stays recorded and the actor stays dead in this session; the next
    /// admission places it at the recorded cell (DEATH-2b) and consumes the row.
    Kept,
}

/// DEATH-RESTART-SAFETY-1: whether the pending respawn of `occurrence`, read before the
/// in-session consumption, may be consumed. The in-session placement is volatile, so after the
/// consumption only the next admission's own position can stand in for the recorded respawn: a
/// fresh admission writes the pinned generation's entry start, which is also
/// `ChannelRuntimeV1::respawn_position`, the `admission_cell` here. The row is consumed only when
/// it records exactly that cell; otherwise the durable evidence is kept.
fn settled_respawn_disposition(
    occurrence: crate::durability::character_death::PlayerDeathOccurrence,
    admission_cell: crate::foundation::MovementLocalPosition,
    pending: Option<&crate::durability::character_death::ConsumedRespawn>,
) -> SettledRespawn {
    match pending {
        Some(pending) if pending.occurrence == occurrence => {
            if respawn_cell(&pending.respawn_position) == Some(admission_cell) {
                SettledRespawn::Reproduced
            } else {
                SettledRespawn::Kept
            }
        }
        _ => SettledRespawn::Gone,
    }
}

pub use quest_admission::QuestSession;

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

/// PREM-1b (D476): whether a terminal release outcome proves the session over, so the
/// account's Premium pulls stop. Only a durable `Released` or `Terminal` outcome does; an
/// unended session and an unknown outcome keep them.
fn ends_session<E>(outcome: &Result<ExpiredLossReleaseV1, E>) -> bool {
    matches!(
        outcome,
        Ok(ExpiredLossReleaseV1::Released { .. } | ExpiredLossReleaseV1::Terminal)
    )
}

/// The durable terminal release a connection decides.
#[derive(Clone, Copy)]
enum TerminalRelease {
    /// A resumed session whose recovered connection ended again, on its exact transport.
    Abandoned(AuthenticatedTransportRefV1),
    /// D449: a lost session whose otherwise valid resume lacked a selected capability, at the
    /// exact loss epoch the resume verified.
    CapabilityMismatch(ControlLossEpochRefV1),
    /// LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6): the session's own `LOGOUT_INTENT`, on its
    /// exact current transport, with or without a control-loss epoch.
    Logout(AuthenticatedTransportRefV1),
}

impl TerminalRelease {
    /// The write-fence token of the release. The mismatch release fences with its lost epoch,
    /// as grace expiry does: its own retry and grace expiry join that fence instead of waiting
    /// on it, a resume of the epoch that wins the race lifts it (`restore_control`), and once
    /// control is restored no mismatch attempt can fence the slot again. An abandoned resumed
    /// session has no loss mark, so its release mints a transition id; so does a logout.
    fn fence(self, runtime: &mut ChannelRuntimeV1) -> Result<TransitionFence, CarrierError> {
        match self {
            Self::Abandoned(_) | Self::Logout(_) => runtime
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
/// A grace expiry that ends unknown keeps its hold here for the next expiry of the session;
/// concurrent lifecycles of the session each keep their own.
#[derive(Default)]
pub struct FenceHolders(
    std::sync::Mutex<std::collections::HashMap<(GameSessionId, TransitionFence), u32>>,
    std::sync::Mutex<std::collections::HashMap<GameSessionId, Vec<FenceHold>>>,
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
        self.kept().remove(&session);
    }

    fn kept(
        &self,
    ) -> std::sync::MutexGuard<'_, std::collections::HashMap<GameSessionId, Vec<FenceHold>>> {
        self.1
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// A grace expiry ended unknown still holding `hold`: the next expiry of the session takes
    /// it over instead of counting the same release again. A hold another lifecycle of the
    /// session kept stays beside it, each counted once.
    fn keep(&self, hold: FenceHold) {
        if hold.held {
            self.kept().entry(hold.session).or_default().push(hold);
        }
    }

    /// LOGOUT-WIRE-1: every hold an unknown logout of `session` kept on its minted transition
    /// token. Only a logout keeps a `Transition` hold; grace expiry and the D449 release keep
    /// their epoch's.
    fn take_transitions(&self, session: GameSessionId) -> Vec<FenceHold> {
        let mut kept = self.kept();
        let Some(holds) = kept.get_mut(&session) else {
            return Vec::new();
        };
        let (taken, rest): (Vec<_>, Vec<_>) = std::mem::take(holds)
            .into_iter()
            .partition(|hold| matches!(hold.token, TransitionFence::Transition(_)));
        if rest.is_empty() {
            kept.remove(&session);
        } else {
            *holds = rest;
        }
        taken
    }

    /// One hold an unknown grace expiry of `session` kept, if any.
    fn resume(&self, session: GameSessionId) -> Option<FenceHold> {
        let mut kept = self.kept();
        let holds = kept.get_mut(&session)?;
        let hold = holds.pop();
        if holds.is_empty() {
            kept.remove(&session);
        }
        hold
    }
}

/// One attempt of a terminal release.
enum ReleaseAttempt {
    /// A step before the durable transaction had no outcome; nothing was sent.
    Retry,
    /// The transaction's outcome, or its settle, is unknown: the fence and the kill mark stay.
    Unknown,
    /// The release settled.
    Done(GraceExpiryResult),
}

/// Step (c) of a transition whose durable attempt did not end the hold.
enum UnendedSettle {
    Lifted,
    Terminal,
    Unknown,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn save_familiar_at_actor_end(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        logout: Option<(AuthenticatedTransportRefV1, u64)>,
    ) -> actor_spell::FamiliarLogoutSave {
        // LOGOUT-WIRE-1 (#2127 Codex P1 4278730847): an ACTIVE session's own logout ends the
        // actor too; the familiar is saved on the exact transport before the terminal release.
        if let Some((transport, attempt)) = logout {
            return self
                .save_familiar_logout(
                    actor,
                    session,
                    actor_spell::FamiliarActorEnd::Logout(transport, attempt),
                )
                .await;
        }
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let Ok((current, now)) = store.current_session_at(session).await else {
            return actor_spell::FamiliarLogoutSave::Unknown;
        };
        if current.session_state() != GameSessionState::Reconnectable {
            return actor_spell::FamiliarLogoutSave::NotApplicable;
        }
        let Some(deadline) = current.current_original_grace_deadline() else {
            return actor_spell::FamiliarLogoutSave::Unknown;
        };
        if now < deadline {
            return actor_spell::FamiliarLogoutSave::NotApplicable;
        }
        let Some(epoch) = current.current_control_loss_epoch() else {
            return actor_spell::FamiliarLogoutSave::Unknown;
        };
        self.save_familiar_logout(
            actor,
            session,
            actor_spell::FamiliarActorEnd::GraceExpiry(epoch.get()),
        )
        .await
    }

    /// DEATH-2 (first player death decision §4.3-§4.5): settle one runtime death and respawn the
    /// actor, in the order of [`respawn_in_order`]. With the session's progression binding the
    /// death is committed (or replayed) off the owner lane, with no lock held across the database;
    /// the dead actor is then placed at its respawn position in one owner step while the session's
    /// current durable fence is held, and its pending respawn consumed after that placement in the
    /// same transaction ([`Self::place_and_consume_settled_respawn`]); only then is it refilled in
    /// another owner step, which ends the death. `None` (a durable attempt without an outcome, a
    /// session without a durable death, a refused fence, a stale actor, a recorded cell no
    /// admission reproduces or a consumption without an outcome) keeps the player dead, and the
    /// next cadence tick retries the same occurrence.
    async fn respawn_after_death(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        death: actor_spell::PlayerDeath,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        // DEATH-ML-LOSS-1 (CP D1433, #2193 Codex P1 4304911165): the committed death's
        // post-loss build, carried from the settle to the respawn step that rebases the live
        // actor onto it, so the next training save submits the lowered durable build.
        let lowered = std::sync::OnceLock::new();
        respawn_in_order(
            async {
                let (settled, after) = self.settle_death(actor, session, death, false).await;
                if let Some(after) = after {
                    let _ = lowered.set(after);
                }
                settled
            },
            self.place_and_consume_settled_respawn(actor, session, death.occurrence),
            async {
                let now = self.owner_now();
                let runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.lock().await;
                if let Some(after) = lowered.get() {
                    // A replayed death rebases again onto the same receipt; an actor this step
                    // cannot reach stays dead for the next cadence.
                    states
                        .get_mut(&runtime, actor, session)?
                        .rebase_after_death(after);
                }
                states.respawn(&runtime, actor, session, death.occurrence, now)
            },
        )
        .await
    }

    /// DEATH-2 §4.3-§4.4: with the session's progression binding, commit (or replay) the durable
    /// death of `death`, with no lock held across the database. Its pending respawn stays
    /// recorded for the placement.
    ///
    /// DEATH-ML-LOSS-1 (A13 §4.6, CP D1433): before a new death the actor's pending training is
    /// force-saved ([`Self::flush_training_before_death`]), and a committed death that lowered the
    /// build returns its post-loss build for the respawn's rebase. `releasing` marks the actor-end
    /// release, which settles a session whose training writes are fenced out once the fence left
    /// no training in memory (Codex P1 4307011952).
    async fn settle_death(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        death: actor_spell::PlayerDeath,
        releasing: bool,
    ) -> (
        SessionDeath,
        Option<crate::durability::character_build::DurableBuildState>,
    ) {
        self.states.newhaven_travel.death(session);
        let mut lowered = None;
        let lowered_slot = &mut lowered;
        let settled =
            settle_session_death(&self.states.progression_sessions, session, |progression| {
                Box::pin(async move {
                    if !self
                        .flush_training_before_death(actor, session, death, releasing)
                        .await
                    {
                        return DeathCommit::Unavailable;
                    }
                    let (commit, after) = self.commit_death(session, &progression, death).await;
                    *lowered_slot = after;
                    commit
                })
            })
            .await;
        (settled, lowered)
    }

    /// DEATH-ML-LOSS-1 (A13 §4.6, CP D1433, #2193 Codex P1 4304911161): the death's loss is
    /// priced from a build that holds the actor's trained progress, so a new death first
    /// force-saves the pending training. A death the retained receipts already hold is not new:
    /// its build was priced when it committed and its pending respawn refuses build writes, so
    /// the flush is skipped. `false` (an unknown save or a failed reconcile) keeps the actor
    /// dead for the next attempt; see [`death_flush_permits_settle`] for a fenced-out save.
    async fn flush_training_before_death(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        death: actor_spell::PlayerDeath,
        releasing: bool,
    ) -> bool {
        match self
            .root
            .reconcile_character_death(self.character, death.occurrence)
            .await
        {
            Ok(Some(_)) => true,
            Ok(None) => {
                let mut save = self.save_spell_training(actor, session, true).await;
                // #2193 Codex P1 4306765047, 4307011952: a flush whose retained checkpoint
                // committed under a since-moved session fence replays that fence forever; its
                // durable receipt is applied first, on the live and the release path, and the
                // post-checkpoint training saves under the current fence.
                if save == actor_spell::TrainingSave::FencedOut
                    && self
                        .adopt_committed_training_checkpoint(actor, session)
                        .await
                {
                    save = self.save_spell_training(actor, session, true).await;
                }
                // #2193 Codex P1 4307011952: a fenced-out flush proves nothing committed (a
                // write the store refused as `AuthorityRejected` keeps its checkpoint and the
                // pending training only in memory), so whether the actor still holds training is
                // read back before the release may settle from the durable build.
                let training_left = save == actor_spell::TrainingSave::FencedOut && {
                    let runtime = self.runtime.lock().await;
                    self.states
                        .spell_states
                        .lock()
                        .await
                        .has_training_to_flush(&runtime, actor, session)
                };
                death_flush_permits_settle(save, releasing, training_left)
            }
            Err(error) => {
                operator_event(&format!("player_death_reconcile_failed reason={error}"));
                false
            }
        }
    }

    /// DEATH-ML-LOSS-1 (#2193 Codex P1 4306765047): a forced training save whose retained
    /// checkpoint committed while the session's connection, lease or scope generation moved
    /// returns `FencedOut` without applying the receipt, and every replay fails the same
    /// immutable retained fence. When the checkpoint's receipt is durable and the live actor is
    /// still this session's under the current session, scope and owner authority, the receipt is
    /// applied to the live actor: the checkpoint retires, the receipt's build is the actor's
    /// durable training base, and its post-checkpoint training stays pending for the next save,
    /// which prepares under the current fence. Nothing is written. `false` (no retained
    /// checkpoint, no durable receipt, a terminal or moved session, a failed read or a refused
    /// receipt) changes nothing.
    async fn adopt_committed_training_checkpoint(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let Ok((current, _)) = store.current_session_at(session).await else {
            return false;
        };
        if current.session_state() == GameSessionState::Terminal {
            return false;
        }
        let Ok(character) =
            domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes())
        else {
            return false;
        };
        // The Character's revision writer, as the training save holds it, before the runtime lock.
        let _slot = self.revision_sequencer.acquire(character).await;
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let binding = runtime.binding();
        if current.current_runtime_scope()
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
            || current.current_scope_generation() != binding.scope_generation()
            || runtime.player_control_facts(actor, session).is_err()
            || runtime.owner_fence().is_err()
        {
            return false;
        }
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return false;
        };
        let Some(retained) = state.pending_training_checkpoint().cloned() else {
            return false;
        };
        let Ok(Some(receipt)) = self
            .root
            .reconcile_character_build(self.character, retained.prepared.request.occurrence)
            .await
        else {
            return false;
        };
        state
            .commit_training_checkpoint(retained.prepared, &receipt)
            .is_ok()
    }

    /// DEATH-RESTART-SAFETY-1 (#2004 Codex P1 4248558737): resolve the session's current fences
    /// and, in the Character's revision slot, place the dead actor while PostgreSQL holds them
    /// ([`DurabilityRoot::place_and_consume_pending_respawn_placed`]): the placement runs inside
    /// the fenced transaction, so a session that turns terminal or is superseded, a moved lease, a
    /// moved scope assignment or a revoked node incarnation commits either before the fence is
    /// asserted, and nothing is placed, or after the placement. A settle that replayed a retained
    /// receipt checked none of them. The owner step itself revalidates the owner's binding
    /// ([`place_authorized_respawn`]) and decides from the pending row read under the fence whether
    /// it may be consumed ([`settled_respawn_disposition`]); the row is deleted only after the
    /// placement, in the same transaction, which also upserts the placed respawn cell as the
    /// Character's last position (CHAR-POSITION-0 §3.3 step 1), so a crash right after the
    /// placement never puts the Character back at its death cell. No I/O runs while the runtime
    /// lock is held. `false` (no current fence, a failed cursor read, a refused fence or
    /// placement, a recorded cell no admission reproduces or a failed write) keeps the actor dead;
    /// the row stays for the next attempt or, after a restart, for the next admission.
    ///
    /// [`DurabilityRoot::place_and_consume_pending_respawn_placed`]: crate::durability::DurabilityRoot::place_and_consume_pending_respawn_placed
    async fn place_and_consume_settled_respawn(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        occurrence: crate::durability::character_death::PlayerDeathOccurrence,
    ) -> bool {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(expected_character_revision) = slot.cursor(self.root, self.character).await else {
            return false;
        };
        let authorized = AuthorizedRespawn {
            character_id: fence.character_id,
            lease_generation: fence.character_lease_generation,
            scope_generation: fence.scope_ownership_generation,
        };
        // CHAR-POSITION-0 §3.2-§3.3: the respawn cell the placement puts the actor at, upserted as
        // the last position in the transaction that consumes the respawn. Its sequence is taken
        // after the death, and a dead actor is never captured, so no capture of the living actor
        // orders after it.
        let respawned = {
            let runtime = self.runtime.lock().await;
            self.placed_record(&runtime, runtime.respawn_position())
        };
        let runtime = &self.runtime;
        let placed = self
            .root
            .place_and_consume_pending_respawn_placed(
                self.character,
                self.holder,
                crate::durability::character_progression::CurrentCharacterGameplayFence {
                    expected_character_revision,
                    ..fence
                },
                occurrence,
                respawned,
                move |pending| async move {
                    let mut runtime = runtime.lock().await;
                    let admission_cell = runtime.respawn_position();
                    if !last_position::respawn_cell_recorded(admission_cell, &respawned) {
                        operator_event("settled_respawn_placement_refused reason=cell_changed");
                        return false;
                    }
                    match settled_respawn_disposition(occurrence, admission_cell, pending.as_ref())
                    {
                        SettledRespawn::Kept => {
                            operator_event(
                                "settled_respawn_kept reason=not_reproduced_by_admission",
                            );
                            false
                        }
                        SettledRespawn::Gone | SettledRespawn::Reproduced => {
                            place_authorized_respawn(&mut runtime, actor, session, &authorized)
                        }
                    }
                },
            )
            .await;
        drop(slot);
        match placed {
            Ok(placed) => {
                if placed {
                    self.states
                        .last_positions
                        .respawned(session, respawned.record.cell);
                    // NPC-TALK-2: a respawned customer away from its NPC ends its conversation.
                    self.states
                        .chat
                        .close_out_of_range(&*self.runtime.lock().await, session);
                }
                placed
            }
            Err(error) => {
                operator_event(&format!("settled_respawn_placement_failed reason={error}"));
                false
            }
        }
    }

    /// DEATH-2 for a closed client: an actor that died after its connection ended has no cadence
    /// left to settle the death, so its release settles it before the actor leaves, and the next
    /// admission consumes the respawn. `settled` is the death this release already committed, so a
    /// retry after the durable release does not write it again. `false` keeps the actor: the
    /// durable attempt had no outcome.
    async fn settle_released_death(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        settled: &mut Option<actor_spell::PlayerDeath>,
    ) -> bool {
        let death = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            states.player_death(&runtime, actor, session)
        };
        match death {
            // A durable death leaves its pending respawn for the next admission (DEATH-0 §3.4); a
            // session without a durable death releases the actor with nothing written.
            Some(death) if *settled != Some(death) => {
                let (outcome, lowered) = self.settle_death(actor, session, death, true).await;
                match outcome {
                    SessionDeath::Durable | SessionDeath::NoDurableDeath => {
                        // DEATH-ML-LOSS-1 (A13 §4.6, CP D1439): the released actor continues
                        // from the death's post-loss build, as a respawn would rebase it, so the
                        // release's own training save and a resumed epoch never submit the
                        // pre-death build.
                        if let Some(after) = lowered {
                            let runtime = self.runtime.lock().await;
                            let mut states = self.states.spell_states.lock().await;
                            if let Some(state) = states.get_mut(&runtime, actor, session) {
                                state.rebase_after_death(&after);
                            }
                        }
                        *settled = Some(death);
                        true
                    }
                    SessionDeath::Retry => false,
                }
            }
            _ => true,
        }
    }

    /// DEATH-0 §3.4 and DEATH-2: a fresh admission with full vitals is the respawn of a death
    /// committed before the previous actor ended. In the Character's revision slot and under the
    /// admitted session's fences, before the session's other Character writes, its pending
    /// respawn is read, the new actor is placed at the recorded respawn position, and only then
    /// is the obligation consumed. The first-entry owner step already placed the actor for the
    /// respawn it read (`decided`, CHAR-POSITION-0 §3.3); only a respawn it did not see is placed
    /// here. A recorded cell that is not valid or not free uses the CHAR-POSITION-0 §3.3
    /// fallback, so a placement is always defined. A placement that cannot complete or a failed
    /// read or write leaves the row for the next admission; the Character writers refuse with
    /// `RespawnPending` until then. `true` only when exactly the placed respawn was consumed, or
    /// no respawn is pending and the first entry was placed from a successful last-position read
    /// (CHAR-POSITION-0 §3.3 steps 3 and 4): a failed read or placement leaves this admission not
    /// playable, as an unplaced respawn does.
    async fn consume_admitted_respawn(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        decided: Option<&last_position::AdmissionPosition>,
    ) -> bool {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return false;
        };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(expected_character_revision) = slot.cursor(self.root, self.character).await else {
            return false;
        };
        let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
            expected_character_revision,
            ..fence
        };
        let pending = match self
            .root
            .pending_respawn(self.character, self.holder, fence)
            .await
        {
            Ok(Some(pending)) => pending,
            Ok(None) => {
                // CHAR-POSITION-0 §3.3 steps 3 and 4: no pending respawn, so the last position
                // the first-entry step was placed by. An entry without a successful read, or one
                // placed for a respawn that is gone, keeps the admission not playable.
                drop(slot);
                return decided
                    .is_some_and(|decided| self.track_restored(session, actor, decided, fence));
            }
            Err(error) => {
                operator_event(&format!("pending_respawn_read_failed reason={error}"));
                return false;
            }
        };
        let placed_at_entry = matches!(
            decided,
            Some(last_position::AdmissionPosition::Respawn(placed))
                if placed.occurrence == pending.occurrence
        );
        if !placed_at_entry
            && !self
                .place_admitted_respawn(session, actor, &pending.respawn_position)
                .await
        {
            operator_event("pending_respawn_placement_failed");
            return false;
        }
        // CHAR-POSITION-0 §3.3: the consumption upserts the placed cell as the last position, in
        // the same transaction, under the new lease's key.
        // A placed actor whose cell cannot be read back (stale or gone) consumes nothing: the
        // obligation stays for the next admission rather than be consumed without its cell.
        let Some(placed) = self.respawn_last_position(actor).await else {
            operator_event("pending_respawn_placed_cell_unavailable");
            return false;
        };
        let consumed = self
            .root
            .consume_pending_respawn_placed(
                self.character,
                self.holder,
                fence,
                Some(pending.occurrence),
                placed,
            )
            .await;
        if matches!(&consumed, Ok(Some(consumed)) if consumed.occurrence == pending.occurrence) {
            self.track_respawned(session, actor, &placed, fence);
        }
        if let Err(error) = &consumed {
            operator_event(&format!(
                "pending_respawn_consumption_failed reason={error}"
            ));
        }
        admitted_respawn_consumed(pending.occurrence, consumed)
    }

    /// DEATH-2b: place the admitted actor for its pending respawn, in one Channel owner turn: at
    /// the recorded cell if it is an admissible free cell, else by the CHAR-POSITION-0 §3.3
    /// fallback (`ChannelRuntimeV1::place_admitted_respawn`). `false` only when the actor cannot
    /// be placed at all, and then nothing moves.
    async fn place_admitted_respawn(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        recorded: &[u8],
    ) -> bool {
        let mut runtime = self.runtime.lock().await;
        let pinned_digest = runtime.content_pin().server_artifact_digest();
        let door = self.door.lock().await;
        runtime
            .place_admitted_respawn(actor, session, respawn_cell(recorded), |cell| {
                respawn_cell_admissible(
                    self.movement_cells.index(),
                    self.movement_cells.scope(),
                    self.world_id,
                    pinned_digest,
                    door.blocking_cells(),
                    cell,
                )
            })
            .is_ok()
    }

    /// QUEST-STATE-0 §7 and §5.4: load the admitted session's quest copy and request its
    /// pending obligations again, in the Character's revision slot. A failed load fails the
    /// session's quest actions closed, never the login.
    async fn admit_quest_session(&self, admitted: &AdmittedSession) {
        self.refresh_quest_session(admitted.game_session_id, true)
            .await;
        self.admit_wheel_session(admitted.game_session_id).await;
    }

    /// WHEEL-0 §4 "Wheel reset" and "Load": the admission Wheel reset in the Character's
    /// revision slot, then the session's allocation. A failed load fails the session's Wheel
    /// closed (every stage 0), never the login.
    async fn admit_wheel_session(&self, session: GameSessionId) {
        let allocation = match (self.current_quest_fence(session).await, &self.wheel_ruleset) {
            (Ok(Some(fence)), Some(ruleset)) => {
                crate::durability::character_wheel::admit_character_wheel(
                    &self.revision_sequencer,
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    ruleset,
                )
                .await
            }
            _ => None,
        };
        if let Ok(mut sessions) = self.states.wheel_sessions.lock() {
            sessions.insert(session, allocation);
        }
    }

    /// The Wheel stages of `session` for one cast snapshot (WHEEL0-EL-1): derived from the cached
    /// allocation and the eligibility facts read at this use. All 0 without an allocation.
    #[allow(
        dead_code,
        reason = "SPELL-WHEEL-GATE-1 passes these stages into the cast snapshot"
    )]
    pub fn wheel_stages(
        &self,
        session: GameSessionId,
        vocation: &str,
        level: u32,
        promoted: bool,
    ) -> crate::durability::character_wheel::WheelStages {
        let Some(ruleset) = &self.wheel_ruleset else {
            return crate::durability::character_wheel::WheelStages::default();
        };
        self.states
            .wheel_sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&session).cloned().flatten())
            .map(|allocation| {
                crate::durability::character_wheel::WheelStages::derive(
                    ruleset,
                    &allocation,
                    vocation,
                    level,
                    promoted,
                )
            })
            .unwrap_or_default()
    }

    /// The owner cadence: refresh every session whose backoff has passed. The map lock is
    /// never held across an await.
    async fn refresh_due_quest_sessions(&self) {
        let now = std::time::Instant::now();
        let due: Vec<GameSessionId> = match self.states.quest_sessions.lock() {
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
        if let Ok(mut sessions) = self.states.quest_sessions.lock()
            && let Some(entry) = sessions.get_mut(&session)
        {
            entry.retry_at = Some(std::time::Instant::now() + QUEST_OBLIGATION_RETRY);
        }
    }

    /// The progression binding of `session`; `None` for an unbound or unknown session. The lock
    /// is released before the caller awaits.
    pub fn progression_binding(
        &self,
        session: GameSessionId,
    ) -> Option<std::sync::Arc<SessionProgressionBinding>> {
        session_progression_binding(&self.states.progression_sessions, session)
    }

    /// ARCH-PROGRESSION-SOURCE-0 §1.5: the progression step of a fresh admission whose first
    /// entry is positioned. A World pin without the section binds nothing; otherwise the session
    /// is bound or logs why not once, and a step whose rounds end undecided leaves the actor
    /// not input-eligible.
    async fn admit_progression_session(&self, admitted: &mut AdmittedSession) {
        let Some(content) = self
            .active_generation
            .and_then(|active| active.progression())
        else {
            return;
        };
        let session = admitted.game_session_id;
        // The step holds the request, whose policy carries the whole threshold table by value,
        // across its rounds: on the heap, so the admission future does not grow by it.
        let binding = Box::pin(character_progression_binding::bind_character_progression(
            character_progression_binding::ProgressionDurable {
                root: self.root,
                character: self.character,
                holder: self.holder,
            },
            |profile, ruleset, revision| content.policy_for(profile, ruleset, revision).ok(),
            || self.current_quest_fence(session),
            character_progression_binding::ProgressionRounds {
                attempts: RECONCILE_ATTEMPTS,
                backoff: RECONCILE_BACKOFF,
            },
        ))
        .await;
        match binding {
            character_progression_binding::ProgressionBinding::Bound(policy) => {
                let binding = std::sync::Arc::new(SessionProgressionBinding::from_content(*policy));
                if let Ok(mut sessions) = self.states.progression_sessions.lock() {
                    sessions.insert(session, binding);
                }
            }
            character_progression_binding::ProgressionBinding::Unbound(unbound) => {
                operator_event(&format!("progression_unbound reason={}", unbound.reason()));
                if unbound
                    == character_progression_binding::ProgressionUnbound::InitializationUnavailable
                {
                    admitted.first_entry = FirstEntryOutcome::RefusedUnavailable;
                }
            }
        }
    }

    /// Drop the lost entry of `session` only if it is still the one that ended at
    /// `generation`; a later resumed and lost again connection keeps its own entry.
    /// Keep this conditional removal separate from terminal cleanup: resume uses it too.
    fn forget_lost(&self, session: GameSessionId, generation: u64) {
        if let Ok(mut lost) = self.states.lost.lock()
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
    /// joins its own fence and a resume of that epoch lifts it. An unknown expiry keeps its hold
    /// for the next one, so the lifecycle's retries count the release once.
    async fn release_after_grace(&self, admitted: AdmittedSession) -> GraceExpiryResult {
        let mut held = self.states.fence_holders.resume(admitted.game_session_id);
        let result = self.release_grace_attempts(admitted, &mut held).await;
        if result == GraceExpiryResult::Unknown
            && let Some(hold) = held
        {
            self.states.fence_holders.keep(hold);
        }
        result
    }

    async fn release_grace_attempts(
        &self,
        admitted: AdmittedSession,
        held: &mut Option<FenceHold>,
    ) -> GraceExpiryResult {
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
        let mut settled_death = None;
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
                    self.states.fence_holders.release(hold);
                }
                // Every hold kept on the session is of a lost epoch that is over.
                while let Some(mut stale) = self.states.fence_holders.resume(session) {
                    self.states.fence_holders.release(&mut stale);
                }
                // KILL-REWARD-COMP-1 §1.3: no release of the resumed epoch committed, so a kill
                // mark an unknown outcome kept is cleared as retryable.
                self.release_orphaned_kills(session).await;
                return GraceExpiryResult::NotApplicable;
            };
            let token = TransitionFence::GraceExpiry(epoch);
            if let Some(stale) = held.as_mut().filter(|hold| hold.token != token) {
                self.states.fence_holders.release(stale);
            }
            let hold = match &mut *held {
                Some(hold) if hold.token == token => hold,
                slot => slot.insert(FenceHold::new(session, token)),
            };
            // §3 item 6: fence only once the held deadline has passed.
            if let Some(ahead) = unix_seconds()
                .and_then(|now| u64::try_from(deadline.saturating_sub(now)).ok())
                .filter(|ahead| *ahead > 0)
            {
                self.wait_grace(
                    actor,
                    session,
                    Duration::from_secs(ahead).saturating_add(EXPIRY_SLACK),
                )
                .await;
                continue;
            }
            // ATTACK0-RL-03: a closed client stays in the world until its in-fight deadline ends.
            // Each hold step runs the melee pass; the whole hold counts as one attempt.
            if self.hold_while_in_fight(actor, session).await {
                while self.hold_while_in_fight(actor, session).await {}
                continue;
            }
            if !self
                .settle_released_death(actor, session, &mut settled_death)
                .await
            {
                tokio::time::sleep(next_backoff()).await;
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
                    self.release_orphaned_kills(session).await;
                    return GraceExpiryResult::NotApplicable;
                }
                FenceStep::Refused(_) => return GraceExpiryResult::Unknown,
            }
            if self.save_familiar_at_actor_end(actor, session, None).await
                == actor_spell::FamiliarLogoutSave::Unknown
                || self.save_spell_training(actor, session, true).await
                    == actor_spell::TrainingSave::Unknown
            {
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            // SPELL-D8 §8.2 save point 1: the actor's monk values are durable, or fenced out,
            // before the release can end the Character lease.
            if self.save_monk_state(&admitted, actor).await == monk_save::MonkSave::Unknown {
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            // KILL-REWARD-COMP-1 §1.3: the session's own kills settle before it ends; a deferred
            // drain sends nothing, and the lifecycle retries the expiry behind its backoff.
            let kills = match self.seal_kill_rewards(session).await {
                KillRelease::Send(mark) => mark,
                KillRelease::Deferred => return GraceExpiryResult::Unknown,
            };
            // CHAR-POSITION-0 §3.2 "At logout": the release writes the fenced actor's cell.
            let pause = match self
                .final_position_store(&store, session, actor)
                .await
                .release_expired_loss(session, &account_id)
                .await
            {
                Ok(ExpiredLossReleaseV1::NotApplicable) => {
                    match self.settle_unended(&store, actor, hold).await {
                        UnendedSettle::Lifted => {
                            self.settle_kill_release(kills, ReleaseEnd::Retryable).await;
                            return GraceExpiryResult::NotApplicable;
                        }
                        UnendedSettle::Terminal => {
                            self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                            let result = self
                                .retire_reconciled(controller.account_id, session, actor)
                                .await;
                            self.forget_kill_release(kills).await;
                            return result;
                        }
                        UnendedSettle::Unknown => {
                            self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                            next_backoff()
                        }
                    }
                }
                Ok(ExpiredLossReleaseV1::NotExpired { deadline, now }) => {
                    #[cfg(test)]
                    monk_save::postgres_tests::record_not_expired();
                    match self.settle_unended(&store, actor, hold).await {
                        UnendedSettle::Lifted => {
                            self.settle_kill_release(kills, ReleaseEnd::Retryable).await;
                            Duration::from_secs(u64::try_from(deadline - now).unwrap_or(0))
                                .saturating_add(EXPIRY_SLACK)
                        }
                        UnendedSettle::Terminal => {
                            self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                            let result = self
                                .retire_reconciled(controller.account_id, session, actor)
                                .await;
                            self.forget_kill_release(kills).await;
                            return result;
                        }
                        UnendedSettle::Unknown => {
                            self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                            next_backoff()
                        }
                    }
                }
                Ok(ExpiredLossReleaseV1::Released { .. } | ExpiredLossReleaseV1::Terminal) => {
                    self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                    // PREM-1b: the session is over; its Premium pulls stop.
                    self.release_premium(controller.account_id, session);
                    let result = self.retire(session, actor).await;
                    self.forget_kill_release(kills).await;
                    return result;
                }
                // Unknown outcome: keep the fence and the kill mark; the retry reconciles from
                // the durable row.
                Err(_) => {
                    self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                    next_backoff()
                }
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
        match self.states.fence_holders.fence(&mut runtime, actor, hold) {
            Ok(()) => FenceStep::Fenced,
            Err(CarrierError::WriteFenceBusy) => FenceStep::Wait,
            Err(error) => {
                self.states.fence_holders.release(hold);
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
            .states
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
                self.states.forget_session(session);
                GraceExpiryResult::Released
            }
            Err(_) => GraceExpiryResult::Unknown,
        }
    }

    /// PREM-1b (D476, #1722 Codex P2 4176492358): a release whose durable attempt did not end
    /// the session, but whose reconciliation found it TERMINAL, retires it and stops its Premium
    /// pulls like a proven release does.
    async fn retire_reconciled(
        &self,
        account_id: [u8; 16],
        session: GameSessionId,
        actor: ExactActorRef,
    ) -> GraceExpiryResult {
        self.release_premium(account_id, session);
        self.retire(session, actor).await
    }

    /// PREM-1b: start (or wake) the account's Premium pulls for `session`.
    fn admit_premium(&self, account_id: [u8; 16], session: GameSessionId) {
        let mut sessions = self
            .states
            .premium_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        sessions.admit(account_id, session);
        self.premium.admit(account_id);
    }

    /// PREM-1b (D476): `session` is over; the account's Premium pulls stop only when no other
    /// session of the account holds them. Decided under the same lock as admission, so a
    /// successor admitted before a late release keeps its pulls.
    /// Keep this here: the non-moved refresher must release under the registration lock.
    fn release_premium(&self, account_id: [u8; 16], session: GameSessionId) {
        let mut sessions = self
            .states
            .premium_sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if sessions.release(account_id, session) {
            self.premium.release(account_id);
        }
    }

    /// The fence, monk save, durable commit and settle steps of a terminal release that a
    /// connection decides (abandoned resume, capability mismatch), under one transition token
    /// minted for it. The actor leaves the Channel only after the durable TERMINAL fact.
    /// ATTACK0-RL-03 (ATTACK-0 §4): while `actor` has a running in-fight deadline, wait one hold
    /// step, run the Channel's coalesced monster melee pass and return `true`, so the caller
    /// checks again (a hit taken meanwhile extends the deadline); `false` once no deadline runs.
    /// The hold drives the pass itself because the held actor may be the Channel's last
    /// connection, and only connection cadences run it otherwise.
    async fn hold_while_in_fight(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.hold_step_within(actor, session, Duration::MAX).await
    }

    /// One hold step of at most `limit`; `false`, without waiting, when nothing holds the actor.
    async fn hold_step_within(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        limit: Duration,
    ) -> bool {
        let now = self.owner_now();
        let step = {
            let states = self.states.spell_states.lock().await;
            let attack = self.states.attack.lock().await;
            held_step(&states, &attack, actor, session, now, limit)
        };
        let Some(step) = step else {
            return false;
        };
        tokio::time::sleep(step).await;
        if self.ensure_source_map_initialized().await {
            self.drain_monster_melee().await;
        }
        true
    }

    /// The grace wait of a lost client (§3 item 6). While the actor is held in fight, the wait
    /// runs in hold steps that drive the melee pass, so a monster can still hit it and extend the
    /// hold when its connection was the Channel's last.
    async fn wait_grace(&self, actor: ExactActorRef, session: GameSessionId, wait: Duration) {
        let end = tokio::time::Instant::now() + wait;
        loop {
            let left = end.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return;
            }
            if !self.hold_step_within(actor, session, left).await {
                tokio::time::sleep(left).await;
                return;
            }
        }
    }

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
        // ATTACK0-RL-03: a closed client stays in the world until its in-fight deadline ends.
        while self.hold_while_in_fight(actor, session).await {}
        let Ok(token) = release.fence(&mut *self.runtime.lock().await) else {
            return GraceExpiryResult::Unknown;
        };
        let mut hold = FenceHold::new(session, token);
        let mut settled_death = None;
        let mut kills = None;
        let abandoned = matches!(release, TerminalRelease::Abandoned(_));
        let mut backoff = RECONCILE_BACKOFF;
        let mut next_backoff = || {
            let pause = backoff;
            backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
            pause
        };
        let mut attempt = 0;
        while retries_release(attempt, abandoned) {
            attempt = attempt.saturating_add(1);
            let step = self
                .release_attempt(
                    &admitted,
                    &store,
                    release,
                    &mut hold,
                    &mut settled_death,
                    &mut kills,
                )
                .await;
            match step {
                ReleaseAttempt::Done(result) => return result,
                ReleaseAttempt::Retry | ReleaseAttempt::Unknown => {
                    tokio::time::sleep(next_backoff()).await;
                }
            }
        }
        if matches!(release, TerminalRelease::CapabilityMismatch(_)) {
            // D449: an unproven mismatch release is reconciled from the durable row. TERMINAL
            // settles as released; a session still holding the lease gives back this release's
            // hold, and the epoch fence is lifted once no other release holds it, so the client's
            // retry can repeat the release.
            // Every attempt gave up its kill hold, so the kill mark is reconciled, not settled.
            return match self.settle_unended(&store, actor, &mut hold).await {
                UnendedSettle::Terminal => {
                    self.reconcile_kill_release(kills, ReleaseEnd::Committed)
                        .await;
                    let result = self
                        .retire_reconciled(controller.account_id, session, actor)
                        .await;
                    self.forget_kill_release(kills).await;
                    result
                }
                UnendedSettle::Lifted => {
                    self.reconcile_kill_release(kills, ReleaseEnd::Retryable)
                        .await;
                    GraceExpiryResult::Unknown
                }
                // The fence stays: the epoch's grace expiry, which joins the same token, takes
                // the hold over instead of counting it again.
                UnendedSettle::Unknown => {
                    self.states.fence_holders.keep(hold);
                    GraceExpiryResult::Unknown
                }
            };
        }
        GraceExpiryResult::Unknown
    }

    /// One attempt of a terminal release (`release_terminal`, `release_logout`): the death
    /// settle, the fence, the actor-end saves, the §1.3 kill seal, the durable transaction and
    /// its settle. `kills` keeps the attempt's kill mark for a later reconcile.
    async fn release_attempt(
        &self,
        admitted: &AdmittedSession,
        store: &FreshAdmissionStore,
        release: TerminalRelease,
        hold: &mut FenceHold,
        settled_death: &mut Option<actor_spell::PlayerDeath>,
        kills: &mut Option<KillReleaseMark>,
    ) -> ReleaseAttempt {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return ReleaseAttempt::Done(GraceExpiryResult::NotApplicable);
        };
        let session = admitted.game_session_id;
        let account_id = canonical_uuid(&controller.account_id);
        if !self
            .settle_released_death(actor, session, settled_death)
            .await
        {
            return ReleaseAttempt::Retry;
        }
        match self.fence_transition(actor, hold).await {
            FenceStep::Fenced => {}
            FenceStep::Wait => return ReleaseAttempt::Retry,
            FenceStep::Refused(_) => return ReleaseAttempt::Done(GraceExpiryResult::Unknown),
        }
        // A logout's fence is its own minted transition id: one per attempt.
        let logout = match (release, hold.token) {
            (TerminalRelease::Logout(transport), TransitionFence::Transition(attempt)) => {
                Some((transport, attempt))
            }
            (TerminalRelease::Logout(_), TransitionFence::GraceExpiry(_)) => {
                return ReleaseAttempt::Done(GraceExpiryResult::Unknown);
            }
            (TerminalRelease::Abandoned(_) | TerminalRelease::CapabilityMismatch(_), _) => None,
        };
        // SPELL-D8 §8.2 save point 1: the actor's monk values are durable, or fenced out,
        // before the release can end the Character lease.
        if self.save_monk_state(admitted, actor).await == monk_save::MonkSave::Unknown {
            return ReleaseAttempt::Retry;
        }
        if self.save_spell_training(actor, session, true).await
            == actor_spell::TrainingSave::Unknown
        {
            return ReleaseAttempt::Retry;
        }
        // KILL-REWARD-COMP-1 §1.3: the session's own kills settle before it ends.
        *kills = match self.seal_kill_rewards(session).await {
            KillRelease::Send(mark) => mark,
            KillRelease::Deferred => return ReleaseAttempt::Retry,
        };
        let kills = *kills;
        // The familiar's actor-end snapshot is the last save before the durable release, so no
        // retryable step can follow a committed logout snapshot and leave the session running
        // on it (#2127 Codex P1 4279636456).
        if self
            .save_familiar_at_actor_end(actor, session, logout)
            .await
            == actor_spell::FamiliarLogoutSave::Unknown
        {
            self.settle_kill_release(kills, ReleaseEnd::Retryable).await;
            return ReleaseAttempt::Retry;
        }
        // CHAR-POSITION-0 §3.2 "At logout": the release writes the fenced actor's cell.
        let releasing = self.final_position_store(store, session, actor).await;
        let outcome = match release {
            TerminalRelease::Abandoned(transport) => {
                releasing
                    .release_abandoned_session(session, &account_id, transport)
                    .await
            }
            TerminalRelease::CapabilityMismatch(epoch) => {
                releasing
                    .release_capability_mismatch(session, &account_id, epoch)
                    .await
            }
            TerminalRelease::Logout(transport) => {
                releasing
                    .release_logout_session(session, &account_id, transport)
                    .await
            }
        };
        if ends_session(&outcome) {
            // PREM-1b (D476): the session is over, whether abandoned, mismatched or logged out;
            // its Premium pulls stop, as at grace expiry.
            self.release_premium(controller.account_id, session);
        }
        match outcome {
            Ok(ExpiredLossReleaseV1::NotApplicable | ExpiredLossReleaseV1::NotExpired { .. }) => {
                match self.settle_unended(store, actor, hold).await {
                    UnendedSettle::Lifted => {
                        self.settle_kill_release(kills, ReleaseEnd::Retryable).await;
                        ReleaseAttempt::Done(GraceExpiryResult::NotApplicable)
                    }
                    UnendedSettle::Terminal => {
                        self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                        let result = self
                            .retire_reconciled(controller.account_id, session, actor)
                            .await;
                        self.forget_kill_release(kills).await;
                        ReleaseAttempt::Done(result)
                    }
                    UnendedSettle::Unknown => {
                        self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                        ReleaseAttempt::Unknown
                    }
                }
            }
            Ok(ExpiredLossReleaseV1::Released { .. } | ExpiredLossReleaseV1::Terminal) => {
                self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                let result = self.retire(session, actor).await;
                self.forget_kill_release(kills).await;
                ReleaseAttempt::Done(result)
            }
            // Unknown outcome: keep the fence and the kill mark; the retry reconciles from
            // the durable row.
            Err(_) => {
                self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                ReleaseAttempt::Unknown
            }
        }
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

    /// MAP-CUTOVER-1b: the map view of a bundle World, which its movement cells carry; `None`
    /// on a fixture World.
    fn bundle_map(&self) -> Option<&crate::map::boot::BundleMap> {
        match self.movement_cells.index() {
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) => {
                Some(index.map())
            }
            _ => None,
        }
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
            oteryn_gameplay_items::reach::within_reach(
                (actor_x, actor_y, actor_floor),
                (cell.x, cell.y, cell.z),
            )
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

    /// MAP-DOOR-1: one `USE_INTENT` on a door of a bundle World. Under the Channel-owner lock
    /// that `step` takes, so the actor and every other committed position cannot move between
    /// the occupancy read and the door's compare-and-swap.
    async fn use_bundle_door(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &crate::map::boot::BundleMap,
        target: &oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> UseOutcome {
        let mut runtime = self.runtime.lock().await;
        if self.states.spell_states.lock().await.is_dead(actor) {
            return UseOutcome::rejected();
        }
        // The USE's own GameSession must still be the actor's bound session with no control
        // loss, read under the same lock as the commit (a stale or superseded session acts on
        // nothing).
        if !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return UseOutcome::rejected();
        }
        // Admission is taken under the owner lock that also covers the commit below, and the
        // owner pass resets the turn cap under the same lock, so an act is counted in the turn
        // that commits it and its order is the owner lock's.
        let admitted = self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()));
        if !admitted {
            return UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Exhausted,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            };
        }
        let Ok(expected) = runtime.borrow_movement_position().read(actor) else {
            return UseOutcome::rejected();
        };
        let native = |position: crate::foundation::MovementLocalPosition| {
            Some(crate::map::overlay::TilePos {
                x: u16::try_from(position.x).ok()?,
                y: u16::try_from(position.y).ok()?,
                floor: i8::try_from(position.floor).ok()?.checked_neg()?,
            })
        };
        let Some(actor_tile) = native(expected.position()) else {
            return UseOutcome::rejected();
        };
        map_door::use_door(
            &map.overlay,
            &map.facts,
            runtime.content_pin().client_artifact_digest(),
            actor_tile,
            |door| map_door::doorway_occupied(&runtime, actor, actor_tile, door),
            target,
        )
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
            if self.states.spell_states.lock().await.is_dead(actor) {
                return UseOutcome::rejected();
            }
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
                    disposition: oteryn_protocol_oteryn::world_object::UseDisposition::TooFar,
                    committed: None,
                    earned: connection::EarnedNotice::NoneEarned,
                    gate_message_id: None,
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
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed => {
                    self.refresh_quest_session(command.game_session_id, false)
                        .await;
                }
                oteryn_protocol_oteryn::world_object::UseDisposition::Rejected => {
                    self.schedule_quest_refresh(command.game_session_id);
                }
                _ => {}
            }
        }
        UseOutcome {
            disposition,
            committed: None,
            earned: earned_notice(self.achievements, notice, &mut operator_event),
            gate_message_id: None,
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
    appearance_change::composed_methods!();
    guild_intent::composed_methods!();
    async fn observe_quest_log(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        since: Option<u64>,
    ) -> quest_log::QuestLogObservation {
        self.quest_log_observation(actor, session, since).await
    }

    fn world_weather(&self) -> Option<&crate::world_weather::WorldWeatherSource> {
        // WEATHER-1 (D1027 signed off, D1448): the shipped Oteryn-designed data, `None` (never
        // offered) when it is missing or invalid. The data is public and stateless: its binding
        // token is the content revision, so no per-World secret is read.
        crate::world_weather::shipped()
    }

    async fn chat_admit(
        &self,
        _actor: ExactActorRef,
        session: GameSessionId,
    ) -> oteryn_protocol_oteryn::chat::ChatRoomSet {
        self.chat_open(session).await
    }

    async fn chat_intent(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: oteryn_protocol_oteryn::chat::ChatIntent,
    ) -> oteryn_protocol_oteryn::chat::ChatIntentResult {
        self.chat_speak(actor, session, command_id, &intent).await
    }

    async fn chat_guild_admit(
        &self,
        admitted: AdmittedSession,
    ) -> oteryn_protocol_oteryn::chat::guild_room::GuildRoomState {
        self.guild_chat_admit(admitted).await
    }

    async fn chat_guild_intent(
        &self,
        admitted: AdmittedSession,
        text: String,
    ) -> oteryn_protocol_oteryn::chat::ChatIntentResult {
        self.guild_chat_send(admitted, text).await
    }

    async fn chat_next_bound(
        &self,
        admitted: AdmittedSession,
        remaining: &mut usize,
    ) -> Option<oteryn_protocol_oteryn::chat::ChatLine> {
        self.guild_chat_next(admitted, remaining).await
    }

    async fn chat_delivered(&self, session: GameSessionId) {
        self.states.chat.delivered(session);
    }

    async fn chat_rooms_changed(
        &self,
        session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::chat::ChatRoomSet> {
        self.states.chat.take_rooms_change(session)
    }

    async fn npc_session_open(&self, session: GameSessionId) {
        // A client without chat still has a conversation state, held with the chat session.
        if !self.states.chat.knows(session) {
            self.chat_open(session).await;
        }
        self.states.npc_conversations.cancel_bank(session);
        self.states.chat.set_npc_service(session);
    }

    async fn npc_follow(
        &self,
        _actor: ExactActorRef,
        session: GameSessionId,
        shown: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    ) -> Option<npc_command::NpcUpdate> {
        let runtime = self.runtime.lock().await;
        self.states
            .npc_conversations
            .follow(&self.states.chat, &runtime, session, shown)
    }

    async fn resume_npc_bank(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Option<npc_command::NpcTalkOutcome> {
        npc_command::resume_npc_talk(self, actor, command).await
    }

    async fn npc_talk(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::npc_service::NpcTalkIntent,
    ) -> npc_command::NpcTalkOutcome {
        self.vocation_talk(actor, command, intent).await
    }

    fn take_tutorial_notice(
        &self,
        session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice> {
        self.states.tutorial_sessions.take(session)
    }

    async fn npc_trade(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::npc_service::NpcTradeIntent,
    ) -> npc_trade_command::NpcTradeStep {
        npc_trade_command::npc_trade(self, actor, command, intent).await
    }

    /// MAP-CUTOVER-1b: a bundle World offers the Item view set plus capability 18.
    /// MAP-ITEM-REF-1: any other World offers capability 4 (once Part B sets its gate) only when
    /// the active generation pins a non-empty Item key set. ADMIT0: boot validated
    /// `required_capabilities` against this same offer; a bundle World's floor holds 18, so
    /// admission requires it through the authenticated check in `admit`, not here.
    fn offered_capabilities(&self) -> &'static [capabilities::OfferedCapability] {
        world_required_capabilities::offered_for_scope(
            self.bundle_map().is_some(),
            self.active_generation,
        )
    }

    /// MAP-CUTOVER-1b: domain 17 of a bundle World at the actor's current position. The position
    /// is read under the runtime lock `step` takes (§1.4); the window is composed and encoded
    /// after the lock is released.
    async fn observe_world_map(
        &self,
        actor: ExactActorRef,
        view: &mut world_map::SessionMapView,
        items: &mut item_view::SessionItemView,
    ) -> Option<Result<Option<world_map::MapUpdate>, world_map::MapViewError>> {
        bundle_map_position::observe(self, actor, view, items).await
    }

    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        let mut runtime = self.runtime.lock().await;
        let snapshot = runtime.borrow_movement_position().read(actor).ok()?;
        if snapshot.context() != runtime.pinned_movement_context() {
            return None;
        }
        Some(Self::observation(&runtime, snapshot.position()))
    }

    /// MAP-ITEM-REF-1: domain 9 from the durable backpack, each definition through the Channel
    /// content pin's Item definition index.
    async fn observe_character_inventory(
        &self,
        _actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<item_view::InventoryItems> {
        item_ref_admission::observe_character_inventory(self, game_session_id).await
    }

    async fn observe_character_equipment(
        &self,
        _actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<
        Vec<(
            oteryn_protocol_oteryn::item_view::EquipmentSlot,
            item_view::ViewItem,
        )>,
    > {
        equipment_move::observe_character_equipment(self, game_session_id).await
    }

    async fn committed_equipment_move(
        &self,
        _actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Option<item_move::EquipmentMoveReplay> {
        equipment_move::committed_equipment_move(self, command).await
    }

    async fn move_equipment(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        request: item_move::EquipmentMove,
    ) -> item_move::EquipmentMoveResult {
        equipment_move::move_equipment(self, actor, command, request).await
    }

    async fn depot(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::depot::DepotIntent,
        item: Option<item_view::ItemKey>,
    ) -> Option<oteryn_protocol_oteryn::depot::DepotReply> {
        depot::execute(self, actor, command, intent, item).await
    }

    async fn committed_loop_move(
        &self,
        _actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Option<item_move::LoopMoveReplay> {
        loop_move::committed_loop_move(self, command).await
    }

    async fn move_loop_item(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        request: item_move::LoopMove,
    ) -> item_move::LoopMoveResult {
        loop_move::move_loop_item(self, actor, command, request).await
    }

    /// BAGS-1: a container of the carried tree (capability 14).
    async fn observe_container(
        &self,
        _actor: ExactActorRef,
        fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
        target: item_view::ItemKey,
    ) -> Option<container_view::ContainerObservation> {
        loop_move::observe_container(self, fence, target).await
    }

    /// VIS-3: the Channel's players, live creatures and (NPC-LOAD-1) NPC actors, read in one
    /// owner work item, and (MAP-ITEM-REF-1 Part B) its bound corpses as objects in the same
    /// read.
    async fn observe_visible_entities(
        &self,
        actor: ExactActorRef,
    ) -> Option<world_spatial::ChannelEntities> {
        use world_spatial::VisibleKind;
        let runtime = self.runtime.lock().await;
        let visible = runtime.visible_entities();
        let entities = (visible
            .players
            .iter()
            .map(|entry| (VisibleKind::Player, entry)))
        .chain(
            visible
                .creatures
                .iter()
                .map(|entry| (VisibleKind::Creature, entry)),
        )
        .chain(visible.npcs.iter().map(|entry| (VisibleKind::Npc, entry)))
        .map(|(kind, entry)| world_spatial::ChannelEntity {
            kind,
            identity: entry.actor.placement_identity(),
            generation: entry.generation,
            position: ActorPosition {
                x: entry.position.x,
                y: entry.position.y,
                floor: entry.position.floor,
            },
            revision: entry.revision,
        })
        .chain(item_ref_admission::visible_corpses(
            &runtime,
            self.active_generation,
        ))
        .collect();
        // CLOCK-1 (§11.4): an actor's light is its brightest current LIGHT condition, from the
        // runtime actor-condition owner (players and creatures) and, for a player, its spell
        // state, read under the same runtime lock (lock order: runtime, then spell_states). A
        // store whose owner time is ahead of `now_us` gives no light. Equipped items and monster
        // content carry no light source yet, so they add none. The census includes players whose
        // control was lost: they stay visible during reconnect grace, so their light stays too.
        let now_us = self.owner_now().get();
        let mut lights = world_spatial::EntityLights::new();
        if let Ok(census) = runtime.positioned_melee_census() {
            let states = self.states.spell_states.lock().await;
            for (actor, _, session) in census {
                let spell = session
                    .and_then(|session| states.get(&runtime, actor, session))
                    .map(|state| state.owned_conditions())
                    .filter(|conditions| conditions.accepts_time(now_us))
                    .and_then(|conditions| conditions.light_at(now_us));
                let owned = runtime
                    .actor_conditions_at(actor, session, now_us)
                    .ok()
                    .and_then(|conditions| conditions.light_at(now_us));
                let brightest = match (spell, owned) {
                    (Some(a), Some(b)) => Some(if b.0 > a.0 { b } else { a }),
                    (a, b) => a.or(b),
                };
                if let Some((level, color)) = brightest
                    && level != 0
                {
                    lights.insert(
                        actor.placement_identity(),
                        world_spatial::EntityLight {
                            level,
                            color: color.unwrap_or(crate::world_clock::LIGHT_COLOR),
                        },
                    );
                }
            }
        }
        Some(world_spatial::ChannelEntities {
            content_generation: runtime.content_pin().client_artifact_digest(),
            observer: actor.placement_identity(),
            entities,
            lights,
            observed_at_ms: now_us / 1_000,
        })
    }

    /// The Channel's current door `WORLD_OBJECT_OVERLAY` (USE-WIRE-V1, #162 5868482467), for
    /// the join/resync snapshot. Channel-global, unlike `observe`: no actor is involved.
    async fn observe_world_object_overlay(
        &self,
    ) -> Option<oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry> {
        if self
            .rooms
            .gate_source_world()
            .is_some_and(|room| room.source_world().is_some())
            || self.bundle_map().is_some()
        {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let door = self.door.lock().await;
        Some(
            oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry {
                content_generation: runtime.content_pin().client_artifact_digest(),
                placement: door.placement_key().as_str().as_bytes().to_vec(),
                state: door.state_key().as_str().as_bytes().to_vec(),
                revision: door.revision(),
            },
        )
    }

    /// One Channel-owner work item for one actor (`MOVE-RL-02` = 1): the pinned context, one
    /// direct lookup in the active generation's qualified cells (`MOVE-RL-03` = 1) and the
    /// owner's compare-commit. A blocked or out-of-room destination is `Blocked`; any stale,
    /// unpositioned or mismatched binding is `Rejected`. Nothing moves unless the step commits.
    ///
    /// SPEED-1: the step is paced by the step duration onto its destination (CONDITIONS-0 §4.2):
    /// the player's effective speed, with the actor's active `SPEED` condition delta for
    /// `session` at the owner's time, and the destination's ground speed from the §1.11 seam.
    /// A destination whose duration cannot be computed is refused before anything moves.
    async fn paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<Duration>) {
        self.tutorial_paced_step(actor, session, direction).await
    }

    /// CHASE-1 (RANGED-0 §8): the owner's chase step (`attack::chase_step_now`).
    async fn chase_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(StepOutcome, Option<Duration>)> {
        self.newhaven_chase_step(actor, session).await
    }

    async fn yield_chase_to_client(&self, actor: ExactActorRef, session: GameSessionId) {
        self.chase_yield_to_client(actor, session).await;
    }

    async fn chase_search_waiting(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.chase_search_waiting_now(actor, session).await
    }

    async fn actor_step_deadline(&self, actor: ExactActorRef) -> Option<tokio::time::Instant> {
        self.actor_step_deadline_now(actor).await
    }

    async fn begin_actor_step(
        &self,
        actor: ExactActorRef,
        holder: &std::sync::Arc<()>,
    ) -> Option<tokio::time::Instant> {
        self.begin_actor_step_now(actor, holder).await
    }

    async fn record_actor_step_deadline(
        &self,
        actor: ExactActorRef,
        holder: &std::sync::Arc<()>,
        due: Option<tokio::time::Instant>,
    ) {
        self.record_actor_step_deadline_now(actor, holder, due)
            .await;
    }

    /// ITEM-USE-1c: one `USE` of a main backpack entry under capability 15.
    async fn use_item(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        item: item_view::ItemKey,
    ) -> UseOutcome {
        self.use_backpack_item(actor, command, item).await
    }

    async fn use_pick(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::world_object::UseIntent,
        source: Option<item_view::ItemKey>,
        target: Option<item_view::ItemKey>,
    ) -> Option<UseOutcome> {
        self.use_pick_impl(actor, command, intent, source, target)
            .await
    }

    async fn use_item_by_definition(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        definition: std::num::NonZeroU32,
    ) -> UseOutcome {
        self.use_item_definition(actor, command, definition).await
    }

    async fn use_rune(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::world_object::UseIntent,
        key: Option<item_view::ItemKey>,
        visible: bool,
    ) -> Option<UseOutcome> {
        self.use_legacy_rune(actor, command, intent, key, visible)
            .await
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
        target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> UseOutcome {
        if self
            .rooms
            .gate_source_world()
            .is_some_and(|room| room.source_world().is_some())
        {
            return UseOutcome::rejected();
        }
        if let Some(map) = self.bundle_map() {
            return self
                .use_bundle_door(actor, command.game_session_id, map, &target)
                .await;
        }
        if let Some(chest) = Self::chest_target(self.chest, &target.placement) {
            return self.use_chest(actor, command, chest).await;
        }
        let mut runtime = self.runtime.lock().await;
        // DEATH-2 §4.2: a dead player takes no input until its respawn.
        if self.states.spell_states.lock().await.is_dead(actor) {
            return UseOutcome::rejected();
        }
        let Ok(expected) = runtime.borrow_movement_position().read(actor) else {
            return UseOutcome::rejected();
        };
        let position = expected.position();
        let mut door = self.door.lock().await;
        if target.placement.as_slice() != door.placement_key().as_str().as_bytes() {
            return UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            };
        }
        let actor_floor = i32::from(position.floor);
        if !Self::use_object_reachable(position.x, position.y, actor_floor, door.collision_cells())
        {
            return UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::TooFar,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
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
                    disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
                    committed: Some(
                        oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry {
                            content_generation,
                            placement: door.placement_key().as_str().as_bytes().to_vec(),
                            state: state.as_str().as_bytes().to_vec(),
                            revision,
                        },
                    ),
                    earned: connection::EarnedNotice::NoneEarned,
                    gate_message_id: None,
                }
            }
            Ok(crate::world_runtime::LocalObjectUseOutcome::NothingToUse) => UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            },
            Ok(crate::world_runtime::LocalObjectUseOutcome::Occupied) => UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Occupied,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            },
            Ok(crate::world_runtime::LocalObjectUseOutcome::StaleState) => UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::StaleState,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
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
        let states = self.states.spell_states.lock().await;
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
        if let Some(outcome) = self
            .cast_durable_stance(actor, game_session_id, command_id, &intent)
            .await
        {
            return outcome;
        }
        let now = self.owner_now();
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
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

    async fn cast_spell_completion(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: actor_spell::SpellCastIntent,
    ) -> Option<actor_spell::SpellCastOutcome> {
        let outcome = self
            .cast_spell_dispatch(actor, game_session_id, command_id, intent)
            .await;
        if outcome.is_some() {
            self.settle_pacified_target(actor, game_session_id).await;
        }
        outcome
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

    async fn highscores(
        &self,
        world: WorldId,
        session: GameSessionId,
        query: oteryn_protocol_oteryn::highscores::HighscoresQuery,
    ) -> highscores::HighscoresReply {
        highscores::answer_session_highscores(self.root, self.world_id, world, session, query).await
    }

    /// The periodic 1000 ms Serene evaluation of the admitted actor (SPELL-D8 §8.2), under the
    /// same runtime lock as a cast. DEATH-2: a dead actor is respawned on this cadence instead
    /// ([`Self::respawn_after_death`]), and its respawn vitals are the published delta.
    async fn tick_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        if !self.ensure_source_map_initialized().await {
            return None;
        }
        self.drain_monster_melee().await;
        self.refresh_attack_weapon(actor, game_session_id).await;
        self.drain_auto_attacks().await;
        let _ = self.drain_kill_rewards(game_session_id).await;
        self.drain_source_item_deadlines().await;
        self.drain_source_party_deadlines_bounded().await;
        // DEATH-2 §4.5: a dead player's cadence tick settles its death and respawns it.
        let death = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            states.player_death(&runtime, actor, game_session_id)
        };
        if let Some(death) = death {
            // #2199 Codex P1 4303261095 (FAMILIARS0 §7): the owner's death ends its live
            // familiar (remaining 0) through the fenced lifecycle writer before the respawn; a
            // failed write keeps the actor dead and the familiar present for the next tick.
            if self
                .end_familiars_at_owner_death(actor, game_session_id, death.occurrence)
                .await
                .is_err()
            {
                return None;
            }
            return self
                .respawn_after_death(actor, game_session_id, death)
                .await;
        }
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
        {
            let _ = self
                .refresh_source_party_presence(actor, game_session_id)
                .await;
        }
        let _ = self
            .refresh_and_apply_spell_access(actor, game_session_id)
            .await;
        let party_vitals = self
            .refresh_source_party_serene(actor, game_session_id)
            .await;
        let result = if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
        {
            self.tick_spell_periodic(actor, game_session_id).await
        } else {
            let now = self.owner_now();
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            states.tick_in_environment(
                &mut runtime,
                self.movement_cells,
                actor,
                game_session_id,
                now,
            )
        };
        let _ = self
            .save_spell_training(actor, game_session_id, false)
            .await;
        let cleanup_access = self.refresh_spell_access(actor, game_session_id).await;
        let _ = self
            .drain_familiar_deaths(actor, game_session_id, &cleanup_access)
            .await;
        // #2199 Codex P2 4302041461: a familiar Login stored for lack of a free tile returns
        // on this cadence once a tile frees.
        let _ = self
            .retry_waiting_familiar_return(actor, game_session_id, &cleanup_access)
            .await;
        let _ = self.tick_familiar_defenses(actor, game_session_id).await;
        let _ = self.drain_native_ai_timers().await;
        result.or(party_vitals)
    }

    /// ATTACK-1b: one `ATTACK_TARGET_INTENT` as one Channel-owner work item, under the runtime
    /// lock; a dead or unbound actor takes no target.
    async fn attack_target(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    ) -> oteryn_protocol_oteryn::attack::AttackIntentDisposition {
        let now = self.owner_now();
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let Some(state) = states.get(&runtime, actor, game_session_id) else {
            return oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected;
        };
        // CONDITIONS-0 §3.5 / ATTACK-0 §3: a `pacified` held in the player store or the runtime
        // slot refuses a new target first; the typed gate stays in the operator log (§7) and
        // `set_target` rechecks the slot. Clearing (`None`) stays open.
        if target.is_some() {
            let gate = state
                .owned_conditions()
                .attack_target_gate(now.get())
                .err()
                .or_else(|| {
                    runtime
                        .actor_conditions(actor, Some(game_session_id))
                        .ok()
                        .and_then(|store| store.attack_target_gate(now.get()).err())
                });
            if let Some(gate) = gate {
                operator_event(&format!(
                    "event=attack_target_condition_gated gate={}",
                    gate.key()
                ));
                return oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected;
            }
            if states.is_dead(actor) {
                return oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected;
            }
        }
        self.states.attack.lock().await.set_target(
            &runtime,
            self.rooms.gate_attack_target(),
            actor,
            game_session_id,
            now,
            target,
            command_id,
        )
    }

    async fn fight_modes(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        modes: oteryn_protocol_oteryn::attack::FightModes,
    ) -> oteryn_protocol_oteryn::attack::AttackIntentDisposition {
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        if states.get(&runtime, actor, game_session_id).is_some()
            && self
                .states
                .attack
                .lock()
                .await
                .set_modes(&runtime, actor, game_session_id, modes)
        {
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        } else {
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected
        }
    }

    async fn observe_combat_state(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::attack::ActorCombatState> {
        let now = self.owner_now();
        // ATTACK-1b lock order: runtime, spell states, attack.
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let mut attack = self.states.attack.lock().await;
        attack.settle_pacified_target(&runtime, &states, actor, game_session_id);
        Some(attack.combat_state(actor, game_session_id, now))
    }

    /// ATTACK-1b: every 250 ms while a session has domain 10, so a 2000 ms swing is late by at
    /// most one refresh; the Serene cadence drains as well.
    async fn tick_combat(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::attack::ActorCombatState> {
        if !self.ensure_source_map_initialized().await {
            return None;
        }
        self.refresh_attack_weapon(actor, game_session_id).await;
        self.drain_auto_attacks().await;
        let _ = self.drain_kill_rewards(game_session_id).await;
        self.observe_combat_state(actor, game_session_id).await
    }

    async fn lose_control(
        &self,
        admitted: AdmittedSession,
        wait: Duration,
        reset: Option<&crate::durability::world_reset::RetiringWorldReset>,
    ) -> ControlLossResult {
        self.lose_control_for_cause(admitted, wait, reset).await
    }

    async fn enter_input(&self) -> Option<input_gate::InputPermit<'_>> {
        self.states.input_gate.enter().await
    }

    async fn resume(
        &self,
        attempt: connection::ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        // OPS-SAVE-2: no session begins while the node quiesces for its shutdown save.
        let Some(_permit) = self.states.input_gate.enter().await else {
            return Err(AdmissionRefusal::Unavailable);
        };
        let admitted = self.resume_lost(attempt).await?;
        // ATTACK-0 §3 Reconnect: only the target is cleared; the in-fight deadline is kept.
        if let Some(actor) = admitted.runtime_actor {
            self.states
                .attack
                .lock()
                .await
                .clear_target(actor, admitted.game_session_id);
        }
        // PREM-1b: a reconnect pulls Premium again before any Premium read, without waiting.
        if let Some(controller) = admitted.controller {
            self.admit_premium(controller.account_id, admitted.game_session_id);
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

    async fn reconcile_terminal(&self, admitted: AdmittedSession) -> GraceExpiryResult {
        self.reconcile_terminal_session(admitted).await
    }

    async fn logout(&self, admitted: AdmittedSession) -> logout::LogoutReply {
        self.logout_session(admitted).await
    }

    async fn expire_control_loss(&self, admitted: AdmittedSession) -> GraceExpiryResult {
        let result = self.release_after_grace(admitted).await;
        // Resumed or released: this lost connection can no longer be resumed. An unknown result
        // keeps it; the lifecycle runs the expiry again.
        if result != GraceExpiryResult::Unknown {
            self.forget_lost(
                admitted.game_session_id,
                admitted.continuity.connection_generation,
            );
        }
        result
    }

    async fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        use AdmissionRefusal::{Classified, Rejected, Unavailable};
        use FoundationProtocolError as N8;
        // OPS-SAVE-2: no session begins while the node quiesces for its shutdown save.
        let Some(_permit) = self.states.input_gate.enter().await else {
            return Err(Unavailable);
        };
        let malformed = Classified(N8::AdmissionGrantMalformed);
        let token = std::str::from_utf8(attempt.admission_material).map_err(|_| malformed)?;
        // Untrusted selector only; verification below checks the same token.
        let signing_key_id = fresh_grant_signing_key_id(token).ok_or(malformed)?;
        let character_id = domain::CharacterId::from_bytes(*attempt.character_id.as_bytes())
            .map_err(|_| malformed)?;
        // Every durable pass shares the single, non-blocking durability holder, so a
        // concurrent pass (another attempt, or the node's background work) fails a
        // pass at once with nothing written. Such a busy holder is retried, bounded,
        // with a per-attempt backoff so concurrent attempts do not stay in lockstep.
        let backoff = fresh_admission_backoff(&attempt.game_session_id);
        let retry = move || async move {
            tokio::time::sleep(backoff).await;
        };
        let mut read_round = 0;
        let record = loop {
            read_round += 1;
            match self
                .root
                .read_current_character(self.character, character_id)
                .await
            {
                Ok(record) => break record,
                Err(CharacterAuthorityError::Unavailable(_))
                    if fresh_round_retries(Unavailable, read_round >= FRESH_ADMISSION_ROUNDS) =>
                {
                    retry().await;
                }
                Err(CharacterAuthorityError::Unavailable(_)) => return Err(Unavailable),
                Err(_) => return Err(Rejected),
            }
        };
        // The grant is not authenticated yet: a classified answer here would reveal another
        // world's character, so only 1100/1101 may precede authentication (N8 precedence).
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
            .map_err(fresh_grant_refusal)?;
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
            // ADMIT0 §4 / FND-04A §7 step 14: only now, after grant authentication and the
            // safe ownership/world classification, may a missing requirement be revealed.
            // The binding is this composition's independently resolved observation; the
            // commit below rechecks it under the deciding transaction's locks. Refusing here
            // precedes capacity reservation and the durable commit: no nonce is consumed.
            let (scope, generation, policy) = composition.runtime_capability_binding();
            if !self
                .required_capabilities
                .matches_current(scope, generation, policy)
            {
                return Err(Rejected);
            }
            let capabilities = self
                .required_capabilities
                .authenticate_support(attempt.supported_capabilities)
                .map_err(|error| match error {
                    world_required_capabilities::SessionCapabilityError::MissingSupport(_) => {
                        Classified(N8::AdmissionCapabilityRequired)
                    }
                    _ => Unavailable,
                })?;
            let mut flow = FreshAdmissionDurabilityFlowV1::begin(authorization, transition)
                .map_err(|_| Rejected)?;
            let mut prepared = PreparedRequest(None);
            flow.submit(&mut prepared).map_err(|_| Rejected)?;
            let request = prepared.0.ok_or(Unavailable)?;
            // Capacity is reserved only after every semantic/authentication check,
            // but before the durable GameSession mutation. M+1 therefore rejects
            // without creating a playable/committed session.
            // A busy holder on the assignment read reserves nothing: retry the round.
            let reservation = match self.reserve_runtime_player(attempt.game_session_id).await {
                Ok(reservation) => reservation,
                Err(refusal) if fresh_round_retries(refusal, last) => {
                    retry().await;
                    continue;
                }
                Err(refusal) => return Err(refusal),
            };
            match self
                .root
                .commit_composed_fresh_admission(
                    self.character,
                    self.holder,
                    &composition,
                    &request,
                    capabilities.current_check(),
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
                // N8: proven noncommits FND-04A names a row for.
                Ok(
                    outcome @ (FreshAdmissionDurableOutcomeV1::RejectedReplayConflict
                    | FreshAdmissionDurableOutcomeV1::RejectedIncumbent
                    | FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority),
                ) => {
                    self.rollback_runtime_player(reservation).await?;
                    return Err(Classified(match outcome {
                        FreshAdmissionDurableOutcomeV1::RejectedReplayConflict => {
                            N8::AdmissionGrantReplayed
                        }
                        FreshAdmissionDurableOutcomeV1::RejectedIncumbent => {
                            N8::AdmissionIncumbentProtected
                        }
                        _ => N8::AdmissionGrantRuntimeGenerationStale,
                    }));
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
                            // No receipt under the admission locks proves noncommit (for
                            // example a commit refused on a busy holder): retry the round;
                            // the grant replay key still admits at most one GameSession.
                            if fresh_round_retries(refusal, last) {
                                retry().await;
                                continue;
                            }
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
        // Every refusal before the Channel-owner write is a read of current authority or data
        // through the single ready-only holder, which a concurrent pass (another session's
        // release or tick) can briefly hold; the attempt wrote no runtime state, so it is
        // repeated from fresh reads, bounded, before the actor is left unpositioned.
        let mut attempt_round = 0;
        let (first_entry, item_fence, decided) = loop {
            attempt_round += 1;
            let result = self
                .initialize_first_entry(&request, attempt.game_session_id, attempt.transport, actor)
                .await;
            if !matches!(
                result.0,
                FirstEntryOutcome::RefusedUnavailable | FirstEntryOutcome::RefusedStaleAuthority
            ) || attempt_round >= RECONCILE_ATTEMPTS
            {
                break result;
            }
            tokio::time::sleep(RECONCILE_BACKOFF).await;
        };
        #[cfg(test)]
        if std::env::var_os("OTERYN_SEAM_SPELL_MANIFEST").is_some() {
            eprintln!("SEAM_EVIDENCE spell_first_entry outcome={first_entry:?}");
        }
        // PREM-1b: pull Premium before any Premium read; admission does not wait on it.
        self.admit_premium(*record.account_id.as_bytes(), attempt.game_session_id);
        self.admit_promotion(attempt.game_session_id).await;
        let mut admitted = AdmittedSession {
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
        // ARCH-PROGRESSION-SOURCE-0 §1.5: before the respawn settle and the quest session, so
        // before the session is input-eligible or writes anything that advances the root.
        let positioned = matches!(
            admitted.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        );
        if positioned {
            self.admit_progression_session(&mut admitted).await;
        }
        // DEATH-2b: a pending respawn that is not placed and consumed leaves the actor
        // input-ineligible, its pending row intact for the next admission.
        if matches!(
            admitted.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ) {
            let settled = self
                .consume_admitted_newhaven_arrival(
                    admitted.game_session_id,
                    actor,
                    decided.as_ref(),
                )
                .await;
            admitted.first_entry = respawned_first_entry(admitted.first_entry, settled);
        }
        if writes_after_progression(positioned, admitted.first_entry) {
            self.admit_quest_session(&admitted).await;
        }
        Ok(admitted)
    }

    /// ITEM-MOVE-1 replay first: the receipt of this command 9's CommandRef in
    /// `game_item_transfer_receipts`, read under the cause lock. Without an item fence nothing can
    /// have committed for the session.
    async fn committed_item_move(
        &self,
        _actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Result<
        Option<crate::durability::item_transfer::CommittedItemTransfer>,
        crate::durability::item_transfer::ItemTransferError,
    > {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let command_id = crate::foundation::CommandId::new(command.command_id)
            .map_err(|_| crate::durability::item_transfer::ItemTransferError::InvalidInput)?;
        self.root
            .read_item_transfer_receipt(
                self.character,
                fence.character_id,
                crate::foundation::CommandRef::new(command.game_session_id, command_id),
            )
            .await
    }

    /// LOOK-2: examine a freshly observed item admitted by the session's handle table.
    async fn look_item(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        fence: Option<CurrentCharacterItemFence>,
        views: &item_view::SessionItemView,
        target: ItemHandle,
    ) -> ItemLookResult {
        item_look::look_item(self, actor, game_session_id, fence, views, target).await
    }

    /// LOOK-2: examine a current visible actor incarnation.
    async fn look_creature(
        &self,
        actor: ExactActorRef,
        _game_session_id: GameSessionId,
        target: EntityRef,
    ) -> ItemLookResult {
        item_look::look_creature(self, actor, target).await
    }

    /// MAP-ITEM-REF-1 Part B: a bound corpse's durable contents for `USE` (domain 11).
    async fn observe_item_target(
        &self,
        actor: ExactActorRef,
        target: item_view::ItemKey,
    ) -> Option<item_view::ItemTargetObservation> {
        item_ref_admission::observe_item_target(self, actor, target).await
    }

    /// MAP-ITEM-REF-1 Part B: command 9's corpse-entry TRANSFER into the main backpack.
    async fn take_corpse_entry(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        corpse: item_view::ItemKey,
        entry: item_view::ItemKey,
    ) -> Result<
        crate::durability::item_transfer::ItemTransferOutcome,
        crate::combat_pickup::GroundPickupError,
    > {
        item_ref_admission::take_corpse_entry(self, actor, command, corpse, entry).await
    }
}

/// Same-session grace from the authoritative `ControlLossEpoch` boundary: registry row
/// `FND04B-SAME-SESSION-GRACE-S` (`DISCONNECT-PROTECTION-V1` §4, provisional until measured).
pub const SAME_SESSION_GRACE_SECONDS: i64 = 60;

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

/// Whether a fresh-admission step that ended on a proven noncommit is retried. Only an
/// unavailable outcome (such as a busy durability holder) is transient, and only while
/// rounds remain; a rejection or a classified refusal is final.
fn fresh_round_retries(refusal: AdmissionRefusal, last: bool) -> bool {
    !last && refusal == AdmissionRefusal::Unavailable
}

/// The fresh-admission retry backoff: the base plus up to 63 ms taken from the random
/// tail of the attempt's UUIDv7 GameSession id, so concurrent attempts spread out.
fn fresh_admission_backoff(game_session_id: &GameSessionId) -> Duration {
    let jitter = u64::from(game_session_id.as_bytes()[15] & 0x3f);
    RECONCILE_BACKOFF + Duration::from_millis(jitter)
}
/// Bound on grace-expiry release attempts (waits for the deadline and store
/// retries): with the capped backoff, about two minutes of owner unavailability.
const EXPIRY_ATTEMPTS: u32 = 32;

/// Whether a terminal release makes another attempt. KILL-REWARD-COMP-1 §1.3: an abandoned
/// release runs on behind the backoff, unbounded in count, until it ends, since its callers
/// cannot retry it: a kill drain deferred on every attempt, or a kill mark an unknown outcome
/// kept, never strands the session ACTIVE on its dead transport with its fence held.
const fn retries_release(attempt: u32, abandoned: bool) -> bool {
    attempt < EXPIRY_ATTEMPTS || abandoned
}

/// Cap of the grace-expiry store retry backoff.
const EXPIRY_MAX_BACKOFF: Duration = Duration::from_secs(5);
/// Whole-second durable clock: wait just past the deadline second.
const EXPIRY_SLACK: Duration = Duration::from_millis(1100);

/// N8 (ARCH-LOGIN-FIRST-PACKETS-V1 §1.2): each fresh-grant verifier refusal carries its FND-04A
/// row. A recovery error cannot come from the fresh verifier; it stays frameless.
fn fresh_grant_refusal(error: Fnd04ConsumerError) -> AdmissionRefusal {
    use Fnd04ConsumerError as F;
    use FoundationProtocolError as N8;
    AdmissionRefusal::Classified(match error {
        F::FreshMalformed => N8::AdmissionGrantMalformed,
        F::FreshAuthenticationFailed => N8::AdmissionGrantAuthenticationFailed,
        F::FreshBindingMismatch => N8::AdmissionGrantBindingMismatch,
        F::FreshRevisionUnsupported => N8::AdmissionGrantRevisionUnsupported,
        F::FreshNotYetValid => N8::AdmissionGrantNotYetValid,
        F::FreshExpired => N8::AdmissionGrantExpired,
        F::FreshSecurityEvidenceStale => N8::AdmissionGrantSecurityEvidenceStale,
        F::FreshSecurityStateRevoked => N8::AdmissionGrantSecurityStateRevoked,
        F::FreshAccountCharacterConflict => N8::AdmissionAccountCharacterConflict,
        F::FreshWorldStale => N8::AdmissionGrantWorldStale,
        F::FreshRouteStale => N8::AdmissionGrantRouteStale,
        F::FreshRuntimeStale => N8::AdmissionGrantRuntimeGenerationStale,
        F::RecoveryMalformed
        | F::RecoveryAuthenticationFailed
        | F::RecoveryBindingMismatch
        | F::RecoveryRevisionUnsupported
        | F::RecoveryNotYetValid
        | F::RecoveryExpired
        | F::RecoverySecurityEvidenceStale
        | F::RecoverySecurityStateRevoked
        | F::RecoveryWorldStale => return AdmissionRefusal::Rejected,
    })
}

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
    async fn cast_spell_dispatch(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: actor_spell::SpellCastIntent,
    ) -> Option<actor_spell::SpellCastOutcome> {
        // CHAT-0 §6: a muted character cannot cast any spell.
        if let Some(outcome) = self
            .states
            .chat
            .muted_cast(game_session_id, self.owner_now())
        {
            return Some(outcome);
        }
        // CONDITIONS-0 §3.5: the condition gates answer next, ahead of every dispatcher.
        let gated = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            actor_spell::condition_gated_cast(
                &runtime,
                &states,
                self.spells,
                actor,
                game_session_id,
                &intent,
                self.owner_now(),
            )
        };
        if let Some(gate) = gated {
            // §7: the typed gate stays in the operator log; the wire answer is `REJECTED`.
            operator_event(&format!(
                "event=spell_cast_condition_gated gate={}",
                gate.key()
            ));
            return Some(actor_spell::SpellCastOutcome::rejected());
        }
        if !self.ensure_source_map_initialized().await {
            return None;
        }
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
        {
            let _ = self
                .refresh_source_party_presence(actor, game_session_id)
                .await;
        }
        // This explicit unavailable producer grants no commercial, Wheel or
        // control benefit. A configured authenticated owner replaces this port.
        let access = self
            .refresh_and_apply_spell_access(actor, game_session_id)
            .await;
        match self
            .cast_native_familiar(actor, game_session_id, command_id, &intent, &access)
            .await
        {
            actor_spell::NativeCastDispatch::Outcome(outcome) => return Some(outcome),
            actor_spell::NativeCastDispatch::Pending => return None,
            actor_spell::NativeCastDispatch::RuneRefused(_) => {
                return Some(actor_spell::SpellCastOutcome::rejected());
            }
            actor_spell::NativeCastDispatch::NotApplicable => (),
        }
        match self
            .cast_world_items(actor, game_session_id, command_id, &intent, &access)
            .await
        {
            actor_spell::NativeCastDispatch::Outcome(outcome) => return Some(outcome),
            actor_spell::NativeCastDispatch::Pending => return None,
            actor_spell::NativeCastDispatch::RuneRefused(_) => {
                return Some(actor_spell::SpellCastOutcome::rejected());
            }
            actor_spell::NativeCastDispatch::NotApplicable => (),
        }
        match self
            .cast_native_combat(actor, game_session_id, command_id, &intent, &access)
            .await
        {
            actor_spell::NativeCastDispatch::Outcome(outcome) => {
                let _ = self.drain_kill_rewards(game_session_id).await;
                self.party_spell_feedback(game_session_id, &intent, &outcome);
                Some(outcome)
            }
            actor_spell::NativeCastDispatch::Pending => None,
            actor_spell::NativeCastDispatch::RuneRefused(_) => {
                Some(actor_spell::SpellCastOutcome::rejected())
            }
            actor_spell::NativeCastDispatch::NotApplicable => {
                let outcome = self
                    .cast_spell(actor, game_session_id, command_id, intent)
                    .await;
                if self
                    .states
                    .spell_states
                    .lock()
                    .await
                    .has_pending_spell_commit(actor, game_session_id)
                {
                    None
                } else {
                    Some(outcome)
                }
            }
        }
    }

    /// ATTACK-0 §4 (as amended by #2106): a cast that applied `pacified` to its caster clears
    /// the caster's held target before the intent's completion is answered.
    async fn settle_pacified_target(&self, actor: ExactActorRef, game_session_id: GameSessionId) {
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        self.states.attack.lock().await.settle_pacified_target(
            &runtime,
            &states,
            actor,
            game_session_id,
        );
    }

    /// Record authoritative unexpected control loss, fresh-origin or after a resume. The owning
    /// source is composed at decision time from the Channel owner (actor present, placement,
    /// assignment revision), the current durable GameSession and the admitted account; the
    /// durable commit revalidates session, claims and runtime guard under its relation locks.
    async fn reserve_runtime_player(
        &self,
        game_session_id: GameSessionId,
    ) -> Result<PlayerActorReservation, AdmissionRefusal> {
        use AdmissionRefusal::{Classified, Unavailable};
        self.reserve_precondition().await?;
        self.runtime
            .lock()
            .await
            .reserve_fresh_session(game_session_id)
            .map_err(|error| match error {
                CarrierError::CapacityExceeded => {
                    Classified(FoundationProtocolError::AdmissionCapacityExceeded)
                }
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
    ) -> (
        FirstEntryOutcome,
        Option<CurrentCharacterItemFence>,
        Option<last_position::AdmissionPosition>,
    ) {
        #[cfg(test)]
        let stage = |name: &str| {
            if std::env::var_os("OTERYN_SEAM_SPELL_MANIFEST").is_some() {
                eprintln!("SEAM_EVIDENCE spell_first_entry stage={name}");
            }
        };
        #[cfg(test)]
        stage("resolve_session");
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let current = match store.reconcile(request.operation()).await {
            Ok(FreshReconciliation::Committed(snapshot)) => snapshot.current_session,
            Ok(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        match self.reserve_precondition().await {
            Ok(()) => {}
            Err(AdmissionRefusal::Unavailable) => {
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
            Err(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
        }
        // SPELL-D8 §8.2: a new runtime actor loads the durable Harmony and remaining forced
        // Serene time, read before the Channel-owner lock; a failed or corrupt load fails closed.
        let character_id =
            match domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes()) {
                Ok(character_id) => character_id,
                Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
            };
        let (facts, character_revision) = match spell_character_facts::load_character_cast_facts(
            self.root,
            self.character,
            character_id,
        )
        .await
        {
            spell_character_facts::CastFactsLoad::Ready {
                facts,
                character_revision,
            }
            | spell_character_facts::CastFactsLoad::NoVocation {
                facts,
                character_revision,
            } => (facts, character_revision),
            spell_character_facts::CastFactsLoad::Unavailable => {
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
        };
        #[cfg(test)]
        stage("character_facts_loaded");
        let Some(monk) = self.load_monk_state(character_id).await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        let build = match self
            .root
            .read_character_build_state(self.character, character_id)
            .await
        {
            Ok(build) => build,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        let stance = match self
            .root
            .read_character_stance(self.character, character_id)
            .await
        {
            Ok(stance) => stance,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        if let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        {
            let Some(fence) = item_fence_of(current) else {
                return (FirstEntryOutcome::RefusedStaleAuthority, None, None);
            };
            #[cfg(test)]
            stage("equipment_initialize");
            if let Err(_error) = self
                .root
                .initialize_admission_equipment(
                    self.character,
                    self.holder,
                    &fence,
                    native.source_digest(),
                    request.operation(),
                )
                .await
            {
                #[cfg(test)]
                stage(&format!("equipment_refused:{_error:?}"));
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
            // STARTER-1 (CP D1438): the starter grant of the Character's first entry, once per
            // Character and template; a replay writes nothing.
            if self
                .root
                .grant_starter_items(
                    self.character,
                    self.holder,
                    &fence,
                    &item_ref_admission::NativeItemWeights(native),
                    &crate::durability::starter_grant::StarterTemplate::main(),
                )
                .await
                .is_err()
            {
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
            let gameplay =
                crate::durability::character_progression::CurrentCharacterGameplayFence {
                    character_id,
                    game_session_id,
                    connection_generation: current.current_connection_generation(),
                    character_lease_generation: current.current_character_lease().generation(),
                    runtime_scope: current.current_runtime_scope(),
                    scope_ownership_generation: current.current_scope_generation(),
                    expected_character_revision: character_revision,
                };
            #[cfg(test)]
            stage("familiar_initialize");
            if let Err(_error) = self
                .root
                .initialize_familiar_group(self.character, self.holder, gameplay)
                .await
            {
                #[cfg(test)]
                stage(&format!("familiar_refused:{_error:?}"));
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
        }
        // EQUIP-0 §4 / SPELL-LOCK-2: acquire the lane before final authority revalidation.
        let Some(permit) = self.spell_lane_permit().await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        // Character facts, monk and stance loads are data reads, never current session authority.
        // Reject a changed root and resolve session, guard and assignment again after them.
        let Ok(latest_character) = self
            .root
            .read_current_character(self.character, character_id)
            .await
        else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        if latest_character.revision != character_revision {
            return (FirstEntryOutcome::RefusedStaleAuthority, None, None);
        }
        let current = match store.reconcile(request.operation()).await {
            Ok(FreshReconciliation::Committed(snapshot)) => snapshot.current_session,
            Ok(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        if current.commit().character_id().as_bytes() != character_id.as_bytes() {
            return (FirstEntryOutcome::RefusedStaleAuthority, None, None);
        }
        let key = AdmissionAuthorityGuardKeyV1::Character(current.commit().character_id());
        let character = match AdmissionGuardStore::from_root(self.root.clone())
            .load(&[key])
            .await
        {
            Ok(rows) => rows.into_iter().next().flatten().map(|row| row.state),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        // CHAR-POSITION-0 §3.3: the durable position replaces `entry_start` as the input of the
        // owner step below, so it is read now, under this admission's session fence and before
        // the Channel-owner lock: party presence and the familiar's login placement, which run
        // after that step, then read the placed cell. A failed read leaves the actor unpositioned.
        let position_fence =
            crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_id,
                game_session_id,
                connection_generation: current.current_connection_generation(),
                character_lease_generation: current.current_character_lease().generation(),
                runtime_scope: current.current_runtime_scope(),
                scope_ownership_generation: current.current_scope_generation(),
                expected_character_revision: character_revision,
            };
        let Ok(decided) = self.read_tutorial_admission_position(position_fence).await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        #[cfg(test)]
        stage("durable_position_read");
        match self.reserve_precondition().await {
            Ok(()) => {}
            Err(AdmissionRefusal::Unavailable) => {
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
            Err(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
        }
        // Hold the lane through installation; the equipment reader runs before runtime locks.
        let Ok(equipment_effects) = self.read_admission_effects(game_session_id).await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
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
            return (FirstEntryOutcome::RefusedStaleAuthority, None, None);
        }
        // A2 (D295 item 4): bind the admitted session's lease to its slot in this owner step,
        // from the read that just proved it current. No damage write is admitted before.
        if runtime
            .bind_attacker_lease(actor, game_session_id, current.current_character_lease())
            .is_err()
        {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        // CHAR-POSITION-0 §3.3: initialized and placed at the durable position in this one
        // owner step, before the lock is released and any owner reads the actor's position.
        let outcome = match self
            .initialize_admitted_position(&mut runtime, actor, game_session_id, &decided)
            .await
        {
            Ok(FirstEntryPosition::Initialized(_)) => FirstEntryOutcome::Positioned,
            Ok(FirstEntryPosition::Reconciled(_)) => FirstEntryOutcome::Reconciled,
            Err(_) => return (FirstEntryOutcome::RefusedByChannel, None, None),
        };
        #[cfg(test)]
        stage("position_initialized");
        // C2: the item fence of the admitted session, from the same current session read that
        // just proved it is this admission's own.
        let item_fence = item_fence_of(current);
        // Spell cast §4: the vitals and cooldowns are created in this same owner step, before the
        // actor's first command, and only from Character-owned facts; a monk's Serene is
        // evaluated here too (§8.2). CHAR-VITALS-1: a newly placed actor whose last-position row
        // saved its health and mana takes them, capped at its maxima, in this same step.
        let saved_vitals = (outcome == FirstEntryOutcome::Positioned)
            .then(|| last_position::admitted_vitals(&decided, self.world_id))
            .flatten();
        if self
            .states
            .spell_states
            .lock()
            .await
            .initialize_admitted(
                &runtime,
                actor,
                game_session_id,
                facts,
                monk,
                self.owner_now(),
                saved_vitals,
                (outcome == FirstEntryOutcome::Positioned)
                    .then(|| bed_regen::admitted_rest(&decided, self.world_id))
                    .flatten(),
            )
            .is_none()
        {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        if self
            .install_admission_suppressions(&runtime, actor, game_session_id, equipment_effects)
            .await
            .is_none()
        {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        if !self.states.spell_states.lock().await.load_owned_stance(
            &runtime,
            actor,
            game_session_id,
            &stance,
            self.spells,
        ) {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        if !self.states.spell_states.lock().await.load_owned_build(
            &runtime,
            actor,
            game_session_id,
            &build,
        ) {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        if let Some(formula) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula())
        {
            let mut states = self.states.spell_states.lock().await;
            if states
                .get_mut(&runtime, actor, game_session_id)
                .is_none_or(|state| state.enable_owned_training(&build, formula).is_err())
            {
                return (FirstEntryOutcome::RefusedByChannel, None, None);
            }
        }
        drop(runtime);
        drop(permit);
        #[cfg(test)]
        stage("runtime_facts_initialized");
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
        {
            let _ = self
                .refresh_source_party_presence(actor, game_session_id)
                .await;
        }
        if let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            && let Some(config) = native.familiar_config()
        {
            let access = self
                .refresh_and_apply_spell_access(actor, game_session_id)
                .await;
            // #2199: a familiar Login that did not complete restored neither the stored familiar
            // nor its cooldowns, so the entry fails closed like the other post-position owners.
            let initialized = self
                .initialize_familiar_login(actor, game_session_id, request, native, &access, config)
                .await
                .is_ok();
            let outcome = familiar_login_first_entry(outcome, initialized);
            if outcome == FirstEntryOutcome::RefusedByChannel {
                return (outcome, None, None);
            }
        }
        (outcome, item_fence, Some(decided))
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

/// The durable intent of one runtime death (DEATH-0 §3.1): the death cell in this Channel at the
/// pinned map revision and the respawn position, both as the big-endian `(x, y, floor)` cell.
/// No path inserts blessings before DEATH-4, so the held set is empty.
fn player_death_request<const N: usize>(
    progression: &crate::combat::RewardProgressionBinding<N>,
    (world_id, channel_id): (WorldId, ChannelId),
    map_revision: [u8; 32],
    death: actor_spell::PlayerDeath,
    respawn: crate::foundation::MovementLocalPosition,
) -> crate::durability::character_death::CharacterDeathRequest<N> {
    let map: String = map_revision
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    crate::durability::character_death::CharacterDeathRequest {
        pvp_contributors: Vec::new(),
        occurrence: death.occurrence,
        context: progression.context.clone(),
        policy_revision: progression.policy_revision.clone(),
        reward_revision: progression.reward_revision.clone(),
        policy: progression.policy.clone(),
        held_blessings: Vec::new(),
        death_cell: crate::durability::character_death::DeathCell {
            world_id,
            channel_id,
            spatial_position: cell_bytes(death.cell),
            map_revision: format!("map:{map}"),
        },
        respawn_position: cell_bytes(respawn),
    }
}

fn cell_bytes(cell: crate::foundation::MovementLocalPosition) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(10);
    bytes.extend_from_slice(&cell.x.to_be_bytes());
    bytes.extend_from_slice(&cell.y.to_be_bytes());
    bytes.extend_from_slice(&cell.floor.to_be_bytes());
    bytes
}

/// DEATH-2b and CHAR-POSITION-0 §3.3 "valid": `cell` is a Walkable cell of the pinned
/// generation's own movement cells in this World, and no closed door blocks it.
fn respawn_cell_admissible(
    index: &impl crate::content::native_cell_lookup::NativeStaticCellLookup,
    scope: &crate::content::static_cell_engine::EngineeringStaticCellScope,
    world_id: crate::foundation::WorldId,
    pinned_digest: [u8; 32],
    door_cells: &std::collections::BTreeSet<crate::content::LogicalCell>,
    cell: crate::foundation::MovementLocalPosition,
) -> bool {
    let target = crate::content::LogicalCell {
        x: cell.x,
        y: cell.y,
        z: i32::from(cell.floor),
    };
    scope.world_id == world_id
        && scope.generation_digest == pinned_digest
        && index.lookup(scope, target) == Ok(crate::content::CollisionClass::Walkable)
        && !door_cells.contains(&target)
}

/// DEATH-ML-LOSS-1 (#2193 Codex P1 4306417846): a live death settles only after its training
/// flush is durable (`Saved`) or had nothing to write (`NotApplicable`). A fenced-out flush may
/// leave a merged successor only in memory, so the live actor stays dead and retries (a
/// committed checkpoint behind a moved fence is applied first, Codex P1 4306765047).
///
/// The actor-end release settles a fenced-out flush from the durable build only when the fence
/// left no training in memory (`training_left` is `false`): the durable build then holds all of
/// the actor's training, and no write remains to land. A fenced-out flush that still holds a
/// retained checkpoint or pending training (a pre-commit `AuthorityRejected` that raced a
/// reconnect or a generation change, Codex P1 4307011952) keeps the actor dead, and the release
/// retries the flush under the session's current authority, so the death is never priced from
/// a build its training would have moved and the pending progress is never discarded.
fn death_flush_permits_settle(
    save: actor_spell::TrainingSave,
    releasing: bool,
    training_left: bool,
) -> bool {
    match save {
        actor_spell::TrainingSave::Saved | actor_spell::TrainingSave::NotApplicable => true,
        actor_spell::TrainingSave::FencedOut => releasing && !training_left,
        actor_spell::TrainingSave::Unknown => false,
    }
}

/// DEATH-2b: the consumption settled the respawn only if it deleted exactly the occurrence the
/// admission placed; an error or another row settles nothing.
fn admitted_respawn_consumed<E>(
    placed: crate::durability::character_death::PlayerDeathOccurrence,
    consumed: Result<Option<crate::durability::character_death::ConsumedRespawn>, E>,
) -> bool {
    matches!(consumed, Ok(Some(consumed)) if consumed.occurrence == placed)
}

/// ARCH-PROGRESSION-SOURCE-0 §1.5: whether the admission writers after the progression step (the
/// quest session and its Wheel reset; the respawn settle is gated on a playable entry already)
/// run. A positioned first entry that step left `RefusedUnavailable`, its rounds ended undecided,
/// writes nothing more to the Character root: the decision reads only that outcome, so storage
/// that recovers after the last round admits no writer. Any other entry keeps its quest admission.
const fn writes_after_progression(positioned: bool, first_entry: FirstEntryOutcome) -> bool {
    !(positioned && matches!(first_entry, FirstEntryOutcome::RefusedUnavailable))
}

/// DEATH-2b: an unsettled pending respawn refuses the admission's playable first entry, so the
/// connection holds the actor input-ineligible.
const fn respawned_first_entry(first_entry: FirstEntryOutcome, settled: bool) -> FirstEntryOutcome {
    if settled {
        first_entry
    } else {
        FirstEntryOutcome::RefusedByChannel
    }
}

/// #2199: a first entry whose familiar Login did not complete (an unknown or refused lifecycle
/// write, with the stored familiar and its cooldowns unrestored) refuses its playable entry, so
/// the connection holds the actor input-ineligible instead of playing without them.
const fn familiar_login_first_entry(
    first_entry: FirstEntryOutcome,
    initialized: bool,
) -> FirstEntryOutcome {
    if initialized {
        first_entry
    } else {
        FirstEntryOutcome::RefusedByChannel
    }
}

/// The cell [`cell_bytes`] recorded; `None` for any other length.
fn respawn_cell(bytes: &[u8]) -> Option<crate::foundation::MovementLocalPosition> {
    let (x, rest) = bytes.split_first_chunk::<4>()?;
    let (y, rest) = rest.split_first_chunk::<4>()?;
    let floor: &[u8; 2] = rest.try_into().ok()?;
    Some(crate::foundation::MovementLocalPosition {
        x: i32::from_be_bytes(*x),
        y: i32::from_be_bytes(*y),
        floor: i16::from_be_bytes(*floor),
    })
}

fn unix_seconds() -> Option<i64> {
    #[cfg(test)]
    if let Some(now) = monk_save::postgres_tests::host_now() {
        return Some(now);
    }
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

    /// A busy durability holder fails a pass at once with nothing written. Fresh admission
    /// retries that proven noncommit while rounds remain and fails closed on the last one;
    /// rejections and classified refusals are never retried.
    #[test]
    fn fresh_admission_retries_only_transient_noncommits() {
        assert!(fresh_round_retries(AdmissionRefusal::Unavailable, false));
        assert!(!fresh_round_retries(AdmissionRefusal::Unavailable, true));
        for last in [false, true] {
            assert!(!fresh_round_retries(AdmissionRefusal::Rejected, last));
            assert!(!fresh_round_retries(
                AdmissionRefusal::Classified(FoundationProtocolError::AdmissionGrantReplayed),
                last
            ));
        }
        let low = GameSessionId::decode(&uuid_v7(0x40)).expect("low session");
        let high = GameSessionId::decode(&uuid_v7(0x7f)).expect("high session");
        assert_eq!(fresh_admission_backoff(&low), RECONCILE_BACKOFF);
        assert_eq!(
            fresh_admission_backoff(&high),
            RECONCILE_BACKOFF + Duration::from_millis(63)
        );
    }

    /// N8 (ARCH-LOGIN-FIRST-PACKETS-V1 §1.2): each fresh verifier refusal carries its FND-04A
    /// row; a recovery error stays frameless.
    #[test]
    fn fresh_grant_refusals_carry_their_n8_code() {
        use Fnd04ConsumerError as F;
        let rows = [
            (F::FreshMalformed, 1100),
            (F::FreshAuthenticationFailed, 1101),
            (F::FreshBindingMismatch, 1102),
            (F::FreshNotYetValid, 1103),
            (F::FreshExpired, 1104),
            (F::FreshSecurityStateRevoked, 1107),
            (F::FreshSecurityEvidenceStale, 1108),
            (F::FreshRouteStale, 1109),
            (F::FreshRuntimeStale, 1110),
            (F::FreshWorldStale, 1111),
            (F::FreshRevisionUnsupported, 1112),
            (F::FreshAccountCharacterConflict, 1113),
        ];
        for (error, code) in rows {
            assert!(
                matches!(
                    fresh_grant_refusal(error),
                    AdmissionRefusal::Classified(refusal) if refusal.code() == code
                ),
                "{error:?}"
            );
        }
        assert_eq!(
            fresh_grant_refusal(F::RecoveryExpired),
            AdmissionRefusal::Rejected
        );
    }

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
        use crate::world_runtime::{
            LocalObjectUseOutcome, ReferenceContentGeneration, ScopeContentGenerationFence,
            bind_native_entry_door,
        };
        use {
            crate::foundation::{MovementLocalPosition, NodeId},
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
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
        async fn lose_control(
            &self,
            _: AdmittedSession,
            _: Duration,
            _: Option<&crate::durability::world_reset::RetiringWorldReset>,
        ) -> ControlLossResult {
            self.losses.fetch_add(1, Ordering::SeqCst);
            ControlLossResult::Recorded
        }
        async fn expire_control_loss(&self, _: AdmittedSession) -> GraceExpiryResult {
            self.expiring.fetch_add(1, Ordering::SeqCst);
            std::future::pending().await
        }
    }

    /// KILL-REWARD-COMP-1 §1.3 (#1908 Codex P1 4202825538): an abandoned release, whose kill
    /// drain may stay deferred or whose kill mark an unknown outcome kept on every attempt, runs
    /// past the attempt bound until it ends; a mismatch release stops at it.
    #[test]
    fn an_abandoned_release_retries_past_the_bound() {
        assert!(retries_release(0, true));
        assert!(retries_release(0, false));
        assert!(!retries_release(EXPIRY_ATTEMPTS, false));
        assert!(retries_release(EXPIRY_ATTEMPTS, true));
        assert!(retries_release(u32::MAX, true));
    }

    /// Records the loss; the first two grace expiries end unknown.
    struct UnknownExpiryAuthority {
        inner: GatedAuthority,
        expiries: AtomicUsize,
    }

    impl FreshAdmissionAuthority for UnknownExpiryAuthority {
        async fn admit(
            &self,
            attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            self.inner.admit(attempt).await
        }
        async fn lose_control(
            &self,
            _: AdmittedSession,
            _: Duration,
            _: Option<&crate::durability::world_reset::RetiringWorldReset>,
        ) -> ControlLossResult {
            ControlLossResult::Recorded
        }
        async fn expire_control_loss(&self, _: AdmittedSession) -> GraceExpiryResult {
            if self.expiries.fetch_add(1, Ordering::SeqCst) < 2 {
                GraceExpiryResult::Unknown
            } else {
                GraceExpiryResult::Released
            }
        }
    }

    /// KILL-REWARD-COMP-1 §1.3: an unknown grace expiry runs again behind the backoff until it
    /// reaches a final result.
    #[test]
    fn an_unknown_grace_expiry_is_retried_until_final() -> Result<(), Box<dyn Error>> {
        runtime()?.block_on(async {
            let authority = UnknownExpiryAuthority {
                inner: GatedAuthority::new(true),
                expiries: AtomicUsize::new(0),
            };
            let session = AdmittedSession {
                game_session_id: GameSessionId::decode(&CHARACTER)?,
                world_id: crate::foundation::WorldId::decode(&CHARACTER)?,
                channel_id: crate::foundation::ChannelId::decode(&CHARACTER)?,
                runtime_actor: None,
                first_entry: FirstEntryOutcome::NotApplicable,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            };
            let lost = LostControl {
                reset: None,
                session,
                wait: Duration::ZERO,
            };
            control_loss_lifecycle(&authority, lost, &CancellationToken::new()).await;
            assert_eq!(authority.expiries.load(Ordering::SeqCst), 3);
            Ok(())
        })
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

    /// PREM-1b (D476): every terminal release, abandoned or mismatched, stops the account's
    /// Premium pulls on a proven `Released` or `Terminal` outcome, as grace expiry does, and
    /// never on an unended or unknown one.
    #[test]
    fn only_a_proven_terminal_release_stops_premium() {
        assert!(ends_session::<()>(&Ok(ExpiredLossReleaseV1::Released {
            decided_at: 1
        })));
        assert!(ends_session::<()>(&Ok(ExpiredLossReleaseV1::Terminal)));
        assert!(!ends_session::<()>(&Ok(
            ExpiredLossReleaseV1::NotApplicable
        )));
        assert!(!ends_session::<()>(&Ok(ExpiredLossReleaseV1::NotExpired {
            deadline: 2,
            now: 1
        })));
        assert!(!ends_session(&Err::<ExpiredLossReleaseV1, ()>(())));
    }

    /// PREM-1b (D476, #1722 Codex P1 4176449967): a late or repeated terminal release of a
    /// session never stops the pulls of a same-account successor admitted before it lands.
    #[test]
    fn a_late_release_keeps_a_successor_session_premium() {
        let account = [9; 16];
        let old = GameSessionId::decode(&uuid_v7(0x70)).expect("old");
        let successor = GameSessionId::decode(&uuid_v7(0x71)).expect("successor");
        let mut sessions = PremiumSessions::default();
        sessions.admit(account, old);
        // A resume of the same session registers once.
        sessions.admit(account, old);
        sessions.admit(account, successor);
        // The old session's Terminal outcome lands after the successor's admission.
        assert!(!sessions.release(account, old));
        // A retry of that release, and a release never admitted, stop nothing.
        assert!(!sessions.release(account, old));
        assert!(!sessions.release([8; 16], old));
        // The successor's own release is the account's last: its pulls stop.
        assert!(sessions.release(account, successor));
        assert!(!sessions.release(account, successor));
        // Grace expiry or a mismatch release of a sole session stops them at once.
        sessions.admit(account, old);
        assert!(sessions.release(account, old));
    }

    /// PREM-1b (D476, #1722 Codex P2 4176492358): a reconciled TERMINAL settle releases the
    /// session's registration like a proven release. When a joined release of the same session
    /// already released it, the second release is a no-op and a successor keeps its pulls.
    #[test]
    fn a_reconciled_terminal_release_stops_premium_once() {
        let account = [7; 16];
        let lost = GameSessionId::decode(&uuid_v7(0x72)).expect("lost");
        let successor = GameSessionId::decode(&uuid_v7(0x73)).expect("successor");
        let mut sessions = PremiumSessions::default();
        sessions.admit(account, lost);
        // The abandoned release's attempt is unended; its reconciliation finds TERMINAL.
        assert!(sessions.release(account, lost));
        // Grace expiry of the same session, joined on its fence, reconciles TERMINAL too.
        assert!(!sessions.release(account, lost));
        // A successor admitted afterwards is untouched by any later reconciliation.
        sessions.admit(account, successor);
        assert!(!sessions.release(account, lost));
        assert!(sessions.release(account, successor));
    }

    /// D449 (#1708 Codex P1 4175882774): the capability-mismatch release fences with the lost
    /// epoch. Its retry and grace expiry join that fence, a compatible resume that wins the race
    /// lifts it when it restores control, and no mismatch attempt can fence the slot again
    /// afterwards, so an unproven mismatch release never leaves a fence behind a resumed session.
    /// A slot whose bound session lost control at epoch 1.
    fn lost_slot() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        use {
            crate::foundation::{CharacterLease, NodeId},
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
        };
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

    /// KILL-REWARD-COMP-1 (#1908 Codex P1 4202367021): a grace expiry that ends unknown keeps
    /// its hold for the lifecycle's retry, which counts the release once, so the retry's
    /// non-terminal settle lifts the fence and leaves no count.
    #[test]
    fn a_retried_unknown_grace_expiry_counts_its_hold_once() {
        let (mut runtime, actor, session) = lost_slot();
        let holders = FenceHolders::default();
        let token = TransitionFence::GraceExpiry(1);
        let other = TransitionFence::Transition(9);
        let mut first = FenceHold::new(session, token);
        assert_eq!(holders.fence(&mut runtime, actor, &mut first), Ok(()));
        // The expiry ends unknown holding the fence; the retry takes the hold over.
        holders.keep(first);
        let mut retry = holders.resume(session).expect("kept hold");
        assert!(holders.resume(session).is_none());
        assert_eq!(holders.fence(&mut runtime, actor, &mut retry), Ok(()));
        assert_eq!(holders.counts().get(&(session, token)), Some(&1));
        // The retry's non-terminal settle lifts the fence and no count is left.
        assert_eq!(holders.lift(&mut runtime, actor, &mut retry), Ok(()));
        assert!(holders.counts().is_empty());
        assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
        assert_eq!(other.lift(&mut runtime, actor, session), Ok(true));
        // A hold that ended is not kept, and a retired session leaves none behind.
        holders.keep(retry);
        assert!(holders.resume(session).is_none());
        let mut kept = FenceHold::new(session, token);
        assert_eq!(holders.fence(&mut runtime, actor, &mut kept), Ok(()));
        holders.keep(kept);
        holders.forget(session);
        assert!(holders.resume(session).is_none());
    }

    /// KILL-REWARD-COMP-1 (#1908 Codex P1 4202825532): two grace-expiry lifecycles of one
    /// session that both end unknown keep both holds; their retries settle each once, so the
    /// fence is lifted and no count is left.
    #[test]
    fn concurrent_unknown_grace_expiries_keep_every_hold() {
        let (mut runtime, actor, session) = lost_slot();
        let holders = FenceHolders::default();
        let token = TransitionFence::GraceExpiry(1);
        let other = TransitionFence::Transition(9);
        let mut first = FenceHold::new(session, token);
        let mut second = FenceHold::new(session, token);
        assert_eq!(holders.fence(&mut runtime, actor, &mut first), Ok(()));
        assert_eq!(holders.fence(&mut runtime, actor, &mut second), Ok(()));
        // Both end unknown: neither kept hold replaces the other.
        holders.keep(first);
        holders.keep(second);
        let mut retries = [
            holders.resume(session).expect("kept hold"),
            holders.resume(session).expect("kept hold"),
        ];
        assert!(holders.resume(session).is_none());
        assert_eq!(holders.counts().get(&(session, token)), Some(&2));
        // Each retry joins the fence without counting again, and settles its own hold once.
        for retry in &mut retries {
            assert_eq!(holders.fence(&mut runtime, actor, retry), Ok(()));
        }
        assert_eq!(holders.counts().get(&(session, token)), Some(&2));
        let [early, last] = &mut retries;
        assert_eq!(holders.lift(&mut runtime, actor, early), Ok(()));
        assert_eq!(
            other.fence(&mut runtime, actor, session),
            Err(CarrierError::WriteFenceBusy)
        );
        assert_eq!(holders.lift(&mut runtime, actor, last), Ok(()));
        assert!(holders.counts().is_empty());
        assert_eq!(other.fence(&mut runtime, actor, session), Ok(()));
        assert_eq!(other.lift(&mut runtime, actor, session), Ok(true));
    }

    /// DEATH-2b: only a cell that is Walkable in the pinned generation's own movement cells of
    /// this World, and not a closed door, is admissible; a blocked or absent cell, a door and a
    /// foreign World or generation are not, and an undecodable record has no cell.
    #[test]
    fn a_respawn_cell_must_be_walkable_unblocked_and_of_the_pinned_generation() {
        use crate::content::static_cell_engine::{
            EngineeringCollisionClaim, EngineeringStaticCellClaim, EngineeringStaticCellIndex,
            EngineeringStaticCellScope,
        };
        use crate::content::{
            CollisionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef, LogicalCell,
            MapRevisionRef, ProductionAtom, ProductionKey, Sha256HexDigest,
        };
        use crate::foundation::MovementLocalPosition;
        use std::collections::BTreeSet;
        let world = WorldId::decode(&CHARACTER).expect("world");
        let scope = EngineeringStaticCellScope {
            world_id: world,
            coordinate_frame: CoordinateFrameRef::new("death-2b-frame").expect("frame"),
            map_revision: MapRevisionRef::new("death-2b-map").expect("map"),
            generation_digest: [9; 32],
            content_lock: ContentLockBinding {
                revision_digest_token: ProductionAtom::new("lock", "death-2b-lock").expect("lock"),
                entries: vec![ContentLockEntry::exact(
                    ProductionKey::new("engineering:death-2b").expect("key"),
                    ProductionAtom::new("revision", "death-2b-r1").expect("revision"),
                    Sha256HexDigest::new(&"b".repeat(64)).expect("digest"),
                )],
            },
        };
        let claim = |x, collision| EngineeringStaticCellClaim {
            scope: scope.clone(),
            cell: LogicalCell { x, y: 4, z: 7 },
            collision: EngineeringCollisionClaim::Qualified(collision),
        };
        let index = EngineeringStaticCellIndex::from_claims(vec![
            claim(3, CollisionClass::Walkable),
            claim(5, CollisionClass::Walkable),
            claim(6, CollisionClass::Blocked),
        ])
        .expect("index");
        let doors = BTreeSet::from([LogicalCell { x: 5, y: 4, z: 7 }]);
        let at = |x| MovementLocalPosition { x, y: 4, floor: 7 };
        let admit = |world, digest, cell| {
            respawn_cell_admissible(&index, &scope, world, digest, &doors, cell)
        };
        assert!(admit(world, [9; 32], at(3)));
        assert!(!admit(world, [9; 32], at(6)), "blocked cell");
        assert!(!admit(world, [9; 32], at(4)), "absent cell");
        assert!(!admit(world, [9; 32], at(5)), "closed door");
        assert!(!admit(world, [8; 32], at(3)), "foreign generation");
        let foreign = WorldId::decode(&[
            0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x72, 0x22, 0x92, 0x22, 0x22, 0x22, 0x22, 0x22,
            0x22, 0x22,
        ])
        .expect("foreign world");
        assert!(!admit(foreign, [9; 32], at(3)), "foreign World");
        assert_eq!(
            respawn_cell(&cell_bytes(at(3))[..9]),
            None,
            "undecodable record"
        );
    }

    /// DEATH-2b: an admission is playable after a pending respawn only when the consumption
    /// deleted exactly the placed occurrence; a consumption error, an already consumed row or
    /// another occurrence refuses the first entry, so the actor stays input-ineligible and the
    /// respawn is not replayed by this admission.
    #[test]
    fn an_unsettled_respawn_refuses_the_playable_first_entry() {
        use crate::durability::character_death::{ConsumedRespawn, PlayerDeathOccurrence};
        let placed = PlayerDeathOccurrence::from_bytes(CHARACTER).expect("placed");
        let mut other_bytes = CHARACTER;
        other_bytes[15] = 0x12;
        let other = PlayerDeathOccurrence::from_bytes(other_bytes).expect("other");
        let row = |occurrence| {
            Ok::<_, &str>(Some(ConsumedRespawn {
                occurrence,
                respawn_position: vec![0; 10],
            }))
        };
        assert!(admitted_respawn_consumed(placed, row(placed)));
        assert!(!admitted_respawn_consumed(placed, Err("consumption")));
        assert!(!admitted_respawn_consumed(placed, Ok::<_, &str>(None)));
        assert!(!admitted_respawn_consumed(placed, row(other)));
        for entry in [FirstEntryOutcome::Positioned, FirstEntryOutcome::Reconciled] {
            assert_eq!(respawned_first_entry(entry, true), entry);
            assert_eq!(
                respawned_first_entry(entry, false),
                FirstEntryOutcome::RefusedByChannel
            );
        }
    }

    /// DEATH-ML-LOSS-1 (#2193 Codex P1 4306417846, 4307011952): a live death settles only on a
    /// durable or empty training flush; a fenced-out flush keeps the live actor dead, and only
    /// the actor-end release settles past it, and only when the fence left no training in
    /// memory. An unknown flush never settles.
    #[test]
    fn a_fenced_out_training_flush_refuses_a_live_death() {
        use super::actor_spell::TrainingSave;
        for releasing in [false, true] {
            for training_left in [false, true] {
                assert!(death_flush_permits_settle(
                    TrainingSave::Saved,
                    releasing,
                    training_left
                ));
                assert!(death_flush_permits_settle(
                    TrainingSave::NotApplicable,
                    releasing,
                    training_left
                ));
                assert!(!death_flush_permits_settle(
                    TrainingSave::Unknown,
                    releasing,
                    training_left
                ));
            }
            // #2193 Codex P1 4307011952: a refused write that keeps training in memory never
            // settles, on the live or the release path.
            assert!(!death_flush_permits_settle(
                TrainingSave::FencedOut,
                releasing,
                true
            ));
        }
        assert!(!death_flush_permits_settle(
            TrainingSave::FencedOut,
            false,
            false
        ));
        assert!(death_flush_permits_settle(
            TrainingSave::FencedOut,
            true,
            false
        ));
    }

    /// #2199: a familiar Login that did not complete refuses the playable first entry, so the
    /// stored familiar and its cooldowns are never silently left unrestored.
    #[test]
    fn an_incomplete_familiar_login_refuses_the_playable_first_entry() {
        for entry in [FirstEntryOutcome::Positioned, FirstEntryOutcome::Reconciled] {
            assert_eq!(familiar_login_first_entry(entry, true), entry);
            assert_eq!(
                familiar_login_first_entry(entry, false),
                FirstEntryOutcome::RefusedByChannel
            );
        }
    }

    /// ARCH-PROGRESSION-SOURCE-0 §1.5 (#1964 Codex P1 4243646040): a positioned admission whose
    /// progression rounds all ended undecided runs no later admission writer, even when storage
    /// recovers right after the last round; the gate reads only the refused outcome. A bound or
    /// unbound-but-decided step, and entries refused before the step, keep their quest admission.
    #[test]
    fn an_exhausted_progression_step_admits_no_later_writer() {
        for entry in [FirstEntryOutcome::Positioned, FirstEntryOutcome::Reconciled] {
            assert!(writes_after_progression(true, entry));
        }
        // Exhausted rounds: the step left the positioned entry `RefusedUnavailable`.
        let exhausted = FirstEntryOutcome::RefusedUnavailable;
        assert!(!writes_after_progression(true, exhausted));
        assert!(!matches!(
            exhausted,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ));
        // Entries that never reached the step are unchanged by it.
        for entry in [
            FirstEntryOutcome::NotApplicable,
            FirstEntryOutcome::RefusedStaleAuthority,
            FirstEntryOutcome::RefusedUnavailable,
            FirstEntryOutcome::RefusedByChannel,
        ] {
            assert!(writes_after_progression(false, entry));
        }
        // An unsettled respawn after a bound step keeps the quest admission it had.
        assert!(writes_after_progression(
            true,
            respawned_first_entry(FirstEntryOutcome::Positioned, false)
        ));
    }

    /// DEATH-2: the durable death intent carries the occurrence, the death cell in this Channel
    /// at the pinned map revision, the respawn position and the empty held set, both cells as the
    /// big-endian `(x, y, floor)`.
    #[test]
    fn a_player_death_request_binds_the_cell_the_map_revision_and_the_respawn() {
        use crate::domain::progression::{
            FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
        };
        use crate::foundation::MovementLocalPosition;
        use oteryn_simulation_determinism::{ExactI64, RoundingMode};
        let context = ProgressionRevisionContext {
            profile: "profile-1".to_owned(),
            ruleset: "ruleset-1".to_owned(),
            content: "content-1".to_owned(),
            simulation: "simulation-1".to_owned(),
            evidence: "evidence-1".to_owned(),
            declaration: "declaration-1".to_owned(),
        };
        let progression = crate::combat::RewardProgressionBinding {
            context: context.clone(),
            policy_revision: "policy-1".to_owned(),
            reward_revision: "reward-1".to_owned(),
            policy: FiniteProgressionPolicy {
                context: context.clone(),
                policy_revision: "policy-1".to_owned(),
                reward_revision: "reward-1".to_owned(),
                death_policy_revision: "death-1".to_owned(),
                declared_difference_revision: "declaration-1".to_owned(),
                thresholds: [LevelThreshold {
                    level: 1,
                    minimum_experience: ExactI64::new(0),
                }],
                terminal_exclusive_experience: ExactI64::new(100),
                death_loss_numerator: 1,
                death_loss_denominator: 1,
                death_loss_rounding: RoundingMode::Floor,
            },
        };
        let occurrence =
            crate::durability::character_death::PlayerDeathOccurrence::from_bytes(CHARACTER)
                .expect("occurrence");
        let world = WorldId::decode(&CHARACTER).expect("world");
        let channel = ChannelId::decode(&CHARACTER).expect("channel");
        let request = player_death_request(
            &progression,
            (world, channel),
            [0xab; 32],
            actor_spell::PlayerDeath {
                occurrence,
                cell: MovementLocalPosition {
                    x: -2,
                    y: 258,
                    floor: 7,
                },
            },
            MovementLocalPosition {
                x: 3,
                y: 4,
                floor: 7,
            },
        );
        assert_eq!(request.occurrence, occurrence);
        assert_eq!(request.policy, progression.policy);
        assert!(request.held_blessings.is_empty());
        assert_eq!(
            (request.death_cell.world_id, request.death_cell.channel_id),
            (world, channel)
        );
        assert_eq!(
            request.death_cell.spatial_position,
            [0xff, 0xff, 0xff, 0xfe, 0, 0, 1, 2, 0, 7]
        );
        assert_eq!(
            request.death_cell.map_revision,
            format!("map:{}", "ab".repeat(32))
        );
        assert_eq!(request.respawn_position, [0, 0, 0, 3, 0, 0, 0, 4, 0, 7]);
        // DEATH-2b: the admission reads the recorded cell back exactly; any other length is no
        // cell, so nothing is placed and the obligation stays.
        let cell = MovementLocalPosition {
            x: -2,
            y: 258,
            floor: -1,
        };
        assert_eq!(respawn_cell(&cell_bytes(cell)), Some(cell));
        assert_eq!(respawn_cell(&request.respawn_position[..9]), None);
        assert_eq!(respawn_cell(&[0; 11]), None);
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

mod party_kill_settlement;
mod party_spell_feedback;
mod party_spell_owner;
mod party_xp;
mod party_xp_command;
mod spell_parameter_cast;
#[cfg(test)]
pub use actor_spell::ChannelSpellStates;
#[cfg(test)]
pub use party_spell_owner::read_source_party_world_in_transaction;
#[cfg(any(test, feature = "test-support"))]
pub use spell_character_facts::{CastFactsLoad, load_character_cast_facts};
#[cfg(test)]
pub use spell_presentation_publisher::{
    CandidatePresentationTurn, CandidateSessionPublisher, ObserverSource,
};
#[cfg(test)]
pub use spell_presentations::SpellPresentationOwner;

pub use crate::spell::spell_presentations;

/// DEATH-ML-LOSS-1 (CP D1433, #2193 Codex P1 4304911165) on PostgreSQL 18.6: a trained actor
/// whose death lowers its build is rebased onto the receipt's post-loss build at respawn, so its
/// next training save commits on the lowered durable build instead of returning `Unknown`.
#[cfg(test)]
mod death_ml_loss_postgres {
    use crate::bestiary_postgres_harness::{
        self as harness, Harness, TestResult, debug, fence, id,
    };
    use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildFormula, BuildOccurrence,
        DurableBuildState, convert_vocation,
    };
    use crate::durability::character_death::{
        CharacterDeathRequest, DeathCell, PlayerDeathOccurrence,
    };
    use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
    use crate::foundation::{ChannelId, WorldId};
    use crate::spell::cast::{CharacterCastFacts, PlayerSpellState};
    use crate::spell::mana_training::CompiledTrainingFormula;
    use oteryn_gameplay_progression::hit_training::SkillFamily;
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};

    pub(super) fn death(tag: u8) -> TestResult<CharacterDeathRequest<50>> {
        let thresholds = std::array::from_fn(|index| LevelThreshold {
            level: u32::try_from(index + 1).unwrap_or(u32::MAX),
            minimum_experience: ExactI64::new(i64::try_from(index).unwrap_or(i64::MAX) * 20),
        });
        let context = harness::context();
        Ok(CharacterDeathRequest {
            pvp_contributors: Vec::new(),
            occurrence: PlayerDeathOccurrence::from_bytes(id(tag)).map_err(debug)?,
            policy: FiniteProgressionPolicy {
                context: context.clone(),
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                death_policy_revision: "death-1".into(),
                declared_difference_revision: "declaration-1".into(),
                thresholds,
                terminal_exclusive_experience: ExactI64::new(1_000_000),
                death_loss_numerator: 1,
                death_loss_denominator: 1,
                death_loss_rounding: RoundingMode::Floor,
            },
            context,
            policy_revision: "policy-1".into(),
            reward_revision: "reward-1".into(),
            held_blessings: vec![],
            death_cell: DeathCell {
                world_id: WorldId::decode(&id(harness::WORLD)).map_err(debug)?,
                channel_id: ChannelId::decode(&id(harness::CHANNEL)).map_err(debug)?,
                spatial_position: vec![1, 2, 3, 7],
                map_revision: "map-1".into(),
            },
            respawn_position: b"temple:thais".to_vec(),
        })
    }

    #[test]
    fn train_lowering_death_respawn_then_train_save_commits() -> TestResult {
        let Some(admin) = harness::configured_admin() else {
            return Ok(());
        };
        harness::runtime()?.block_on(async move {
            let h = Harness::create(admin, "death_ml_rebase", true).await?;
            let result = train_die_respawn_train(&h).await;
            h.cleanup().await?;
            result
        })
    }

    async fn train_die_respawn_train(h: &Harness) -> TestResult {
        let formula = CompiledTrainingFormula::from_profile(
            include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/build-training.json"
            ),
            &harness::context().content,
        )
        .map_err(debug)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let occurrence = |tag| BuildOccurrence::from_bytes(id(tag)).map_err(debug);
        // The vocation choice the actor trains under: revision 1 to 2.
        let druid =
            convert_vocation(&formula, &DurableBuildState::default(), "druid").map_err(debug)?;
        let chosen = h
            .root
            .commit_character_build(
                &authority,
                &h.node,
                fence(1)?,
                BuildChangeRequest {
                    occurrence: occurrence(80)?,
                    cause: BuildCause::VocationChoice,
                    before: DurableBuildState::default(),
                    after: druid.clone(),
                    pruned_stance: None,
                },
                &formula,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));
        let mut actor = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: crate::spell::Vocation::Druid,
                level: 8,
                magic_level: u32::from(druid.magic().0),
                max_health: 185,
                max_mana: 90,
                max_soul: 100,
            },
            0,
            0,
        )
        .ok_or("actor")?;
        actor
            .load_owned_build(&druid)
            .map_err(|_| "load_owned_build")?;
        actor
            .enable_owned_training(&druid, &formula)
            .map_err(debug)?;

        // Train: the 50 druid sword tries reach sword 11, saved at revision 2 to 3.
        assert!(
            actor
                .train_skill(SkillFamily::Sword, 1, &formula, |_| 50)
                .map_err(debug)?
                .checkpoint_due
        );
        let prepared = actor
            .prepare_training_checkpoint(2, &formula, occurrence(81)?, true)
            .map_err(debug)?
            .ok_or("no training to save")?;
        let BuildCommitOutcome::Committed(receipt) = h
            .root
            .commit_character_build(
                &authority,
                &h.node,
                fence(2)?,
                prepared.request.clone(),
                &formula,
            )
            .await
            .map_err(debug)?
        else {
            return Err("training save replayed".into());
        };
        let trained = receipt.after.clone();
        actor
            .commit_training_checkpoint(prepared, &receipt)
            .map_err(debug)?;
        assert_eq!(trained.skills()[2].0, 11);

        // A death that lowers the build: revision 3 to 4, then its respawn is consumed.
        let request = death(82)?;
        let committed = h
            .root
            .settle_runtime_player_death(
                &CharacterRevisionSequencer::new(),
                &authority,
                &h.node,
                fence(3)?,
                request.clone(),
                Some(&formula as &dyn BuildFormula),
                None,
            )
            .await
            .map_err(debug)?;
        let lowered = committed.build_loss.ok_or("death lowered no build")?;
        assert_eq!(lowered.before, trained);
        assert!(lowered.after.skills()[2].0 < trained.skills()[2].0);
        h.root
            .consume_pending_respawn(&authority, &h.node, fence(4)?, Some(request.occurrence))
            .await
            .map_err(debug)?
            .ok_or("missing pending respawn")?;
        assert_eq!(h.root_revision().await?, "4");

        // Without the rebase the save submits the pre-death build and is refused (the owner's
        // `Unknown`), writing nothing.
        let mut stale = actor.clone();
        stale
            .train_skill(SkillFamily::Sword, 3, &formula, |_| 1)
            .map_err(debug)?;
        let refused = stale
            .prepare_training_checkpoint(4, &formula, occurrence(83)?, true)
            .map_err(debug)?
            .ok_or("no stale training to save")?;
        assert_eq!(refused.request.before, trained);
        assert!(
            h.root
                .commit_character_build(&authority, &h.node, fence(4)?, refused.request, &formula)
                .await
                .is_err()
        );
        assert_eq!(h.root_revision().await?, "4");

        // The respawn's rebase: the next training save commits on the lowered build.
        actor.rebase_after_death(&lowered.after);
        assert_eq!(
            actor.character_facts().magic_level,
            u32::from(lowered.after.magic().0)
        );
        assert!(!actor.has_training_to_flush());
        actor
            .train_skill(SkillFamily::Sword, 3, &formula, |_| 1)
            .map_err(debug)?;
        let prepared = actor
            .prepare_training_checkpoint(4, &formula, occurrence(84)?, true)
            .map_err(debug)?
            .ok_or("no training to save after respawn")?;
        assert_eq!(prepared.request.before, lowered.after);
        let BuildCommitOutcome::Committed(receipt) = h
            .root
            .commit_character_build(
                &authority,
                &h.node,
                fence(4)?,
                prepared.request.clone(),
                &formula,
            )
            .await
            .map_err(debug)?
        else {
            return Err("training save after respawn replayed".into());
        };
        actor
            .commit_training_checkpoint(prepared, &receipt)
            .map_err(debug)?;
        assert_eq!(h.root_revision().await?, "5");
        Ok(())
    }

    /// DEATH-ML-LOSS-1 (CP D1439, A13 §4.6, SKILLS-0 §3.6): a trained level 50 druid whose death
    /// takes one third of its experience loses one third of every family's cumulative progress in
    /// the death commit itself. The receipt binds the before and after build under binding
    /// version 2, the respawned actor continues from the lowered build, and a relog through a
    /// restarted root admits the same lowered magic level and skills and trains on them.
    #[test]
    fn trained_death_lowers_magic_and_skills_and_both_survive_respawn_and_relog() -> TestResult {
        let Some(admin) = harness::configured_admin() else {
            return Ok(());
        };
        harness::runtime()?.block_on(async move {
            let h = Harness::create(admin, "death_ml_relog", true).await?;
            let result = die_respawn_relog(&h).await;
            h.cleanup().await?;
            result
        })
    }

    /// Independent restatement of SKILLS-0 §3.6: sum the paid requirements and current progress,
    /// remove `floor(total * lost / before)` and re-level from the family floor.
    fn expected_loss(
        formula: &dyn BuildFormula,
        before: &DurableBuildState,
        experience_before: u64,
        experience_lost: u64,
    ) -> TestResult<DurableBuildState> {
        let vocation = before.vocation();
        let mut families = [before.magic(); 8];
        families[1..].copy_from_slice(&before.skills());
        for (family, (level, progress)) in families.iter_mut().enumerate() {
            let floor = if family == 0 { 0 } else { 10 };
            let mut total = *progress;
            for next in floor + 1..=*level {
                total += formula
                    .required(vocation, family, next)
                    .ok_or("paid requirement")?;
            }
            let removed = u64::try_from(
                u128::from(total) * u128::from(experience_lost) / u128::from(experience_before),
            )?;
            let mut remaining = total - removed;
            let mut relevelled = floor;
            while let Some(required) = formula.required(vocation, family, relevelled + 1) {
                if remaining < required {
                    break;
                }
                remaining -= required;
                relevelled += 1;
            }
            (*level, *progress) = (relevelled, remaining);
        }
        let mut skills = [(0, 0); 7];
        skills.copy_from_slice(&families[1..]);
        Ok(DurableBuildState::new(vocation, families[0], skills).map_err(debug)?)
    }

    async fn die_respawn_relog(h: &Harness) -> TestResult {
        use crate::domain::CharacterId;
        use crate::durability::DurabilityRoot;
        use crate::durability::character_progression::{
            ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
        };
        use crate::gameplay_transport::{CastFactsLoad, load_character_cast_facts};
        let formula = CompiledTrainingFormula::from_profile(
            include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/build-training.json"
            ),
            &harness::context().content,
        )
        .map_err(debug)?;
        let table: &dyn BuildFormula = &formula;
        let character = CharacterId::from_bytes(id(harness::CHARACTER)).map_err(debug)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let occurrence = |tag| BuildOccurrence::from_bytes(id(tag)).map_err(debug);

        // Druid at revision 1 to 2.
        let druid =
            convert_vocation(&formula, &DurableBuildState::default(), "druid").map_err(debug)?;
        let chosen = h
            .root
            .commit_character_build(
                &authority,
                &h.node,
                fence(1)?,
                BuildChangeRequest {
                    occurrence: occurrence(90)?,
                    cause: BuildCause::VocationChoice,
                    before: DurableBuildState::default(),
                    after: druid.clone(),
                    pruned_stance: None,
                },
                &formula,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(chosen, BuildCommitOutcome::Committed(_)));

        // Experience to three level 50 losses (L = 112,900 unblessed, unpromoted): 2 to 3.
        let loss = u64::try_from(
            crate::domain::death::death_experience_loss(50, 0, false).map_err(debug)?,
        )?;
        assert_eq!(loss, 112_900);
        let total = 3 * loss;
        let award = h
            .root
            .commit_character_experience(
                &authority,
                &h.node,
                fence(2)?,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                    amount: ExactI64::new(i64::try_from(total - 1_000)?),
                    context: harness::context(),
                    policy_revision: "policy-1".into(),
                    reward_revision: "reward-1".into(),
                    policy: death(92)?.policy,
                },
            )
            .await
            .map_err(debug)?;
        assert!(matches!(award, ExperienceCommitOutcome::Committed(_)));

        // A trained build: magic level 3 and sword 14, each half-way to the next: 3 to 4.
        let half = |family, level| {
            table
                .required("druid", family, level)
                .map(|required| required / 2)
                .ok_or("requirement")
        };
        let mut skills = druid.skills();
        skills[2] = (14, half(3, 15)?);
        let trained = DurableBuildState::new("druid", (3, half(0, 4)?), skills).map_err(debug)?;
        let BuildCommitOutcome::Committed(_) = h
            .root
            .commit_character_build(
                &authority,
                &h.node,
                fence(3)?,
                BuildChangeRequest {
                    occurrence: occurrence(93)?,
                    cause: BuildCause::Training,
                    before: druid.clone(),
                    after: trained.clone(),
                    pruned_stance: None,
                },
                &formula,
            )
            .await
            .map_err(debug)?
        else {
            return Err("trained build replayed".into());
        };

        // The live actor trains one more sword try, and the death flush saves it: 4 to 5.
        let mut actor = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: crate::spell::Vocation::Druid,
                level: 50,
                magic_level: u32::from(trained.magic().0),
                max_health: 185,
                max_mana: 90,
                max_soul: 100,
            },
            0,
            0,
        )
        .ok_or("actor")?;
        actor
            .load_owned_build(&trained)
            .map_err(|_| "load_owned_build")?;
        actor
            .enable_owned_training(&trained, &formula)
            .map_err(debug)?;
        actor
            .train_skill(SkillFamily::Sword, 1, &formula, |_| 1)
            .map_err(debug)?;
        let prepared = actor
            .prepare_training_checkpoint(2, &formula, occurrence(94)?, true)
            .map_err(debug)?
            .ok_or("no training to flush")?;
        let BuildCommitOutcome::Committed(receipt) = h
            .root
            .commit_character_build(
                &authority,
                &h.node,
                fence(4)?,
                prepared.request.clone(),
                &formula,
            )
            .await
            .map_err(debug)?
        else {
            return Err("death flush replayed".into());
        };
        let flushed = receipt.after.clone();
        actor
            .commit_training_checkpoint(prepared, &receipt)
            .map_err(debug)?;
        assert_eq!(flushed.skills()[2].1, trained.skills()[2].1 + 1);

        // The death: XP and build loss in one commit, 5 to 6.
        let request = death(95)?;
        let committed = h
            .root
            .settle_runtime_player_death(
                &CharacterRevisionSequencer::new(),
                &authority,
                &h.node,
                fence(5)?,
                request.clone(),
                Some(table),
                None,
            )
            .await
            .map_err(debug)?;
        assert_eq!(committed.experience_before.get(), i64::try_from(total)?);
        assert_eq!(committed.experience_lost.get(), i64::try_from(loss)?);
        assert_eq!(h.root_revision().await?, "6");
        let lowered = committed
            .build_loss
            .clone()
            .ok_or("death lowered no build")?;
        let expected = expected_loss(table, &flushed, total, loss)?;
        assert_eq!(lowered.before, flushed);
        assert_eq!(lowered.after, expected);
        assert_eq!(lowered.formula_digest, table.digest());
        assert_eq!(lowered.after.vocation(), "druid");
        assert!(lowered.after.magic() < flushed.magic());
        assert!(lowered.after.skills()[2] < flushed.skills()[2]);
        // Families at their seed have no progress to lose.
        assert_eq!(lowered.after.skills()[0], flushed.skills()[0]);

        // The receipt holds both builds under the bumped binding version.
        let row: (i64, i64, i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
            "SELECT get_byte(command_binding, 0)::bigint, \
                    magic_level_before::bigint, mana_spent_before::bigint, \
                    magic_level_after::bigint, mana_spent_after::bigint, \
                    sword_level_before::bigint, sword_tries_before::bigint, \
                    sword_level_after::bigint, sword_tries_after::bigint \
               FROM game_character_death_receipts \
              WHERE death_occurrence_id = encode($1,'hex')::uuid",
        )
        .bind(id(95).as_slice())
        .fetch_one(&h.pool)
        .await?;
        let wide = |(level, progress): (u16, u64)| -> TestResult<(i64, i64)> {
            Ok((i64::from(level), i64::try_from(progress)?))
        };
        assert_eq!(row.0, 2);
        assert_eq!((row.1, row.2), wide(flushed.magic())?);
        assert_eq!((row.3, row.4), wide(expected.magic())?);
        assert_eq!((row.5, row.6), wide(flushed.skills()[2])?);
        assert_eq!((row.7, row.8), wide(expected.skills()[2])?);

        // Respawn: the pending respawn is consumed and the actor rebased on the lowered build.
        h.root
            .consume_pending_respawn(&authority, &h.node, fence(6)?, Some(request.occurrence))
            .await
            .map_err(debug)?
            .ok_or("missing pending respawn")?;
        actor.rebase_after_death(&expected);
        assert_eq!(
            actor.character_facts().magic_level,
            u32::from(expected.magic().0)
        );
        assert!(!actor.has_training_to_flush());

        // Relog through a restarted root: the death replays the same loss, admission reads the
        // lowered magic level and build, and a new actor trains on them.
        let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let relogged = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let replayed = restarted
            .reconcile_character_death(&relogged, request.occurrence)
            .await
            .map_err(debug)?
            .ok_or("death not reconciled after relog")?;
        assert_eq!(replayed.build_loss, committed.build_loss);
        let CastFactsLoad::Ready { facts, .. } =
            load_character_cast_facts(&restarted, &relogged, character).await
        else {
            return Err("relogged facts unavailable".into());
        };
        assert_eq!(facts.magic_level, u32::from(expected.magic().0));
        let build = restarted
            .read_character_build_state(&relogged, character)
            .await
            .map_err(debug)?;
        assert_eq!(build, expected);
        let mut relog_actor = PlayerSpellState::new(facts, 0, 0).ok_or("relogged actor")?;
        relog_actor
            .load_owned_build(&build)
            .map_err(|_| "relogged load_owned_build")?;
        relog_actor
            .enable_owned_training(&build, &formula)
            .map_err(debug)?;
        relog_actor
            .train_skill(SkillFamily::Sword, 3, &formula, |_| 1)
            .map_err(debug)?;
        let prepared = relog_actor
            .prepare_training_checkpoint(4, &formula, occurrence(96)?, true)
            .map_err(debug)?
            .ok_or("no training to save after relog")?;
        assert_eq!(prepared.request.before, expected);
        let BuildCommitOutcome::Committed(_) = restarted
            .commit_character_build(&relogged, &h.node, fence(6)?, prepared.request, &formula)
            .await
            .map_err(debug)?
        else {
            return Err("training save after relog replayed".into());
        };
        assert_eq!(h.root_revision().await?, "7");
        Ok(())
    }
}

mod first_vocation_vitals;
mod vocation_choice;

mod monster_combat_states;

#[cfg(any(test, feature = "test-support"))]
pub use connection::SERVER_SCHEMA_REVISION;
#[cfg(any(test, feature = "test-support"))]
pub use monster_ai_cycle::drive_owner_spawns;
