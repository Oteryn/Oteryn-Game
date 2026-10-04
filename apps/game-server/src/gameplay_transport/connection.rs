//! One gameplay connection. The transport decodes bounded Foundation frames and
//! hands the validated bootstrap to the owning admission authority; it never
//! decides admission itself and fails closed for every message it does not own.

use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelId, CharacterId, ExactActorRef, FoundationProtocolError,
    GameSessionId, MessageType, ServerResumeAcceptedValue, WorldId, decode_wire_envelope,
    encode_protocol_error, encode_server_accepted, encode_server_resume_accepted,
};
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};

use super::actor_spell::{
    ActorVitals, COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT, DELTA_TYPE_ACTOR_VITALS_V1,
    SNAPSHOT_TYPE_ACTOR_VITALS_V1, STATE_DOMAIN_ACTOR_VITALS, SpellCastDisposition,
    SpellCastIntent, SpellCastOutcome, decode_spell_cast_intent, encode_actor_vitals,
    encode_spell_cast_result,
};
use super::capabilities::{
    OfferedCapability, PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use super::container_view::{
    ContainerObservation, ContainerPlan, ContainerViewState, PendingOpen, ViewCommandWindow,
};
use super::item_view::{
    CloseTrigger, InventoryItems, ItemKey, ItemTargetObservation, ItemViewContinuity,
    ItemViewDelta, OpenDecision, SessionItemView,
};
use super::tcp_tls::{FrameReader, read_frame, write_frame};
use super::world_object::{
    COMMAND_TYPE_USE_INTENT, DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
    SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY, UseDisposition,
    UseIntent, UseTarget, WorldObjectOverlayEntry, decode_use_intent_target, encode_use_result,
    encode_world_object_overlay_delta, encode_world_object_overlay_snapshot,
};
use super::world_spatial::{
    CAPABILITY_WORLD_SPATIAL_ENTITIES, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, ChannelEntities,
    ChannelEntity, DELTA_TYPE_WORLD_SPATIAL_V1, SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, SessionVisibility, StepDirection, StepDisposition,
    VisibilityUpdate, VisibleKind, WorldSpatialEntity, WorldSpatialError, WorldSpatialObservation,
    decode_step_intent, encode_step_outcome, encode_visibility_delta, encode_visibility_snapshot,
    encode_world_spatial,
};
use crate::achievement_catalogue::AccountAchievementsRequest;
use crate::foundation::{
    CommandStatus, DomainSnapshot, encode_command_protocol_error, encode_command_result,
    encode_liveness_probe, encode_single_chunk_snapshot, encode_state_delta,
};
use crate::movement::pacing::StepPacer;
use oteryn_protocol_oteryn::account_achievements::{
    COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY, decode_account_achievements_query,
};
use oteryn_protocol_oteryn::achievement_notices::{
    AchievementEarned, AchievementWatermark, CAPABILITY_ACHIEVEMENT_NOTICES_V1,
    DELTA_TYPE_ACHIEVEMENT_EARNED_V1, SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
    STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES, encode_achievement_earned,
    encode_achievement_notices_snapshot,
};
use oteryn_protocol_oteryn::container_tree::{
    COMMAND_TYPE_CONTAINER_VIEW_INTENT, ContainerViewOutcome, STATE_DOMAIN_CONTAINER_VIEWS,
    decode_container_view_intent, encode_container_view_result,
};
use oteryn_protocol_oteryn::encode_command_error_result;
use oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY;
use oteryn_protocol_oteryn::quest_log::{
    COMMAND_TYPE_QUEST_LOG_QUERY, DELTA_TYPE_QUEST_LOG_V1, SNAPSHOT_TYPE_QUEST_LOG_V1,
    STATE_DOMAIN_QUEST_LOG, decode_quest_log_query,
};
use quest_log::{
    QUEST_LOG_REFRESH, QuestLogContinuity, QuestLogDomain, QuestLogObservation, QuestLogState,
};

// QUEST-LOG-WIRE-1: domain 16 and command type 22, a child of the connection that serves them.
#[path = "quest_log.rs"]
pub(crate) mod quest_log;

/// Foundation schema revision served by this build (FND-02 v1 contract).
pub(crate) const SERVER_SCHEMA_REVISION: u32 = 1;

/// Validated fresh-admission input handed to the owning authority.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FreshAdmissionAttempt<'a> {
    pub(crate) character_id: CharacterId,
    pub(crate) admission_material: &'a [u8],
    pub(crate) game_session_id: GameSessionId,
    pub(crate) transport: AuthenticatedTransportRefV1,
}

/// One `ClientResume` attempt: the GameSession to resume, the untrusted reauthenticated
/// recovery credential and the fresh candidate transport of this connection.
pub(crate) struct ResumeAttempt<'a> {
    pub(crate) game_session_id: GameSessionId,
    pub(crate) recovery_material: &'a [u8],
    pub(crate) transport: AuthenticatedTransportRefV1,
    pub(crate) last_applied_server_sequence: u64,
    /// The resuming client's supported capabilities: a resume that lacks one the session
    /// selected is refused (CAP-NEG-1).
    pub(crate) supported_capabilities: &'a [u32],
}

/// Authority-committed admission: the only state that lets a transport claim a
/// GameSession.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AdmittedSession {
    pub(crate) game_session_id: GameSessionId,
    pub(crate) world_id: WorldId,
    pub(crate) channel_id: ChannelId,
    /// Present for the composed production fresh-admission path. Transport-only
    /// fixtures may omit it; the transport never invents or mutates actor authority.
    pub(crate) runtime_actor: Option<ExactActorRef>,
    /// #935 first-entry positioning of `runtime_actor`. Only a positioned actor
    /// may later become input-eligible.
    pub(crate) first_entry: FirstEntryOutcome,
    /// The admitted controller: the exact authenticated transport and the account whose
    /// presence the session holds. Transport-only fixtures omit it.
    pub(crate) controller: Option<ControllerBinding>,
    /// FND-02 continuity of this controller connection, current at the moment the
    /// connection ends (the next CommandId, server_sequence and spatial revision).
    pub(crate) continuity: SessionContinuity,
    /// C2: the Character item fence a fenced item write (the chest `USE` MINT) presents, read
    /// from the durable GameSession this connection serves: set from the committed fresh
    /// admission and refreshed from the committed resume. `None` when it could not be read or
    /// for transport-only fixtures; a `USE` on a chest is then refused. It is expected
    /// evidence only: every fenced write rechecks it against current durable authority.
    pub(crate) item_fence: Option<CurrentCharacterItemFence>,
}

/// FND-02 continuity of one admitted controller connection. A same-session recovery
/// resumes CommandId order and server_sequence from the value the lost connection ended
/// with, on a strictly newer connection generation (FND-04B §16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionContinuity {
    pub(crate) connection_generation: u64,
    pub(crate) next_command_id: u64,
    pub(crate) server_sequence: u64,
    pub(crate) spatial_revision: u64,
    /// The last `WORLD_OBJECT_OVERLAY` (domain 2) revision emitted to this connection, from
    /// either the join/resync snapshot or a committed-use delta (USE-WIRE-V1, #162 5868482467).
    /// Unlike `spatial_revision` (this actor's own per-actor counter), the door overlay is
    /// Channel-global, so this field is written from the live value actually sent, not trusted
    /// to already match it — see `serve_admitted`.
    pub(crate) overlay_revision: u64,
    /// `ACCOUNT_ACHIEVEMENT_NOTICES` (domain 13, ACHIEVEMENT-0 §5): `None` when the session did
    /// not select capability 8, which production never does before capability negotiation is
    /// composed. Otherwise the domain's revision: 0 at the session's first snapshot, plus 1 for
    /// each delta. It is cumulative per GameSession: a resume carries it and never resets it.
    pub(crate) achievement_notice_revision: Option<u64>,
    /// CAP-NEG-1: the capabilities selected at this GameSession's fresh admission. A resume or
    /// channel transfer carries them unchanged and never widens them.
    pub(crate) selected_capabilities: SelectedCapabilities,
    /// ITEM-VIEW-1b: the item handle counter, the domain 9 and 11 high-water revisions and the
    /// open corpse. Used only with capability 4; a resume or channel transfer carries it.
    pub(crate) item_view: ItemViewContinuity,
    /// QUEST-LOG-WIRE-1: the domain 16 revision, the tracked quests and the query window. Used
    /// only with capability 16; a resume or channel transfer carries it.
    pub(crate) quest_log: QuestLogContinuity,
}

impl SessionContinuity {
    /// A fresh admission: generation 1, first CommandId 1, no sequenced output yet, the
    /// baseline spatial revision 1, and no overlay observed yet (0, the door's own initial
    /// revision).
    pub(crate) const FRESH: Self = Self {
        connection_generation: ADMITTED_GENERATION,
        next_command_id: 1,
        server_sequence: 0,
        spatial_revision: 1,
        overlay_revision: 0,
        achievement_notice_revision: None,
        selected_capabilities: SelectedCapabilities::NONE,
        item_view: ItemViewContinuity {
            handle_counter: 0,
            inventory_revision: 0,
            container_revision: 0,
            open_corpse: None,
            views_revision: 0,
            view_commands: ViewCommandWindow::EMPTY,
        },
        quest_log: QuestLogContinuity::FRESH,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ControllerBinding {
    pub(crate) transport: AuthenticatedTransportRefV1,
    pub(crate) account_id: [u8; 16],
}

/// Outcome of turning an ended admitted connection into durable control loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlLossResult {
    /// No controller binding, or the session is no longer this controller's.
    NotApplicable,
    /// The durable GameSession is RECONNECTABLE and the Channel owner mirrors the epoch.
    Recorded,
    /// Current authority refused the loss; nothing changed.
    Refused,
    /// The durable outcome could not be proven within the bounded reconciliation.
    Unknown,
    /// The ended controller had resumed a lost session, but its loss after the resume
    /// could not be recorded (resumed history unproven or refused); the session is
    /// released instead of staying ACTIVE on a dead transport.
    ResumedHistory,
}

/// Outcome of the FND-04B §6 grace-expiry release of a recorded control loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraceExpiryResult {
    /// No controller binding, or the session is no longer a reconnectable loss
    /// (resumed, replaced or already released by another owner).
    NotApplicable,
    /// The durable session is TERMINAL, its claims are released and the Channel
    /// actor is removed.
    Released,
    /// The outcome could not be proven within the bounded attempts.
    Unknown,
}

/// Outcome of #935 first-entry positioning for one admitted session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FirstEntryOutcome {
    /// Transport-only fixture: there is no Channel runtime actor.
    #[cfg_attr(not(test), allow(dead_code))]
    NotApplicable,
    /// The Channel owner wrote the start position.
    Positioned,
    /// The same initialization had already completed; nothing was written.
    Reconciled,
    /// Current authority no longer matches the admission; nothing was written.
    RefusedStaleAuthority,
    /// Current authority could not be read; nothing was written.
    RefusedUnavailable,
    /// The Channel owner refused the write; nothing was written.
    RefusedByChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdmissionRefusal {
    /// The material or current owner facts do not admit this attempt.
    Rejected,
    /// The owning authority could not decide; nothing was admitted.
    Unavailable,
}

/// The owning fresh-admission authority. Production composes the real owners;
/// nothing on the transport side can mark a connection admitted.
pub(crate) trait FreshAdmissionAuthority {
    fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> impl Future<Output = Result<AdmittedSession, AdmissionRefusal>>;

    /// FND-04B §20 same-session reauthenticated recovery of a lost GameSession. Only an
    /// authority-committed switch returns the resumed session; the transport never infers
    /// one from the attempt.
    fn resume(
        &self,
        _attempt: ResumeAttempt<'_>,
    ) -> impl Future<Output = Result<AdmittedSession, AdmissionRefusal>> {
        async { Err(AdmissionRefusal::Unavailable) }
    }

    /// The capabilities this server offers at fresh admission (CAP-NEG-1). Production offers
    /// the registry's offered set; tests inject theirs.
    fn offered_capabilities(&self) -> &'static [OfferedCapability] {
        PRODUCTION_OFFERED_CAPABILITIES
    }

    /// The admitted actor's current own-actor observation for the initial snapshot, or `None`
    /// when this authority serves no gameplay (transport-only fixtures).
    fn observe(
        &self,
        _actor: ExactActorRef,
    ) -> impl Future<Output = Option<WorldSpatialObservation>> {
        async { None }
    }

    /// VIS-3 (MOVE-RL-11 §4): every entity of the admitted actor's Channel with the actor's own
    /// identity among them, read in one Channel-owner work item, for domain 1 under capability 6.
    /// `None` when it cannot be read: a session that selected capability 6 then fails closed.
    /// An authority that knows no other entity (a fixture) shows the own actor alone, from
    /// [`Self::observe`].
    fn observe_visible_entities(
        &self,
        actor: ExactActorRef,
    ) -> impl Future<Output = Option<ChannelEntities>> {
        async move {
            let own = self.observe(actor).await?;
            let identity = actor.placement_identity();
            Some(ChannelEntities {
                content_generation: own.content_generation,
                observer: identity,
                entities: vec![ChannelEntity {
                    kind: VisibleKind::Player,
                    identity,
                    generation: 0,
                    position: own.actor_position,
                    revision: 0,
                }],
            })
        }
    }

    /// The Channel's current `WORLD_OBJECT_OVERLAY` for the join/resync snapshot (USE-WIRE-V1,
    /// #162 5868482467), or `None` when this authority serves no gameplay (transport-only
    /// fixtures).
    fn observe_world_object_overlay(
        &self,
    ) -> impl Future<Output = Option<WorldObjectOverlayEntry>> {
        async { None }
    }

    /// One `WORLD_ACTOR_STEP_INTENT` for the admitted actor, applied by the Channel owner.
    fn step(
        &self,
        _actor: ExactActorRef,
        _direction: StepDirection,
    ) -> impl Future<Output = StepOutcome> {
        async { StepOutcome::rejected() }
    }

    /// SPEED-1: one paced `WORLD_ACTOR_STEP_INTENT`, the step and, only when it moved, its step
    /// duration (CONDITIONS-0 §4.2), which the connection's pacing clock waits for before the
    /// next step runs. Without a duration the clock is unchanged: fixtures that serve no
    /// Movement owner keep this default and are unpaced.
    fn paced_step(
        &self,
        actor: ExactActorRef,
        _session: GameSessionId,
        direction: StepDirection,
    ) -> impl Future<Output = (StepOutcome, Option<std::time::Duration>)> {
        async move { (self.step(actor, direction).await, None) }
    }

    /// One `USE_INTENT` for the admitted actor against a world-object placement (USE-WIRE-V1,
    /// #162 5868482467), applied by the Channel owner.
    ///
    /// `command_id` is the FND-02 CommandId of the `USE` and `item_fence` the admitted session's
    /// [`AdmittedSession::item_fence`]; a chest `USE` keys its fenced MINT by both (C2).
    fn use_object(
        &self,
        _actor: ExactActorRef,
        _use: UseCommand,
        _target: super::world_object::WorldObjectTarget,
    ) -> impl Future<Output = UseOutcome> {
        async { UseOutcome::rejected() }
    }

    /// ITEM-VIEW-1b: the admitted Character's main backpack and its direct entries
    /// (`read_character_backpack`) for domain 9, read for the snapshot and after each committed
    /// `USE`. `None` when it cannot be read: a session that selected capability 4 then fails
    /// closed.
    fn observe_character_inventory(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<InventoryItems>> {
        async { None }
    }

    /// ITEM-VIEW-1b: the Channel owner's view of the item a `USE` handle resolved to, with the
    /// actor's position, read together. `None` when it cannot be observed (`STALE_STATE`).
    fn observe_item_target(
        &self,
        _actor: ExactActorRef,
        _target: ItemKey,
    ) -> impl Future<Output = Option<ItemTargetObservation>> {
        async { None }
    }

    /// BAGS-WIRE-1: the Channel owner's view of the container a view command or `USE` names,
    /// with reach decided against the actor's position. `None` when it cannot be observed
    /// (`STALE`); a `USE` then takes the corpse path.
    fn observe_container(
        &self,
        _actor: ExactActorRef,
        _target: ItemKey,
    ) -> impl Future<Output = Option<ContainerObservation>> {
        async { None }
    }

    /// QUEST-LOG-WIRE-1: the session's quest copy with the quest log content, unless the copy
    /// still has version `since` (`Unchanged`). `Unavailable` while the copy is not loaded: the
    /// domain then shows nothing new and every query is `REJECTED`.
    fn observe_quest_log(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _since: Option<u64>,
    ) -> impl Future<Output = QuestLogObservation> {
        async { QuestLogObservation::Unavailable }
    }

    /// The admitted actor's current `ACTOR_VITALS` revision and value for the initial snapshot,
    /// or `None` while it has no Character cast facts (spell cast §4).
    fn observe_vitals(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<(u64, ActorVitals)>> {
        async { None }
    }

    /// One `WORLD_ACTOR_SPELL_CAST_INTENT` of the admitted actor, applied by the Channel owner
    /// (spell cast §9 step 2). `command_id` is the FND-02 CommandId the cast is bound to.
    fn cast_spell(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _command_id: u64,
        _intent: SpellCastIntent,
    ) -> impl Future<Output = SpellCastOutcome> {
        async { SpellCastOutcome::rejected() }
    }

    /// One `ACCOUNT_ACHIEVEMENTS_QUERY` page of the request's account's earned facts (display
    /// contract §4). The transport passes the account of the admitted controller, never one from
    /// the payload, with the session and CommandId of the request.
    fn account_achievements(
        &self,
        _request: AccountAchievementsRequest,
    ) -> impl Future<Output = AccountAchievementsReply> {
        async { AccountAchievementsReply::Rejected }
    }

    /// The account's achievement watermark for the `ACCOUNT_ACHIEVEMENT_NOTICES` snapshot
    /// (ACHIEVEMENT-0 §5), read for the admitted controller's account; `None` when it cannot be
    /// read.
    fn observe_achievement_notices(
        &self,
        _account_id: [u8; 16],
    ) -> impl Future<Output = Option<AchievementWatermark>> {
        async { None }
    }

    /// The periodic Serene evaluation of the admitted actor (SPELL-D8 §8.2), run every
    /// [`SERENE_EVALUATION`] while it has `ACTOR_VITALS`: the new revision and value when Serene
    /// changed, which the loop publishes as a delta.
    fn tick_vitals(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<(u64, ActorVitals)>> {
        async { None }
    }

    /// After `wait` without restored control, record authoritative unexpected control loss
    /// for the ended admitted connection (`DISCONNECT-PROTECTION-V1` §§1, 4).
    fn lose_control(
        &self,
        _admitted: AdmittedSession,
        _wait: std::time::Duration,
    ) -> impl Future<Output = ControlLossResult> {
        async { ControlLossResult::NotApplicable }
    }

    /// Terminally release a resumed session whose recovered connection ended again, so it
    /// never stays ACTIVE on a dead transport, and remove its Channel actor.
    fn release_abandoned(
        &self,
        _admitted: AdmittedSession,
    ) -> impl Future<Output = GraceExpiryResult> {
        async { GraceExpiryResult::NotApplicable }
    }

    /// Once the original grace deadline of the recorded loss passes without resumed
    /// control, terminally release the session and remove its Channel actor (FND-04B §6).
    fn expire_control_loss(
        &self,
        _admitted: AdmittedSession,
    ) -> impl Future<Output = GraceExpiryResult> {
        async { GraceExpiryResult::NotApplicable }
    }
}

/// The terminal outcome of one `ACCOUNT_ACHIEVEMENTS_QUERY` (display contract §3.3, §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AccountAchievementsReply {
    /// The encoded `AccountAchievementsResult` of the page.
    Page(Vec<u8>),
    /// `REJECTED` with this registered operation-terminal code and no rows (FND-02 §18):
    /// `PAYLOAD_LIMIT_EXCEEDED` for a reply over its bounds, `ACCOUNT_DATA_INTEGRITY` for a fact
    /// under a key the catalogue lacks.
    Terminal(FoundationProtocolError),
    /// No controller, a malformed query, unreadable storage or a malformed row: `REJECTED`, no
    /// rows, no code.
    Rejected,
}

/// The outcome of one step: its disposition and, only when it moved, the new observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StepOutcome {
    pub(crate) disposition: StepDisposition,
    pub(crate) moved_to: Option<WorldSpatialObservation>,
}

