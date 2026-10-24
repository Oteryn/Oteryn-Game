//! One gameplay connection. The transport decodes bounded Foundation frames and
//! hands the validated bootstrap to the owning admission authority; it never
//! decides admission itself and fails closed for every message it does not own.

pub(super) mod appearance_change;
mod moved_step;
mod routing;
mod serve;
use crate::combat_pickup::GroundPickupError;
use routing::Dispatch;
use serve::serve_admitted_actor;
#[path = "newhaven_travel_transport.rs"]
mod newhaven_travel_transport;
#[path = "tutorial_notices_transport.rs"]
mod tutorial_notices_transport;
use crate::durability::item_transfer::{
    CommittedItemTransfer, CurrentCharacterItemFence, ItemTransferError, ItemTransferOutcome,
};
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelId, CharacterId, ExactActorRef, FoundationProtocolError,
    GameSessionId, MessageType, ServerResumeAcceptedValue, WorldId, decode_wire_envelope,
    encode_protocol_error, encode_server_accepted, encode_server_resume_accepted,
};
mod item_use_view_refresh;

use appearance_change::SessionAppearance;
use moved_step::moved_step_frames;
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};

use super::actor_spell::{
    ActorVitals, COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT, DELTA_TYPE_ACTOR_VITALS_V1,
    SNAPSHOT_TYPE_ACTOR_VITALS_V1, STATE_DOMAIN_ACTOR_VITALS, SpellCastDisposition,
    SpellCastIntent, SpellCastOutcome, decode_spell_cast_intent, encode_actor_vitals,
    encode_spell_cast_result,
};
use super::attack::{COMBAT_REFRESH, CombatContinuity};
use super::capabilities::{
    CAPABILITY_ITEM_LOOK_V1, COMMAND_TYPE_ITEM_LOOK_INTENT, OfferedCapability,
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities, with_weather_offer,
};
use super::chat_intent::CHAT_REFRESH;
use super::container_view::{
    ContainerObservation, ContainerPlan, ContainerViewState, PendingOpen, ViewCommandWindow,
};
use super::highscores::HighscoresReply;
use super::input_gate::InputPermit;
use super::item_look::LookWindow;
use super::item_move::{
    self, EquipmentMove, EquipmentMoveRefusal, EquipmentMoveReplay, EquipmentMoveResult,
    ItemMoveStep, LoopMove, LoopMoveRefusal, LoopMoveReplay, LoopMoveResult,
};
use super::item_view::{
    CloseTrigger, InventoryItems, ItemKey, ItemTargetObservation, ItemViewContinuity,
    ItemViewDelta, OpenDecision, SessionItemView, ViewItem,
};
pub use super::logout::LogoutReply;
use super::npc_command::{NpcContinuity, NpcTalkOutcome, NpcUpdate};
use super::npc_trade_command::NpcTradeStep;
use super::tcp_tls::{FrameReader, read_frame, write_frame};
use super::world_map::{MapUpdate, MapViewError, SessionMapView};
use super::world_spatial::{
    CAPABILITY_WORLD_SPATIAL_ENTITIES, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, ChannelEntities,
    ChannelEntity, DELTA_TYPE_WORLD_SPATIAL_V1, EntityLights, SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, SessionVisibility, StepDirection, StepDisposition,
    VisibilityUpdate, VisibleKind, WorldSpatialEntity, WorldSpatialError, WorldSpatialObservation,
    decode_step_intent, encode_step_outcome, encode_visibility_delta_lit,
    encode_visibility_snapshot_lit, encode_world_spatial, moved_step_wait_ms,
    step_wait_receipts_enabled, too_early_step_wait_ms,
};
use crate::achievement_catalogue::AccountAchievementsRequest;
use crate::chat::EGRESS_MAX_LINES;
use crate::foundation::{
    CommandStatus, DomainSnapshot, encode_command_protocol_error, encode_command_result,
    encode_liveness_probe, encode_single_chunk_snapshot, encode_state_delta,
};
use crate::movement::pacing::StepPacer;
use crate::world_clock::{
    DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS, WorldClockDeltaPacer, WorldClockSource,
};
use crate::world_weather::WorldWeatherSource;
use oteryn_protocol_oteryn::account_achievements::{
    COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY, decode_account_achievements_query,
};
use oteryn_protocol_oteryn::achievement_notices::{
    AchievementEarned, AchievementWatermark, CAPABILITY_ACHIEVEMENT_NOTICES_V1,
    DELTA_TYPE_ACHIEVEMENT_EARNED_V1, SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
    STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES, encode_achievement_earned,
    encode_achievement_notices_snapshot,
};
use oteryn_protocol_oteryn::attack::{
    ActorCombatState, AttackIntentDisposition, COMMAND_TYPE_ATTACK_TARGET_INTENT,
    COMMAND_TYPE_FIGHT_MODES_INTENT, ChaseMode, FightModes, SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
    STATE_DOMAIN_ACTOR_COMBAT_STATE, decode_attack_target_intent, decode_fight_modes_intent,
    encode_attack_intent_result,
};
use oteryn_protocol_oteryn::bed::{
    BedSleepIntent, COMMAND_TYPE_BED_SLEEP_INTENT, decode_bed_sleep_intent,
};
use oteryn_protocol_oteryn::chat::{
    COMMAND_TYPE_CHAT_INTENT, ChatIntent, ChatIntentResult, ChatLine, ChatRoomSet,
    DELTA_TYPE_CHAT_LINE_V1, DELTA_TYPE_CHAT_ROOMS_V1, STATE_DOMAIN_CHAT,
    encode_chat_intent_result, encode_chat_line, encode_chat_rooms,
};
use oteryn_protocol_oteryn::container_tree::CAPABILITY_CONTAINER_TREE_V1;
use oteryn_protocol_oteryn::container_tree::{
    COMMAND_TYPE_CONTAINER_VIEW_INTENT, ContainerViewOutcome, STATE_DOMAIN_CONTAINER_VIEWS,
    decode_container_view_intent, encode_container_view_result,
};
use oteryn_protocol_oteryn::encode_command_error_result;
use oteryn_protocol_oteryn::highscores::{
    COMMAND_TYPE_HIGHSCORES_QUERY_V1, HighscoresQuery, decode_highscores_query,
};
use oteryn_protocol_oteryn::item_look::{
    ItemLookRefusal, ItemLookResult, ItemLookSelection, ItemLookTarget,
    decode_item_look_intent_for, encode_item_look_result_for, known_creature_generation,
};
use oteryn_protocol_oteryn::item_view::{
    CAPABILITY_ITEM_EQUIP_DROP_V1, COMMAND_TYPE_ITEM_MOVE_INTENT, EquipmentSlot, ItemMoveOutcome,
    ItemMoveSelection, STATE_DOMAIN_CHARACTER_INVENTORY, encode_item_move_result_for,
};
use oteryn_protocol_oteryn::logout::{
    COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_intent, encode_logout_result,
};
use oteryn_protocol_oteryn::npc_service::{
    COMMAND_TYPE_NPC_TALK_INTENT, COMMAND_TYPE_NPC_TRADE_INTENT, DELTA_TYPE_NPC_CONVERSATION_V1,
    DELTA_TYPE_NPC_TRADE_WINDOW_V1, NpcConversation, NpcIntentDisposition, NpcTalkIntent,
    NpcTradeWindow, SNAPSHOT_TYPE_NPC_CONVERSATION_V1, SNAPSHOT_TYPE_NPC_TRADE_WINDOW_V1,
    STATE_DOMAIN_NPC_CONVERSATION, STATE_DOMAIN_NPC_TRADE_WINDOW, decode_npc_talk_intent,
    decode_npc_trade_intent, encode_npc_conversation, encode_npc_intent_result,
    encode_npc_trade_window,
};
use oteryn_protocol_oteryn::quest_log::{
    COMMAND_TYPE_QUEST_LOG_QUERY, DELTA_TYPE_QUEST_LOG_V1, SNAPSHOT_TYPE_QUEST_LOG_V1,
    STATE_DOMAIN_QUEST_LOG, decode_quest_log_query,
};
use oteryn_protocol_oteryn::world_clock::{
    CAPABILITY_WORLD_LIGHT_V1, DELTA_TYPE_WORLD_CLOCK_V1, SNAPSHOT_TYPE_WORLD_CLOCK_V1,
    STATE_DOMAIN_WORLD_CLOCK, WorldClock, encode_world_clock,
};
use oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW;
use oteryn_protocol_oteryn::world_object::{
    CAPABILITY_ITEM_USE_V1, CAPABILITY_RUNE_USE_V1, CAPABILITY_WORLD_INTERACTION_V1,
    COMMAND_TYPE_USE_INTENT, DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1, MAX_GATE_MESSAGE_ID,
    SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY, UseDisposition,
    UseIntent, UseTarget, WorldObjectOverlayEntry, decode_use_intent_target, encode_use_result,
    encode_use_result_with_gate, encode_world_object_overlay_delta,
    encode_world_object_overlay_snapshot,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef as WireEntityRef;
use oteryn_protocol_oteryn::world_weather::{
    CAPABILITY_WORLD_WEATHER_V1, DELTA_TYPE_WORLD_WEATHER_V1, SNAPSHOT_TYPE_WORLD_WEATHER_V1,
    STATE_DOMAIN_WORLD_WEATHER, WorldWeather, encode_world_weather,
};
use quest_log::{
    QUEST_LOG_REFRESH, QuestLogContinuity, QuestLogDomain, QuestLogObservation, QuestLogState,
};

// QUEST-LOG-WIRE-1: domain 16 and command type 22, a child of the connection that serves them.
#[path = "quest_log.rs"]
pub mod quest_log;

/// Foundation schema revision served by this build (FND-02 v1 contract).
pub const SERVER_SCHEMA_REVISION: u32 = 1;

/// Validated fresh-admission input handed to the owning authority.
#[derive(Debug, Clone, Copy)]
pub struct FreshAdmissionAttempt<'a> {
    pub character_id: CharacterId,
    pub admission_material: &'a [u8],
    pub game_session_id: GameSessionId,
    pub transport: AuthenticatedTransportRefV1,
    /// The bootstrap's supported capabilities, untrusted. ADMIT0 §4: the owner checks them
    /// against the Channel's required set only after authenticating the grant.
    pub supported_capabilities: &'a [u32],
}

