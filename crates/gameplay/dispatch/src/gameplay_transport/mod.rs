//! Production gameplay TCP/TLS seam. Transport-only: Foundation owns protocol
//! semantics and the composed owners decide admission.

include!(concat!(env!("OUT_DIR"), "/mechanic_methods.rs"));
include!(concat!(env!("OUT_DIR"), "/mechanic_modules.rs"));

// Test mounts, including #[path] mounts, share this block.
// Keep sorted by module name; insert in order, never append.
pub mod actor_spell;
mod attack;
mod bank_npc;
mod bed_regen;
mod bed_sleep;
#[cfg(test)]
mod bundle_gates_tests;
mod bundle_map_position;
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
mod composed_admission;
#[cfg(test)]
mod composed_test_owner;
mod condition_snapshots;
mod connection;
mod container_view;
mod control_loss;
mod control_loss_reconciliation;
mod corpse_decay;
/// DEATH-ML-LOSS-1 (CP D1433, #2193 Codex P1 4304911165) on PostgreSQL 18.6: a trained actor
/// whose death lowers its build is rebased onto the receipt's post-loss build at respawn, so its
/// next training save commits on the lowered durable build instead of returning `Unknown`.
#[cfg(test)]
#[path = "tests/death_ml_loss_postgres.rs"]
mod death_ml_loss_postgres;
mod depot;
mod equipment_move;
mod first_vocation_vitals;
pub mod fresh_evidence;
pub mod gameplay_drain;
mod guild_bank;
mod guild_chat;
mod highscores;
mod house_query;
mod hunting_stamina;
mod input_gate;
mod item_craft;
mod item_look;
mod item_move;
mod item_ref_admission;
mod item_use;
mod item_view;
mod kill_reward;
mod last_position;
mod level_up_live;
mod listener;
mod logout;
#[cfg(test)]
mod logout_tests;
mod loop_move;
mod loop_training;
mod map_door;
mod mining;
mod monk_save;
mod monster_ai_cycle;
mod monster_combat_states;
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
pub mod party_command;
mod party_kill_settlement;
mod party_spell_feedback;
mod party_spell_owner;
mod party_xp;
mod party_xp_command;
mod promotion_death;
mod promotion_login;
mod pvp_death;
mod pvp_live;
mod quest_admission;
mod quest_log_source;
mod resume;
mod rune_use_bridge;
mod seam_rooms;
mod serving;
pub mod session_ingress;
mod soul_kill_live;
mod source_item_cycle;
mod source_pvp_expiry;
pub mod spectate_session;
mod spell_access_facts;
mod spell_character_facts;
mod spell_magnitude_facts;
mod spell_parameter_cast;
pub mod spell_premium_coordinator;
mod spell_presentation_publisher;
mod tcp_tls;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
mod timed_item;
mod timed_item_clock;
mod timed_item_content;
mod timed_item_use;
mod timed_item_wire;
mod tutorial_corruptor;
mod tutorial_movement;
mod tutorial_session;
mod vocation_choice;
mod world_event;
mod world_map;
/// ADMIT0: the boot-bound Channel required gameplay capabilities (D1081-D1084).
pub mod world_required_capabilities;
mod world_required_field_guard;
mod world_respawns;
pub mod world_spatial;
// End sorted module declarations.

use crate::map::boot::BundleCollisionIndexExt;
use bundle_map_position::bundle_map_position;
#[cfg(test)]
use bundle_map_position::bundle_world_map;
pub use connection::quest_log;
pub use last_position::LastPositionFlush;
pub use npc_talk::NpcTalkBook;
pub use paced_movement::{AutoWalkRoute, AutoWalkTarget};
pub use promotion_login::PremiumSessions;
pub use seam_rooms::SeamRooms;
#[allow(
    unused_imports,
    reason = "Retain the original presentation type path during the ADR-0019 carve"
)]
pub use spell_presentations::PreparedPresentation as PreparedSpellPresentation;
pub use staff::StaffReply;

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
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, Semaphore};
use tokio_rustls::rustls;
use world_spatial::{ActorPosition, StepDirection, StepDisposition, WorldSpatialObservation};
use {
    crate::foundation::{
        AuthenticatedTransportRefV1, ChannelId, ChannelRuntimeV1, CharacterWorldEligibilityClaimV1,
        ExactActorRef, FirstEntryPosition, GameSessionAuthoritySnapshot, GameSessionId,
        GameSessionState, PlayerActorReservation, RuntimeScopeRefV1, ScopeOwnershipGeneration,
        WorldId,
    },
    oteryn_gameplay_domain::domain::carrier_values::CarrierError,
};

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

pub use listener::serve_listener;
#[cfg(test)]
use listener::{LostControl, control_loss_lifecycle};
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
    if pvp_live::logout_blocked(states, actor, session, now) {
        return Some(Duration::from_millis(250).min(limit));
    }
    let until = attack.in_fight_until(actor, session, now)?;
    Some(hold_step(until.get().saturating_sub(now.get())).min(limit))
}

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

#[cfg(any(test, feature = "test-support"))]
pub use connection::SERVER_SCHEMA_REVISION;
#[cfg(any(test, feature = "test-support"))]
pub use monster_ai_cycle::drive_owner_spawns;

#[cfg(test)]
#[test]
fn module_registration_block_is_sorted_and_complete() -> Result<(), &'static str> {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gameplay_transport/mod.rs"
    ));
    let (before, rest) = source
        .split_once("// Keep sorted by module name; insert in order, never append.\n")
        .ok_or("module registration block header")?;
    let (block, after) = rest
        .split_once("// End sorted module declarations.\n")
        .ok_or("module registration block footer")?;

    fn module_name(line: &str) -> Option<&str> {
        let line = line.trim();
        let declaration = if let Some(rest) = line.strip_prefix("pub(") {
            rest.split_once(')')?.1.trim_start()
        } else {
            line.strip_prefix("pub ").unwrap_or(line)
        };
        declaration.strip_prefix("mod ").map(|name| {
            assert!(name.ends_with(';'), "one module declaration per line");
            name.trim_end_matches(';')
        })
    }

    for line in before.lines().chain(after.lines()) {
        assert!(module_name(line).is_none(), "module outside block: {line}");
    }
    let names: Vec<_> = block.lines().filter_map(module_name).collect();
    assert!(!names.is_empty(), "module registration block is empty");
    for pair in names.windows(2) {
        assert!(pair[0] < pair[1], "modules out of order: {pair:?}");
    }
    Ok(())
}