impl StepOutcome {
    pub(crate) const fn rejected() -> Self {
        Self {
            disposition: StepDisposition::Rejected,
            moved_to: None,
        }
    }

    /// SPEED-1: a second step requested while one waits in the pacing buffer. Encoded as
    /// `TOO_EARLY` only for a session that selected capability 13, otherwise as `REJECTED`.
    pub(crate) const fn too_early() -> Self {
        Self {
            disposition: StepDisposition::TooEarly,
            moved_to: None,
        }
    }
}

/// SPEED-1: the one early step a connection holds, as its frame, until its pacing clock is due.
/// The frame then runs through the ordinary path: its CommandId is still the next one, because
/// no result has been sent for it.
struct BufferedStep {
    frame: Vec<u8>,
    due: tokio::time::Instant,
}

/// SPEED-1: frames read while a step waits in the buffer are held, in order, until its result;
/// at most the FND-02 outstanding-command window. Reading pauses while it is full.
const MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP: usize = 64;

/// The command identity of one `USE_INTENT` (C2): its FND-02 `CommandRef` parts and the
/// admitted session's Character item fence. Never client input beyond the CommandId, which the
/// connection loop has already sequenced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UseCommand {
    pub(crate) game_session_id: GameSessionId,
    pub(crate) command_id: u64,
    pub(crate) item_fence: Option<CurrentCharacterItemFence>,
}

/// The outcome of one `USE_INTENT`: its disposition and, only when it committed a transition,
/// the resulting `WORLD_OBJECT_OVERLAY` delta entry (USE-WIRE-V1, #162 5868482467).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UseOutcome {
    pub(crate) disposition: super::world_object::UseDisposition,
    pub(crate) committed: Option<super::world_object::WorldObjectOverlayEntry>,
    /// ACH-NOTIFY-1/2: what the grant of this `USE` earned, after its commit. The loop acts on
    /// it only for a session that selected capability 8.
    pub(crate) earned: EarnedNotice,
}

impl UseOutcome {
    pub(crate) const fn rejected() -> Self {
        Self {
            disposition: super::world_object::UseDisposition::Rejected,
            committed: None,
            earned: EarnedNotice::NoneEarned,
        }
    }
}

/// The notice of one command's achievement grant (ACH-NOTIFY-2). `Unknown` is never
/// `NoneEarned`: with capability 8 it ends the connection, so the client resynchronises the
/// watermark from the domain-13 snapshot. The grant itself stays durable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EarnedNotice {
    /// No grant applied, or it was `AlreadyHeld` or `Retired`.
    NoneEarned,
    /// A committed `Granted` grant and its notice.
    Earned(AchievementEarned),
    /// A grant may have committed `Granted`, but its notice could not be derived.
    Unknown,
}

/// Fresh identifiers for one connection attempt.
pub(crate) trait ConnectionIdentifiers {
    /// `None` when no unpredictable identifier can be produced.
    fn game_session_id(&self) -> Option<GameSessionId>;
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionEnd {
    /// Closed before admission after a Foundation protocol violation.
    ProtocolViolation(FoundationProtocolError),
    /// Admission refused by the owning authority; nothing was admitted.
    AdmissionRefused(AdmissionRefusal),
    /// Resume was refused or could not be proven; nothing was resumed.
    ResumeUnavailable,
    /// Admitted, then closed after unsupported post-admission input.
    AdmittedThenClosed(AdmittedSession, FoundationProtocolError),
    /// Admitted, then the peer closed or the transport failed.
    AdmittedThenDisconnected(AdmittedSession),
    /// Admitted, then the authenticated liveness cadence proved playable control lost
    /// (`DISCONNECT-PROTECTION-V1` §1). No durable session state is changed here.
    AdmittedThenControlLost(AdmittedSession),
    /// The transport failed before admission.
    TransportFailed,
}

/// Admitted connection generation issued with `ServerAccepted`.
const ADMITTED_GENERATION: u64 = 1;

/// FND-02 §17 liveness cadence of one admitted connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LivenessPolicy {
    pub(crate) interval: std::time::Duration,
    /// Consecutive unanswered probes that prove playable control lost.
    pub(crate) missed_limit: u32,
}

/// Out-of-combat cadence: `FND04B-LIVENESS-IDLE-PROBE-MS` and `FND04B-LIVENESS-IDLE-MISSED`
/// (`DISCONNECT-PROTECTION-V1` §1, provisional until measured). The combat cadence is registered
/// but not composed: no combat state exists yet.
pub(crate) const IDLE_LIVENESS: LivenessPolicy = LivenessPolicy {
    interval: std::time::Duration::from_millis(5_000),
    missed_limit: 3,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LivenessEnd {
    /// `missed_limit` consecutive probes went unanswered.
    ControlLost,
    /// Probe IDs never wrap; exhaustion ends this transport generation (FND-02 §17).
    ProbesExhausted,
}

/// Server-authoritative probe/ack state: only the receipt of the current probe's ack at this
/// server counts; a late ack of an older probe restores nothing.
#[derive(Debug)]
struct Liveness {
    missed_limit: u32,
    last_sent: u64,
    awaiting: Option<u64>,
    missed: u32,
}

impl Liveness {
    const fn new(policy: LivenessPolicy) -> Self {
        Self {
            missed_limit: policy.missed_limit,
            last_sent: 0,
            awaiting: None,
            missed: 0,
        }
    }

    /// One cadence tick: an unanswered previous probe is missed. Returns the next probe ID.
    fn tick(&mut self) -> Result<u64, LivenessEnd> {
        if self.awaiting.is_some() {
            self.missed = self.missed.saturating_add(1);
            if self.missed >= self.missed_limit {
                return Err(LivenessEnd::ControlLost);
            }
        }
        let next = self
            .last_sent
            .checked_add(1)
            .ok_or(LivenessEnd::ProbesExhausted)?;
        self.last_sent = next;
        self.awaiting = Some(next);
        Ok(next)
    }

    /// An ack of a probe never sent is a protocol violation.
    fn ack(&mut self, probe_id: u64) -> Result<(), FoundationProtocolError> {
        if probe_id == 0 || probe_id > self.last_sent {
            return Err(FoundationProtocolError::InvalidWireIdentifier);
        }
        if self.awaiting == Some(probe_id) {
            self.awaiting = None;
            self.missed = 0;
        }
        Ok(())
    }
}

/// Whole connection lifecycle without resource policy; the listener runs the
/// same phases under its budgets.
#[cfg(test)]
pub(crate) async fn serve_connection<S, A, I>(
    stream: &mut S,
    authority: &A,
    identifiers: &I,
) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
{
    let Ok(frame) = read_frame(stream).await else {
        return ConnectionEnd::TransportFailed;
    };
    match admit_frame(stream, &frame, authority, identifiers).await {
        Ok(admitted) => hold_admitted(stream, admitted).await,
        Err(end) => end,
    }
}

/// Decode the entry frame, obtain the owning authority's decision and, only on
/// commit, write `ServerAccepted`.
pub(crate) async fn admit_frame<S, A, I>(
    stream: &mut S,
    frame: &[u8],
    authority: &A,
    identifiers: &I,
) -> Result<AdmittedSession, ConnectionEnd>
where
    S: AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
{
    let envelope = match decode_wire_envelope(frame) {
        Ok(envelope) => envelope,
        Err(error) => return Err(reject(stream, error, 0).await),
    };
    let bootstrap = match envelope.message_type() {
        MessageType::ClientBootstrap => envelope.client_bootstrap(),
        MessageType::ClientResume => {
            let resume = match envelope.client_resume() {
                Ok(resume) => resume,
                Err(error) => return Err(reject(stream, error, 0).await),
            };
            let Some(transport) = identifiers.transport_ref() else {
                return Err(ConnectionEnd::ResumeUnavailable);
            };
            let resumed = authority
                .resume(ResumeAttempt {
                    game_session_id: resume.game_session_id,
                    recovery_material: resume.reconnect_material,
                    transport,
                    last_applied_server_sequence: resume.last_applied_server_sequence,
                    supported_capabilities: resume.supported_capabilities,
                })
                .await
                .map_err(|_| ConnectionEnd::ResumeUnavailable)?;
            let continuity = resumed.continuity;
            // CAP-NEG-1: the resumed session keeps its original selection. The owning authority
            // refuses a resume that cannot keep it before committing; this never acknowledges one.
            if !continuity
                .selected_capabilities
                .resumable_with(resume.supported_capabilities)
            {
                return Err(ConnectionEnd::AdmittedThenDisconnected(resumed));
            }
            let accepted = encode_server_resume_accepted(&ServerResumeAcceptedValue {
                game_session_id: resumed.game_session_id,
                connection_generation: continuity.connection_generation,
                current_server_sequence: continuity.server_sequence,
                next_command_id: continuity.next_command_id,
                schema_revision: SERVER_SCHEMA_REVISION,
                selected_capabilities: continuity.selected_capabilities.as_slice(),
            })
            .map_err(|_| ConnectionEnd::AdmittedThenDisconnected(resumed))?;
            write_frame(stream, &accepted)
                .await
                .map_err(|_| ConnectionEnd::AdmittedThenDisconnected(resumed))?;
            return Ok(resumed);
        }
        _ => Err(FoundationProtocolError::MalformedEnvelope),
    };
    let bootstrap = match bootstrap {
        Ok(bootstrap) => bootstrap,
        Err(error) => return Err(reject(stream, error, 0).await),
    };
    let (Some(game_session_id), Some(transport)) =
        (identifiers.game_session_id(), identifiers.transport_ref())
    else {
        return Err(ConnectionEnd::AdmissionRefused(
            AdmissionRefusal::Unavailable,
        ));
    };
    // CAP-NEG-1: the client's supported capabilities the server offers, with their `requires`.
    let Some(selected) = SelectedCapabilities::select(
        authority.offered_capabilities(),
        bootstrap.supported_capabilities,
    ) else {
        return Err(ConnectionEnd::AdmissionRefused(
            AdmissionRefusal::Unavailable,
        ));
    };
    let attempt = FreshAdmissionAttempt {
        character_id: bootstrap.character_id,
        admission_material: bootstrap.admission_material,
        game_session_id,
        transport,
    };
    let mut admitted = authority
        .admit(attempt)
        .await
        .map_err(ConnectionEnd::AdmissionRefused)?;
    admitted.continuity.selected_capabilities = selected;
    // ACHIEVEMENT-0 §5: a session that selected capability 8 starts domain 13 at revision 0.
    admitted.continuity.achievement_notice_revision = selected
        .contains(CAPABILITY_ACHIEVEMENT_NOTICES_V1)
        .then_some(0);
    let accepted = encode_server_accepted(&crate::foundation::ServerAcceptedValue {
        game_session_id: admitted.game_session_id,
        world_id: admitted.world_id,
        channel_id: admitted.channel_id,
        connection_generation: ADMITTED_GENERATION,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: SERVER_SCHEMA_REVISION,
        selected_capabilities: selected.as_slice(),
    })
    .map_err(|_| ConnectionEnd::AdmittedThenDisconnected(admitted))?;
    write_frame(stream, &accepted)
        .await
        .map_err(|_| ConnectionEnd::AdmittedThenDisconnected(admitted))?;
    Ok(admitted)
}

/// No gameplay command, state or liveness semantics are allocated to the seam:
/// any post-admission input ends the connection without mutation.
pub(crate) async fn hold_admitted<S>(stream: &mut S, admitted: AdmittedSession) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let Ok(frame) = read_frame(stream).await else {
        return ConnectionEnd::AdmittedThenDisconnected(admitted);
    };
    let error = match decode_wire_envelope(&frame) {
        Err(error) => error,
        Ok(envelope)
            if envelope.connection_generation() != admitted.continuity.connection_generation =>
        {
            FoundationProtocolError::StaleConnectionGeneration
        }
        Ok(_) => FoundationProtocolError::UnknownMessageType,
    };
    let _ = send_error(stream, error, admitted.continuity.connection_generation).await;
    ConnectionEnd::AdmittedThenClosed(admitted, error)
}

/// FIRST-CONTROL post-admission play (FND-02 §§14-16, #642/#139). A positioned actor gets the
/// initial `WORLD_SPATIAL_VISIBILITY` snapshot (target sequence 0) before any command is accepted;
/// each `ClientCommand` must carry the next CommandId. A step is one Channel-owner work item
/// (`MOVE-RL-02` = 1) answered by a sequenced `CommandResult` and, when it moved, a sequenced
/// `StateDelta`. An actor that is not positioned, or an authority without gameplay, keeps the
/// admission-only behaviour.
/// How often the Channel owner evaluates a monk actor's Serene (SPELL-D8 §8.2, Canary player
/// think).
const SERENE_EVALUATION: std::time::Duration =
    std::time::Duration::from_micros(crate::spell::harmony::SERENE_EVALUATION_MICROS);

/// VIS-3: how often a session with capability 6 looks for entities of its Channel that changed
/// without its own step.
const VISIBILITY_REFRESH: std::time::Duration = std::time::Duration::from_millis(250);

/// VIS-3: the domain 1 handles of a spatial view's objects under capability 4 (ITEM-VIEW-1b
/// §4.1); without an item view there are none to attach.
fn attach_spatial_handles(
    item_view: Option<&mut SessionItemView>,
    entities: &mut [WorldSpatialEntity],
) -> Result<(), WorldSpatialError> {
    match item_view {
        Some(view) => view
            .attach_spatial_handles(entities)
            .map_err(|_| WorldSpatialError::LimitExceeded),
        None => Ok(()),
    }
}

/// VIS-3: the frames of one visibility refresh after `sequence` and domain 1 `revision`, with the
/// sequence and revision they leave: a sequenced delta to the next revision, or a resync snapshot
/// (MOVE-RL-11 §4.3) at the next revision with the next snapshot id. `None` fails closed.
fn visibility_frames(
    generation: u64,
    sequence: u64,
    revision: u64,
    snapshot_id: &mut u64,
    selected: &[u32],
    update: &VisibilityUpdate,
) -> Option<(Vec<Vec<u8>>, u64, u64)> {
    match update {
        VisibilityUpdate::Unchanged => Some((Vec::new(), sequence, revision)),
        VisibilityUpdate::Delta(delta) => {
            let (delta_sequence, new_revision) =
                (sequence.checked_add(1)?, revision.checked_add(1)?);
            let (delta_type, payload) = encode_visibility_delta(selected, delta).ok()?;
            let frame = encode_state_delta(
                generation,
                delta_sequence,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision,
                new_revision,
                delta_type,
                &payload,
            )
            .ok()?;
            Some((vec![frame], delta_sequence, new_revision))
        }
        VisibilityUpdate::Snapshot(snapshot) => {
            let (next_id, new_revision) = (snapshot_id.checked_add(1)?, revision.checked_add(1)?);
            let (snapshot_type, payload) = encode_visibility_snapshot(selected, snapshot).ok()?;
            let frames = encode_single_chunk_snapshot(
                generation,
                next_id,
                sequence,
                &[DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: new_revision,
                    snapshot_type,
                    payload: &payload,
                }],
            )
            .ok()?;
            *snapshot_id = next_id;
            Some((frames.to_vec(), sequence, new_revision))
        }
    }
}