/// One `ClientResume` attempt: the GameSession to resume, the untrusted reauthenticated
/// recovery credential and the fresh candidate transport of this connection.
pub struct ResumeAttempt<'a> {
    pub game_session_id: GameSessionId,
    pub recovery_material: &'a [u8],
    pub transport: AuthenticatedTransportRefV1,
    pub last_applied_server_sequence: u64,
    /// The resuming client's supported capabilities: a resume that lacks one the session
    /// selected is refused (CAP-NEG-1).
    pub supported_capabilities: &'a [u32],
}

/// Authority-committed admission: the only state that lets a transport claim a
/// GameSession.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmittedSession {
    pub game_session_id: GameSessionId,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    /// Present for the composed production fresh-admission path. Transport-only
    /// fixtures may omit it; the transport never invents or mutates actor authority.
    pub runtime_actor: Option<ExactActorRef>,
    /// #935 first-entry positioning of `runtime_actor`. Only a positioned actor
    /// may later become input-eligible.
    pub first_entry: FirstEntryOutcome,
    /// The admitted controller: the exact authenticated transport and the account whose
    /// presence the session holds. Transport-only fixtures omit it.
    pub controller: Option<ControllerBinding>,
    /// FND-02 continuity of this controller connection, current at the moment the
    /// connection ends (the next CommandId, server_sequence and spatial revision).
    pub continuity: SessionContinuity,
    /// C2: the Character item fence a fenced item write (the chest `USE` MINT) presents, read
    /// from the durable GameSession this connection serves: set from the committed fresh
    /// admission and refreshed from the committed resume. `None` when it could not be read or
    /// for transport-only fixtures; a `USE` on a chest is then refused. It is expected
    /// evidence only: every fenced write rechecks it against current durable authority.
    pub item_fence: Option<CurrentCharacterItemFence>,
}

/// FND-02 continuity of one admitted controller connection. A same-session recovery
/// resumes CommandId order and server_sequence from the value the lost connection ended
/// with, on a strictly newer connection generation (FND-04B §16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionContinuity {
    pub connection_generation: u64,
    pub next_command_id: u64,
    pub server_sequence: u64,
    pub spatial_revision: u64,
    /// The last `WORLD_OBJECT_OVERLAY` (domain 2) revision emitted to this connection, from
    /// either the join/resync snapshot or a committed-use delta (USE-WIRE-V1, #162 5868482467).
    /// Unlike `spatial_revision` (this actor's own per-actor counter), the door overlay is
    /// Channel-global, so this field is written from the live value actually sent, not trusted
    /// to already match it — see `serve_admitted`.
    pub overlay_revision: u64,
    /// Last complete own-actor vitals state actually transmitted.
    pub vitals_revision: u64,
    /// `ACCOUNT_ACHIEVEMENT_NOTICES` (domain 13, ACHIEVEMENT-0 §5): `None` when the session did
    /// not select capability 8, which production never does before capability negotiation is
    /// composed. Otherwise the domain's revision: 0 at the session's first snapshot, plus 1 for
    /// each delta. It is cumulative per GameSession: a resume carries it and never resets it.
    pub achievement_notice_revision: Option<u64>,
    /// CAP-NEG-1: the capabilities selected at this GameSession's fresh admission. A resume or
    /// channel transfer carries them unchanged and never widens them.
    pub selected_capabilities: SelectedCapabilities,
    /// ITEM-VIEW-1b: the item handle counter, the domain 9 and 11 high-water revisions and the
    /// open corpse. Used only with capability 4; a resume or channel transfer carries it.
    pub item_view: ItemViewContinuity,
    /// QUEST-LOG-WIRE-1: the domain 16 revision, the tracked quests and the query window. Used
    /// only with capability 16; a resume or channel transfer carries it.
    pub quest_log: QuestLogContinuity,
    /// ITEMLOOK1-RL-01: carried across resume and transfer with the GameSession.
    pub look: LookWindow,
    /// ATTACK-1b: the domain 10 revision, the last state sent and the command 11 and 12 windows.
    /// Used only with capability 17; a resume or channel transfer carries it.
    pub combat: CombatContinuity,
    /// MAP-WIRE-2: the last domain 17 revision sent. Used only with capability 18; a resume or
    /// channel transfer carries it, and the next connection's snapshot follows it.
    pub world_map_revision: u64,
    /// CHAT-WIRE-1: the last domain 12 revision sent. Used only with capability 7; it is
    /// cumulative per GameSession, so a resume or channel transfer carries it and the next
    /// snapshot follows it.
    pub chat_revision: u64,
    /// NPC-CMD-1a: the last domain 7 and 8 revisions sent. Used only with capability 3; a resume
    /// or channel transfer carries them and the next snapshot follows them.
    pub npc: NpcContinuity,
    /// CLOCK-1: the last domain 18 revision sent. Used only with capability 19; it is cumulative
    /// per GameSession, so a resume or channel transfer carries it and the next snapshot follows
    /// it.
    pub world_clock_revision: u64,
    /// WEATHER-1: the last domain 22 revision sent. Used only with capability 26; it is
    /// cumulative per GameSession, so a resume or channel transfer carries it and the next
    /// snapshot follows it.
    pub world_weather_revision: u64,
    pub tutorial_revision: u64,
}