pub(crate) async fn serve_admitted<S, A>(
    stream: &mut S,
    admitted: AdmittedSession,
    authority: &A,
    policy: LivenessPolicy,
) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let mut admitted = admitted;
    let generation = admitted.continuity.connection_generation;
    let playable = matches!(
        admitted.first_entry,
        FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
    );
    let Some(actor) = admitted.runtime_actor.filter(|_| playable) else {
        return hold_admitted(stream, admitted).await;
    };
    let Some(baseline) = authority.observe(actor).await else {
        return hold_admitted(stream, admitted).await;
    };
    let mut revision = admitted.continuity.spatial_revision;
    // VIS-3: domain 1 goes first; with capability 6 it is composed after the item view below, so
    // that its objects carry their item handles under capability 4.
    let spatial: (u32, Vec<u8>);
    let mut domains = Vec::new();
    // USE-WIRE-V1 (#162 5868482467): the join/resync snapshot also carries the door's current
    // `WORLD_OBJECT_OVERLAY`, the same code path a reconnect resumes through, so a resumed
    // connection gets the door's current overlay exactly as a fresh join does.
    let overlay = authority.observe_world_object_overlay().await;
    let overlay_payload;
    // Spell cast §3/§4: the own actor's `ACTOR_VITALS`, when it has them, in the same snapshot.
    let vitals = authority
        .observe_vitals(actor, admitted.game_session_id)
        .await;
    let vitals_payload;
    // ACHIEVEMENT-0 §5: with capability 8, the account's watermark at the current revision. A
    // join, resync or reconnect sends this snapshot and never re-sends a delta. A selected
    // domain whose watermark cannot be read fails closed.
    let notices_selected = admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES);
    let notices = match admitted
        .continuity
        .achievement_notice_revision
        .filter(|_| notices_selected)
    {
        None => None,
        Some(notice_revision) => {
            let watermark = match admitted.controller {
                Some(controller) => {
                    authority
                        .observe_achievement_notices(controller.account_id)
                        .await
                }
                None => None,
            };
            let Some(watermark) = watermark else {
                return ConnectionEnd::AdmittedThenDisconnected(admitted);
            };
            Some((
                notice_revision,
                encode_achievement_notices_snapshot(watermark),
            ))
        }
    };
    if let Some(entry) = &overlay {
        let Ok(bytes) = encode_world_object_overlay_snapshot(std::slice::from_ref(entry)) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        overlay_payload = bytes;
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: entry.revision,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &overlay_payload,
        });
    }
    if let Some((vitals_revision, value)) = &vitals {
        let Ok(bytes) = encode_actor_vitals(value) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        vitals_payload = bytes;
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_ACTOR_VITALS,
            revision: *vitals_revision,
            snapshot_type: SNAPSHOT_TYPE_ACTOR_VITALS_V1,
            payload: &vitals_payload,
        });
    }
    // ITEM-VIEW-1b: with capability 4, domains 9 and 11 above every revision the session has
    // seen, with fresh handles. The revisions advance before the write. An unreadable backpack
    // fails closed; the carried open corpse is shown again only while it is in reach.
    let mut item_view = None;
    let item_snapshot;
    if admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_CHARACTER_INVENTORY)
    {
        let mut view = SessionItemView::resume(admitted.continuity.item_view);
        if admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_CONTAINER_VIEWS)
        {
            view = view.with_container_tree();
        }
        let Some(inventory) = authority
            .observe_character_inventory(actor, admitted.game_session_id)
            .await
        else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        let reopen = match view.carried_open_corpse() {
            Some(key) => authority.observe_item_target(actor, key).await,
            None => None,
        };
        let snapshot = view.snapshot(inventory, reopen);
        admitted.continuity.item_view = view.continuity();
        let Ok(snapshot) = snapshot else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        item_snapshot = snapshot;
        for domain in &item_snapshot {
            domains.push(DomainSnapshot {
                domain_id: domain.domain_id,
                revision: domain.revision,
                snapshot_type: domain.snapshot_type,
                payload: &domain.payload,
            });
        }
        item_view = Some(view);
    }
    // VIS-3 (MOVE-RL-11 §4): with capability 6, domain 1 is the VIS-1 interest set of the
    // session's Channel; without it, the v1 own-actor type. A resume resends it the same way.
    let selected_capabilities = admitted.continuity.selected_capabilities;
    let mut visibility = None;
    if selected_capabilities.contains(CAPABILITY_WORLD_SPATIAL_ENTITIES) {
        let Some(channel) = authority.observe_visible_entities(actor).await else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        let mut view = SessionVisibility::default();
        let snapshot = view.snapshot(&channel, &mut |entities| {
            attach_spatial_handles(item_view.as_mut(), entities)
        });
        if let Some(item_view) = &item_view {
            admitted.continuity.item_view = item_view.continuity();
        }
        let Ok(encoded) = snapshot.and_then(|snapshot| {
            encode_visibility_snapshot(selected_capabilities.as_slice(), &snapshot)
        }) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        spatial = encoded;
        visibility = Some(view);
    } else {
        spatial = (
            SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            encode_world_spatial(&baseline),
        );
    }
    domains.insert(
        0,
        DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision,
            snapshot_type: spatial.0,
            payload: &spatial.1,
        },
    );
    if let Some((notice_revision, payload)) = &notices {
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
            revision: *notice_revision,
            snapshot_type: SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
            payload,
        });
    }
    // BAGS-WIRE-1: with capability 14 (which requires 4), domain 14 with no open view above
    // every revision the session has seen; views close on every reconnect and transfer.
    let mut container_views = None;
    let views_snapshot;
    if let Some(view) = item_view.as_mut().filter(|_| {
        admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_CONTAINER_VIEWS)
    }) {
        let snapshot = view.views_snapshot();
        admitted.continuity.item_view = view.continuity();
        let Ok(snapshot) = snapshot else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        views_snapshot = snapshot;
        domains.push(DomainSnapshot {
            domain_id: views_snapshot.domain_id,
            revision: views_snapshot.revision,
            snapshot_type: views_snapshot.snapshot_type,
            payload: &views_snapshot.payload,
        });
        container_views = Some(ContainerViewState::default());
    }
    // QUEST-LOG-WIRE-1: with capability 16, domain 16 with no view and the tracked quests, above
    // every revision the session has seen; the requested view closes on every reconnect and
    // transfer.
    let mut quest_log = None;
    let quest_log_snapshot: QuestLogDomain;
    if admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_QUEST_LOG)
    {
        let observation = authority
            .observe_quest_log(actor, admitted.game_session_id, None)
            .await;
        let Ok((state, domain)) =
            QuestLogState::snapshot(&mut admitted.continuity.quest_log, observation)
        else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        quest_log_snapshot = domain;
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_QUEST_LOG,
            revision: quest_log_snapshot.to,
            snapshot_type: SNAPSHOT_TYPE_QUEST_LOG_V1,
            payload: &quest_log_snapshot.payload,
        });
        quest_log = Some(state);
    }
    let snapshot =
        encode_single_chunk_snapshot(generation, 1, admitted.continuity.server_sequence, &domains);
    let Ok(snapshot) = snapshot else {
        return ConnectionEnd::AdmittedThenDisconnected(admitted);
    };
    for frame in &snapshot {
        if write_frame(stream, frame).await.is_err() {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
    }
    // r4122508665: only a fully, successfully transmitted snapshot (every frame, including
    // `SnapshotCommit`, above) may update continuity. The overlay is Channel-global (unlike
    // `spatial_revision`, this actor's own counter), so the reconnect fence (`resume.rs`,
    // r4122215795) must record what was actually confirmed delivered, never a value written
    // before transmission could still fail partway through.
    if let Some(entry) = &overlay {
        admitted.continuity.overlay_revision = entry.revision;
    }
    let mut sequence = admitted.continuity.server_sequence;
    let mut next_command = admitted.continuity.next_command_id;
    // VIS-3: the last snapshot id of this connection; a resync snapshot takes the next.
    let mut snapshot_id = 1_u64;
    let mut frames = FrameReader::default();
    let mut liveness = Liveness::new(policy);
    let mut cadence = tokio::time::interval_at(
        tokio::time::Instant::now() + policy.interval,
        policy.interval,
    );
    cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // SPELL-D8 §8.2: only an actor with `ACTOR_VITALS` has a Serene evaluation to run.
    let mut serene = vitals.is_some().then(|| {
        let mut serene = tokio::time::interval_at(
            tokio::time::Instant::now() + SERENE_EVALUATION,
            SERENE_EVALUATION,
        );
        serene.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        serene
    });
    // QUEST-LOG-WIRE-1: only a session with domain 16 looks for a changed quest copy.
    let mut quest_log_refresh = quest_log.is_some().then(|| {
        let mut refresh = tokio::time::interval_at(
            tokio::time::Instant::now() + QUEST_LOG_REFRESH,
            QUEST_LOG_REFRESH,
        );
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        refresh
    });
    // VIS-3: only a session with capability 6 looks for entities that changed without its own
    // step.
    let mut visibility_refresh = visibility.is_some().then(|| {
        let mut refresh = tokio::time::interval_at(
            tokio::time::Instant::now() + VISIBILITY_REFRESH,
            VISIBILITY_REFRESH,
        );
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        refresh
    });
    // SPEED-1 (CONDITIONS-0 §4.3): this connection's pacing clock, its one-step buffer and the
    // frames read while a step waits in it. Those frames run, in order, only after the buffered
    // step's result, because results commit in CommandId order (FND-02 §13); a step among them
    // arrived while the buffer was full and is refused. All of it ends with the connection, so
    // the buffer is dropped at disconnect and a resumed connection starts empty.
    let mut pacer = StepPacer::default();
    let mut buffered: Option<BufferedStep> = None;
    let mut held: std::collections::VecDeque<Vec<u8>> = std::collections::VecDeque::new();
    loop {
        enum Next {
            Frame(std::io::Result<Vec<u8>>),
            /// A frame read while a step waited in the buffer.
            Held(Vec<u8>),
            StepDue,
            Probe,
            Serene,
            QuestLog,
            Visibility,
        }
        // All futures are cancel-safe: the frame reader keeps partial bytes and a dropped
        // interval tick is not consumed. The ticks are polled first so a client that keeps
        // frames flowing cannot starve a cadence; each is ready at most once per interval.
        let held_frame = if buffered.is_none() {
            held.pop_front()
        } else {
            None
        };
        let next = if let Some(frame) = held_frame {
            Next::Held(frame)
        } else {
            let due = buffered.as_ref().map(|step| step.due);
            let reading = held.len() < MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP;
            let mut step_due = std::pin::pin!(async {
                match due {
                    Some(due) => tokio::time::sleep_until(due).await,
                    None => std::future::pending().await,
                }
            });
            let mut read = std::pin::pin!(frames.next(stream));
            let mut tick = std::pin::pin!(cadence.tick());
            let mut serene_tick = std::pin::pin!(async {
                match serene.as_mut() {
                    Some(serene) => serene.tick().await,
                    None => std::future::pending().await,
                }
            });
            let mut quest_log_tick = std::pin::pin!(async {
                match quest_log_refresh.as_mut() {
                    Some(refresh) => refresh.tick().await,
                    None => std::future::pending().await,
                }
            });
            let mut visibility_tick = std::pin::pin!(async {
                match visibility_refresh.as_mut() {
                    Some(refresh) => refresh.tick().await,
                    None => std::future::pending().await,
                }
            });
            std::future::poll_fn(|context| {
                if tick.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::Probe);
                }
                if serene_tick.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::Serene);
                }
                if quest_log_tick.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::QuestLog);
                }
                if visibility_tick.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::Visibility);
                }
                if step_due.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::StepDue);
                }
                if reading && let std::task::Poll::Ready(read) = read.as_mut().poll(context) {
                    return std::task::Poll::Ready(Next::Frame(read));
                }
                std::task::Poll::Pending
            })
            .await
        };
        let (frame, arrived_while_buffered) = match next {
            // SPEED-1: the buffered step is due; its frame runs now, before any frame held
            // behind it.
            Next::StepDue => match buffered.take() {
                Some(step) => (step.frame, false),
                None => continue,
            },
            Next::Held(frame) => (frame, true),
            Next::Serene => {
                let Some((to, value)) =
                    authority.tick_vitals(actor, admitted.game_session_id).await
                else {
                    continue;
                };
                let Some((delta_sequence, delta)) = vitals_delta(generation, sequence, to, &value)
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                sequence = delta_sequence;
                admitted.continuity.server_sequence = sequence;
                if write_frame(stream, &delta).await.is_err() {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                continue;
            }
            Next::QuestLog => {
                // QUEST-LOG-WIRE-1: a committed receipt that changes what domain 16 shows sends
                // a delta; an unchanged or unavailable copy sends nothing.
                let Some(state) = quest_log.as_mut() else {
                    continue;
                };
                let observation = authority
                    .observe_quest_log(actor, admitted.game_session_id, state.version())
                    .await;
                let Ok(changed) = state.refresh(&mut admitted.continuity.quest_log, observation)
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                let Some(domain) = changed else {
                    continue;
                };
                let Some((delta_sequence, frame)) = quest_log_delta(generation, sequence, &domain)
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                sequence = delta_sequence;
                admitted.continuity.server_sequence = sequence;
                if write_frame(stream, &frame).await.is_err() {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                continue;
            }
            Next::Visibility => {
                // VIS-3: entities that moved, entered or left without this actor's step.
                let Some(view) = visibility.as_mut() else {
                    continue;
                };
                let Some(channel) = authority.observe_visible_entities(actor).await else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                let update = view.refresh(&channel, &mut |entities| {
                    attach_spatial_handles(item_view.as_mut(), entities)
                });
                if let Some(item_view) = &item_view {
                    admitted.continuity.item_view = item_view.continuity();
                }
                let Some((frames_out, to_sequence, to_revision)) = update.ok().and_then(|update| {
                    visibility_frames(
                        generation,
                        sequence,
                        revision,
                        &mut snapshot_id,
                        selected_capabilities.as_slice(),
                        &update,
                    )
                }) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                sequence = to_sequence;
                admitted.continuity.server_sequence = sequence;
                for frame in &frames_out {
                    if write_frame(stream, frame).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
                revision = to_revision;
                admitted.continuity.spatial_revision = revision;
                continue;
            }
            Next::Frame(Ok(frame)) if buffered.is_some() => {
                // SPEED-1: read while a step waits in the buffer. A liveness ack is not a command
                // and is answered at once; anything else waits for the buffered step's result.
                let ack = matches!(
                    decode_wire_envelope(&frame),
                    Ok(envelope) if envelope.connection_generation() == generation
                        && envelope.message_type() == MessageType::LivenessAck
                );
                if !ack {
                    held.push_back(frame);
                    continue;
                }
                match decode_wire_envelope(&frame)
                    .and_then(|envelope| envelope.liveness_ack(generation))
                    .and_then(|ack| liveness.ack(ack.probe_id))
                {
                    Ok(()) => continue,
                    Err(error) => return close_admitted(stream, admitted, error).await,
                }
            }
            Next::Frame(Ok(frame)) => (frame, false),
            Next::Frame(Err(_)) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
            Next::Probe => {
                let probe = match liveness.tick() {
                    Ok(probe_id) => encode_liveness_probe(generation, probe_id),
                    Err(LivenessEnd::ControlLost) => {
                        return ConnectionEnd::AdmittedThenControlLost(admitted);
                    }
                    Err(LivenessEnd::ProbesExhausted) => {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                };
                let Ok(probe) = probe else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                // A peer that does not consume even one small probe within a cadence is not
                // in control; an unbounded write would also stall the cadence itself.
                match tokio::time::timeout(policy.interval, write_frame(stream, &probe)).await {
                    Ok(Ok(())) => continue,
                    Ok(Err(_)) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
                    Err(_) => return ConnectionEnd::AdmittedThenControlLost(admitted),
                }
            }
        };
        let command = match decode_wire_envelope(&frame) {
            Err(error) => return close_admitted(stream, admitted, error).await,
            Ok(envelope) if envelope.connection_generation() != generation => {
                return close_admitted(
                    stream,
                    admitted,
                    FoundationProtocolError::StaleConnectionGeneration,
                )
                .await;
            }
            Ok(envelope) if envelope.message_type() == MessageType::LivenessAck => {
                match envelope
                    .liveness_ack(generation)
                    .and_then(|ack| liveness.ack(ack.probe_id))
                {
                    Ok(()) => continue,
                    Err(error) => return close_admitted(stream, admitted, error).await,
                }
            }
            Ok(envelope) if envelope.message_type() != MessageType::ClientCommand => {
                return close_admitted(
                    stream,
                    admitted,
                    FoundationProtocolError::UnknownMessageType,
                )
                .await;
            }
            Ok(envelope) => match envelope.client_command(generation) {
                Ok(command) => command,
                Err(error) => return close_admitted(stream, admitted, error).await,
            },
        };
        if command.command_id != next_command {
            // FND-02 §13.2: a lower ID is never re-executed and no outcome is retained; a
            // higher ID is a gap that names the expected ID.
            let (error, expected) = if command.command_id < next_command {
                (FoundationProtocolError::CommandOutcomeExpired, 0)
            } else {
                (FoundationProtocolError::CommandSequenceGap, next_command)
            };
            let _ = match encode_command_protocol_error(
                error,
                generation,
                command.command_id,
                expected,
            ) {
                Ok(frame) => write_frame(stream, &frame).await,
                Err(_) => Ok(()),
            };
            return ConnectionEnd::AdmittedThenClosed(admitted, error);
        }
        // Unknown command types and malformed payloads have no effect. The result payload
        // belongs to the command type, so an unregistered type gets none. USE_INTENT (command
        // type 2, USE-WIRE-V1 #162 5868482467) is dispatched under the same FND-02 CommandId
        // sequencing this loop already enforces above for every command: a CommandId can be
        // acted on at most once per connection generation, so a replay of the same CommandId
        // never makes a second transition.
        // WORLD_ACTOR_SPELL_CAST_INTENT (command type 3) follows the same discipline: a replayed
        // CommandId expires above, so a retry never casts or pays a second time (SPELL-D3).
        // ACCOUNT_ACHIEVEMENTS_QUERY (command type 10, display contract D223-D228) is a read of
        // the session's own account: the admitted controller's, never the payload's; a session
        // without a controller binding has none and is REJECTED.
        enum Dispatch {
            Step(StepOutcome),
            Use(UseOutcome),
            Spell(SpellCastOutcome),
            Achievements(AccountAchievementsReply),
            /// ITEM-VIEW-1b: `USE` with an item target; the domain 11 delta follows the result.
            UseItem(OpenDecision, Option<ItemViewDelta>),
            /// BAGS-WIRE-1: command 21; the domain 14 delta follows the result.
            ContainerView(ContainerViewOutcome, Option<ItemViewDelta>),
            /// QUEST-LOG-WIRE-1: an accepted command 22; the domain 16 delta follows the result.
            QuestLog(QuestLogDomain),
            Unregistered,
        }
        // CAP-NEG-1: a command type owned by a capability the session did not select is refused
        // like an unregistered one, before any decode or authority call.
        let mut dispatch = if !admitted
            .continuity
            .selected_capabilities
            .command_selected(command.command_type)
        {
            Dispatch::Unregistered
        } else if command.command_type == COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT {
            // SPEED-1 (CONDITIONS-0 §4.3): every player step is paced. An early step waits in the
            // one-step buffer, without a result yet; one that arrived while the buffer was full is
            // refused and nothing moves.
            match decode_step_intent(command.payload) {
                Err(_) => Dispatch::Step(StepOutcome::rejected()),
                Ok(_) if arrived_while_buffered => Dispatch::Step(StepOutcome::too_early()),
                Ok(direction) => {
                    let now = tokio::time::Instant::now();
                    if let Some(due) = pacer.wait_until(now) {
                        buffered = Some(BufferedStep { frame, due });
                        continue;
                    }
                    let (outcome, duration) = authority
                        .paced_step(actor, admitted.game_session_id, direction)
                        .await;
                    // The clock starts when the step committed, so a slow owner turn never
                    // shortens the next step's wait.
                    pacer.record(tokio::time::Instant::now(), duration);
                    Dispatch::Step(outcome)
                }
            }
        } else if command.command_type == COMMAND_TYPE_USE_INTENT {
            match decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                command.payload,
            ) {
                // ITEM-USE-WIRE-1: fields 4 and 5 decode only under capability 15. Until ITEM-USE-1
                // no item is usable on a creature or by its definition, so a use_with fails closed
                // and a use by definition finds nothing to use.
                Ok(UseIntent {
                    use_with: Some(_), ..
                }) => Dispatch::Use(UseOutcome::rejected()),
                Ok(UseIntent {
                    target: UseTarget::ItemByDefinition(_),
                    use_with: None,
                }) => Dispatch::UseItem(OpenDecision::NothingToUse, None),
                // ITEM-VIEW-1b §4.3: a handle that is not live is STALE before any authority
                // call; opening writes nothing.
                Ok(UseIntent {
                    target: UseTarget::Item(handle),
                    use_with: None,
                }) => {
                    let key = item_view.as_ref().and_then(|view| view.resolve(handle));
                    // BAGS-WIRE-1: with capability 14 a container in reach opens in a new view,
                    // and one out of reach is TOO_FAR; anything else takes the corpse path.
                    let container = match (key, &container_views) {
                        (Some(key), Some(_)) => authority.observe_container(actor, key).await,
                        _ => None,
                    };
                    let container = container.filter(|observation| {
                        matches!(
                            observation,
                            ContainerObservation::Container { .. } | ContainerObservation::TooFar
                        )
                    });
                    let observation = match key.filter(|_| container.is_none()) {
                        Some(key) => authority.observe_item_target(actor, key).await,
                        None => None,
                    };
                    match (item_view.as_mut(), key, observation) {
                        (Some(view), Some(key), _) if container.is_some() => {
                            let Some(views) = container_views.as_mut() else {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            };
                            let opened = views.apply(view, PendingOpen::new_view(key), container);
                            admitted.continuity.item_view = view.continuity();
                            let Ok((outcome, delta)) = opened else {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            };
                            Dispatch::UseItem(
                                match outcome {
                                    ContainerViewOutcome::Opened => OpenDecision::Open,
                                    ContainerViewOutcome::TooFar => OpenDecision::TooFar,
                                    ContainerViewOutcome::Stale | ContainerViewOutcome::Closed => {
                                        OpenDecision::StaleState
                                    }
                                    ContainerViewOutcome::NotAContainer
                                    | ContainerViewOutcome::TooManyViews => {
                                        OpenDecision::NothingToUse
                                    }
                                },
                                delta,
                            )
                        }
                        (Some(view), Some(key), Some(observation)) => {
                            let opened = view.open(key, observation);
                            admitted.continuity.item_view = view.continuity();
                            let Ok((decision, delta)) = opened else {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            };
                            Dispatch::UseItem(decision, delta)
                        }
                        _ => Dispatch::UseItem(OpenDecision::StaleState, None),
                    }
                }
                Ok(UseIntent {
                    target: UseTarget::WorldObject(target),
                    use_with: None,
                }) => Dispatch::Use(
                    authority
                        .use_object(
                            actor,
                            UseCommand {
                                game_session_id: admitted.game_session_id,
                                command_id: command.command_id,
                                item_fence: admitted.item_fence,
                            },
                            target,
                        )
                        .await,
                ),
                Err(_) => Dispatch::Use(UseOutcome::rejected()),
            }
        } else if command.command_type == COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT {
            match decode_spell_cast_intent(command.payload) {
                Ok(intent) => Dispatch::Spell(
                    authority
                        .cast_spell(actor, admitted.game_session_id, command.command_id, intent)
                        .await,
                ),
                Err(_) => Dispatch::Spell(SpellCastOutcome::rejected()),
            }
        } else if command.command_type == COMMAND_TYPE_CONTAINER_VIEW_INTENT {
            // BAGS-WIRE-1: non-durable view command. Over BAGS0-RL-04 (10 per second, sliding
            // window) it is REJECTED with an empty payload before decoding, as is a malformed one.
            // The window is per GameSession and travels in the continuity.
            let admitted_rate = match (&container_views, item_view.as_mut()) {
                (Some(_), Some(view)) => {
                    let admitted_rate = view.admit_view_command(tokio::time::Instant::now());
                    admitted.continuity.item_view = view.continuity();
                    admitted_rate
                }
                _ => false,
            };
            match (
                container_views.as_mut().filter(|_| admitted_rate),
                item_view.as_mut(),
                decode_container_view_intent(command.payload),
            ) {
                (Some(views), Some(view), Ok(intent)) => {
                    let decided = match views.plan(view, intent) {
                        Ok(ContainerPlan::Decided(outcome, delta)) => Ok((outcome, delta)),
                        Ok(ContainerPlan::Observe(pending)) => {
                            let observation =
                                authority.observe_container(actor, pending.key()).await;
                            views.apply(view, pending, observation)
                        }
                        Err(error) => Err(error),
                    };
                    admitted.continuity.item_view = view.continuity();
                    let Ok((outcome, delta)) = decided else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    Dispatch::ContainerView(outcome, delta)
                }
                _ => Dispatch::Unregistered,
            }
        } else if command.command_type == COMMAND_TYPE_QUEST_LOG_QUERY {
            // QUEST-LOG-WIRE-1: a read of the session's own quest copy. Over QUESTGATE0-RL-09 (2
            // per second per GameSession, sliding window) it is REJECTED with an empty payload
            // before decoding, as is a malformed query, one naming a quest that is not listed,
            // and any query while the copy is not loaded.
            let admitted_rate = quest_log.is_some()
                && admitted
                    .continuity
                    .quest_log
                    .queries
                    .admit(tokio::time::Instant::now());
            match (
                quest_log.as_mut().filter(|_| admitted_rate),
                decode_quest_log_query(command.payload),
            ) {
                (Some(state), Ok(query)) => {
                    let observation = authority
                        .observe_quest_log(actor, admitted.game_session_id, None)
                        .await;
                    match state.query(&mut admitted.continuity.quest_log, &query, observation) {
                        Some(Ok(domain)) => Dispatch::QuestLog(domain),
                        Some(Err(_)) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
                        None => Dispatch::Unregistered,
                    }
                }
                _ => Dispatch::Unregistered,
            }
        } else if command.command_type == COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY {
            match (
                decode_account_achievements_query(command.payload),
                admitted.controller,
            ) {
                (Ok(query), Some(controller)) => Dispatch::Achievements(
                    authority
                        .account_achievements(AccountAchievementsRequest {
                            account_id: controller.account_id,
                            page: query.page,
                            game_session_id: *admitted.game_session_id.as_bytes(),
                            command_id: command.command_id,
                        })
                        .await,
                ),
                _ => Dispatch::Achievements(AccountAchievementsReply::Rejected),
            }
        } else {
            Dispatch::Unregistered
        };
        let (Some(result_sequence), Some(following)) =
            (sequence.checked_add(1), next_command.checked_add(1))
        else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        sequence = result_sequence;
        next_command = following;
        admitted.continuity.server_sequence = sequence;
        admitted.continuity.next_command_id = next_command;
        let (status, result_payload) = match &mut dispatch {
            Dispatch::Step(outcome) => (
                if matches!(
                    outcome.disposition,
                    StepDisposition::Rejected | StepDisposition::TooEarly
                ) {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                encode_step_outcome(selected_capabilities.as_slice(), outcome.disposition),
            ),
            Dispatch::Use(outcome) => (
                if outcome.disposition == UseDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                encode_use_result(outcome.disposition),
            ),
            Dispatch::Spell(outcome) => (
                if outcome.disposition == SpellCastDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                encode_spell_cast_result(outcome.disposition),
            ),
            Dispatch::UseItem(decision, _) => (
                CommandStatus::Accepted,
                encode_use_result(match decision {
                    OpenDecision::Open => UseDisposition::Committed,
                    OpenDecision::TooFar => UseDisposition::TooFar,
                    OpenDecision::StaleState => UseDisposition::StaleState,
                    OpenDecision::NothingToUse => UseDisposition::NothingToUse,
                }),
            ),
            Dispatch::ContainerView(outcome, _) => (
                CommandStatus::Accepted,
                encode_container_view_result(*outcome),
            ),
            Dispatch::QuestLog(_) => (CommandStatus::Accepted, Vec::new()),
            Dispatch::Achievements(AccountAchievementsReply::Page(payload)) => {
                // The page is written once; move it out instead of copying up to 32 KiB.
                (CommandStatus::Accepted, std::mem::take(payload))
            }
            Dispatch::Achievements(
                AccountAchievementsReply::Terminal(_) | AccountAchievementsReply::Rejected,
            )
            | Dispatch::Unregistered => (CommandStatus::Rejected, Vec::new()),
        };
        let result =
            if let Dispatch::Achievements(AccountAchievementsReply::Terminal(error)) = &dispatch {
                // Display contract §3.3, §4.3: fail closed with the registered operation-terminal
                // error and nothing else.
                encode_command_error_result(generation, sequence, command.command_id, *error)
            } else {
                encode_command_result(
                    generation,
                    sequence,
                    command.command_id,
                    status,
                    &result_payload,
                )
            };
        let Ok(result) = result else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        if write_frame(stream, &result).await.is_err() {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
        match dispatch {
            Dispatch::Step(outcome) => {
                if let Some(observation) = outcome.moved_to {
                    // VIS-3: with capability 6 the step's delta is the interest set's change (or
                    // a resync snapshot); without it, the v1 own-actor delta.
                    let (frames_out, delta_sequence, new_revision) =
                        if let Some(view) = visibility.as_mut() {
                            let Some(mut channel) = authority.observe_visible_entities(actor).await
                            else {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            };
                            // The step's committed position is the observer's own.
                            let observer = channel.observer;
                            for own in channel
                                .entities
                                .iter_mut()
                                .filter(|entity| entity.identity == observer)
                            {
                                own.position = observation.actor_position;
                            }
                            let update = view.refresh(&channel, &mut |entities| {
                                attach_spatial_handles(item_view.as_mut(), entities)
                            });
                            if let Some(item_view) = &item_view {
                                admitted.continuity.item_view = item_view.continuity();
                            }
                            let Some(sent) = update.ok().and_then(|update| {
                                visibility_frames(
                                    generation,
                                    sequence,
                                    revision,
                                    &mut snapshot_id,
                                    selected_capabilities.as_slice(),
                                    &update,
                                )
                            }) else {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            };
                            sent
                        } else {
                            let (Some(delta_sequence), Some(new_revision)) =
                                (sequence.checked_add(1), revision.checked_add(1))
                            else {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            };
                            let Ok(delta) = encode_state_delta(
                                generation,
                                delta_sequence,
                                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                                revision,
                                new_revision,
                                DELTA_TYPE_WORLD_SPATIAL_V1,
                                &encode_world_spatial(&observation),
                            ) else {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            };
                            (vec![delta], delta_sequence, new_revision)
                        };
                    sequence = delta_sequence;
                    admitted.continuity.server_sequence = sequence;
                    for frame in &frames_out {
                        if write_frame(stream, frame).await.is_err() {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        }
                    }
                    // r4122508665: only record the new spatial revision once the delta that
                    // carries it has actually been transmitted (this had the same
                    // before-the-write ordering bug the overlay path did).
                    revision = new_revision;
                    admitted.continuity.spatial_revision = revision;
                    // ITEM-VIEW-1b §4.3: a step out of reach or to another floor closes the
                    // open corpse.
                    if let Some(view) = item_view.as_mut() {
                        let closed = view.close(&CloseTrigger::Moved {
                            to: observation.actor_position,
                        });
                        admitted.continuity.item_view = view.continuity();
                        let Ok(closed) = closed else {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        };
                        if let Some(delta) = closed {
                            let Some((delta_sequence, frame)) =
                                item_view_delta(generation, sequence, &delta)
                            else {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            };
                            sequence = delta_sequence;
                            admitted.continuity.server_sequence = sequence;
                            if write_frame(stream, &frame).await.is_err() {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            }
                        }
                    }
                }
            }
            Dispatch::Use(outcome) => {
                // ITEM-VIEW-1b: a committed `USE` (the chest MINT) may have changed the
                // backpack; its domain 9 delta follows the durable commit, only when the view
                // changed. An unreadable backpack fails closed; the resumed snapshot restores it.
                if outcome.disposition == UseDisposition::Committed
                    && let Some(view) = item_view.as_mut()
                {
                    let Some(inventory) = authority
                        .observe_character_inventory(actor, admitted.game_session_id)
                        .await
                    else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    let changed = view.inventory_committed(inventory);
                    admitted.continuity.item_view = view.continuity();
                    let Ok(changed) = changed else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    if let Some(delta) = changed {
                        let Some((delta_sequence, frame)) =
                            item_view_delta(generation, sequence, &delta)
                        else {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        };
                        sequence = delta_sequence;
                        admitted.continuity.server_sequence = sequence;
                        if write_frame(stream, &frame).await.is_err() {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        }
                    }
                }
                // WORLD_OBJECT_OVERLAY (domain 2, delta type 1) is Channel-global, not
                // per-connection: its "from" revision is always the committed entry's own
                // revision minus one, since `LocalObjectRuntime::attempt_use` only ever commits
                // by advancing its revision by exactly one.
                if let Some(entry) = outcome.committed {
                    let (Some(delta_sequence), Some(prior_revision)) =
                        (sequence.checked_add(1), entry.revision.checked_sub(1))
                    else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    let Ok(payload) = encode_world_object_overlay_delta(&entry) else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    let Ok(delta) = encode_state_delta(
                        generation,
                        delta_sequence,
                        STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                        prior_revision,
                        entry.revision,
                        DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                        &payload,
                    ) else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    sequence = delta_sequence;
                    admitted.continuity.server_sequence = sequence;
                    if write_frame(stream, &delta).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                    // r4122508665: only record the new overlay revision once the delta that
                    // carries it has actually been transmitted.
                    admitted.continuity.overlay_revision = entry.revision;
                }
                // ACHIEVEMENT-0 §5: one delta per committed `Granted`, after the commit, only
                // with capability 8. The revision advances before the write, so a revision
                // that may have reached the client is never reused, even if the write fails.
                // ACH-NOTIFY-2: an `Unknown` notice with capability 8 fails closed after the
                // result; the resumed connection's snapshot restores the watermark.
                let notice_revision = admitted
                    .continuity
                    .achievement_notice_revision
                    .filter(|_| notices_selected);
                if notice_revision.is_some() && outcome.earned == EarnedNotice::Unknown {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                if let (EarnedNotice::Earned(earned), Some(from)) =
                    (outcome.earned, notice_revision)
                {
                    let (Some(delta_sequence), Some(to)) =
                        (sequence.checked_add(1), from.checked_add(1))
                    else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    let Ok(payload) = encode_achievement_earned(&earned) else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    let Ok(delta) = encode_state_delta(
                        generation,
                        delta_sequence,
                        STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
                        from,
                        to,
                        DELTA_TYPE_ACHIEVEMENT_EARNED_V1,
                        &payload,
                    ) else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    sequence = delta_sequence;
                    admitted.continuity.server_sequence = sequence;
                    admitted.continuity.achievement_notice_revision = Some(to);
                    if write_frame(stream, &delta).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
            }
            Dispatch::Spell(outcome) => {
                // ACTOR_VITALS (domain 3, delta type 1) carries the owner's own per-actor
                // revision: a committed cast advances it by exactly one.
                if let Some((to, value)) = outcome.vitals {
                    let Some((delta_sequence, delta)) =
                        vitals_delta(generation, sequence, to, &value)
                    else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    sequence = delta_sequence;
                    admitted.continuity.server_sequence = sequence;
                    if write_frame(stream, &delta).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
            }
            Dispatch::UseItem(_, Some(delta)) | Dispatch::ContainerView(_, Some(delta)) => {
                let Some((delta_sequence, frame)) = item_view_delta(generation, sequence, &delta)
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                sequence = delta_sequence;
                admitted.continuity.server_sequence = sequence;
                if write_frame(stream, &frame).await.is_err() {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
            }
            Dispatch::QuestLog(domain) => {
                let Some((delta_sequence, frame)) = quest_log_delta(generation, sequence, &domain)
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                sequence = delta_sequence;
                admitted.continuity.server_sequence = sequence;
                if write_frame(stream, &frame).await.is_err() {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
            }
            Dispatch::UseItem(_, None)
            | Dispatch::ContainerView(_, None)
            | Dispatch::Achievements(_)
            | Dispatch::Unregistered => {}
        }
    }
}

/// The `ACTOR_VITALS` delta (domain 3, delta type 1) from revision `to - 1` to `to`, at the
/// sequence after `sequence`; `None` on an encoding fault.
fn vitals_delta(
    generation: u64,
    sequence: u64,
    to: u64,
    value: &ActorVitals,
) -> Option<(u64, Vec<u8>)> {
    let delta_sequence = sequence.checked_add(1)?;
    let from = to.checked_sub(1)?;
    let payload = encode_actor_vitals(value).ok()?;
    let delta = encode_state_delta(
        generation,
        delta_sequence,
        STATE_DOMAIN_ACTOR_VITALS,
        from,
        to,
        DELTA_TYPE_ACTOR_VITALS_V1,
        &payload,
    )
    .ok()?;
    Some((delta_sequence, delta))
}

/// One whole-view domain 9 or 11 delta at the sequence after `sequence`; `None` on an encoding
/// fault.
fn item_view_delta(
    generation: u64,
    sequence: u64,
    delta: &ItemViewDelta,
) -> Option<(u64, Vec<u8>)> {
    let delta_sequence = sequence.checked_add(1)?;
    let frame = encode_state_delta(
        generation,
        delta_sequence,
        delta.domain_id,
        delta.from,
        delta.to,
        delta.delta_type,
        &delta.payload,
    )
    .ok()?;
    Some((delta_sequence, frame))
}

/// The whole-domain 16 delta to `domain.to` at the sequence after `sequence`; `None` on an
/// encoding fault.
fn quest_log_delta(
    generation: u64,
    sequence: u64,
    domain: &QuestLogDomain,
) -> Option<(u64, Vec<u8>)> {
    let delta_sequence = sequence.checked_add(1)?;
    let frame = encode_state_delta(
        generation,
        delta_sequence,
        STATE_DOMAIN_QUEST_LOG,
        domain.to.checked_sub(1)?,
        domain.to,
        DELTA_TYPE_QUEST_LOG_V1,
        &domain.payload,
    )
    .ok()?;
    Some((delta_sequence, frame))
}

async fn close_admitted<S: AsyncWrite + Unpin>(
    stream: &mut S,
    admitted: AdmittedSession,
    error: FoundationProtocolError,
) -> ConnectionEnd {
    let _ = send_error(stream, error, admitted.continuity.connection_generation).await;
    ConnectionEnd::AdmittedThenClosed(admitted, error)
}

async fn reject<S: AsyncWrite + Unpin>(
    stream: &mut S,
    error: FoundationProtocolError,
    generation: u64,
) -> ConnectionEnd {
    let _ = send_error(stream, error, generation).await;
    ConnectionEnd::ProtocolViolation(error)
}

async fn send_error<S: AsyncWrite + Unpin>(
    stream: &mut S,
    error: FoundationProtocolError,
    generation: u64,
) -> std::io::Result<()> {
    let frame = encode_protocol_error(error, generation)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    write_frame(stream, &frame).await
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::gameplay_transport::world_spatial::encode_step_result;
    use std::cell::{Cell, RefCell};
    use std::error::Error;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

    const CHARACTER: [u8; 16] = uuid_v7(0x11);
    const SESSION: [u8; 16] = uuid_v7(0x22);
    const WORLD: [u8; 16] = uuid_v7(0x33);
    const CHANNEL: [u8; 16] = uuid_v7(0x44);

    const fn uuid_v7(tag: u8) -> [u8; 16] {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70 | (tag & 0x0f);
        bytes[8] = 0x80 | (tag & 0x3f);
        bytes
    }

    struct Authority {
        calls: Cell<usize>,
        outcome: Result<(), AdmissionRefusal>,
        seen: RefCell<Vec<(CharacterId, Vec<u8>)>>,
    }

    impl Authority {
        fn new(outcome: Result<(), AdmissionRefusal>) -> Self {
            Self {
                calls: Cell::new(0),
                outcome,
                seen: RefCell::new(Vec::new()),
            }
        }
    }

    impl FreshAdmissionAuthority for Authority {
        async fn admit(
            &self,
            attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            self.calls.set(self.calls.get() + 1);
            self.seen
                .borrow_mut()
                .push((attempt.character_id, attempt.admission_material.to_vec()));
            self.outcome.map(|()| AdmittedSession {
                game_session_id: attempt.game_session_id,
                world_id: WorldId::decode(&WORLD).expect("world"),
                channel_id: ChannelId::decode(&CHANNEL).expect("channel"),
                runtime_actor: None,
                first_entry: FirstEntryOutcome::NotApplicable,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            })
        }
    }

    struct Identifiers;
    impl ConnectionIdentifiers for Identifiers {
        fn game_session_id(&self) -> Option<GameSessionId> {
            GameSessionId::decode(&SESSION).ok()
        }
        fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
            AuthenticatedTransportRefV1::decode(&[0x5a; 16]).ok()
        }
    }

    fn varint(output: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            output.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        output.push(value as u8);
    }

    fn scalar(output: &mut Vec<u8>, field: u64, value: u64) {
        varint(output, field << 3);
        varint(output, value);
    }

    fn bytes(output: &mut Vec<u8>, field: u64, value: &[u8]) {
        varint(output, (field << 3) | 2);
        varint(output, value.len() as u64);
        output.extend_from_slice(value);
    }

    fn envelope(message_type: u64, generation: u64, payload: &[u8]) -> Vec<u8> {
        let mut output = Vec::new();
        scalar(&mut output, 1, message_type);
        if generation != 0 {
            scalar(&mut output, 2, generation);
        }
        bytes(&mut output, 4, payload);
        output
    }

    fn bootstrap(major: u64, profile: u64, material: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, major);
        scalar(&mut payload, 2, profile);
        scalar(&mut payload, 3, 1);
        bytes(&mut payload, 5, material);
        bytes(&mut payload, 6, &CHARACTER);
        bytes(&mut payload, 7, b"seam-test");
        envelope(1, 0, &payload)
    }

    fn resume() -> Vec<u8> {
        let mut payload = Vec::new();
        bytes(&mut payload, 1, &SESSION);
        bytes(&mut payload, 2, b"recovery");
        scalar(&mut payload, 4, 1);
        scalar(&mut payload, 5, 1);
        scalar(&mut payload, 6, 1);
        bytes(&mut payload, 8, b"seam-test");
        envelope(3, 0, &payload)
    }

    fn framed(body: &[u8]) -> Vec<u8> {
        let mut output = (body.len() as u32).to_be_bytes().to_vec();
        output.extend_from_slice(body);
        output
    }

    /// Drive one connection with the given client frames; return the end state
    /// and every frame the server wrote.
    async fn drive(
        authority: &Authority,
        client_frames: &[Vec<u8>],
    ) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
        let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 21);
        for frame in client_frames {
            client.write_all(&framed(frame)).await?;
        }
        client.shutdown().await?;
        let end = serve_connection(&mut server, authority, &Identifiers).await;
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        let mut frames = Vec::new();
        let mut cursor = 0;
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
            cursor += 4 + length;
        }
        Ok((end, frames))
    }

    fn run<F: Future<Output = Result<(), Box<dyn Error>>>>(test: F) -> Result<(), Box<dyn Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(test)
    }

    fn error_frame(error: FoundationProtocolError, generation: u64) -> Vec<u8> {
        encode_protocol_error(error, generation).expect("error frame")
    }

    /// A positioned actor on a one-row room: east of the start is walkable, every other
    /// direction is blocked.
    struct StepAuthority {
        steps: RefCell<Vec<StepDirection>>,
    }

    impl FreshAdmissionAuthority for StepAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }

        async fn step(&self, _actor: ExactActorRef, direction: StepDirection) -> StepOutcome {
            self.steps.borrow_mut().push(direction);
            if direction == StepDirection::East {
                StepOutcome {
                    disposition: StepDisposition::Moved,
                    moved_to: Some(at(1)),
                }
            } else {
                StepOutcome {
                    disposition: StepDisposition::Blocked,
                    moved_to: None,
                }
            }
        }
    }

    fn at(x: i32) -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: super::super::world_spatial::ActorPosition { x, y: 0, floor: 0 },
        }
    }

    fn command(generation: u64, id: u64, command_type: u64, direction: StepDirection) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, id);
        scalar(&mut payload, 2, command_type);
        bytes(
            &mut payload,
            4,
            &super::super::world_spatial::encode_step_intent(direction),
        );
        envelope(7, generation, &payload)
    }

    /// A fixture authority for USE_INTENT dispatch (USE-WIRE-V1, #162 5868482467): every call is
    /// recorded and answered with one canned `UseOutcome`, independent of `target`.
    struct UseAuthority {
        uses: RefCell<Vec<super::super::world_object::WorldObjectTarget>>,
        commands: RefCell<Vec<UseCommand>>,
        overlay: Option<WorldObjectOverlayEntry>,
        /// The account watermark the domain-13 snapshot reads.
        watermark: Option<AchievementWatermark>,
        outcome: UseOutcome,
    }

    impl FreshAdmissionAuthority for UseAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }

        async fn observe_world_object_overlay(&self) -> Option<WorldObjectOverlayEntry> {
            self.overlay.clone()
        }

        async fn observe_achievement_notices(
            &self,
            _account_id: [u8; 16],
        ) -> Option<AchievementWatermark> {
            self.watermark
        }

        async fn use_object(
            &self,
            _actor: ExactActorRef,
            command: UseCommand,
            target: super::super::world_object::WorldObjectTarget,
        ) -> UseOutcome {
            self.commands.borrow_mut().push(command);
            self.uses.borrow_mut().push(target);
            self.outcome.clone()
        }
    }

    fn use_command(
        generation: u64,
        id: u64,
        command_type: u64,
        placement: &[u8],
        expected_revision: u64,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, id);
        scalar(&mut payload, 2, command_type);
        let intent = super::super::world_object::encode_use_intent(
            &super::super::world_object::WorldObjectTarget {
                placement: placement.to_vec(),
                expected_revision,
            },
        )
        .expect("encode use intent");
        bytes(&mut payload, 4, &intent);
        envelope(7, generation, &payload)
    }

    /// Serve one positioned admitted session over the given client frames.
    async fn drive_admitted<A: FreshAdmissionAuthority>(
        authority: &A,
        client_frames: &[Vec<u8>],
    ) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
        let world_id = WorldId::decode(&WORLD)?;
        let channel_id = ChannelId::decode(&CHANNEL)?;
        let admitted = AdmittedSession {
            game_session_id: GameSessionId::decode(&SESSION)?,
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        drive_session(authority, admitted, client_frames).await
    }

    /// Serve `admitted` over the given client frames.
    async fn drive_session<A: FreshAdmissionAuthority>(
        authority: &A,
        admitted: AdmittedSession,
        client_frames: &[Vec<u8>],
    ) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
        let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 21);
        for frame in client_frames {
            client.write_all(&framed(frame)).await?;
        }
        client.shutdown().await?;
        let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        let mut frames = Vec::new();
        let mut cursor = 0;
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
            cursor += 4 + length;
        }
        Ok((end, frames))
    }

    fn baseline() -> Vec<Vec<u8>> {
        encode_single_chunk_snapshot(
            ADMITTED_GENERATION,
            1,
            0,
            &[DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &encode_world_spatial(&at(0)),
            }],
        )
        .expect("snapshot")
        .into()
    }

    const FAST: LivenessPolicy = LivenessPolicy {
        interval: std::time::Duration::from_millis(20),
        missed_limit: 3,
    };

    fn ack(generation: u64, probe_id: u64) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, probe_id);
        envelope(6, generation, &payload)
    }

    fn positioned() -> Result<AdmittedSession, Box<dyn Error>> {
        let world_id = WorldId::decode(&WORLD)?;
        let channel_id = ChannelId::decode(&CHANNEL)?;
        Ok(AdmittedSession {
            game_session_id: GameSessionId::decode(&SESSION)?,
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        })
    }

    fn split_frames(output: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        let mut frames = Vec::new();
        let mut cursor = 0;
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
            cursor += 4 + length;
        }
        Ok(frames)
    }

    #[test]
    fn liveness_counts_only_the_current_probe_and_refuses_unsent_ids() {
        let mut liveness = Liveness::new(FAST);
        assert_eq!(
            liveness.ack(1),
            Err(FoundationProtocolError::InvalidWireIdentifier)
        );
        assert_eq!(liveness.tick(), Ok(1));
        assert_eq!(liveness.tick(), Ok(2));
        // A late ack of probe 1 restores nothing; probe 2 is still missed.
        assert_eq!(liveness.ack(1), Ok(()));
        assert_eq!(liveness.tick(), Ok(3));
        assert_eq!(liveness.ack(3), Ok(()));
        assert_eq!(liveness.ack(3), Ok(()));
        assert_eq!(
            liveness.ack(0),
            Err(FoundationProtocolError::InvalidWireIdentifier)
        );
        // After an answered probe the count restarts: three more misses are needed.
        assert_eq!(liveness.tick(), Ok(4));
        assert_eq!(liveness.tick(), Ok(5));
        assert_eq!(liveness.tick(), Ok(6));
        assert_eq!(liveness.tick(), Err(LivenessEnd::ControlLost));
        let mut exhausted = Liveness::new(FAST);
        exhausted.last_sent = u64::MAX;
        assert_eq!(exhausted.tick(), Err(LivenessEnd::ProbesExhausted));
    }

    #[test]
    fn silent_admitted_client_loses_control_after_the_missed_limit() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
            };
            let (mut server, mut client) = tokio::io::duplex(1 << 16);
            // The client keeps its transport open but never answers.
            let end = serve_admitted(&mut server, positioned()?, &authority, FAST).await;
            assert!(matches!(end, ConnectionEnd::AdmittedThenControlLost(_)));
            drop(server);
            let mut output = Vec::new();
            client.read_to_end(&mut output).await?;
            let mut expected = baseline();
            for probe_id in 1..=3 {
                expected.push(encode_liveness_probe(ADMITTED_GENERATION, probe_id)?);
            }
            assert_eq!(split_frames(&output)?, expected);
            Ok(())
        })
    }

    #[test]
    fn answering_client_keeps_control_and_steps_between_probes() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
            };
            let (mut server, client) = tokio::io::duplex(1 << 16);
            let (mut client_read, mut client_write) = tokio::io::split(client);
            let served = serve_admitted(&mut server, positioned()?, &authority, FAST);
            let client = async {
                let mut frames = super::super::tcp_tls::FrameReader::default();
                let mut probes = 0;
                while probes < 6 {
                    let frame = frames.next(&mut client_read).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    if envelope.message_type() == MessageType::LivenessProbe {
                        probes += 1;
                        // Answer each probe as it arrives; a step between probes is served.
                        client_write.write_all(&framed(&ack(1, probes))).await?;
                        if probes == 3 {
                            client_write
                                .write_all(&framed(&command(
                                    1,
                                    1,
                                    u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
                                    StepDirection::East,
                                )))
                                .await?;
                        }
                    }
                }
                client_write.shutdown().await?;
                Ok::<_, Box<dyn Error>>(())
            };
            let (mut served, mut client) = (std::pin::pin!(served), std::pin::pin!(client));
            let (mut end, mut client_done) = (None, None);
            std::future::poll_fn(|context| {
                if end.is_none()
                    && let std::task::Poll::Ready(value) = served.as_mut().poll(context)
                {
                    end = Some(value);
                }
                if client_done.is_none()
                    && let std::task::Poll::Ready(value) = client.as_mut().poll(context)
                {
                    client_done = Some(value);
                }
                if end.is_some() && client_done.is_some() {
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            })
            .await;
            let (Some(end), Some(client)) = (end, client_done) else {
                return Err("join incomplete".into());
            };
            client?;
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            assert_eq!(*authority.steps.borrow(), [StepDirection::East]);
            Ok(())
        })
    }

    #[test]
    fn flooding_stale_acks_cannot_starve_the_cadence() -> Result<(), Box<dyn Error>> {
        use std::io::{Read, Write};
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
            };
            let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
            let address = listener.local_addr()?;
            // A separate thread writes faster than the server reads, as a real flooding
            // client does: after probe 1 it repeats acks of probe 1 and never answers a later
            // probe.
            let flooder = std::thread::spawn(move || -> std::io::Result<()> {
                let mut socket = std::net::TcpStream::connect(address)?;
                let mut seen = Vec::new();
                let probe = framed(&encode_liveness_probe(ADMITTED_GENERATION, 1).map_err(
                    |error| std::io::Error::new(std::io::ErrorKind::InvalidData, error),
                )?);
                let mut chunk = [0_u8; 4096];
                while !seen
                    .windows(probe.len())
                    .any(|window| window == probe.as_slice())
                {
                    let read = socket.read(&mut chunk)?;
                    if read == 0 {
                        return Ok(());
                    }
                    seen.extend_from_slice(&chunk[..read]);
                }
                let stale: Vec<u8> = (0..512).flat_map(|_| framed(&ack(1, 1))).collect();
                while socket.write_all(&stale).is_ok() {}
                Ok(())
            });
            let (stream, _) = listener.accept()?;
            stream.set_nonblocking(true)?;
            let mut stream = tokio::net::TcpStream::from_std(stream)?;
            let end = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                serve_admitted(&mut stream, positioned()?, &authority, FAST),
            )
            .await
            .map_err(|_| "the flood starved the liveness cadence")?;
            assert!(matches!(end, ConnectionEnd::AdmittedThenControlLost(_)));
            drop(stream);
            let _ = flooder.join();
            Ok(())
        })
    }

    #[test]
    fn ack_of_an_unsent_probe_closes_the_admitted_session() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
            };
            let (end, frames) = drive_admitted(&authority, &[ack(1, 9)]).await?;
            let mut expected = baseline();
            expected.push(error_frame(
                FoundationProtocolError::InvalidWireIdentifier,
                1,
            ));
            assert_eq!(frames, expected);
            assert!(matches!(
                end,
                ConnectionEnd::AdmittedThenClosed(
                    _,
                    FoundationProtocolError::InvalidWireIdentifier
                )
            ));
            Ok(())
        })
    }

    #[test]
    fn idle_liveness_matches_the_registered_provisional_rows() -> Result<(), Box<dyn Error>> {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
        ))?;
        let row = |id: &str| {
            registry["entries"]
                .as_array()
                .and_then(|entries| entries.iter().find(|entry| entry["id"] == id))
                .and_then(|entry| entry["hard_maximum"].as_u64())
        };
        assert_eq!(
            row("FND04B-LIVENESS-IDLE-PROBE-MS"),
            u64::try_from(IDLE_LIVENESS.interval.as_millis()).ok()
        );
        assert_eq!(
            row("FND04B-LIVENESS-IDLE-MISSED"),
            Some(u64::from(IDLE_LIVENESS.missed_limit))
        );
        assert_eq!(row("FND04B-LIVENESS-COMBAT-PROBE-MS"), Some(1_000));
        assert_eq!(row("FND04B-LIVENESS-COMBAT-MISSED"), Some(2));
        assert_eq!(
            row("FND04B-SAME-SESSION-GRACE-S"),
            u64::try_from(super::super::SAME_SESSION_GRACE_SECONDS).ok()
        );
        Ok(())
    }

    #[test]
    fn admitted_steps_are_sequenced_and_a_replayed_id_expires() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
            };
            let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
            let (end, frames) = drive_admitted(
                &authority,
                &[
                    command(1, 1, step, StepDirection::East),
                    command(1, 2, step, StepDirection::North),
                    command(1, 3, 0x7fff, StepDirection::East),
                    command(1, 2, step, StepDirection::East),
                ],
            )
            .await?;
            let mut expected = baseline();
            let moved = encode_step_result(StepDisposition::Moved);
            let blocked = encode_step_result(StepDisposition::Blocked);
            expected.extend([
                encode_command_result(1, 1, 1, CommandStatus::Accepted, &moved)?,
                encode_state_delta(
                    1,
                    2,
                    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    1,
                    2,
                    DELTA_TYPE_WORLD_SPATIAL_V1,
                    &encode_world_spatial(&at(1)),
                )?,
                encode_command_result(1, 3, 2, CommandStatus::Accepted, &blocked)?,
                encode_command_result(1, 4, 3, CommandStatus::Rejected, &[])?,
                encode_command_protocol_error(
                    FoundationProtocolError::CommandOutcomeExpired,
                    1,
                    2,
                    0,
                )?,
            ]);
            assert_eq!(frames, expected);
            let ConnectionEnd::AdmittedThenClosed(
                ended,
                FoundationProtocolError::CommandOutcomeExpired,
            ) = end
            else {
                return Err(format!("unexpected end {end:?}").into());
            };
            // The ended session carries the FND-02 continuity a same-session recovery
            // resumes from: next CommandId 4, server_sequence 4, spatial revision 2.
            assert_eq!(
                ended.continuity,
                SessionContinuity {
                    connection_generation: 1,
                    next_command_id: 4,
                    server_sequence: 4,
                    spatial_revision: 2,
                    overlay_revision: 0,
                    achievement_notice_revision: None,
                    selected_capabilities: SelectedCapabilities::NONE,
                    item_view: ItemViewContinuity::default(),
                    quest_log: QuestLogContinuity::FRESH,
                }
            );
            // The unregistered type and the replayed ID never reached Movement.
            assert_eq!(
                *authority.steps.borrow(),
                [StepDirection::East, StepDirection::North]
            );
            Ok(())
        })
    }

    /// SPEED-1: an East step moves and paces the next step by `PACE`; every other direction is
    /// blocked and leaves the clock unchanged. Each executed step is recorded with its instant;
    /// it then takes `turn` before it commits, as a contended owner turn would.
    struct PacedAuthority {
        steps: RefCell<Vec<(StepDirection, tokio::time::Instant)>>,
        turn: std::time::Duration,
    }

    const PACE: std::time::Duration = std::time::Duration::from_millis(80);

    impl FreshAdmissionAuthority for PacedAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }

        async fn paced_step(
            &self,
            _actor: ExactActorRef,
            _session: GameSessionId,
            direction: StepDirection,
        ) -> (StepOutcome, Option<std::time::Duration>) {
            self.steps
                .borrow_mut()
                .push((direction, tokio::time::Instant::now()));
            tokio::time::sleep(self.turn).await;
            if direction == StepDirection::East {
                let moved = StepOutcome {
                    disposition: StepDisposition::Moved,
                    moved_to: Some(at(1)),
                };
                (moved, Some(PACE))
            } else {
                (
                    StepOutcome {
                        disposition: StepDisposition::Blocked,
                        moved_to: None,
                    },
                    None,
                )
            }
        }
    }

    /// A positioned session that selected `capabilities` from the production offered set.
    fn paced_session(capabilities: &[u32]) -> Result<AdmittedSession, Box<dyn Error>> {
        let mut admitted = positioned()?;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, capabilities)
                .ok_or("bounded")?;
        Ok(admitted)
    }

    /// Serve `admitted`, write `client_frames` at once, and close the client side only after
    /// the server has written `wanted` frames past the snapshot: a buffered step needs the
    /// connection to stay open until it is due.
    async fn drive_open<A: FreshAdmissionAuthority>(
        authority: &A,
        admitted: AdmittedSession,
        client_frames: &[Vec<u8>],
        wanted: usize,
    ) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
        let (mut server, client) = tokio::io::duplex(1 << 21);
        let (mut client_read, mut client_write) = tokio::io::split(client);
        let snapshot = baseline().len();
        let served = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS);
        let client = async {
            for frame in client_frames {
                client_write.write_all(&framed(frame)).await?;
            }
            let mut reader = super::super::tcp_tls::FrameReader::default();
            let mut frames = Vec::new();
            while frames.len() < snapshot + wanted {
                frames.push(reader.next(&mut client_read).await?);
            }
            client_write.shutdown().await?;
            Ok::<_, Box<dyn Error>>(frames)
        };
        let (mut served, mut client) = (std::pin::pin!(served), std::pin::pin!(client));
        let (mut end, mut frames) = (None, None);
        std::future::poll_fn(|context| {
            if end.is_none()
                && let std::task::Poll::Ready(value) = served.as_mut().poll(context)
            {
                end = Some(value);
            }
            if frames.is_none()
                && let std::task::Poll::Ready(value) = client.as_mut().poll(context)
            {
                frames = Some(value);
            }
            if end.is_some() && frames.is_some() {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        })
        .await;
        let (Some(end), Some(frames)) = (end, frames) else {
            return Err("join incomplete".into());
        };
        Ok((end, frames?))
    }

    #[test]
    fn a_buffered_step_runs_when_due_and_a_second_early_step_is_refused()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
            let moved = encode_step_result(StepDisposition::Moved);
            let delta = |sequence: u64, from: u64| {
                encode_state_delta(
                    1,
                    sequence,
                    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    from,
                    from + 1,
                    DELTA_TYPE_WORLD_SPATIAL_V1,
                    &encode_world_spatial(&at(1)),
                )
            };
            // With capability 13 the refusal is TOO_EARLY; without it, REJECTED. Either way it
            // is a REJECTED command and nothing moves.
            for (capabilities, refused) in [
                (&[13][..], StepDisposition::TooEarly),
                (&[][..], StepDisposition::Rejected),
            ] {
                let authority = PacedAuthority {
                    steps: RefCell::new(Vec::new()),
                    turn: std::time::Duration::ZERO,
                };
                let (end, frames) = drive_open(
                    &authority,
                    paced_session(capabilities)?,
                    &[
                        command(1, 1, step, StepDirection::East),
                        command(1, 2, step, StepDirection::East),
                        command(1, 3, step, StepDirection::East),
                    ],
                    5,
                )
                .await?;
                let mut expected = baseline();
                expected.extend([
                    encode_command_result(1, 1, 1, CommandStatus::Accepted, &moved)?,
                    delta(2, 1)?,
                    encode_command_result(1, 3, 2, CommandStatus::Accepted, &moved)?,
                    delta(4, 2)?,
                    encode_command_result(
                        1,
                        5,
                        3,
                        CommandStatus::Rejected,
                        &encode_step_result(refused),
                    )?,
                ]);
                assert_eq!(frames, expected);
                assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
                // The first step ran at once; the buffered one only after its duration; the
                // third never reached Movement.
                let steps = authority.steps.borrow();
                assert_eq!(steps.len(), 2);
                // (The clock starts just before the first step is recorded.)
                let waited = steps[1].1.duration_since(steps[0].1);
                assert!(
                    waited >= PACE - std::time::Duration::from_millis(5),
                    "{waited:?}"
                );
            }
            Ok(())
        })
    }

    #[test]
    fn a_slow_owner_turn_does_not_shorten_the_wait_for_the_next_step() -> Result<(), Box<dyn Error>>
    {
        run(async {
            let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
            // The owner turn takes a whole step duration: the next step still waits a full
            // duration after the first one committed, never running back to back.
            let authority = PacedAuthority {
                steps: RefCell::new(Vec::new()),
                turn: PACE,
            };
            let (_, frames) = drive_open(
                &authority,
                paced_session(&[13])?,
                &[
                    command(1, 1, step, StepDirection::East),
                    command(1, 2, step, StepDirection::East),
                ],
                4,
            )
            .await?;
            assert_eq!(frames.len(), baseline().len() + 4);
            let steps = authority.steps.borrow();
            assert_eq!(steps.len(), 2);
            let committed = steps[0].1 + PACE;
            let waited = steps[1].1.duration_since(committed);
            assert!(
                waited >= PACE - std::time::Duration::from_millis(5),
                "{waited:?}"
            );
            Ok(())
        })
    }

    #[test]
    fn a_step_after_the_duration_runs_at_once_and_a_blocked_step_does_not_pace()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
            let authority = PacedAuthority {
                steps: RefCell::new(Vec::new()),
                turn: std::time::Duration::ZERO,
            };
            // Blocked, blocked, then a move: none waits, because only a moved step paces.
            let (_, frames) = drive_open(
                &authority,
                paced_session(&[13])?,
                &[
                    command(1, 1, step, StepDirection::North),
                    command(1, 2, step, StepDirection::West),
                    command(1, 3, step, StepDirection::East),
                ],
                4,
            )
            .await?;
            assert_eq!(frames.len(), baseline().len() + 4);
            let steps = authority.steps.borrow();
            assert_eq!(steps.len(), 3);
            assert!(steps[2].1.duration_since(steps[0].1) < PACE);
            Ok(())
        })
    }

    #[test]
    fn the_buffer_is_dropped_at_disconnect_and_a_resumed_connection_starts_empty()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
            let authority = PacedAuthority {
                steps: RefCell::new(Vec::new()),
                turn: std::time::Duration::ZERO,
            };
            // The client closes while the second step waits in the buffer.
            let (end, frames) = drive_session(
                &authority,
                paced_session(&[13])?,
                &[
                    command(1, 1, step, StepDirection::East),
                    command(1, 2, step, StepDirection::East),
                ],
            )
            .await?;
            assert_eq!(frames.len(), baseline().len() + 2);
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err(format!("unexpected end {end:?}").into());
            };
            assert_eq!(authority.steps.borrow().len(), 1);
            // The buffered step never got a result: CommandId 2 is still the next one.
            assert_eq!(ended.continuity.next_command_id, 2);
            // The resumed connection (a newer generation, same continuity) has no buffer and
            // no pacing clock: its first step runs at once, inside the old duration.
            let mut resumed = ended;
            resumed.continuity.connection_generation = 2;
            let (_, frames) = drive_open(
                &authority,
                resumed,
                &[command(2, 2, step, StepDirection::East)],
                2,
            )
            .await?;
            assert_eq!(
                frames.get(baseline().len()),
                Some(&encode_command_result(
                    2,
                    3,
                    2,
                    CommandStatus::Accepted,
                    &encode_step_result(StepDisposition::Moved)
                )?)
            );
            let steps = authority.steps.borrow();
            assert_eq!(steps.len(), 2);
            assert!(steps[1].1.duration_since(steps[0].1) < PACE);
            Ok(())
        })
    }

    #[test]
    fn admitted_session_closes_on_gap_foreign_message_and_stale_generation()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
            for (frame, error, expected) in [
                (
                    command(1, 3, step, StepDirection::East),
                    FoundationProtocolError::CommandSequenceGap,
                    encode_command_protocol_error(
                        FoundationProtocolError::CommandSequenceGap,
                        1,
                        3,
                        1,
                    )?,
                ),
                (
                    envelope(99, 1, &[]),
                    FoundationProtocolError::UnknownMessageType,
                    error_frame(FoundationProtocolError::UnknownMessageType, 1),
                ),
                (
                    command(2, 1, step, StepDirection::East),
                    FoundationProtocolError::StaleConnectionGeneration,
                    error_frame(FoundationProtocolError::StaleConnectionGeneration, 1),
                ),
            ] {
                let authority = StepAuthority {
                    steps: RefCell::new(Vec::new()),
                };
                let (end, frames) = drive_admitted(&authority, &[frame]).await?;
                let mut all = baseline();
                all.push(expected);
                assert_eq!(frames, all);
                assert!(matches!(end, ConnectionEnd::AdmittedThenClosed(_, e) if e == error));
                assert!(authority.steps.borrow().is_empty());
            }
            Ok(())
        })
    }

    #[test]
    fn admission_accepts_only_after_the_owning_authority_commits() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = Authority::new(Ok(()));
            let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
            let admitted = AdmittedSession {
                game_session_id: GameSessionId::decode(&SESSION)?,
                world_id: WorldId::decode(&WORLD)?,
                channel_id: ChannelId::decode(&CHANNEL)?,
                runtime_actor: None,
                first_entry: FirstEntryOutcome::NotApplicable,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            };
            assert_eq!(end, ConnectionEnd::AdmittedThenDisconnected(admitted));
            assert_eq!(authority.calls.get(), 1);
            assert_eq!(
                authority.seen.borrow().as_slice(),
                &[(CharacterId::decode(&CHARACTER)?, b"grant".to_vec())]
            );
            let accepted = decode_wire_envelope(&frames[0])?;
            assert_eq!(accepted.message_type(), MessageType::ServerAccepted);
            assert_eq!(accepted.connection_generation(), 0);
            assert_eq!(frames.len(), 1);
            Ok(())
        })
    }

    #[test]
    fn admission_refusal_writes_no_acceptance() -> Result<(), Box<dyn Error>> {
        run(async {
            for refusal in [AdmissionRefusal::Rejected, AdmissionRefusal::Unavailable] {
                let authority = Authority::new(Err(refusal));
                let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
                assert_eq!(end, ConnectionEnd::AdmissionRefused(refusal));
                assert_eq!(authority.calls.get(), 1);
                assert!(frames.is_empty());
            }
            Ok(())
        })
    }

    #[test]
    fn admission_version_and_profile_rejected_before_authority() -> Result<(), Box<dyn Error>> {
        run(async {
            for (frame, error) in [
                (
                    bootstrap(2, 1, b"grant"),
                    FoundationProtocolError::ProtocolMajorMismatch,
                ),
                (
                    bootstrap(1, 2, b"grant"),
                    FoundationProtocolError::TransportProfileMismatch,
                ),
            ] {
                let authority = Authority::new(Ok(()));
                let (end, frames) = drive(&authority, &[frame]).await?;
                assert_eq!(end, ConnectionEnd::ProtocolViolation(error));
                assert_eq!(authority.calls.get(), 0);
                assert_eq!(frames, vec![error_frame(error, 0)]);
            }
            Ok(())
        })
    }

    #[test]
    fn admission_phase_direction_and_malformed_input_rejected_before_authority()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let command = envelope(7, 0, &[]);
            let server_direction = envelope(2, 0, &[]);
            let unknown = envelope(99, 0, &[]);
            let generation = {
                let mut payload = Vec::new();
                scalar(&mut payload, 1, 1);
                scalar(&mut payload, 2, 1);
                scalar(&mut payload, 3, 1);
                bytes(&mut payload, 5, b"grant");
                bytes(&mut payload, 6, &CHARACTER);
                bytes(&mut payload, 7, b"seam-test");
                envelope(1, 3, &payload)
            };
            for frame in [command, server_direction, unknown, generation, vec![0xff]] {
                let authority = Authority::new(Ok(()));
                let (end, frames) = drive(&authority, &[frame]).await?;
                assert!(
                    matches!(end, ConnectionEnd::ProtocolViolation(_)),
                    "{end:?}"
                );
                assert_eq!(authority.calls.get(), 0);
                assert_eq!(frames.len(), 1);
                assert_eq!(
                    decode_wire_envelope(&frames[0])?.message_type(),
                    MessageType::ProtocolError
                );
            }
            Ok(())
        })
    }

    #[test]
    fn admission_resume_is_not_served_and_admits_nothing() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = Authority::new(Ok(()));
            let (end, frames) = drive(&authority, &[resume()]).await?;
            assert_eq!(end, ConnectionEnd::ResumeUnavailable);
            assert_eq!(authority.calls.get(), 0);
            assert!(frames.is_empty());
            Ok(())
        })
    }

    #[test]
    fn admission_post_admission_input_closes_without_gameplay() -> Result<(), Box<dyn Error>> {
        run(async {
            for (frame, error) in [
                (
                    envelope(7, 1, &[]),
                    FoundationProtocolError::UnknownMessageType,
                ),
                (
                    envelope(7, 2, &[]),
                    FoundationProtocolError::StaleConnectionGeneration,
                ),
            ] {
                let authority = Authority::new(Ok(()));
                let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant"), frame]).await?;
                let ConnectionEnd::AdmittedThenClosed(_, reported) = end else {
                    return Err(format!("{end:?}").into());
                };
                assert_eq!(reported, error);
                assert_eq!(authority.calls.get(), 1);
                assert_eq!(frames.len(), 2);
                assert_eq!(frames[1], error_frame(error, ADMITTED_GENERATION));
            }
            Ok(())
        })
    }

    /// USE-WIRE-V1 (#162 5868482467): the join snapshot carries `WORLD_OBJECT_OVERLAY` (domain
    /// 2, snapshot type 1) alongside `WORLD_SPATIAL_VISIBILITY`, and a COMMITTED `USE_INTENT`
    /// gets a `CommandResult` followed by a sequenced `WORLD_OBJECT_OVERLAY` delta (domain 2,
    /// delta type 1) whose `from`/`to` revision matches the committed entry's own revision.
    #[test]
    fn admitted_use_commits_and_the_join_snapshot_carries_the_door_overlay()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let overlay = WorldObjectOverlayEntry {
                content_generation: [0x11; 32],
                placement: b"oteryn:cell/entry-door".to_vec(),
                state: b"oteryn:reference.state.closed".to_vec(),
                revision: 0,
            };
            let committed = WorldObjectOverlayEntry {
                content_generation: [0x11; 32],
                placement: b"oteryn:cell/entry-door".to_vec(),
                state: b"oteryn:reference.state.open".to_vec(),
                revision: 1,
            };
            let authority = UseAuthority {
                uses: RefCell::new(Vec::new()),
                commands: RefCell::new(Vec::new()),
                overlay: Some(overlay.clone()),
                watermark: None,
                outcome: UseOutcome {
                    disposition: UseDisposition::Committed,
                    committed: Some(committed.clone()),
                    earned: EarnedNotice::NoneEarned,
                },
            };
            let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
            let (end, frames) = drive_admitted(
                &authority,
                &[use_command(1, 1, use_type, b"oteryn:cell/entry-door", 0)],
            )
            .await?;
            let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
                ADMITTED_GENERATION,
                1,
                0,
                &[
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                        payload: &encode_world_spatial(&at(0)),
                    },
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                        revision: 0,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                        payload: &encode_world_object_overlay_snapshot(&[overlay])
                            .expect("encode overlay snapshot"),
                    },
                ],
            )?
            .into();
            expected.extend([
                encode_command_result(
                    1,
                    1,
                    1,
                    CommandStatus::Accepted,
                    &encode_use_result(UseDisposition::Committed),
                )?,
                encode_state_delta(
                    1,
                    2,
                    STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    0,
                    1,
                    DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    &encode_world_object_overlay_delta(&committed).expect("encode delta"),
                )?,
            ]);
            assert_eq!(frames, expected);
            assert_eq!(authority.uses.borrow().len(), 1);
            assert_eq!(
                authority.uses.borrow()[0],
                super::super::world_object::WorldObjectTarget {
                    placement: b"oteryn:cell/entry-door".to_vec(),
                    expected_revision: 0,
                }
            );
            // r4122215795: the ended session's continuity records the last
            // WORLD_OBJECT_OVERLAY revision emitted (here, the committed delta's own
            // revision 1) — the same field a reconnect's FND-02 fence must carry next to
            // `spatial_revision` (`resume.rs`, out of this seam's owned paths).
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err(format!("unexpected end {end:?}").into());
            };
            assert_eq!(ended.continuity.overlay_revision, 1);
            Ok(())
        })
    }

    /// r4122215795: the join snapshot alone (before any `USE_INTENT`) already sets the ended
    /// session's continuity to the current overlay revision it was actually sent, not left at
    /// its prior/default value.
    #[test]
    fn admitted_join_snapshot_alone_records_the_overlay_revision_it_sent()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let overlay = WorldObjectOverlayEntry {
                content_generation: [0x22; 32],
                placement: b"oteryn:cell/entry-door".to_vec(),
                state: b"oteryn:reference.state.closed".to_vec(),
                revision: 5,
            };
            let authority = UseAuthority {
                uses: RefCell::new(Vec::new()),
                commands: RefCell::new(Vec::new()),
                overlay: Some(overlay),
                watermark: None,
                outcome: UseOutcome::rejected(),
            };
            let (end, _frames) = drive_admitted(&authority, &[]).await?;
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err(format!("unexpected end {end:?}").into());
            };
            assert_eq!(ended.continuity.overlay_revision, 5);
            Ok(())
        })
    }

    /// Wraps a stream so its `poll_write` deterministically fails once `ok_writes` successful
    /// calls have gone through the inner stream, then fails every call after. Reads are
    /// unaffected. Used to prove a specific frame's transmission fails, without racing a real
    /// duplex's buffering/timing.
    struct FailNthWrite<S> {
        inner: S,
        ok_writes: usize,
    }

    impl<S: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for FailNthWrite<S> {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            context: &mut std::task::Context<'_>,
            buffer: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let this = self.get_mut();
            std::pin::Pin::new(&mut this.inner).poll_read(context, buffer)
        }
    }

    impl<S: AsyncWrite + Unpin> AsyncWrite for FailNthWrite<S> {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            context: &mut std::task::Context<'_>,
            buffer: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            let this = self.get_mut();
            if this.ok_writes == 0 {
                return std::task::Poll::Ready(Err(std::io::Error::other(
                    "injected write failure",
                )));
            }
            this.ok_writes -= 1;
            std::pin::Pin::new(&mut this.inner).poll_write(context, buffer)
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            context: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let this = self.get_mut();
            std::pin::Pin::new(&mut this.inner).poll_flush(context)
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            context: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let this = self.get_mut();
            std::pin::Pin::new(&mut this.inner).poll_shutdown(context)
        }
    }

    /// r4122508665: a write failing *after* the committed overlay entry has already been
    /// encoded must not update `continuity.overlay_revision`. `write_frame` issues exactly two
    /// low-level writes per frame (the length prefix, then the body); the join snapshot (3
    /// frames) plus the `USE_INTENT`'s own `CommandResult` (1 frame) are let through in full (8
    /// writes), and the 5th frame — the `WORLD_OBJECT_OVERLAY` `StateDelta` — is made to fail on
    /// its very first write. `continuity.overlay_revision` must still hold the value from before
    /// this connection (the FRESH baseline, 0), never the committed entry's revision (7) that
    /// was never confirmed delivered.
    #[test]
    fn admitted_use_commit_write_failure_does_not_record_the_overlay_revision()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let committed = WorldObjectOverlayEntry {
                content_generation: [0x44; 32],
                placement: b"oteryn:cell/entry-door".to_vec(),
                state: b"oteryn:reference.state.open".to_vec(),
                revision: 7,
            };
            let authority = UseAuthority {
                uses: RefCell::new(Vec::new()),
                commands: RefCell::new(Vec::new()),
                overlay: None,
                watermark: None,
                outcome: UseOutcome {
                    disposition: UseDisposition::Committed,
                    committed: Some(committed),
                    earned: EarnedNotice::NoneEarned,
                },
            };
            let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
            let world_id = WorldId::decode(&WORLD)?;
            let channel_id = ChannelId::decode(&CHANNEL)?;
            let admitted = AdmittedSession {
                game_session_id: GameSessionId::decode(&SESSION)?,
                world_id,
                channel_id,
                runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
                first_entry: FirstEntryOutcome::Positioned,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            };
            let (server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 21);
            client
                .write_all(&framed(&use_command(
                    1,
                    1,
                    use_type,
                    b"oteryn:cell/entry-door",
                    0,
                )))
                .await?;
            let mut server = FailNthWrite {
                inner: server,
                ok_writes: 8,
            };
            let end = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err(format!("unexpected end {end:?}").into());
            };
            assert_eq!(ended.continuity.overlay_revision, 0);
            Ok(())
        })
    }

    /// C2: the dispatch hands `use_object` the `USE`'s own CommandId and GameSession and the
    /// admitted session's item fence, and a COMMITTED `USE` without an overlay entry (a chest's
    /// MINT) gets its `CommandResult` and no `WORLD_OBJECT_OVERLAY` delta.
    #[test]
    fn admitted_use_passes_command_identity_and_fence_and_chest_commit_emits_no_delta()
    -> Result<(), Box<dyn Error>> {
        use crate::foundation::{
            ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration,
        };
        run(async {
            let authority = UseAuthority {
                uses: RefCell::new(Vec::new()),
                commands: RefCell::new(Vec::new()),
                overlay: None,
                watermark: None,
                outcome: UseOutcome {
                    disposition: UseDisposition::Committed,
                    committed: None,
                    earned: EarnedNotice::NoneEarned,
                },
            };
            let world_id = WorldId::decode(&WORLD)?;
            let channel_id = ChannelId::decode(&CHANNEL)?;
            let game_session_id = GameSessionId::decode(&SESSION)?;
            let fence = CurrentCharacterItemFence {
                character_id: crate::domain::CharacterId::from_bytes(uuid_v7(0x55))
                    .map_err(|error| format!("{error:?}"))?,
                game_session_id,
                connection_generation: ConnectionGeneration::new(1)
                    .map_err(|error| format!("{error:?}"))?,
                character_lease_generation: 3,
                runtime_scope: RuntimeScopeRefV1::channel(world_id, channel_id),
                scope_ownership_generation: ScopeOwnershipGeneration::new(4)
                    .map_err(|error| format!("{error:?}"))?,
            };
            let admitted = AdmittedSession {
                game_session_id,
                world_id,
                channel_id,
                runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
                first_entry: FirstEntryOutcome::Positioned,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: Some(fence),
            };
            let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
            let (_end, frames) = drive_session(
                &authority,
                admitted,
                &[use_command(
                    1,
                    1,
                    use_type,
                    b"oteryn:placement/entry-chest",
                    0,
                )],
            )
            .await?;
            let mut expected = baseline();
            expected.push(encode_command_result(
                1,
                1,
                1,
                CommandStatus::Accepted,
                &encode_use_result(UseDisposition::Committed),
            )?);
            assert_eq!(frames, expected);
            assert_eq!(
                authority.commands.borrow().as_slice(),
                &[UseCommand {
                    game_session_id,
                    command_id: 1,
                    item_fence: Some(fence),
                }]
            );
            Ok(())
        })
    }

    /// A non-committing `USE_INTENT` disposition (here NOTHING_TO_USE) gets its own encoded
    /// `CommandResult` and never a `WORLD_OBJECT_OVERLAY` delta, and an unregistered command
    /// type still gets an empty result payload exactly as it does for STEP.
    #[test]
    fn admitted_use_non_committing_disposition_emits_no_delta_and_unregistered_type_is_empty()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = UseAuthority {
                uses: RefCell::new(Vec::new()),
                commands: RefCell::new(Vec::new()),
                overlay: None,
                watermark: None,
                outcome: UseOutcome {
                    disposition: UseDisposition::NothingToUse,
                    committed: None,
                    earned: EarnedNotice::NoneEarned,
                },
            };
            let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
            let (_end, frames) = drive_admitted(
                &authority,
                &[
                    use_command(1, 1, use_type, b"oteryn:cell/unknown", 0),
                    use_command(1, 2, 0x7fff, b"oteryn:cell/entry-door", 0),
                ],
            )
            .await?;
            let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
                ADMITTED_GENERATION,
                1,
                0,
                &[DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &encode_world_spatial(&at(0)),
                }],
            )?
            .into();
            expected.extend([
                encode_command_result(
                    1,
                    1,
                    1,
                    CommandStatus::Accepted,
                    &encode_use_result(UseDisposition::NothingToUse),
                )?,
                encode_command_result(1, 2, 2, CommandStatus::Rejected, &[])?,
            ]);
            assert_eq!(frames, expected);
            // The unregistered type never reached `use_object`.
            assert_eq!(authority.uses.borrow().len(), 1);
            Ok(())
        })
    }

    /// Spell cast §9 step 2 over the real Channel owner: a positioned actor of a real
    /// `ChannelRuntimeV1` whose Character cast facts are supplied, the V1 spell book and the
    /// composed cast work item, served by the same `serve_admitted` loop as production.
    struct SpellAuthority {
        runtime: crate::foundation::ChannelRuntimeV1,
        states: RefCell<super::super::actor_spell::ChannelSpellStates>,
        book: crate::spell::SpellBook,
        casts: Cell<usize>,
    }

    impl FreshAdmissionAuthority for SpellAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }

        async fn observe_vitals(
            &self,
            actor: ExactActorRef,
            game_session_id: GameSessionId,
        ) -> Option<(u64, ActorVitals)> {
            super::super::actor_spell::observe_vitals(
                &self.runtime,
                &self.states.borrow(),
                actor,
                game_session_id,
            )
        }

        async fn cast_spell(
            &self,
            actor: ExactActorRef,
            game_session_id: GameSessionId,
            command_id: u64,
            intent: SpellCastIntent,
        ) -> SpellCastOutcome {
            self.casts.set(self.casts.get() + 1);
            super::super::actor_spell::cast_in_channel(
                &self.runtime,
                &mut self.states.borrow_mut(),
                &self.book,
                actor,
                game_session_id,
                command_id,
                &intent,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
        }
    }

    fn cast_command(id: u64, intent: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, id);
        scalar(
            &mut payload,
            2,
            u64::from(COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT),
        );
        bytes(&mut payload, 4, intent);
        envelope(7, ADMITTED_GENERATION, &payload)
    }

    /// A monk actor whose first periodic Serene evaluation changes Serene (SPELL-D8 §8.2).
    struct SereneAuthority {
        ticks: Cell<usize>,
    }

    const SERENE_VITALS: ActorVitals = ActorVitals {
        health: 185,
        max_health: 185,
        mana: 90,
        max_mana: 90,
        soul: 100,
        harmony: 2,
        serene: true,
    };

    impl FreshAdmissionAuthority for SereneAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }

        async fn observe_vitals(
            &self,
            _actor: ExactActorRef,
            _game_session_id: GameSessionId,
        ) -> Option<(u64, ActorVitals)> {
            Some((1, SERENE_VITALS))
        }

        async fn tick_vitals(
            &self,
            _actor: ExactActorRef,
            _game_session_id: GameSessionId,
        ) -> Option<(u64, ActorVitals)> {
            self.ticks.set(self.ticks.get() + 1);
            Some((
                2,
                ActorVitals {
                    serene: false,
                    ..SERENE_VITALS
                },
            ))
        }
    }

    /// The loop runs the Serene evaluation every 1000 ms while the actor has `ACTOR_VITALS`, and
    /// publishes a change as the next `ACTOR_VITALS` delta without any command.
    #[test]
    fn the_serene_cadence_publishes_a_changed_vitals_delta() -> Result<(), Box<dyn Error>> {
        use super::super::actor_spell::tests as spell;
        run(async {
            let (_runtime, actor, session) = spell::runtime_with_player(0x71);
            let authority = SereneAuthority {
                ticks: Cell::new(0),
            };
            let admitted = AdmittedSession {
                game_session_id: session,
                world_id: WorldId::decode(&uuid_v7(0x60))?,
                channel_id: ChannelId::decode(&uuid_v7(0x61))?,
                runtime_actor: Some(actor),
                first_entry: FirstEntryOutcome::Positioned,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            };
            let vitals_payload = encode_actor_vitals(&SERENE_VITALS).map_err(|_| "vitals")?;
            let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
                ADMITTED_GENERATION,
                1,
                0,
                &[
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                        payload: &encode_world_spatial(&at(0)),
                    },
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_ACTOR_VITALS,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                        payload: &vitals_payload,
                    },
                ],
            )?
            .into();
            expected.push(encode_state_delta(
                ADMITTED_GENERATION,
                1,
                STATE_DOMAIN_ACTOR_VITALS,
                1,
                2,
                DELTA_TYPE_ACTOR_VITALS_V1,
                &encode_actor_vitals(&ActorVitals {
                    serene: false,
                    ..SERENE_VITALS
                })
                .map_err(|_| "vitals")?,
            )?);
            let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 16);
            let wanted = expected.len();
            let read = async move {
                let mut frames = Vec::new();
                while frames.len() < wanted {
                    let mut length = [0_u8; 4];
                    client.read_exact(&mut length).await?;
                    let mut frame = vec![0_u8; u32::from_be_bytes(length) as usize];
                    client.read_exact(&mut frame).await?;
                    frames.push(frame);
                }
                // Closing the client ends the served connection.
                drop(client);
                Ok::<_, std::io::Error>(frames)
            };
            let reader = tokio::spawn(read);
            let end = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
            let frames = reader.await?;
            assert_eq!(frames?, expected);
            assert_eq!(authority.ticks.get(), 1);
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            Ok(())
        })
    }

    #[test]
    fn admitted_cast_pays_once_publishes_vitals_and_a_retry_expires() -> Result<(), Box<dyn Error>>
    {
        use super::super::actor_spell::{encode_spell_cast_intent, tests as spell};
        run(async {
            let (runtime, actor, session) = spell::runtime_with_player(0x70);
            let mut states = super::super::actor_spell::ChannelSpellStates::default();
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    spell::FACTS,
                    (0, 0),
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
                )
                .ok_or("initialize")?;
            let authority = SpellAuthority {
                runtime,
                states: RefCell::new(states),
                book: crate::spell::cast::v1_spell_book()?,
                casts: Cell::new(0),
            };
            let exura = encode_spell_cast_intent(&spell::exura());
            let unknown = encode_spell_cast_intent(&SpellCastIntent {
                spell: std::num::NonZeroU32::new(9).ok_or("index")?,
                ..spell::exura()
            });
            let admitted = AdmittedSession {
                game_session_id: session,
                world_id: WorldId::decode(&uuid_v7(0x60))?,
                channel_id: ChannelId::decode(&uuid_v7(0x61))?,
                runtime_actor: Some(actor),
                first_entry: FirstEntryOutcome::Positioned,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            };
            let (end, frames) = drive_session(
                &authority,
                admitted,
                &[
                    cast_command(1, &exura),
                    cast_command(2, &exura),
                    // Spell index 0 does not decode: REJECTED before the Channel owner.
                    cast_command(3, &[0x08, 0x00, 0x10, 0x01]),
                    cast_command(4, &unknown),
                    // The retry of the cast that paid: never executed again.
                    cast_command(1, &exura),
                ],
            )
            .await?;
            let full = ActorVitals {
                health: 185,
                max_health: 185,
                mana: 90,
                max_mana: 90,
                soul: 100,
                harmony: 0,
                serene: false,
            };
            let paid = ActorVitals { mana: 70, ..full };
            let result = |sequence, id, status, disposition| {
                encode_command_result(
                    ADMITTED_GENERATION,
                    sequence,
                    id,
                    status,
                    &encode_spell_cast_result(disposition),
                )
            };
            let full_payload = encode_actor_vitals(&full).map_err(|_| "vitals")?;
            let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
                ADMITTED_GENERATION,
                1,
                0,
                &[
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                        payload: &encode_world_spatial(&at(0)),
                    },
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_ACTOR_VITALS,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                        payload: &full_payload,
                    },
                ],
            )?
            .into();
            expected.extend([
                result(1, 1, CommandStatus::Accepted, SpellCastDisposition::Cast)?,
                encode_state_delta(
                    ADMITTED_GENERATION,
                    2,
                    STATE_DOMAIN_ACTOR_VITALS,
                    1,
                    2,
                    DELTA_TYPE_ACTOR_VITALS_V1,
                    &encode_actor_vitals(&paid).map_err(|_| "vitals")?,
                )?,
                result(
                    3,
                    2,
                    CommandStatus::Accepted,
                    SpellCastDisposition::CoolingDown,
                )?,
                result(
                    4,
                    3,
                    CommandStatus::Rejected,
                    SpellCastDisposition::Rejected,
                )?,
                result(
                    5,
                    4,
                    CommandStatus::Rejected,
                    SpellCastDisposition::Rejected,
                )?,
                encode_command_protocol_error(
                    FoundationProtocolError::CommandOutcomeExpired,
                    ADMITTED_GENERATION,
                    1,
                    0,
                )?,
            ]);
            assert_eq!(frames, expected);
            assert!(matches!(
                end,
                ConnectionEnd::AdmittedThenClosed(
                    _,
                    FoundationProtocolError::CommandOutcomeExpired
                )
            ));
            // The malformed intent and the retry never reached the Channel owner; mana was paid
            // exactly once.
            assert_eq!(authority.casts.get(), 3);
            assert_eq!(
                authority.observe_vitals(actor, session).await,
                Some((2, paid))
            );
            Ok(())
        })
    }

    /// Spell cast §4 over the real owner and the real serve loop: after a committed cast, a
    /// same-GameSession resume (a second `serve_admitted` over the same owner state, with the
    /// initialization the first-entry step repeats) re-snapshots the PAID vitals, and the
    /// cooldown is still running: a second cast pays nothing.
    #[test]
    fn spell_same_session_resume_snapshots_the_paid_vitals_and_keeps_the_cooldown()
    -> Result<(), Box<dyn Error>> {
        use super::super::actor_spell::{encode_spell_cast_intent, tests as spell};
        run(async {
            let (runtime, actor, session) = spell::runtime_with_player(0x71);
            let mut states = super::super::actor_spell::ChannelSpellStates::default();
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    spell::FACTS,
                    (0, 0),
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
                )
                .ok_or("initialize")?;
            spell::wound(&mut states, actor, session, 100);
            let authority = SpellAuthority {
                runtime,
                states: RefCell::new(states),
                book: crate::spell::cast::v1_spell_book()?,
                casts: Cell::new(0),
            };
            let exura = encode_spell_cast_intent(&spell::exura());
            let admitted = |continuity| -> Result<AdmittedSession, Box<dyn Error>> {
                Ok(AdmittedSession {
                    game_session_id: session,
                    world_id: WorldId::decode(&uuid_v7(0x60))?,
                    channel_id: ChannelId::decode(&uuid_v7(0x61))?,
                    runtime_actor: Some(actor),
                    first_entry: FirstEntryOutcome::Positioned,
                    controller: None,
                    continuity,
                    item_fence: None,
                })
            };
            drive_session(
                &authority,
                admitted(SessionContinuity::FRESH)?,
                &[cast_command(1, &exura)],
            )
            .await?;
            let (revision, paid) = authority
                .observe_vitals(actor, session)
                .await
                .ok_or("vitals")?;
            assert_eq!(revision, 2);
            assert_eq!((paid.mana, paid.max_mana), (70, 90));
            assert!(paid.health > 100 && paid.health <= paid.max_health);
            // The reconnect's owner step: initialization of a present actor keeps its state.
            authority
                .states
                .borrow_mut()
                .initialize(
                    &authority.runtime,
                    actor,
                    session,
                    spell::FACTS,
                    (0, 0),
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
                )
                .ok_or("reinitialize")?;
            let resumed = SessionContinuity {
                next_command_id: 2,
                server_sequence: 3,
                ..SessionContinuity::FRESH
            };
            let (_end, frames) =
                drive_session(&authority, admitted(resumed)?, &[cast_command(2, &exura)]).await?;
            let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
                ADMITTED_GENERATION,
                1,
                3,
                &[
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                        payload: &encode_world_spatial(&at(0)),
                    },
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_ACTOR_VITALS,
                        revision: 2,
                        snapshot_type: SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                        payload: &encode_actor_vitals(&paid).map_err(|_| "vitals")?,
                    },
                ],
            )?
            .into();
            // Still cooling down: answered, accepted, and no delta (nothing paid).
            expected.push(encode_command_result(
                ADMITTED_GENERATION,
                4,
                2,
                CommandStatus::Accepted,
                &encode_spell_cast_result(SpellCastDisposition::CoolingDown),
            )?);
            assert_eq!(frames, expected);
            assert_eq!(
                authority.observe_vitals(actor, session).await,
                Some((2, paid))
            );
            Ok(())
        })
    }

    /// A fixture authority for `ACCOUNT_ACHIEVEMENTS_QUERY` dispatch: every call is recorded and
    /// answered with one canned reply.
    struct AchievementsAuthority {
        calls: RefCell<Vec<AccountAchievementsRequest>>,
        reply: AccountAchievementsReply,
    }

    impl FreshAdmissionAuthority for AchievementsAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }

        async fn account_achievements(
            &self,
            request: AccountAchievementsRequest,
        ) -> AccountAchievementsReply {
            self.calls.borrow_mut().push(request);
            self.reply.clone()
        }
    }

    fn achievements_command(id: u64, query: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, id);
        scalar(
            &mut payload,
            2,
            u64::from(COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY),
        );
        bytes(&mut payload, 4, query);
        envelope(7, ADMITTED_GENERATION, &payload)
    }

    /// Display contract §4: the query reads the admitted controller's account, never one from the
    /// payload; a malformed query or a session without a controller is REJECTED without a read;
    /// an over-bound reply is REJECTED with `PAYLOAD_LIMIT_EXCEEDED` and an integrity fault with
    /// `ACCOUNT_DATA_INTEGRITY` (owner decision 2026-09-30), each with no rows.
    #[test]
    fn account_achievements_query_reads_only_the_controller_account() -> Result<(), Box<dyn Error>>
    {
        const ACCOUNT: [u8; 16] = uuid_v7(0x55);
        let request = |page, command_id| AccountAchievementsRequest {
            account_id: ACCOUNT,
            page,
            game_session_id: SESSION,
            command_id,
        };
        run(async {
            let world_id = WorldId::decode(&WORLD)?;
            let channel_id = ChannelId::decode(&CHANNEL)?;
            let session = |controller| AdmittedSession {
                game_session_id: GameSessionId::decode(&SESSION).expect("session"),
                world_id,
                channel_id,
                runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
                first_entry: FirstEntryOutcome::Positioned,
                controller,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            };
            let controller = Some(ControllerBinding {
                transport: AuthenticatedTransportRefV1::decode(&[0x5a; 16])?,
                account_id: ACCOUNT,
            });
            let authority = AchievementsAuthority {
                calls: RefCell::new(Vec::new()),
                reply: AccountAchievementsReply::Page(vec![0x08, 0x02]),
            };
            let (_end, frames) = drive_session(
                &authority,
                session(controller),
                &[
                    // Page 3; a payload has no field for an account.
                    achievements_command(1, &[0x08, 0x03]),
                    // An unknown field (as if naming an account) does not decode.
                    achievements_command(2, &[0x08, 0x00, 0x12, 0x01, 0x07]),
                ],
            )
            .await?;
            let mut expected = baseline();
            expected.extend([
                encode_command_result(
                    ADMITTED_GENERATION,
                    1,
                    1,
                    CommandStatus::Accepted,
                    &[0x08, 0x02],
                )?,
                encode_command_result(ADMITTED_GENERATION, 2, 2, CommandStatus::Rejected, &[])?,
            ]);
            assert_eq!(frames, expected);
            assert_eq!(*authority.calls.borrow(), [request(3, 1)]);

            // No controller binding: no account to read.
            let (_end, frames) =
                drive_session(&authority, session(None), &[achievements_command(1, &[])]).await?;
            assert_eq!(
                frames.last(),
                Some(&encode_command_result(
                    ADMITTED_GENERATION,
                    1,
                    1,
                    CommandStatus::Rejected,
                    &[]
                )?)
            );
            assert_eq!(authority.calls.borrow().len(), 1);

            for error in [
                FoundationProtocolError::PayloadLimitExceeded,
                FoundationProtocolError::AccountDataIntegrity,
            ] {
                let failing = AchievementsAuthority {
                    calls: RefCell::new(Vec::new()),
                    reply: AccountAchievementsReply::Terminal(error),
                };
                let (_end, frames) = drive_session(
                    &failing,
                    session(controller),
                    &[achievements_command(1, &[])],
                )
                .await?;
                let terminal = encode_command_error_result(ADMITTED_GENERATION, 1, 1, error)?;
                assert_ne!(
                    terminal,
                    encode_command_result(ADMITTED_GENERATION, 1, 1, CommandStatus::Rejected, &[])?
                );
                assert_eq!(frames.last(), Some(&terminal), "{error:?}");
                assert_eq!(*failing.calls.borrow(), [request(0, 1)]);
            }
            Ok(())
        })
    }

    const NOTICE_ACCOUNT: [u8; 16] = [0x7a; 16];

    fn earned(fact_count: u32, total_points: u32) -> AchievementEarned {
        AchievementEarned {
            key: "oteryn:achievement/allow_cookies".to_owned(),
            name: "Allow Cookies?!".to_owned(),
            watermark: AchievementWatermark {
                fact_count,
                total_points,
            },
        }
    }

    /// A chest `USE` fixture whose every call commits with the notice `earned`.
    fn notice_authority(
        watermark: Option<AchievementWatermark>,
        earned: EarnedNotice,
    ) -> UseAuthority {
        UseAuthority {
            uses: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            overlay: None,
            watermark,
            outcome: UseOutcome {
                disposition: UseDisposition::Committed,
                committed: None,
                earned,
            },
        }
    }

    /// A positioned session with a controller and `continuity`.
    fn controlled(continuity: SessionContinuity) -> Result<AdmittedSession, Box<dyn Error>> {
        Ok(AdmittedSession {
            controller: Some(ControllerBinding {
                transport: AuthenticatedTransportRefV1::decode(&[0x5a; 16])?,
                account_id: NOTICE_ACCOUNT,
            }),
            continuity,
            ..positioned()?
        })
    }

    /// A session that selected capability 8, at notice `revision`.
    fn with_notices(revision: u64) -> SessionContinuity {
        SessionContinuity {
            achievement_notice_revision: Some(revision),
            selected_capabilities: SelectedCapabilities::select(
                &[OfferedCapability {
                    id: CAPABILITY_ACHIEVEMENT_NOTICES_V1,
                    requires: &[],
                }],
                &[CAPABILITY_ACHIEVEMENT_NOTICES_V1],
            )
            .expect("selection"),
            ..SessionContinuity::FRESH
        }
    }

    fn notice_snapshot(
        generation: u64,
        sequence: u64,
        revision: u64,
        watermark: AchievementWatermark,
    ) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        Ok(encode_single_chunk_snapshot(
            generation,
            1,
            sequence,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &encode_world_spatial(&at(0)),
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
                    revision,
                    snapshot_type: SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
                    payload: &encode_achievement_notices_snapshot(watermark),
                },
            ],
        )?
        .into())
    }

    fn committed_use(generation: u64, sequence: u64, id: u64) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(encode_command_result(
            generation,
            sequence,
            id,
            CommandStatus::Accepted,
            &encode_use_result(UseDisposition::Committed),
        )?)
    }

    fn notice_delta(
        generation: u64,
        sequence: u64,
        from: u64,
        notice: &AchievementEarned,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(encode_state_delta(
            generation,
            sequence,
            STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
            from,
            from + 1,
            DELTA_TYPE_ACHIEVEMENT_EARNED_V1,
            &encode_achievement_earned(notice).map_err(|error| format!("{error:?}"))?,
        )?)
    }

    fn ended_continuity(end: ConnectionEnd) -> Result<SessionContinuity, Box<dyn Error>> {
        match end {
            ConnectionEnd::AdmittedThenDisconnected(ended) => Ok(ended.continuity),
            other => Err(format!("unexpected end {other:?}").into()),
        }
    }

    /// ACHIEVEMENT-0 §5: with capability 8 the join snapshot carries the watermark at revision 0
    /// and each committed `Granted` sends exactly one delta after its result, from the current
    /// revision to the next. Without capability 8 nothing of domain 13 is sent.
    #[test]
    fn each_granted_use_sends_one_notice_only_with_capability_8() -> Result<(), Box<dyn Error>> {
        run(async {
            let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
            let joined = AchievementWatermark {
                fact_count: 1,
                total_points: 3,
            };
            let notice = earned(2, 13);
            let uses = [
                use_command(1, 1, use_type, b"oteryn:placement/entry-chest", 0),
                use_command(1, 2, use_type, b"oteryn:placement/entry-chest", 0),
            ];
            let authority = notice_authority(Some(joined), EarnedNotice::Earned(notice.clone()));
            let (end, frames) =
                drive_session(&authority, controlled(with_notices(0))?, &uses).await?;
            let mut expected = notice_snapshot(1, 0, 0, joined)?;
            expected.extend([
                committed_use(1, 1, 1)?,
                notice_delta(1, 2, 0, &notice)?,
                committed_use(1, 3, 2)?,
                notice_delta(1, 4, 1, &notice)?,
            ]);
            assert_eq!(frames, expected);
            let ended = ended_continuity(end)?;
            assert_eq!(ended.achievement_notice_revision, Some(2));
            assert_eq!(ended.server_sequence, 4);

            // Without capability 8 (production today): the same grants send no domain 13 frame.
            let (end, frames) =
                drive_session(&authority, controlled(SessionContinuity::FRESH)?, &uses).await?;
            let mut expected = baseline();
            expected.extend([committed_use(1, 1, 1)?, committed_use(1, 2, 2)?]);
            assert_eq!(frames, expected);
            assert_eq!(ended_continuity(end)?.achievement_notice_revision, None);

            // A commit without a `Granted` grant (none, `AlreadyHeld` or `Retired`): no delta.
            let authority = notice_authority(Some(joined), EarnedNotice::NoneEarned);
            let (end, frames) =
                drive_session(&authority, controlled(with_notices(0))?, &uses[..1]).await?;
            let mut expected = notice_snapshot(1, 0, 0, joined)?;
            expected.push(committed_use(1, 1, 1)?);
            assert_eq!(frames, expected);
            assert_eq!(ended_continuity(end)?.achievement_notice_revision, Some(0));
            Ok(())
        })
    }

    /// ACHIEVEMENT-0 §5: a reconnect within the GameSession sends only the snapshot, at the
    /// cumulative revision, and never replays a delta; the next delta continues from it. A
    /// selected domain whose watermark cannot be read fails closed before any frame.
    #[test]
    fn a_reconnect_sends_only_the_snapshot_and_the_revision_stays_monotonic()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
            let notice = earned(2, 13);
            let watermark = notice.watermark;
            let authority = notice_authority(Some(watermark), EarnedNotice::Earned(notice.clone()));
            let (end, _) = drive_session(
                &authority,
                controlled(with_notices(0))?,
                &[use_command(
                    1,
                    1,
                    use_type,
                    b"oteryn:placement/entry-chest",
                    0,
                )],
            )
            .await?;
            let lost = ended_continuity(end)?;
            assert_eq!(lost.achievement_notice_revision, Some(1));

            // The resumed connection, as `resume.rs` builds it: the lost continuity on the next
            // generation.
            let resumed = SessionContinuity {
                connection_generation: 2,
                ..lost
            };
            let (end, frames) = drive_session(&authority, controlled(resumed)?, &[]).await?;
            assert_eq!(frames, notice_snapshot(2, 2, 1, watermark)?);
            assert_eq!(ended_continuity(end)?.achievement_notice_revision, Some(1));

            let (end, frames) = drive_session(
                &authority,
                controlled(resumed)?,
                &[use_command(
                    2,
                    2,
                    use_type,
                    b"oteryn:placement/entry-chest",
                    0,
                )],
            )
            .await?;
            let mut expected = notice_snapshot(2, 2, 1, watermark)?;
            expected.extend([committed_use(2, 3, 2)?, notice_delta(2, 4, 1, &notice)?]);
            assert_eq!(frames, expected);
            assert_eq!(ended_continuity(end)?.achievement_notice_revision, Some(2));

            // Unreadable watermark, or no controller account: no snapshot, nothing sent.
            let unreadable = notice_authority(None, EarnedNotice::NoneEarned);
            let (end, frames) =
                drive_session(&unreadable, controlled(with_notices(0))?, &[]).await?;
            assert!(frames.is_empty());
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            let uncontrolled = AdmittedSession {
                continuity: with_notices(0),
                ..positioned()?
            };
            let (_, frames) = drive_session(&authority, uncontrolled, &[]).await?;
            assert!(frames.is_empty());
            Ok(())
        })
    }

    /// ACH-NOTIFY-2: an `Unknown` notice with capability 8 ends the connection right after the
    /// command result, with no delta and the revision unchanged; the resumed connection's
    /// snapshot carries the watermark with the grant. Without capability 8 it changes nothing.
    #[test]
    fn an_unknown_notice_disconnects_only_with_capability_8_and_resume_restores_the_watermark()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
            let before = AchievementWatermark {
                fact_count: 1,
                total_points: 3,
            };
            let uses = [
                use_command(1, 1, use_type, b"oteryn:placement/entry-chest", 0),
                use_command(1, 2, use_type, b"oteryn:placement/entry-chest", 0),
            ];
            let authority = notice_authority(Some(before), EarnedNotice::Unknown);
            let (end, frames) =
                drive_session(&authority, controlled(with_notices(0))?, &uses).await?;
            let mut expected = notice_snapshot(1, 0, 0, before)?;
            expected.push(committed_use(1, 1, 1)?);
            assert_eq!(frames, expected);
            assert_eq!(
                authority.uses.borrow().len(),
                1,
                "nothing after the disconnect"
            );
            let lost = ended_continuity(end)?;
            assert_eq!(lost.achievement_notice_revision, Some(0));
            assert_eq!(lost.server_sequence, 1);

            // The grant is durable: the resumed connection's snapshot carries it, at the
            // unchanged revision.
            let after = AchievementWatermark {
                fact_count: 2,
                total_points: 8,
            };
            let resumed = SessionContinuity {
                connection_generation: 2,
                ..lost
            };
            let authority = notice_authority(Some(after), EarnedNotice::NoneEarned);
            let (_, frames) = drive_session(&authority, controlled(resumed)?, &[]).await?;
            assert_eq!(frames, notice_snapshot(2, 1, 0, after)?);

            // Without capability 8 the same outcome sends nothing and keeps the connection.
            let authority = notice_authority(Some(before), EarnedNotice::Unknown);
            let (end, frames) =
                drive_session(&authority, controlled(SessionContinuity::FRESH)?, &uses).await?;
            let mut expected = baseline();
            expected.extend([committed_use(1, 1, 1)?, committed_use(1, 2, 2)?]);
            assert_eq!(frames, expected);
            assert_eq!(authority.uses.borrow().len(), 2);
            assert_eq!(ended_continuity(end)?.server_sequence, 2);
            Ok(())
        })
    }
}