impl SessionContinuity {
    /// A fresh admission: generation 1, first CommandId 1, no sequenced output yet, the
    /// baseline spatial revision 1, and no overlay observed yet (0, the door's own initial
    /// revision).
    pub const FRESH: Self = Self {
        connection_generation: ADMITTED_GENERATION,
        next_command_id: 1,
        server_sequence: 0,
        spatial_revision: 1,
        overlay_revision: 0,
        vitals_revision: 0,
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
        look: LookWindow::EMPTY,
        combat: CombatContinuity::FRESH,
        world_map_revision: 0,
        chat_revision: 0,
        npc: NpcContinuity {
            conversation_revision: 0,
            window_revision: 0,
        },
        world_clock_revision: 0,
        world_weather_revision: 0,
        tutorial_revision: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControllerBinding {
    pub transport: AuthenticatedTransportRefV1,
    pub account_id: [u8; 16],
}

/// Outcome of turning an ended admitted connection into durable control loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlLossResult {
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
    /// LOGOUT-WIRE-1 (§1.6): the current row is TERMINAL. A release committed but its
    /// acknowledgement was lost; the lifecycle reconciles the session (`reconcile_terminal`).
    Terminal,
}

/// Outcome of the FND-04B §6 grace-expiry release of a recorded control loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraceExpiryResult {
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
pub enum FirstEntryOutcome {
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
pub enum AdmissionRefusal {
    /// The material or current owner facts do not admit this attempt.
    Rejected,
    /// The owning authority could not decide; nothing was admitted.
    Unavailable,
    /// Refused with an N8 admission refusal code (ARCH-LOGIN-FIRST-PACKETS-V1 §1.2); nothing
    /// was admitted. The client receives the code in one `ProtocolError` before the close.
    Classified(FoundationProtocolError),
    /// ADMIT0 §4: an authenticated recovery whose support lacks the Channel's required set.
    /// Sent as `RECOVERY_CAPABILITY_REQUIRED` (1118, CP D1187); no claim is released and no
    /// recovery grant is consumed.
    RecoveryCapabilityRequired,
}

/// N8: the one pre-admission refusal frame is written and flushed within this bound, or dropped.
const ADMISSION_REFUSAL_WRITE_BOUND: std::time::Duration = std::time::Duration::from_secs(2);

/// The owning fresh-admission authority. Production composes the real owners;
/// nothing on the transport side can mark a connection admitted.
pub trait FreshAdmissionAuthority {
    super::vip::authority_methods!();
    super::house_intent::authority_methods!();
    /// The production session owner retains command truth across connection replacement.
    /// Transport-only authorities may omit the resumable command directory.
    fn session_commands(&self) -> Option<&super::session_ingress::SessionCommands> {
        None
    }
    super::appearance_change::authority_methods!();
    super::guild_intent::authority_methods!();
    super::daily_reward::authority_methods!();
    super::house_move_out::authority_methods!();
    super::cyclopedia::authority_methods!();
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

    /// OPS-SAVE-2: the permit one player command runs under, or `None` once the node is
    /// quiescing for its shutdown save: the command then has no effect and gets a rejected
    /// result. Authorities without a shutdown save never refuse.
    fn enter_input(&self) -> impl Future<Output = Option<InputPermit<'_>>> {
        async { Some(InputPermit::ungated()) }
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
                lights: EntityLights::new(),
                observed_at_ms: 0,
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

    /// CHASE-1 (RANGED-0 §8): the server-driven chase step of `actor` due now, taken like
    /// [`Self::paced_step`] and answered the same way; `None` when the chase takes no step.
    /// Fixtures without a chase keep this default.
    fn chase_step(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> impl Future<Output = Option<(StepOutcome, Option<std::time::Duration>)>> {
        async { None }
    }

    /// CHASE-1 (`RANGED0-RL-03`): whether `actor`'s chase has a due search the Channel budget
    /// has not admitted yet. Fixtures without a chase keep this default.
    fn chase_search_waiting(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> impl Future<Output = bool> {
        async { false }
    }

    /// CHASE-1 (RANGED-0 §8): the actor-owned pacing deadline of `actor` still ahead, which a
    /// replaced or resumed connection starts from. Fixtures without one keep this default.
    fn actor_step_deadline(
        &self,
        _actor: ExactActorRef,
    ) -> impl Future<Output = Option<tokio::time::Instant>> {
        async { None }
    }

    /// CHASE-1: claim `actor`'s pacing clock for one step of the connection owning `holder`,
    /// atomically with the check; `Some` with when to ask again while it is still held.
    fn begin_actor_step(
        &self,
        _actor: ExactActorRef,
        _holder: &std::sync::Arc<()>,
    ) -> impl Future<Output = Option<tokio::time::Instant>> {
        async { None }
    }

    /// CHASE-1: the step `holder` claimed ended; `actor`'s pacing clock is now due at `due`, or
    /// free.
    fn record_actor_step_deadline(
        &self,
        _actor: ExactActorRef,
        _holder: &std::sync::Arc<()>,
        _due: Option<tokio::time::Instant>,
    ) -> impl Future<Output = ()> {
        async {}
    }

    /// CHASE-1: a client step of `actor` arrived; it wins over the chase, whose route is dropped.
    fn yield_chase_to_client(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> impl Future<Output = ()> {
        async {}
    }

    /// STEP-PACE-WIRE-1 rollout switch: whether a capability 13 session's step results carry the
    /// `step_wait_ms` receipt. Off by default: the process switch
    /// [`step_wait_receipts_enabled`] (`OTERYN_STEP_WAIT_RECEIPTS=1`). Read once per connection.
    fn step_wait_receipts(&self) -> bool {
        step_wait_receipts_enabled()
    }

    /// CLOCK-1 (§11.2): the UTC time fact the World clock is read at, in milliseconds. `None`
    /// when it cannot be read: a session that selected capability 19 then fails closed.
    fn world_clock_utc_ms(&self) -> Option<u64> {
        crate::world_clock::now_utc_ms()
    }

    /// WEATHER-1 (D1025/D1026/D1448): the weather source, or `None`. With `None` capability 26 is
    /// never offered at fresh admission, and a session that selected it earlier is sent one full
    /// `Unavailable`. The default is `None` (fail closed): only an authority that has resolved
    /// the retained World binding for its World exposes a source (D1027 gate in the production
    /// authority).
    fn world_weather(&self) -> Option<&WorldWeatherSource> {
        None
    }

    fn take_tutorial_notice(
        &self,
        _session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice> {
        None
    }

    /// ITEM-TIMED-1: one bounded timed-item payload; None preserves an unknown commit.
    fn timed_item(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _payload: &[u8],
        _target: Option<ItemKey>,
    ) -> impl Future<Output = Option<Vec<u8>>> {
        async {
            oteryn_protocol_oteryn::timed_item::encode_timed_item_result(
                oteryn_protocol_oteryn::timed_item::TimedItemResult::refused(
                    oteryn_protocol_oteryn::timed_item::TimedDisposition::Rejected,
                ),
            )
            .ok()
        }
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
        _target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> impl Future<Output = UseOutcome> {
        async { UseOutcome::rejected() }
    }

    /// ITEM-USE-1c: one `USE` of the main backpack direct entry `item` (food or a potion) under
    /// capability 15. `command` is as for [`Self::use_object`].
    fn use_item(
        &self,
        _actor: ExactActorRef,
        _use: UseCommand,
        _item: ItemKey,
    ) -> impl Future<Output = UseOutcome> {
        async { UseOutcome::rejected() }
    }

    /// A timed USE keeps an ambiguous commit open for receipt reconciliation.
    fn use_item_completion(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> impl Future<Output = Option<UseOutcome>> {
        async move { Some(self.use_item(actor, command, item).await) }
    }

    /// BAGS-USE-1: field 5, resolved by the owner against the complete carried tree.
    fn use_item_by_definition(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _definition: std::num::NonZeroU32,
    ) -> impl Future<Output = UseOutcome> {
        async {
            UseOutcome {
                disposition: UseDisposition::NothingToUse,
                ..UseOutcome::rejected()
            }
        }
    }

    /// MINING-1: original USE is retained for replay before either handle is resolved.
    fn use_pick(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: UseIntent,
        _source: Option<ItemKey>,
        _target: Option<ItemKey>,
    ) -> impl Future<Output = Option<UseOutcome>> {
        async { Some(UseOutcome::rejected()) }
    }

    fn offline_training(
        &self,
        _admitted: AdmittedSession,
        _command: UseCommand,
        _target: ItemKey,
    ) -> impl Future<Output = Option<oteryn_protocol_oteryn::loop_training::TrainingResult>> {
        async { Some(oteryn_protocol_oteryn::loop_training::TrainingResult::REFUSED) }
    }
    fn exercise_training(
        &self,
        _admitted: AdmittedSession,
        _command: UseCommand,
        _intent: oteryn_protocol_oteryn::loop_training::ExerciseIntent,
        _weapon: Option<ItemKey>,
        _dummy: Option<ItemKey>,
    ) -> impl Future<Output = Option<oteryn_protocol_oteryn::loop_training::TrainingResult>> {
        async { Some(oteryn_protocol_oteryn::loop_training::TrainingResult::REFUSED) }
    }
    fn training_activity(
        &self,
        _admitted: AdmittedSession,
        _kind: u32,
    ) -> impl Future<Output = bool> {
        async { true }
    }
    /// BED0 §9: choice from a live map-item handle. An authority without the
    /// house reservation/logout owner refuses; decoding never grants bed access.
    fn sleep_in_bed(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: BedSleepIntent,
        target: ItemKey,
    ) -> impl Future<Output = Result<UseOutcome, crate::durability::DurabilityError>> {
        super::bed_sleep::sleep_with_owner(self, actor, command, intent, target)
    }

    /// The owning House runtime resolves the reciprocal bed, reservation,
    /// Premium/reach/block checks and native sex before supplying this context.
    /// Channel authorities have no House owner and retain the closed default.
    fn prepare_bed_sleep(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: BedSleepIntent,
        _target: ItemKey,
    ) -> impl Future<Output = Option<crate::durability::bed_sleep::BedSleepOwner<'_, '_, '_>>> {
        async { None }
    }

    /// CRAFT-1: use-with a map cooking station. Replay resolves before current handles.
    fn use_craft(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: UseIntent,
        _source: Option<ItemKey>,
        _target: Option<ItemKey>,
    ) -> impl Future<Output = Option<UseOutcome>> {
        async { Some(UseOutcome::rejected()) }
    }

    /// Capability 22's exact direct-entry Rune adapter. Unknown commits keep the command open.
    fn use_rune(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: UseIntent,
        _key: Option<ItemKey>,
        _visible: bool,
    ) -> impl Future<Output = Option<UseOutcome>> {
        async { Some(UseOutcome::rejected()) }
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

    /// EQUIP-WIRE-1: the admitted Character's occupied equipment slots for domain 9 under
    /// capability 12, in ascending slot order, read for the snapshot and after each commit that
    /// may change domain 9. `None` when they cannot be read: the session then fails closed.
    fn observe_character_equipment(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<Vec<(EquipmentSlot, ViewItem)>>> {
        async { None }
    }

    /// EQUIP-WIRE-1 replay first: the equipment receipt of this command 9's CommandRef, read
    /// after the TRANSFER replay and before its handle is resolved. `None` when it cannot be
    /// read: the connection ends before the CommandId is sequenced.
    fn committed_equipment_move(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> impl Future<Output = Option<EquipmentMoveReplay>> {
        async { Some(EquipmentMoveReplay::Absent) }
    }

    /// EQUIP-WIRE-1 (ITEM-MOVE-WIRE-1 §4): one equip or unequip through the equipment owner,
    /// keyed by this command 9's CommandRef. The connection has checked that the source is shown
    /// where the move needs it and that the destination slot is shown free; the owner decides the
    /// rest in one transaction.
    fn move_equipment(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _request: EquipmentMove,
    ) -> impl Future<Output = EquipmentMoveResult> {
        async { EquipmentMoveResult::Refused(EquipmentMoveRefusal::Rejected) }
    }

    /// Depot requests are resolved by the live Channel and fenced native item owner.
    fn depot(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: oteryn_protocol_oteryn::depot::DepotIntent,
        _item: Option<ItemKey>,
    ) -> impl Future<Output = Option<oteryn_protocol_oteryn::depot::DepotReply>> {
        async { None }
    }

    /// ITEM-LOOP-1 replay: the item loop receipt of this command 9's CommandRef, read after the
    /// equipment replay and before its handle is resolved. `None` when it cannot be read: the
    /// connection ends before the CommandId is sequenced.
    fn committed_loop_move(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> impl Future<Output = Option<LoopMoveReplay>> {
        async { Some(LoopMoveReplay::Absent) }
    }

    /// ITEM-LOOP-1 (CP D1438): one move, merge, drop or pickup of the carried tree through the
    /// item loop owner, keyed by this command 9's CommandRef. The connection has resolved the
    /// handles; the owner checks reach and decides the rest in one transaction.
    fn move_loop_item(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _request: LoopMove,
    ) -> impl Future<Output = LoopMoveResult> {
        async { LoopMoveResult::Refused(LoopMoveRefusal::Rejected) }
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

    fn look_item(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _fence: Option<CurrentCharacterItemFence>,
        _views: &SessionItemView,
        _target: oteryn_protocol_oteryn::item_view::ItemHandle,
    ) -> impl Future<Output = ItemLookResult> {
        async { ItemLookResult::Refused(ItemLookRefusal::NotAvailable) }
    }

    fn look_creature(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _target: WireEntityRef,
    ) -> impl Future<Output = ItemLookResult> {
        async { ItemLookResult::Refused(ItemLookRefusal::NotVisible) }
    }

    /// ITEM-MOVE-1 replay first (WIRE-0 §5): the committed TRANSFER of this command 9, looked
    /// up by its CommandRef alone before its handle is resolved, so a command that committed
    /// before a reconnect answers `MOVED` although its handle is now stale. `Ok(None)` proves
    /// that nothing committed for it.
    fn committed_item_move(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> impl Future<Output = Result<Option<CommittedItemTransfer>, ItemTransferError>> {
        async { Ok(None) }
    }

    /// ITEM-MOVE-1: the corpse-entry TRANSFER of `entry` out of the open `corpse` into the
    /// Character's inventory (D134), keyed by this command 9's CommandRef. The connection has
    /// checked reach and that the entry is shown in the corpse; the TRANSFER decides the rest.
    fn take_corpse_entry(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _corpse: ItemKey,
        _entry: ItemKey,
    ) -> impl Future<Output = Result<ItemTransferOutcome, GroundPickupError>> {
        async {
            Err(GroundPickupError::Transfer(
                ItemTransferError::AuthorityRejected,
            ))
        }
    }

    /// BAGS-WIRE-1: the Channel owner's view of the container a view command or `USE` names,
    /// with reach decided against the actor's position. `None` when it cannot be observed
    /// (`STALE`); a `USE` then takes the corpse path.
    fn observe_container(
        &self,
        _actor: ExactActorRef,
        _fence: Option<CurrentCharacterItemFence>,
        _target: ItemKey,
    ) -> impl Future<Output = Option<ContainerObservation>> {
        async { None }
    }

    /// PARTY-1: basic session-fenced membership commands and own-state query.
    fn party_command(
        &self,
        _admitted: AdmittedSession,
        _command_id: u64,
        _command_type: u32,
        _payload: &[u8],
        _view: &mut super::party_command::SessionPartyView,
    ) -> impl Future<Output = Option<Vec<u8>>> {
        std::future::ready(None)
    }

    fn staff_command(
        &self,
        _admitted: AdmittedSession,
        _receipt: bool,
        _payload: &[u8],
    ) -> impl Future<Output = super::StaffReply> {
        async { super::StaffReply::Rejected }
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

    /// CHAT-WIRE-1: a session with capability 7 starts chatting. Returns the rooms it has open
    /// (CHAT-2B); the authority loads the speaker name and drops undelivered lines.
    fn chat_admit(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
    ) -> impl Future<Output = ChatRoomSet> {
        async { ChatRoomSet::default() }
    }

    /// CHAT-WIRE-1: one `CHAT_INTENT` (command 13) of the admitted actor.
    fn chat_intent(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _command_id: u64,
        _intent: ChatIntent,
    ) -> impl Future<Output = ChatIntentResult> {
        async {
            ChatIntentResult {
                disposition: oteryn_protocol_oteryn::chat::ChatDisposition::ChatUnavailable,
                wait_seconds: 0,
            }
        }
    }

    /// CHAT-2B: the open rooms of the session when an intent changed them since its client was
    /// last shown them (a domain 12 delta type 2); `None` when unchanged.
    fn chat_rooms_changed(
        &self,
        _session: GameSessionId,
    ) -> impl Future<Output = Option<ChatRoomSet>> {
        async { None }
    }

    fn chat_guild_admit(
        &self,
        _admitted: AdmittedSession,
    ) -> impl Future<Output = oteryn_protocol_oteryn::chat::guild_room::GuildRoomState> {
        std::future::ready(oteryn_protocol_oteryn::chat::guild_room::GuildRoomState::Unavailable)
    }

    fn chat_guild_intent(
        &self,
        _admitted: AdmittedSession,
        _text: String,
    ) -> impl Future<Output = ChatIntentResult> {
        std::future::ready(ChatIntentResult {
            disposition: oteryn_protocol_oteryn::chat::ChatDisposition::Rejected,
            wait_seconds: 0,
        })
    }

    fn chat_next_bound(
        &self,
        admitted: AdmittedSession,
        _remaining: &mut usize,
    ) -> impl Future<Output = Option<ChatLine>> {
        self.chat_next(admitted.game_session_id)
    }

    /// CHAT-WIRE-1: the oldest line queued for the session, if any.
    fn chat_next(&self, _session: GameSessionId) -> impl Future<Output = Option<ChatLine>> {
        async { None }
    }

    /// CHAT-WIRE-1: the real line last returned by `chat_next` was written to the client; it
    /// stops counting against `CHAT0-RL-11`.
    fn chat_delivered(&self, _session: GameSessionId) -> impl Future<Output = ()> {
        async {}
    }

    /// NPC-CMD-1a: a session with capability 3 starts (or restarts) without a conversation.
    fn npc_session_open(&self, _session: GameSessionId) -> impl Future<Output = ()> {
        async {}
    }

    /// NPC-CMD-1a: the domain 7 payload when the conversation differs from the NPC `shown` to
    /// the client: one opened from chat, or ended by walking out of range.
    fn npc_follow(
        &self,
        _actor: ExactActorRef,
        _session: GameSessionId,
        _shown: Option<WireEntityRef>,
    ) -> impl Future<Output = Option<NpcUpdate>> {
        async { None }
    }

    /// NPC-TRADE-2: one `NPC_TRADE_INTENT` (command 8) of the admitted actor, keyed by its
    /// CommandRef: replay first, then one durable NPC trade.
    fn npc_trade(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: oteryn_protocol_oteryn::npc_service::NpcTradeIntent,
    ) -> impl Future<Output = NpcTradeStep> {
        async {
            NpcTradeStep::Refused(super::npc_command::NpcTradeRefusal::only(
                NpcIntentDisposition::Rejected,
            ))
        }
    }

    /// Reconcile only the original pending CommandRef at connection entry. Replacement wire
    /// bytes cannot cancel accepted Bank work. This is not dispatch of a new client command.
    fn resume_npc_bank(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> impl Future<Output = Option<NpcTalkOutcome>> {
        async { None }
    }

    /// NPC-CMD-1a: one `NPC_TALK_INTENT` (command 7) of the admitted actor.
    fn npc_talk(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
        _intent: NpcTalkIntent,
    ) -> impl Future<Output = NpcTalkOutcome> {
        async {
            NpcTalkOutcome {
                unknown: false,
                inventory_changed: false,
                disposition: NpcIntentDisposition::Rejected,
                conversation: None,
                window: None,
            }
        }
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

    /// None keeps an ambiguously committed original spell outstanding. It is
    /// never encoded as a terminal rejection or advanced to the next CommandId.
    fn cast_spell_completion(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: SpellCastIntent,
    ) -> impl Future<Output = Option<SpellCastOutcome>> {
        async move { Some(self.cast_spell(actor, session, command_id, intent).await) }
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

    /// Persist a fenced observation of the reset-selected World announcement.
    /// None leaves a possibly committed result outstanding for reconciliation.
    fn world_event(&self, _command: UseCommand) -> impl Future<Output = super::world_event::Reply> {
        async { Some(Err(())) }
    }

    /// RANK-1 (HS-WIRE-1): one `HIGHSCORES_QUERY_V1` page of `world`'s persisted snapshot for
    /// the Character the admitted `session` controls. The transport passes the session's World
    /// and GameSession, never values from the payload.
    fn highscores(
        &self,
        _world: WorldId,
        _session: GameSessionId,
        _query: HighscoresQuery,
    ) -> impl Future<Output = HighscoresReply> {
        async { HighscoresReply::Rejected }
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

    /// ATTACK-1b: one `ATTACK_TARGET_INTENT` (command 11) of the admitted actor, applied by the
    /// Channel owner; `None` stops attacking. `command_id` is the CommandId of the command, whose
    /// `CommandRef` starts the new target's swing lineage.
    fn attack_target(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _command_id: u64,
        _target: Option<WireEntityRef>,
    ) -> impl Future<Output = AttackIntentDisposition> {
        async { AttackIntentDisposition::Rejected }
    }

    /// ATTACK-1b: one `FIGHT_MODES_INTENT` (command 12) of the admitted actor.
    fn fight_modes(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
        _modes: FightModes,
    ) -> impl Future<Output = AttackIntentDisposition> {
        async { AttackIntentDisposition::Rejected }
    }

    /// ATTACK-1b: the admitted actor's `ACTOR_COMBAT_STATE` (domain 10) for the snapshot, or
    /// `None` while it has none, which ends a session that selected capability 17.
    fn observe_combat_state(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<ActorCombatState>> {
        async { None }
    }

    /// MAP-WIRE-2: domain 17 for the admitted actor's current position, through `view` and the
    /// map handles of `items`: a snapshot when `view` has sent nothing, else a delta or nothing.
    /// `None` when this authority serves no world map: a session that selected capability 18
    /// then fails closed. MAP-CUTOVER-1 composes the bundle World's map facts here.
    fn observe_world_map(
        &self,
        _actor: ExactActorRef,
        _view: &mut SessionMapView,
        _items: &mut SessionItemView,
    ) -> impl Future<Output = Option<Result<Option<MapUpdate>, MapViewError>>> {
        async { None }
    }

    /// ATTACK-1b: run the Channel's due auto-attack swings, every [`COMBAT_REFRESH`] while the
    /// session has domain 10, and return the actor's current combat state.
    fn tick_combat(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> impl Future<Output = Option<ActorCombatState>> {
        async { None }
    }

    /// After `wait` without restored control, record authoritative unexpected control loss
    /// for the ended admitted connection (`DISCONNECT-PROTECTION-V1` §§1, 4).
    fn lose_control(
        &self,
        _admitted: AdmittedSession,
        _wait: std::time::Duration,
        _reset: Option<&crate::durability::world_reset::RetiringWorldReset>,
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

    /// LOGOUT-WIRE-1 (§1.6): retire the Channel actor of a session whose current row is
    /// TERMINAL. `Unknown` when the row cannot be read; the lifecycle runs it again.
    fn reconcile_terminal(
        &self,
        _admitted: AdmittedSession,
    ) -> impl Future<Output = GraceExpiryResult> {
        async { GraceExpiryResult::NotApplicable }
    }

    /// LOGOUT-WIRE-1 (§1.6): one `LOGOUT_INTENT` of the admitted session. Authorities that end
    /// no session refuse it as `BUSY`, with no effect.
    fn logout(&self, _admitted: AdmittedSession) -> impl Future<Output = LogoutReply> {
        async { LogoutReply::Result(LogoutResult::Busy) }
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
pub enum AccountAchievementsReply {
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
pub struct StepOutcome {
    pub disposition: StepDisposition,
    pub moved_to: Option<WorldSpatialObservation>,
}

impl StepOutcome {
    pub const fn rejected() -> Self {
        Self {
            disposition: StepDisposition::Rejected,
            moved_to: None,
        }
    }

    /// SPEED-1: a second step requested while one waits in the pacing buffer. Encoded as
    /// `TOO_EARLY` only for a session that selected capability 13, otherwise as `REJECTED`.
    pub const fn too_early() -> Self {
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
pub struct UseCommand {
    pub game_session_id: GameSessionId,
    pub command_id: u64,
    pub item_fence: Option<CurrentCharacterItemFence>,
}

/// The outcome of one `USE_INTENT`: its disposition and, only when it committed a transition,
/// the resulting `WORLD_OBJECT_OVERLAY` delta entry (USE-WIRE-V1, #162 5868482467).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseOutcome {
    pub disposition: oteryn_protocol_oteryn::world_object::UseDisposition,
    pub committed: Option<oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry>,
    /// ACH-NOTIFY-1/2: what the grant of this `USE` earned, after its commit. The loop acts on
    /// it only for a session that selected capability 8.
    pub earned: EarnedNotice,
    /// The gate message id of a `SEALED` outcome (1 to 127): carried to the result under
    /// capability 11, required by the codec for `SEALED` and absent for every other disposition.
    pub gate_message_id: Option<u8>,
}

impl UseOutcome {
    pub const fn rejected() -> Self {
        Self {
            disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Rejected,
            committed: None,
            earned: EarnedNotice::NoneEarned,
            gate_message_id: None,
        }
    }
}

/// The notice of one command's achievement grant (ACH-NOTIFY-2). `Unknown` is never
/// `NoneEarned`: with capability 8 it ends the connection, so the client resynchronises the
/// watermark from the domain-13 snapshot. The grant itself stays durable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EarnedNotice {
    /// No grant applied, or it was `AlreadyHeld` or `Retired`.
    NoneEarned,
    /// A committed `Granted` grant and its notice.
    Earned(AchievementEarned),
    /// A grant may have committed `Granted`, but its notice could not be derived.
    Unknown,
}

/// Fresh identifiers for one connection attempt.
pub trait ConnectionIdentifiers {
    /// `None` when no unpredictable identifier can be produced.
    fn game_session_id(&self) -> Option<GameSessionId>;
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionEnd {
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
    /// LOGOUT-WIRE-1 (§1.6): admitted, then logged out. The terminal release committed before
    /// `ACCEPTED` was written, so no control loss follows.
    AdmittedThenLoggedOut(AdmittedSession),
    /// The transport failed before admission.
    TransportFailed,
}

/// Admitted connection generation issued with `ServerAccepted`.
const ADMITTED_GENERATION: u64 = 1;

/// FND-02 §17 liveness cadence of one admitted connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LivenessPolicy {
    pub interval: std::time::Duration,
    /// Consecutive unanswered probes that prove playable control lost.
    pub missed_limit: u32,
}

/// Out-of-combat cadence: `FND04B-LIVENESS-IDLE-PROBE-MS` and `FND04B-LIVENESS-IDLE-MISSED`
/// (`DISCONNECT-PROTECTION-V1` §1, provisional until measured). The combat cadence is registered
/// but not composed: no combat state exists yet.
pub const IDLE_LIVENESS: LivenessPolicy = LivenessPolicy {
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

#[cfg(test)]
mod test_support;
#[cfg(test)]
pub use test_support::serve_connection;

/// Decode the entry frame, obtain the owning authority's decision and, only on
/// commit, write `ServerAccepted`.
#[allow(
    clippy::result_large_err,
    reason = "one terminal end per connection; boxing would only add an allocation"
)]
pub async fn admit_frame<S, A, I>(
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
    super::spectate_session::refuse_observer_bootstrap(stream, frame).await?;
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
                .await;
            let resumed = match resumed {
                Ok(resumed) => resumed,
                // ADMIT0 §4: the owner reached this only after authenticating the recovery
                // credential, so the typed refusal may be revealed. One frame, then close.
                Err(AdmissionRefusal::RecoveryCapabilityRequired) => {
                    let _ = tokio::time::timeout(
                        ADMISSION_REFUSAL_WRITE_BOUND,
                        send_error(
                            stream,
                            FoundationProtocolError::RecoveryCapabilityRequired,
                            0,
                        ),
                    )
                    .await;
                    return Err(ConnectionEnd::AdmissionRefused(
                        AdmissionRefusal::RecoveryCapabilityRequired,
                    ));
                }
                // Every other refusal stays silent: nothing about the session is revealed.
                Err(_) => return Err(ConnectionEnd::ResumeUnavailable),
            };
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
    // WEATHER-1: capability 26 is offered only while the authority holds a weather source.
    let offered = with_weather_offer(
        authority.offered_capabilities(),
        authority.world_weather().is_some(),
    );
    let offered =
        super::appearance_change::offer::offered(&offered, authority.appearance_available());
    let offered = super::daily_reward::offered(&offered, authority.daily_reward_available());
    let offered = super::cyclopedia::offered(&offered, authority.cyclopedia_available());
    let Some(selected) = SelectedCapabilities::select(&offered, bootstrap.supported_capabilities)
    else {
        return Err(ConnectionEnd::AdmissionRefused(
            AdmissionRefusal::Unavailable,
        ));
    };
    // ADMIT0 §4: no capability refusal precedes the owner's grant authentication; the owner
    // checks the Channel's required set itself and answers ADMISSION_CAPABILITY_REQUIRED.
    let attempt = FreshAdmissionAttempt {
        character_id: bootstrap.character_id,
        admission_material: bootstrap.admission_material,
        game_session_id,
        transport,
        supported_capabilities: bootstrap.supported_capabilities,
    };
    let mut admitted = match authority.admit(attempt).await {
        Ok(admitted) => admitted,
        Err(refusal) => {
            if let AdmissionRefusal::Classified(error) = refusal {
                // N8: exactly one ProtocolError at generation 0, then close. A slow or
                // failed peer write changes nothing: the refusal already stands.
                let _ = tokio::time::timeout(
                    ADMISSION_REFUSAL_WRITE_BOUND,
                    send_error(stream, error, 0),
                )
                .await;
            }
            return Err(ConnectionEnd::AdmissionRefused(refusal));
        }
    };
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
pub async fn hold_admitted<S>(stream: &mut S, admitted: AdmittedSession) -> ConnectionEnd
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

/// ARCH-PROGRESSION-SOURCE-0 §1.5: an admitted session that is not playable but holds a runtime
/// actor is held at most `policy.interval * policy.missed_limit`; then the connection closes
/// with no frame, so a silent client cannot keep its committed GameSession and slot. A frame
/// before the bound ends it as `hold_admitted` does. A session with no actor is held unbounded.
async fn hold_refused<S>(
    stream: &mut S,
    admitted: AdmittedSession,
    policy: LivenessPolicy,
) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    if admitted.runtime_actor.is_none() {
        return hold_admitted(stream, admitted).await;
    }
    let bound = policy.interval.saturating_mul(policy.missed_limit);
    match tokio::time::timeout(bound, hold_admitted(stream, admitted)).await {
        Ok(end) => end,
        Err(_) => {
            let _ = tokio::io::AsyncWriteExt::shutdown(stream).await;
            ConnectionEnd::AdmittedThenDisconnected(admitted)
        }
    }
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

/// CHASE-1 (`RANGED0-RL-03`): a chase search the Channel budget refused asks again one owner
/// window later.
const CHASE_SEARCH_RETRY: std::time::Duration =
    std::time::Duration::from_micros(crate::combat_attack::CHASE_WINDOW_MICROS);

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

/// VIS-3: the frame of one visibility refresh after `sequence` and domain 1 `revision`, with the
/// sequence and revision it leaves: a sequenced delta to the next revision, or nothing.
///
/// A change above the delta ceiling (MOVE-RL-11 §4.3 resync) returns `None`, and the caller ends
/// the connection with its continuity at the last delivered sequence and revision. An admitted
/// client accepts no `SnapshotBegin` after its join (`oteryn-session` reads only `StateDelta`,
/// `CommandResult` and `LivenessProbe` there), so the resync snapshot is the one the resume of
/// that continuity sends from the interest set. `None` also fails closed on an encoding error.
fn visibility_frames(
    generation: u64,
    sequence: u64,
    revision: u64,
    selected: &[u32],
    lights: &EntityLights,
    update: &VisibilityUpdate,
) -> Option<(Vec<Vec<u8>>, u64, u64)> {
    match update {
        VisibilityUpdate::Unchanged => Some((Vec::new(), sequence, revision)),
        VisibilityUpdate::Delta(delta) => {
            let (delta_sequence, new_revision) =
                (sequence.checked_add(1)?, revision.checked_add(1)?);
            let (delta_type, payload) =
                encode_visibility_delta_lit(selected, delta, lights).ok()?;
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
        VisibilityUpdate::Snapshot(_) => None,
    }
}

/// CLOCK-1: the domain 18 delta after `revision`, with its sequence and new revision.
fn world_clock_delta(
    generation: u64,
    sequence: u64,
    revision: u64,
    clock: WorldClock,
) -> Option<(u64, Vec<u8>, u64)> {
    let (delta_sequence, new_revision) = (sequence.checked_add(1)?, revision.checked_add(1)?);
    let payload = encode_world_clock(clock).ok()?;
    let frame = encode_state_delta(
        generation,
        delta_sequence,
        STATE_DOMAIN_WORLD_CLOCK,
        revision,
        new_revision,
        DELTA_TYPE_WORLD_CLOCK_V1,
        &payload,
    )
    .ok()?;
    Some((delta_sequence, frame, new_revision))
}

/// WEATHER-1: the full-replacement domain 22 value for the actor's area at the authority's UTC
/// time fact. `Unavailable` when the authority holds no source (lost after selection) or no time
/// fact.
fn observe_weather<A: FreshAdmissionAuthority>(
    authority: &A,
    position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
) -> WorldWeather {
    match (authority.world_weather(), authority.world_clock_utc_ms()) {
        (Some(source), Some(now)) => source.observe(position, now),
        _ => WorldWeather::Unavailable,
    }
}

/// WEATHER-1: the domain 22 delta after `revision`, with its sequence and new revision.
fn world_weather_delta(
    generation: u64,
    sequence: u64,
    revision: u64,
    weather: WorldWeather,
) -> Option<(u64, Vec<u8>, u64)> {
    let (delta_sequence, new_revision) = (sequence.checked_add(1)?, revision.checked_add(1)?);
    let payload = encode_world_weather(weather).ok()?;
    let frame = encode_state_delta(
        generation,
        delta_sequence,
        STATE_DOMAIN_WORLD_WEATHER,
        revision,
        new_revision,
        DELTA_TYPE_WORLD_WEATHER_V1,
        &payload,
    )
    .ok()?;
    Some((delta_sequence, frame, new_revision))
}

pub async fn serve_admitted<S, A>(
    stream: &mut S,
    admitted: AdmittedSession,
    authority: &A,
    policy: LivenessPolicy,
) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let (actor, session) = (admitted.runtime_actor, admitted.game_session_id);
    let end = serve_admitted_actor(stream, admitted, authority, policy).await;
    // CHASE-1 (`RANGED0-RL-03`): the ended connection drives no more chase, so its waiting
    // search leaves the Channel's ordering now rather than holding a place through the grace.
    if let Some(actor) = actor {
        authority.yield_chase_to_client(actor, session).await;
    }
    end
}

/// The `ACTOR_VITALS` delta (domain 3, delta type 1) from the last transmitted revision to `to`, at the
/// sequence after `sequence`; `None` on an encoding fault. The complete value
/// may coalesce multiple actual owner changes from the last transmitted revision.
/// Dispositions 7 to 10 exist only under capability 15 (ITEM-USE-0 §3): a session without it
/// gets `Rejected`, the one refusal it can decode, for any of them. So does a session without
/// capability 11 for the dispositions 11 to 17 (WORLD-INTERACTION-0 §10.1), and a session without
/// capability 22 for the dispositions 18 and 19 (RUNE-USE-0 §4).
fn negotiated_use_disposition(
    capabilities: UseCapabilities,
    disposition: UseDisposition,
    gate_message_id: Option<u8>,
) -> (UseDisposition, Option<u8>) {
    let UseCapabilities {
        item_use,
        world_interaction,
        rune_use,
    } = capabilities;
    if (disposition.needs_world_interaction() && !world_interaction)
        || (disposition.needs_rune_use() && !rune_use)
    {
        return (UseDisposition::Rejected, None);
    }
    // `SEALED` carries its gate message id and no other disposition does; a mismatch is a server
    // fault, and the one refusal every session can decode replaces it.
    let valid_gate = match (disposition, gate_message_id) {
        (UseDisposition::Sealed, Some(id)) => (1..=MAX_GATE_MESSAGE_ID).contains(&id),
        (UseDisposition::Sealed, None) | (_, Some(_)) => false,
        (_, None) => true,
    };
    if !valid_gate {
        return (UseDisposition::Rejected, None);
    }
    match disposition {
        UseDisposition::RequirementNotMet
        | UseDisposition::Exhausted
        | UseDisposition::Full
        | UseDisposition::NoTarget
            if !item_use =>
        {
            (UseDisposition::Rejected, None)
        }
        other => (other, gate_message_id),
    }
}

/// The capabilities that widen the USE result: 15 `ITEM_USE_V1`, 11 `WORLD_INTERACTION_V1` and 22
/// `RUNE_USE_V1`.
#[derive(Debug, Clone, Copy)]
struct UseCapabilities {
    item_use: bool,
    world_interaction: bool,
    rune_use: bool,
}

fn vitals_delta(
    generation: u64,
    sequence: u64,
    from: u64,
    to: u64,
    value: &ActorVitals,
) -> Option<(u64, Vec<u8>)> {
    let delta_sequence = sequence.checked_add(1)?;
    if from == 0 || to <= from {
        return None;
    }
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

/// SPEED-1 with CHASE-1 (RANGED-0 §8): claim `actor`'s pacing clock for one step of the
/// connection owning `holder`; the claim holds until [`record_actor_step`], or until the
/// connection and its holder are gone. `Some(due)` when another connection's deadline or step in
/// flight still holds it; the connection's copy then waits for it too.
async fn begin_actor_step<A: FreshAdmissionAuthority>(
    pacer: &mut StepPacer,
    authority: &A,
    actor: ExactActorRef,
    holder: &std::sync::Arc<()>,
) -> Option<tokio::time::Instant> {
    let due = authority.begin_actor_step(actor, holder).await?;
    *pacer = StepPacer::resumed(Some(due));
    Some(due)
}

/// SPEED-1 with CHASE-1: the step claimed by [`begin_actor_step`] ended; its pacing clock
/// starts now, on the connection's copy and on the actor's own deadline (RANGED-0 §8). Only a
/// moved step has a duration; any other end releases the claim.
async fn record_actor_step<A: FreshAdmissionAuthority>(
    pacer: &mut StepPacer,
    authority: &A,
    actor: ExactActorRef,
    holder: &std::sync::Arc<()>,
    duration: Option<std::time::Duration>,
) {
    pacer.record(tokio::time::Instant::now(), duration);
    let due = duration.and(pacer.next_due());
    authority
        .record_actor_step_deadline(actor, holder, due)
        .await;
}

/// MAP-WIRE-2: the domain 17 delta frame at the sequence after `sequence`, `Some(None)` when no
/// tile in view changed, or `None` when the session must end: the map is unserved or does not
/// encode, or it needs a snapshot (a jump or a delta over `MAPW-RL-03`), which a running
/// connection does not send, so the client resumes and gets one.
async fn world_map_delta<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    generation: u64,
    sequence: u64,
    view: &mut SessionMapView,
    items: &mut SessionItemView,
) -> Option<Option<(u64, Vec<u8>)>> {
    match authority.observe_world_map(actor, view, items).await? {
        Ok(None) => Some(None),
        Ok(Some(MapUpdate::Delta(delta))) => {
            item_view_delta(generation, sequence, &delta).map(Some)
        }
        Ok(Some(MapUpdate::Snapshot(_))) | Err(_) => None,
    }
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

/// The domain 12 line delta from `revision` at the sequence after `sequence`, with the new
/// revision; `None` on an encoding fault.
fn chat_line_delta(
    generation: u64,
    sequence: u64,
    revision: u64,
    line: &ChatLine,
) -> Option<(u64, Vec<u8>, u64)> {
    let payload = encode_chat_line(line).ok()?;
    chat_delta(
        generation,
        sequence,
        revision,
        DELTA_TYPE_CHAT_LINE_V1,
        &payload,
    )
}

/// One domain 12 delta of `delta_type` from the sequence after `sequence` and the revision
/// after `revision`: `(sequence, frame, revision)` of the delta, `None` on an encoding fault.
fn chat_delta(
    generation: u64,
    sequence: u64,
    revision: u64,
    delta_type: u32,
    payload: &[u8],
) -> Option<(u64, Vec<u8>, u64)> {
    let delta_sequence = sequence.checked_add(1)?;
    let to = revision.checked_add(1)?;
    let frame = encode_state_delta(
        generation,
        delta_sequence,
        STATE_DOMAIN_CHAT,
        revision,
        to,
        delta_type,
        payload,
    )
    .ok()?;
    Some((delta_sequence, frame, to))
}

/// The domain 7 and 8 delta frames of `update`, in that order, from the sequence after
/// `sequence` (advanced to the last one); `npc_shown` follows the conversation. `None` on an
/// encoding fault.
fn npc_update_frames(
    generation: u64,
    sequence: &mut u64,
    continuity: &mut NpcContinuity,
    update: &NpcUpdate,
    npc_shown: &mut Option<WireEntityRef>,
) -> Option<Vec<Vec<u8>>> {
    let mut frames = Vec::new();
    let mut next = *sequence;
    let mut conversation_revision = continuity.conversation_revision;
    let mut window_revision = continuity.window_revision;
    let mut shown = *npc_shown;
    if let Some(conversation) = &update.conversation {
        let (delta_sequence, frame, revision) =
            npc_conversation_delta(generation, next, conversation_revision, conversation)?;
        (next, conversation_revision) = (delta_sequence, revision);
        shown = conversation.open.map(|(npc, _)| npc);
        frames.push(frame);
    }
    if let Some(window) = &update.window {
        let delta_sequence = next.checked_add(1)?;
        let to = window_revision.checked_add(1)?;
        let frame = encode_state_delta(
            generation,
            delta_sequence,
            STATE_DOMAIN_NPC_TRADE_WINDOW,
            window_revision,
            to,
            DELTA_TYPE_NPC_TRADE_WINDOW_V1,
            &encode_npc_trade_window(window).ok()?,
        )
        .ok()?;
        (next, window_revision) = (delta_sequence, to);
        frames.push(frame);
    }
    *sequence = next;
    continuity.conversation_revision = conversation_revision;
    continuity.window_revision = window_revision;
    *npc_shown = shown;
    Some(frames)
}

/// The domain 7 delta replacing the whole conversation, from `revision` at the sequence after
/// `sequence`, with the new revision; `None` on an encoding fault.
fn npc_conversation_delta(
    generation: u64,
    sequence: u64,
    revision: u64,
    conversation: &NpcConversation,
) -> Option<(u64, Vec<u8>, u64)> {
    let delta_sequence = sequence.checked_add(1)?;
    let to = revision.checked_add(1)?;
    let payload = encode_npc_conversation(conversation).ok()?;
    let frame = encode_state_delta(
        generation,
        delta_sequence,
        STATE_DOMAIN_NPC_CONVERSATION,
        revision,
        to,
        DELTA_TYPE_NPC_CONVERSATION_V1,
        &payload,
    )
    .ok()?;
    Some((delta_sequence, frame, to))
}

/// LOGOUT-RL-01: whether `frame` is a `LOGOUT_INTENT` of this connection generation.
fn is_logout_command(frame: &[u8], generation: u64) -> bool {
    decode_wire_envelope(frame).is_ok_and(|envelope| {
        envelope.message_type() == MessageType::ClientCommand
            && envelope
                .client_command(generation)
                .is_ok_and(|command| command.command_type == COMMAND_TYPE_LOGOUT_INTENT)
    })
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
mod tests;

/// VIS-3 (MOVE-RL-11 §4): domain 1 composed from the interest set on the connection.
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod visibility_tests;

#[cfg(test)]
#[path = "item_look_dispatch_tests.rs"]
mod item_look_dispatch_tests;