/// VIS-3 (MOVE-RL-11 §4): domain 1 composed from the interest set on the connection.
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod visibility_tests {
    use super::*;
    use crate::gameplay_transport::world_spatial::{
        ActorPosition, ChannelEntity, EntityDetail, EntityKind, EntityRef,
        PLACEHOLDER_ACTOR_DIRECTION, PLACEHOLDER_APPEARANCE_REF, PLACEHOLDER_HEALTH_PERCENT,
        WorldSpatialEntitiesDelta, WorldSpatialEntitiesSnapshot, encode_step_intent,
        encode_step_result,
    };
    use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
    use std::cell::{Cell, RefCell};
    use std::error::Error;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

    type TestResult = Result<(), Box<dyn Error>>;

    const fn uuid_v7(tag: u8) -> [u8; 16] {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70 | (tag & 0x0f);
        bytes[8] = 0x80 | (tag & 0x3f);
        bytes
    }

    const fn at(x: i32, y: i32) -> ActorPosition {
        ActorPosition { x, y, floor: 7 }
    }

    const CONTENT: [u8; 32] = [0x5c; 32];

    fn creature(identity: [u8; 16], position: ActorPosition, revision: u64) -> ChannelEntity {
        ChannelEntity {
            kind: VisibleKind::Creature,
            identity,
            generation: 1,
            position,
            revision,
        }
    }

    fn actor() -> ExactActorRef {
        ExactActorRef::transport_fixture(
            WorldId::decode(&uuid_v7(0x33)).expect("world"),
            ChannelId::decode(&uuid_v7(0x44)).expect("channel"),
        )
    }

    /// The wire entry of an actor of the fixture Channel.
    fn wire(kind: EntityKind, identity: [u8; 16], position: ActorPosition) -> WorldSpatialEntity {
        WorldSpatialEntity {
            kind,
            entity: EntityRef {
                identity,
                generation: 1,
            },
            position,
            detail: EntityDetail::Actor {
                direction: PLACEHOLDER_ACTOR_DIRECTION,
                appearance_ref: PLACEHOLDER_APPEARANCE_REF,
                health_percent: PLACEHOLDER_HEALTH_PERCENT,
            },
        }
    }

    fn own(position: ActorPosition) -> WorldSpatialEntity {
        wire(EntityKind::Player, actor().placement_identity(), position)
    }

    /// The Channel owner as domain 1 sees it: the own actor, which steps east, and the others.
    struct Channel {
        own: Cell<ActorPosition>,
        others: RefCell<Vec<ChannelEntity>>,
    }

    impl Channel {
        fn new(others: Vec<ChannelEntity>) -> Self {
            Self {
                own: Cell::new(at(100, 100)),
                others: RefCell::new(others),
            }
        }

        fn observation(&self) -> WorldSpatialObservation {
            WorldSpatialObservation {
                content_generation: CONTENT,
                actor_position: self.own.get(),
            }
        }
    }

    impl FreshAdmissionAuthority for Channel {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Unavailable)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(self.observation())
        }

        async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
            let mut position = self.own.get();
            position.x += 1;
            self.own.set(position);
            StepOutcome {
                disposition: StepDisposition::Moved,
                moved_to: Some(self.observation()),
            }
        }

        async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
            let own = actor.placement_identity();
            let mut entities = vec![ChannelEntity {
                kind: VisibleKind::Player,
                identity: own,
                generation: 1,
                position: self.own.get(),
                revision: u64::try_from(self.own.get().x).ok()?,
            }];
            entities.extend(self.others.borrow().iter().copied());
            Some(ChannelEntities {
                content_generation: CONTENT,
                observer: own,
                entities,
            })
        }
    }

    fn session(selected: &[u32], continuity: SessionContinuity) -> AdmittedSession {
        AdmittedSession {
            game_session_id: GameSessionId::decode(&uuid_v7(0x22)).expect("session"),
            world_id: WorldId::decode(&uuid_v7(0x33)).expect("world"),
            channel_id: ChannelId::decode(&uuid_v7(0x44)).expect("channel"),
            runtime_actor: Some(actor()),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity {
                selected_capabilities: SelectedCapabilities::select(
                    PRODUCTION_OFFERED_CAPABILITIES,
                    selected,
                )
                .expect("select"),
                ..continuity
            },
            item_fence: None,
        }
    }

    fn snapshot_frames(
        generation: u64,
        snapshot_id: u64,
        target: u64,
        revision: u64,
        selected: &[u32],
        entities: Vec<WorldSpatialEntity>,
    ) -> Vec<Vec<u8>> {
        let snapshot = WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT,
            actor_position: entities[0].position,
            own_identity: entities[0].entity.identity,
            entities,
        };
        let (snapshot_type, payload) =
            encode_visibility_snapshot(selected, &snapshot).expect("snapshot");
        encode_single_chunk_snapshot(
            generation,
            snapshot_id,
            target,
            &[DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision,
                snapshot_type,
                payload: &payload,
            }],
        )
        .expect("frames")
        .to_vec()
    }

    fn delta_frame(
        sequence: u64,
        from: u64,
        selected: &[u32],
        delta: &WorldSpatialEntitiesDelta,
    ) -> Vec<u8> {
        let (delta_type, payload) = encode_visibility_delta(selected, delta).expect("delta");
        encode_state_delta(
            1,
            sequence,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            from,
            from + 1,
            delta_type,
            &payload,
        )
        .expect("frame")
    }

    async fn read_one(client: &mut DuplexStream) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut length = [0_u8; 4];
        client.read_exact(&mut length).await?;
        let mut frame = vec![0_u8; u32::from_be_bytes(length) as usize];
        client.read_exact(&mut frame).await?;
        Ok(frame)
    }

    /// One step, then the client closes: every frame the connection sent.
    async fn join_and_step(selected: &[u32]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        let channel = Channel::new(vec![creature([9; 16], at(102, 100), 0)]);
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let frame = encode_client_command(
            1,
            &ClientCommandValue {
                command_id: 1,
                command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                payload: &encode_step_intent(StepDirection::East),
            },
        )?;
        let mut framed = u32::try_from(frame.len())?.to_be_bytes().to_vec();
        framed.extend_from_slice(&frame);
        client.write_all(&framed).await?;
        client.shutdown().await?;
        serve_admitted(
            &mut server,
            session(selected, SessionContinuity::FRESH),
            &channel,
            IDLE_LIVENESS,
        )
        .await;
        drop(server);
        let mut frames = Vec::new();
        while let Ok(frame) = read_one(&mut client).await {
            frames.push(frame);
        }
        Ok(frames)
    }

    fn run(test: impl Future<Output = TestResult>) -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(test)
    }

    #[test]
    fn domain_1_is_the_interest_set_with_capability_6_and_the_own_actor_without() -> TestResult {
        run(async {
            let result = encode_command_result(
                1,
                1,
                1,
                CommandStatus::Accepted,
                &encode_step_result(StepDisposition::Moved),
            )?;
            let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
            let creature = wire(EntityKind::Creature, [9; 16], at(102, 100));
            let mut expected =
                snapshot_frames(1, 1, 0, 1, &selected, vec![own(at(100, 100)), creature]);
            expected.push(result.clone());
            expected.push(delta_frame(
                2,
                1,
                &selected,
                &WorldSpatialEntitiesDelta {
                    content_generation: CONTENT,
                    actor_position: at(101, 100),
                    enter: Vec::new(),
                    update: vec![own(at(101, 100))],
                    leave: Vec::new(),
                },
            ));
            assert_eq!(join_and_step(&selected).await?, expected);

            // Without capability 6: the v1 own-actor snapshot and delta, as before VIS-3.
            let at_start = encode_world_spatial(&WorldSpatialObservation {
                content_generation: CONTENT,
                actor_position: at(100, 100),
            });
            let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
                1,
                1,
                0,
                &[DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &at_start,
                }],
            )?
            .to_vec();
            expected.push(result);
            expected.push(encode_state_delta(
                1,
                2,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                1,
                2,
                DELTA_TYPE_WORLD_SPATIAL_V1,
                &encode_world_spatial(&WorldSpatialObservation {
                    content_generation: CONTENT,
                    actor_position: at(101, 100),
                }),
            )?);
            assert_eq!(join_and_step(&[]).await?, expected);
            Ok(())
        })
    }

    #[test]
    fn others_moving_send_a_delta_and_a_change_above_256_a_new_snapshot() -> TestResult {
        run(async {
            let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
            let channel = Channel::new(vec![creature([9; 16], at(102, 100), 0)]);
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let admitted = session(&selected, SessionContinuity::FRESH);
            let serving = serve_admitted(&mut server, admitted, &channel, IDLE_LIVENESS);
            let observing = async {
                let mut snapshot = Vec::new();
                for _ in 0..3 {
                    snapshot.push(read_one(&mut client).await?);
                }
                assert_eq!(
                    snapshot,
                    snapshot_frames(
                        1,
                        1,
                        0,
                        1,
                        &selected,
                        vec![
                            own(at(100, 100)),
                            wire(EntityKind::Creature, [9; 16], at(102, 100))
                        ],
                    )
                );
                // The creature steps: one delta on the next refresh, sequence 1.
                channel.others.borrow_mut()[0] = creature([9; 16], at(103, 100), 1);
                assert_eq!(
                    read_one(&mut client).await?,
                    delta_frame(
                        1,
                        1,
                        &selected,
                        &WorldSpatialEntitiesDelta {
                            content_generation: CONTENT,
                            actor_position: at(100, 100),
                            enter: Vec::new(),
                            update: vec![wire(EntityKind::Creature, [9; 16], at(103, 100))],
                            leave: Vec::new(),
                        },
                    )
                );
                // 200 creatures replace it: 201 changes, one delta (sequence 2, revision 3).
                let identities = |tag: u8, count: u16| -> Vec<[u8; 16]> {
                    (0..count)
                        .map(|n| {
                            let mut identity = [tag; 16];
                            identity[14..].copy_from_slice(&n.to_be_bytes());
                            identity
                        })
                        .collect()
                };
                *channel.others.borrow_mut() = identities(0x30, 200)
                    .into_iter()
                    .map(|identity| creature(identity, at(99, 100), 0))
                    .collect();
                let mut entered = WorldSpatialEntitiesDelta {
                    content_generation: CONTENT,
                    actor_position: at(100, 100),
                    enter: identities(0x30, 200)
                        .into_iter()
                        .map(|identity| wire(EntityKind::Creature, identity, at(99, 100)))
                        .collect(),
                    update: Vec::new(),
                    leave: vec![EntityRef {
                        identity: [9; 16],
                        generation: 1,
                    }],
                };
                entered.enter.sort_by_key(|entity| entity.entity.identity);
                assert_eq!(
                    read_one(&mut client).await?,
                    delta_frame(2, 2, &selected, &entered)
                );
                // 257 others replace those: 200 leave and the nearest 255 enter, more than 256
                // changes, so a new snapshot (id 2, revision 4, at the current sequence 2) with
                // the own actor and the 255 lowest identities (the degrade disposition).
                let crowd = identities(0x40, 257);
                *channel.others.borrow_mut() = crowd
                    .iter()
                    .map(|identity| creature(*identity, at(99, 100), 0))
                    .collect();
                let mut resync = Vec::new();
                for _ in 0..3 {
                    resync.push(read_one(&mut client).await?);
                }
                let mut nearest = vec![own(at(100, 100))];
                nearest.extend(
                    crowd[..255]
                        .iter()
                        .map(|identity| wire(EntityKind::Creature, *identity, at(99, 100))),
                );
                assert_eq!(resync, snapshot_frames(1, 2, 2, 4, &selected, nearest));
                drop(client);
                Ok::<_, Box<dyn Error>>(())
            };
            let (mut serving, mut observing) = (std::pin::pin!(serving), std::pin::pin!(observing));
            let (mut end, mut observed) = (None, None);
            std::future::poll_fn(|context| {
                if end.is_none()
                    && let std::task::Poll::Ready(done) = serving.as_mut().poll(context)
                {
                    end = Some(done);
                }
                if observed.is_none()
                    && let std::task::Poll::Ready(done) = observing.as_mut().poll(context)
                {
                    observed = Some(done);
                }
                if end.is_some() && observed.is_some() {
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            })
            .await;
            observed.ok_or("observed")??;
            let end = end.ok_or("ended")?;
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err(format!("unexpected end {end:?}").into());
            };
            assert_eq!(ended.continuity.spatial_revision, 4);
            assert_eq!(ended.continuity.server_sequence, 2);
            Ok(())
        })
    }

    #[test]
    fn a_resumed_connection_keeps_the_selection_and_gets_a_snapshot_of_the_interest_set()
    -> TestResult {
        run(async {
            let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
            let channel = Channel::new(vec![creature([9; 16], at(102, 100), 0)]);
            let resumed = SessionContinuity {
                connection_generation: 2,
                next_command_id: 4,
                server_sequence: 6,
                spatial_revision: 7,
                ..SessionContinuity::FRESH
            };
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            client.shutdown().await?;
            let end = serve_admitted(
                &mut server,
                session(&selected, resumed),
                &channel,
                IDLE_LIVENESS,
            )
            .await;
            drop(server);
            let mut frames = Vec::new();
            while let Ok(frame) = read_one(&mut client).await {
                frames.push(frame);
            }
            assert_eq!(
                frames,
                snapshot_frames(
                    2,
                    1,
                    6,
                    7,
                    &selected,
                    vec![
                        own(at(100, 100)),
                        wire(EntityKind::Creature, [9; 16], at(102, 100))
                    ],
                )
            );
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err(format!("unexpected end {end:?}").into());
            };
            assert_eq!(ended.continuity.selected_capabilities.as_slice(), selected);
            Ok(())
        })
    }

    #[test]
    fn a_session_with_capability_6_whose_channel_cannot_be_read_fails_closed() -> TestResult {
        struct Unreadable;
        impl FreshAdmissionAuthority for Unreadable {
            async fn admit(
                &self,
                _attempt: FreshAdmissionAttempt<'_>,
            ) -> Result<AdmittedSession, AdmissionRefusal> {
                Err(AdmissionRefusal::Unavailable)
            }

            async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
                Some(WorldSpatialObservation {
                    content_generation: CONTENT,
                    actor_position: at(100, 100),
                })
            }

            async fn observe_visible_entities(
                &self,
                _actor: ExactActorRef,
            ) -> Option<ChannelEntities> {
                None
            }
        }
        run(async {
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            client.shutdown().await?;
            let end = serve_admitted(
                &mut server,
                session(
                    &[CAPABILITY_WORLD_SPATIAL_ENTITIES],
                    SessionContinuity::FRESH,
                ),
                &Unreadable,
                IDLE_LIVENESS,
            )
            .await;
            drop(server);
            assert!(read_one(&mut client).await.is_err());
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            Ok(())
        })
    }
}
