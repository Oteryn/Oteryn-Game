//! One gameplay connection. The transport decodes bounded Foundation frames and
//! hands the validated bootstrap to the owning admission authority; it never
//! decides admission itself and fails closed for every message it does not own.

pub(super) mod appearance_change;
mod moved_step;
use crate::combat_pickup::GroundPickupError;
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
    super::appearance_change::authority_methods!();
    super::guild_intent::authority_methods!();
    super::daily_reward::authority_methods!();
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

/// Whole connection lifecycle without resource policy; the listener runs the
/// same phases under its budgets.
#[cfg(test)]
pub async fn serve_connection<S, A, I>(
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

async fn serve_admitted_actor<S, A>(
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
        return hold_refused(stream, admitted, policy).await;
    };
    let Some(baseline) = authority.observe(actor).await else {
        return hold_refused(stream, admitted, policy).await;
    };
    let mut revision = admitted.continuity.spatial_revision;
    // VIS-3: domain 1 goes first; with capability 6 it is composed after the item view below, so
    // that its objects carry their item handles under capability 4.
    let spatial: (u32, Vec<u8>);
    let mut domains = Vec::new();
    // USE-WIRE-V1 (#162 5868482467): for a session served domain 2 (a fixture World session,
    // without capability 18), the join/resync snapshot also carries the door's current
    // `WORLD_OBJECT_OVERLAY`, the same code path a reconnect resumes through, so a resumed
    // connection gets the door's current overlay exactly as a fresh join does.
    // MAP-WIRE-1 contract §3: a session that selected capability 18 is served by a bundle World,
    // which carries its object states in domain 17 and never serves domain 2.
    let overlay_served = admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_WORLD_OBJECT_OVERLAY);
    let overlay = authority.observe_world_object_overlay().await;
    match (&overlay, admitted.continuity.overlay_revision) {
        // Overlay state for a session with 18 has no domain 17 form: fail closed rather than
        // send domain 2 or drop the state.
        (Some(_), _) if !overlay_served => {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
        // Absence qualifies the initial empty domain, not a reset of an overlay revision already
        // delivered on this session: refuse an unavailable owner. A session with 18 never holds
        // a revision above 0.
        (None, 1..) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
        _ => {}
    }
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
    let overlay_payload;
    let overlay_revision = overlay.as_ref().map_or(0, |entry| entry.revision);
    if overlay_served {
        // A session served domain 2 (without capability 18): an empty overlay is still the
        // registered domain's complete snapshot. Source-qualified maps have no legacy entry-room
        // door; publish no invented placement, while allowing the client to distinguish empty
        // from missing.
        let Ok(bytes) = encode_world_object_overlay_snapshot(overlay.as_slice()) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        overlay_payload = bytes;
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: overlay_revision,
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
    let mut party_view = super::party_command::SessionPartyView::default();
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
        if admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_WORLD_MAP_VIEW)
        {
            view = view.with_map_view();
        }
        if admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ITEM_EQUIP_DROP_V1)
        {
            view = view.with_equip_drop();
        }
        let Some(inventory) =
            item_move::observe_inventory(authority, actor, admitted.game_session_id, &view).await
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
    let mut appearances = SessionAppearance::new(selected_capabilities);
    let mut visibility = None;
    if selected_capabilities.contains(CAPABILITY_WORLD_SPATIAL_ENTITIES) {
        let Some(channel) = authority.observe_visible_entities(actor).await else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        let mut view = SessionVisibility::with_objects(item_view.is_some())
            .lit(selected_capabilities.contains(CAPABILITY_WORLD_LIGHT_V1));
        let Some(encoded) = appearances
            .initial(
                authority,
                actor,
                &admitted,
                &mut view,
                &channel,
                item_view.as_mut(),
            )
            .await
        else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        if let Some(item_view) = &item_view {
            admitted.continuity.item_view = item_view.continuity();
        }
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
    // CHAT-WIRE-1: with capability 7, domain 12 with the open rooms, above every revision the
    // session has seen. Lines are never replayed.
    let chat_snapshot: Vec<u8>;
    let chat_active = admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_CHAT);
    if chat_active {
        let rooms = authority.chat_admit(actor, admitted.game_session_id).await;
        let Some(revision) = admitted.continuity.chat_revision.checked_add(1) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        admitted.continuity.chat_revision = revision;
        let Some((snapshot_type, payload)) =
            super::guild_chat::wire::snapshot(authority, admitted, rooms).await
        else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        chat_snapshot = payload;
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_CHAT,
            revision,
            snapshot_type,
            payload: &chat_snapshot,
        });
    }
    // NPC-CMD-1a: with capability 3, domain 7 (no conversation: a fresh snapshot ends it) and
    // domain 8 (a closed window), each above its last revision.
    let npc_snapshots: Option<(Vec<u8>, Vec<u8>)> = if admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_NPC_CONVERSATION)
    {
        authority.npc_session_open(admitted.game_session_id).await;
        let (Some(conversation_revision), Some(window_revision)) = (
            admitted.continuity.npc.conversation_revision.checked_add(1),
            admitted.continuity.npc.window_revision.checked_add(1),
        ) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        let (Ok(conversation), Ok(window)) = (
            encode_npc_conversation(&NpcConversation::default()),
            encode_npc_trade_window(&NpcTradeWindow::default()),
        ) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        admitted.continuity.npc = NpcContinuity {
            conversation_revision,
            window_revision,
        };
        Some((conversation, window))
    } else {
        None
    };
    if let Some((conversation, window)) = &npc_snapshots {
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_NPC_CONVERSATION,
            revision: admitted.continuity.npc.conversation_revision,
            snapshot_type: SNAPSHOT_TYPE_NPC_CONVERSATION_V1,
            payload: conversation,
        });
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_NPC_TRADE_WINDOW,
            revision: admitted.continuity.npc.window_revision,
            snapshot_type: SNAPSHOT_TYPE_NPC_TRADE_WINDOW_V1,
            payload: window,
        });
    }
    // ATTACK-1b: with capability 17, domain 10 above every revision the session has seen. The
    // target was cleared by the reconnect or transfer; the in-fight deadline survives it.
    let combat_snapshot = if admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_ACTOR_COMBAT_STATE)
    {
        let Some(state) = authority
            .observe_combat_state(actor, admitted.game_session_id)
            .await
        else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        let Some((revision, payload)) = admitted.continuity.combat.snapshot(&state) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        Some((revision, payload, state))
    } else {
        None
    };
    if let Some((revision, payload, _)) = &combat_snapshot {
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_ACTOR_COMBAT_STATE,
            revision: *revision,
            snapshot_type: SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
            payload,
        });
    }
    // MAP-WIRE-2: with capability 18 (which requires 4), domain 17 above every revision the
    // session has seen, with fresh map handles. It must fit the single snapshot chunk.
    let mut world_map = None;
    let map_snapshot;
    if let Some(items) = item_view.as_mut().filter(|_| {
        admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_WORLD_MAP_VIEW)
    }) {
        let mut view = SessionMapView::starting_after(admitted.continuity.world_map_revision);
        let observed = authority.observe_world_map(actor, &mut view, items).await;
        admitted.continuity.item_view = items.continuity();
        let Some(Ok(Some(MapUpdate::Snapshot(snapshot)))) = observed else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        admitted.continuity.world_map_revision = view.revision();
        map_snapshot = snapshot;
        domains.push(DomainSnapshot {
            domain_id: map_snapshot.domain_id,
            revision: map_snapshot.revision,
            snapshot_type: map_snapshot.snapshot_type,
            payload: &map_snapshot.payload,
        });
        world_map = Some(view);
    }
    // CLOCK-1 (§11.3): with capability 19 (which requires 6), domain 18 at the session's current
    // revision; a resume or transfer sends only this snapshot and never replays a delta.
    let world_clock = WorldClockSource::new(DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS);
    let mut world_clock_pacer = WorldClockDeltaPacer::default();
    let clock_snapshot;
    if selected_capabilities.contains(CAPABILITY_WORLD_LIGHT_V1) {
        let Some((now, reading)) = authority
            .world_clock_utc_ms()
            .and_then(|now| Some((now, world_clock.reading(now)?)))
        else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        let Ok(payload) = encode_world_clock(reading) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        world_clock_pacer.reset(reading, now);
        clock_snapshot = payload;
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_CLOCK,
            revision: admitted.continuity.world_clock_revision,
            snapshot_type: SNAPSHOT_TYPE_WORLD_CLOCK_V1,
            payload: &clock_snapshot,
        });
    }
    // WEATHER-1 (D1026): with capability 26 (which requires 6), domain 22 at the session's current
    // revision; a resume or transfer sends only this full snapshot, never a delta replay, and a
    // lost binding is one full `Unavailable`, never a fictitious preset.
    let mut weather_sent = None;
    let weather_snapshot;
    if selected_capabilities.contains(CAPABILITY_WORLD_WEATHER_V1) {
        let observed = observe_weather(authority, baseline.actor_position);
        let Ok(payload) = encode_world_weather(observed) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        weather_sent = Some(observed);
        weather_snapshot = payload;
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_WEATHER,
            revision: admitted.continuity.world_weather_revision,
            snapshot_type: SNAPSHOT_TYPE_WORLD_WEATHER_V1,
            payload: &weather_snapshot,
        });
    }
    appearances.domain(&mut domains);
    domains.extend(tutorial_notices_transport::snapshot(&admitted));
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
    admitted.continuity.overlay_revision = overlay_revision;
    if let Some((vitals_revision, _)) = &vitals {
        admitted.continuity.vitals_revision = *vitals_revision;
    }
    if let Some((revision, _, state)) = &combat_snapshot {
        admitted.continuity.combat.snapshot_sent(*revision, *state);
    }
    let mut sequence = admitted.continuity.server_sequence;
    let mut next_command = admitted.continuity.next_command_id;
    // NPC-ECON-1: only a connection that continues an earlier one of this GameSession can carry
    // an accepted original Bank command. A fresh admission (generation 1 at CommandId 1, FND-02)
    // is the session's first connection, so its first command never touches the Bank journal;
    // a same-session resume (FND-04B §16: a strictly newer generation) or any other carried
    // continuity reconciles it before replacement input.
    let mut reconcile_original_bank = admitted.continuity.connection_generation
        != ADMITTED_GENERATION
        || admitted.continuity.next_command_id != 1;
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
    // CHAT-WIRE-1: only a session with domain 12 drains its queued lines.
    let mut chat_refresh = chat_active.then(|| {
        let mut refresh =
            tokio::time::interval_at(tokio::time::Instant::now() + CHAT_REFRESH, CHAT_REFRESH);
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        refresh
    });
    // NPC-CMD-1a: a session with domain 7 follows its conversation (chat greeting, walking away).
    let mut npc_refresh = npc_snapshots.is_some().then(|| {
        let mut refresh =
            tokio::time::interval_at(tokio::time::Instant::now() + CHAT_REFRESH, CHAT_REFRESH);
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        refresh
    });
    // The NPC domain 7 shows as the conversation partner; a fresh snapshot shows none.
    let mut npc_shown: Option<WireEntityRef> = None;
    // ATTACK-1b: only a session with domain 10 drives swings and follows its combat state.
    let mut combat_refresh = combat_snapshot.is_some().then(|| {
        let mut refresh =
            tokio::time::interval_at(tokio::time::Instant::now() + COMBAT_REFRESH, COMBAT_REFRESH);
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
    // SPEED-1 (CONDITIONS-0 §4.3): the actor's pacing clock, this connection's one-step buffer
    // and the frames read while a step waits in it. Those frames run, in order, only after the
    // buffered step's result, because results commit in CommandId order (FND-02 §13); a step
    // among them arrived while the buffer was full and is refused. The buffer ends with the
    // connection, so a resumed connection starts with an empty one; the clock is the actor's
    // (CHASE-1, RANGED-0 §8): it starts from the actor's deadline and records to it, so a
    // replaced or resumed connection never steps sooner.
    let mut pacer = StepPacer::resumed(authority.actor_step_deadline(actor).await);
    // CHASE-1: this connection's claims on the actor's pacing clock live while it does.
    let step_holder = std::sync::Arc::new(());
    // STEP-PACE-WIRE-1: fixed for the connection; off, every step result keeps its former bytes.
    let step_wait_receipts = authority.step_wait_receipts();
    let mut buffered: Option<BufferedStep> = None;
    // Each held frame carries whether it arrived behind a buffered step (SPEED-1) rather than
    // behind a running logout (LOGOUT-RL-01).
    let mut held: std::collections::VecDeque<(Vec<u8>, bool)> = std::collections::VecDeque::new();
    // CHASE-1 (RANGED-0 §8): when the next chase step is checked, on the same pacing clock as a
    // client step; at most one is pending. Armed by a combat tick that holds a chasing target
    // and by a chase step that moved.
    let mut chase_due: Option<tokio::time::Instant> = None;
    // CHASE-1: a ready client frame was already read ahead of the due chase step; the next poll
    // runs that step first, so non-step traffic never postpones it indefinitely.
    let mut chase_yielded = false;
    loop {
        enum Next {
            Frame(std::io::Result<Vec<u8>>),
            /// A frame read while a step waited in the buffer, or while a logout ran.
            Held(Vec<u8>, bool),
            StepDue,
            Probe,
            Serene,
            QuestLog,
            Chat,
            Visibility,
            Combat,
            Chase,
            Npc,
        }
        // All futures are cancel-safe: the frame reader keeps partial bytes and a dropped
        // interval tick is not consumed. The ticks are polled first so a client that keeps
        // frames flowing cannot starve a cadence; each is ready at most once per interval.
        let held_frame = if buffered.is_none() {
            held.pop_front()
        } else {
            None
        };
        let next = if let Some((frame, behind_step)) = held_frame {
            Next::Held(frame, behind_step)
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
            let mut chat_tick = std::pin::pin!(async {
                match chat_refresh.as_mut() {
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
            let mut npc_tick = std::pin::pin!(async {
                match npc_refresh.as_mut() {
                    Some(refresh) => refresh.tick().await,
                    None => std::future::pending().await,
                }
            });
            let mut combat_tick = std::pin::pin!(async {
                match combat_refresh.as_mut() {
                    Some(refresh) => refresh.tick().await,
                    None => std::future::pending().await,
                }
            });
            let mut chase_tick = std::pin::pin!(async {
                match chase_due {
                    Some(due) => tokio::time::sleep_until(due).await,
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
                if chat_tick.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::Chat);
                }
                if visibility_tick.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::Visibility);
                }
                if combat_tick.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::Combat);
                }
                if npc_tick.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::Npc);
                }
                if step_due.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(Next::StepDue);
                }
                // CHASE-1: one ready client frame is read before a due chase step, so a client
                // step wins over the chase (RANGED-0 §8) and is never paced behind it; after
                // that frame the due chase step runs before the next one.
                let chase_ready = chase_tick.as_mut().poll(context).is_ready();
                if chase_ready && chase_yielded {
                    return std::task::Poll::Ready(Next::Chase);
                }
                if reading && let std::task::Poll::Ready(read) = read.as_mut().poll(context) {
                    chase_yielded = chase_ready;
                    return std::task::Poll::Ready(Next::Frame(read));
                }
                if chase_ready {
                    return std::task::Poll::Ready(Next::Chase);
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
            Next::Held(frame, behind_step) => (frame, behind_step),
            Next::Serene => {
                let changed = authority.tick_vitals(actor, admitted.game_session_id).await;
                let current = authority
                    .observe_vitals(actor, admitted.game_session_id)
                    .await;
                let Some((to, value)) = current
                    .filter(|(revision, _)| *revision > admitted.continuity.vitals_revision)
                    .or(changed
                        .filter(|(revision, _)| *revision > admitted.continuity.vitals_revision))
                else {
                    continue;
                };
                let Some((delta_sequence, delta)) = vitals_delta(
                    generation,
                    sequence,
                    admitted.continuity.vitals_revision,
                    to,
                    &value,
                ) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                sequence = delta_sequence;
                admitted.continuity.server_sequence = sequence;
                if write_frame(stream, &delta).await.is_err() {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                admitted.continuity.vitals_revision = to;
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
            Next::Chat => {
                // CHAT-WIRE-1: every heard line queued since the last tick is one domain 12
                // delta; an empty queue sends nothing.
                // At most one full queue per refresh: lines that keep arriving cannot hold the
                // connection here, and the rest waits for the next tick.
                let mut owner_reads = EGRESS_MAX_LINES;
                for _ in 0..EGRESS_MAX_LINES {
                    let Some(line) = authority.chat_next_bound(admitted, &mut owner_reads).await
                    else {
                        break;
                    };
                    let Some((delta_sequence, frame, revision)) = chat_line_delta(
                        generation,
                        sequence,
                        admitted.continuity.chat_revision,
                        &line,
                    ) else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    sequence = delta_sequence;
                    admitted.continuity.server_sequence = sequence;
                    admitted.continuity.chat_revision = revision;
                    if write_frame(stream, &frame).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                    // A marker takes no slot, so only a real line frees one.
                    if !matches!(line, ChatLine::Dropped) {
                        authority.chat_delivered(admitted.game_session_id).await;
                    }
                }
                continue;
            }
            Next::Npc => {
                let Some(update) = authority
                    .npc_follow(actor, admitted.game_session_id, npc_shown)
                    .await
                else {
                    continue;
                };
                let Some(frames) = npc_update_frames(
                    generation,
                    &mut sequence,
                    &mut admitted.continuity.npc,
                    &update,
                    &mut npc_shown,
                ) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                admitted.continuity.server_sequence = sequence;
                for frame in frames {
                    if write_frame(stream, &frame).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
                continue;
            }
            Next::Combat => {
                // ATTACK-1b: due swings run; a changed domain 10 state sends a delta.
                let Some(state) = authority.tick_combat(actor, admitted.game_session_id).await
                else {
                    continue;
                };
                if chase_due.is_none()
                    && state.target.is_some()
                    && state.modes.chase == ChaseMode::Chase
                {
                    let now = tokio::time::Instant::now();
                    chase_due = Some(pacer.wait_until(now).unwrap_or(now));
                }
                let Some(change) = admitted
                    .continuity
                    .combat
                    .delta(generation, sequence, state)
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                let Some((delta_sequence, frame)) = change else {
                    continue;
                };
                sequence = delta_sequence;
                admitted.continuity.server_sequence = sequence;
                if write_frame(stream, &frame).await.is_err() {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                continue;
            }
            Next::Chase => {
                // CHASE-1: a client step waiting in the buffer wins over the chase.
                chase_due = None;
                chase_yielded = false;
                if buffered.is_some() {
                    continue;
                }
                if let Some(due) =
                    begin_actor_step(&mut pacer, authority, actor, &step_holder).await
                {
                    chase_due = Some(due);
                    continue;
                }
                let step = authority.chase_step(actor, admitted.game_session_id).await;
                let duration = step.as_ref().and_then(|(_, duration)| *duration);
                record_actor_step(&mut pacer, authority, actor, &step_holder, duration).await;
                let Some((outcome, _)) = step else {
                    // RANGED0-RL-03: a search the Channel budget refused retries in the next
                    // owner window, not at the next combat tick.
                    if authority
                        .chase_search_waiting(actor, admitted.game_session_id)
                        .await
                    {
                        chase_due = Some(tokio::time::Instant::now() + CHASE_SEARCH_RETRY);
                    }
                    continue;
                };
                let Some(observation) = outcome.moved_to else {
                    continue;
                };
                if moved_step_frames(
                    stream,
                    authority,
                    actor,
                    &mut admitted,
                    visibility.as_mut(),
                    item_view.as_mut(),
                    world_map.as_mut(),
                    generation,
                    &mut sequence,
                    &mut revision,
                    &mut appearances,
                    observation,
                )
                .await
                .is_none()
                {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                let now = tokio::time::Instant::now();
                chase_due = Some(pacer.wait_until(now).unwrap_or(now));
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
                let Some((to_sequence, to_revision)) = appearances
                    .refresh(
                        stream,
                        authority,
                        actor,
                        &mut admitted,
                        view,
                        &channel,
                        item_view.as_mut(),
                        sequence,
                        revision,
                    )
                    .await
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                sequence = to_sequence;
                revision = to_revision;
                admitted.continuity.spatial_revision = revision;
                // MAP-WIRE-2: tiles in view changed by others (an item dropped, a door opened).
                if let (Some(view), Some(items)) = (world_map.as_mut(), item_view.as_mut()) {
                    let update =
                        world_map_delta(authority, actor, generation, sequence, view, items).await;
                    admitted.continuity.item_view = items.continuity();
                    let Some(update) = update else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    if let Some((delta_sequence, frame)) = update {
                        sequence = delta_sequence;
                        admitted.continuity.server_sequence = sequence;
                        admitted.continuity.world_map_revision = view.revision();
                        if write_frame(stream, &frame).await.is_err() {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        }
                    }
                }
                // CLOCK-1 (§11.3): a due domain 18 delta, at most one per 10,000 ms.
                if selected_capabilities.contains(CAPABILITY_WORLD_LIGHT_V1) {
                    let Some(now) = authority.world_clock_utc_ms() else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    if let Some(reading) = world_clock_pacer.due(&world_clock, now) {
                        let Some((delta_sequence, frame, to)) = world_clock_delta(
                            generation,
                            sequence,
                            admitted.continuity.world_clock_revision,
                            reading,
                        ) else {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        };
                        sequence = delta_sequence;
                        admitted.continuity.server_sequence = sequence;
                        admitted.continuity.world_clock_revision = to;
                        if write_frame(stream, &frame).await.is_err() {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        }
                    }
                }
                // WEATHER-1: a full-replacement domain 22 delta only when the observation
                // changed (a new absolute slot, or a lost or restored binding). Visual only: no
                // gameplay owner is read or written.
                if let (Some(sent), Some(here)) = (weather_sent, authority.observe(actor).await) {
                    let observed = observe_weather(authority, here.actor_position);
                    if observed != sent {
                        let Some((delta_sequence, frame, to)) = world_weather_delta(
                            generation,
                            sequence,
                            admitted.continuity.world_weather_revision,
                            observed,
                        ) else {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        };
                        sequence = delta_sequence;
                        admitted.continuity.server_sequence = sequence;
                        admitted.continuity.world_weather_revision = to;
                        if write_frame(stream, &frame).await.is_err() {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        }
                        weather_sent = Some(observed);
                    }
                }
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
                    held.push_back((frame, true));
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
            Vip(Option<Vec<u8>>),
            /// STEP-PACE-WIRE-1: the step and its `step_wait_ms` pacing receipt (0: none).
            Step(StepOutcome, u32),
            Use(UseOutcome),
            Spell(SpellCastOutcome),
            Achievements(AccountAchievementsReply),
            /// RANK-1 (HS-WIRE-1): command 14, a read of the session's World snapshots.
            Highscores(HighscoresReply),
            Guild(Vec<u8>),
            Party(Vec<u8>),
            Appearance(Option<oteryn_protocol_oteryn::appearance::ChangeResult>),
            DailyReward(Option<Vec<u8>>),
            /// ITEM-VIEW-1b: `USE` with an item target; the domain 11 delta follows the result.
            UseItem(OpenDecision, Option<ItemViewDelta>),
            /// BAGS-WIRE-1: command 21; the domain 14 delta follows the result.
            ContainerView(ContainerViewOutcome, Option<ItemViewDelta>),
            /// ITEM-MOVE-1: command 9; the domain 11 and domain 9 deltas follow the result.
            ItemMove(ItemMoveOutcome, Vec<ItemViewDelta>),
            Depot(oteryn_protocol_oteryn::depot::DepotReply),
            /// QUEST-LOG-WIRE-1: an accepted command 22; the domain 16 delta follows the result.
            QuestLog(QuestLogDomain),
            Staff(Option<Vec<u8>>),
            Look(ItemLookResult, ItemLookSelection),
            /// ATTACK-1b: commands 11 and 12; the domain 10 delta follows the result.
            Attack(AttackIntentDisposition),
            /// CHAT-WIRE-1: command 13; lines reach the hearers' domain 12 on their own ticks.
            Chat(ChatIntentResult),
            /// NPC-CMD-1a: commands 7 and 8; the domain 7 payload follows the result.
            Npc(NpcIntentDisposition, NpcUpdate, bool),
            /// NPC-TRADE-2: the result, the domain 9 deltas of a committed trade, then the domain
            /// 7 and 8 payloads.
            NpcTrade(NpcIntentDisposition, NpcUpdate, Vec<ItemViewDelta>),
            /// LOGOUT-WIRE-1: command 23; `ACCEPTED` ends the connection after the result.
            Logout(LogoutResult),
            House(Option<Vec<u8>>),
            Unregistered,
        }
        // LOGOUT-RL-01: a second logout read before the first one's result.
        let mut second_logout = false;
        // A read that failed while a logout ran: the peer is gone once the result is written.
        let mut lost_while_logging_out = false;
        // CAP-NEG-1: a command type owned by a capability the session did not select is refused
        // like an unregistered one, before any decode or authority call.
        // OPS-SAVE-2: the permit is held while the command runs, not while its result is
        // written, so the shutdown snapshot waits for the effect only.
        let input_permit = authority.enter_input().await;
        if input_permit.is_none() && reconcile_original_bank {
            // No current input grant: do not run money work, and do not falsely terminate an
            // accepted original command whose retained Bank status has not been classified.
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
        let retained_bank = if input_permit.is_some() && reconcile_original_bank {
            let outcome = authority
                .resume_npc_bank(
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                )
                .await;
            if outcome.as_ref().is_some_and(|outcome| outcome.unknown) {
                return ConnectionEnd::AdmittedThenDisconnected(admitted);
            }
            reconcile_original_bank = false;
            outcome
        } else {
            None
        };
        let mut npc_relocation = None;
        let mut dispatch = if let Some(outcome) = retained_bank {
            Dispatch::Npc(
                outcome.disposition,
                NpcUpdate {
                    conversation: outcome.conversation,
                    window: outcome.window,
                },
                outcome.inventory_changed,
            )
        } else if input_permit.is_none()
            || !admitted
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
                Err(_) => Dispatch::Step(StepOutcome::rejected(), 0),
                // STEP-PACE-WIRE-1: the receipt is the time left until the pacing clock is due,
                // after the buffered step ran; none when that step did not move.
                Ok(_) if arrived_while_buffered => {
                    let now = tokio::time::Instant::now();
                    let left = pacer.wait_until(now).map(|due| due.duration_since(now));
                    Dispatch::Step(StepOutcome::too_early(), too_early_step_wait_ms(left))
                }
                Ok(direction) => {
                    // CHASE-1: client intent wins; the pending chase step and its route drop.
                    chase_due = None;
                    chase_yielded = false;
                    authority
                        .yield_chase_to_client(actor, admitted.game_session_id)
                        .await;
                    let now = tokio::time::Instant::now();
                    let wait = match pacer.wait_until(now) {
                        Some(due) => Some(due),
                        None => begin_actor_step(&mut pacer, authority, actor, &step_holder).await,
                    };
                    if let Some(due) = wait {
                        buffered = Some(BufferedStep { frame, due });
                        continue;
                    }
                    let (outcome, duration) = authority
                        .paced_step(actor, admitted.game_session_id, direction)
                        .await;
                    // The clock starts when the step committed, so a slow owner turn never
                    // shortens the next step's wait.
                    record_actor_step(&mut pacer, authority, actor, &step_holder, duration).await;
                    // STEP-PACE-WIRE-1: a moved step's receipt is the duration just recorded.
                    let wait = if outcome.disposition == StepDisposition::Moved {
                        moved_step_wait_ms(duration)
                    } else {
                        0
                    };
                    Dispatch::Step(outcome, wait)
                }
            }
        } else if command.command_type == COMMAND_TYPE_BED_SLEEP_INTENT {
            match super::bed_sleep::dispatch(
                authority,
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                decode_bed_sleep_intent(command.payload),
                item_view.as_ref(),
            )
            .await
            {
                Ok(outcome) => Dispatch::Use(outcome),
                // An unacknowledged commit may already have ended the session.
                // Send no refusal; the terminal/loss lifecycle reconciles it.
                Err(_) => return ConnectionEnd::AdmittedThenControlLost(admitted),
            }
        } else if command.command_type == COMMAND_TYPE_USE_INTENT {
            match decode_use_intent_target(
                admitted.continuity.selected_capabilities.as_slice(),
                command.payload,
            ) {
                Ok(intent) if intent.use_with_map_item.is_some() => {
                    match super::item_craft::dispatch(authority, actor, UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    }, intent, item_view.as_ref()).await {
                        Some(outcome) => Dispatch::Use(outcome),
                        None => return ConnectionEnd::AdmittedThenDisconnected(admitted),
                    }
                }
                Ok(intent) if super::rune_use_bridge::routes(
                    &intent, &admitted.continuity.selected_capabilities,
                ) => match super::rune_use_bridge::dispatch(
                    authority, actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    intent, item_view.as_ref(), visibility.as_ref(),
                ).await {
                    Some(outcome) => Dispatch::Use(outcome),
                    None => return ConnectionEnd::AdmittedThenDisconnected(admitted),
                },
                // ITEM-USE-WIRE-1: fields 4 and 5 decode only under capability 15.
                // Creature, map-item and position targets still fail closed here.
                Ok(UseIntent {
                    use_with: Some(_), ..
                })
                // WORLDINT-WIRE-1: the map_item arm of field 4 decodes only under capability 11, which is not offered
                // before WORLDINT-USE-1 composes the interaction gate.
                | Ok(UseIntent {
                    use_with_map_item: Some(_),
                    ..
                })
                // RUNE-WIRE-1: position uses without a qualified bridge route fail closed.
                | Ok(UseIntent {
                    use_with_position: Some(_),
                    ..
                }) => Dispatch::Use(UseOutcome::rejected()),
                Ok(UseIntent {
                    target: UseTarget::ItemByDefinition(definition),
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: None,
                }) => Dispatch::Use(authority.use_item_by_definition(actor, UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                }, definition).await),
                // ITEM-VIEW-1b §4.3: a handle that is not live is STALE before any authority
                // call; opening writes nothing.
                Ok(UseIntent {
                    target: UseTarget::Item(handle),
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: None,
                }) => {
                    let key = item_view.as_ref().and_then(|view| view.resolve(handle));
                    // BAGS-WIRE-1: with capability 14 a container in reach opens in a new view,
                    // and one out of reach is TOO_FAR; anything else takes the corpse path.
                    let container = match (key, &container_views) {
                        (Some(key), Some(_)) => authority.observe_container(actor, admitted.item_fence, key).await,
                        _ => None,
                    };
                    let carried_entry = matches!(container, Some(ContainerObservation::NotAContainer));
                    let container = container.filter(|observation| {
                        matches!(
                            observation,
                            ContainerObservation::Container { .. } | ContainerObservation::TooFar
                        )
                    });
                    // BAGS-USE-1: a carried entry of an open view can be used under capability
                    // 15. The owner revalidates custody before PREPARE; equipped sources fail closed.
                    let consumable = container.is_none()
                        && matches!(key, Some(ItemKey::Instance(_)))
                        && admitted
                            .continuity
                            .selected_capabilities
                            .contains(CAPABILITY_ITEM_USE_V1)
                        && item_view
                            .as_ref()
                            .zip(key)
                            .is_some_and(|(view, key)| view.is_backpack_entry(&key)
                                || (carried_entry && !view.is_main_backpack(&key)
                                    && view.equipped_slot(&key).is_none()));
                    let observation = match key.filter(|_| container.is_none() && !consumable) {
                        Some(key) => authority.observe_item_target(actor, key).await,
                        None => None,
                    };
                    match (item_view.as_mut(), key, observation) {
                        (_, Some(key), _) if consumable => Dispatch::Use(
                            authority
                                .use_item(
                                    actor,
                                    UseCommand {
                                        game_session_id: admitted.game_session_id,
                                        command_id: command.command_id,
                                        item_fence: admitted.item_fence,
                                    },
                                    key,
                                )
                                .await,
                        ),
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
                    use_with_map_item: None,
                    use_with_position: None,
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
                Ok(intent) => match authority
                    .cast_spell_completion(
                        actor,
                        admitted.game_session_id,
                        command.command_id,
                        intent,
                    )
                    .await
                {
                    Some(outcome) => Dispatch::Spell(outcome),
                    // Preserve continuity's original next command and no terminal
                    // outcome; reconnect/reconciliation cannot pay or reroll twice.
                    None => return ConnectionEnd::AdmittedThenDisconnected(admitted),
                },
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
                            let observation = authority
                                .observe_container(actor, admitted.item_fence, pending.key())
                                .await;
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
        } else if command.command_type == oteryn_protocol_oteryn::depot::COMMAND_TYPE_DEPOT_INTENT {
            if let Ok(intent) = oteryn_protocol_oteryn::depot::decode_depot_intent(command.payload)
            {
                let item = match intent.action {
                    oteryn_protocol_oteryn::depot::DepotAction::Deposit(handle) => {
                        item_view.as_ref().and_then(|view| view.resolve(handle))
                    }
                    _ => None,
                };
                let Some(reply) = authority
                    .depot(
                        actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: command.command_id,
                            item_fence: admitted.item_fence,
                        },
                        intent,
                        item,
                    )
                    .await
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                Dispatch::Depot(reply)
            } else {
                Dispatch::Unregistered
            }
        } else if command.command_type == COMMAND_TYPE_ITEM_MOVE_INTENT {
            // ITEM-MOVE-1 (WIRE-0 §5): replay first, then an entry of the open corpse into the
            // main backpack. An unknown outcome ends the connection before the CommandId is
            // sequenced, so the resent command is answered by replay.
            match item_view.as_mut() {
                Some(view) => {
                    let selection = ItemMoveSelection {
                        equip_drop: selected_capabilities.contains(CAPABILITY_ITEM_EQUIP_DROP_V1),
                        container_tree: selected_capabilities
                            .contains(CAPABILITY_CONTAINER_TREE_V1),
                        world_interaction: selected_capabilities
                            .contains(CAPABILITY_WORLD_INTERACTION_V1),
                    };
                    let step = item_move::item_move(
                        authority,
                        actor,
                        UseCommand {
                            game_session_id: admitted.game_session_id,
                            command_id: command.command_id,
                            item_fence: admitted.item_fence,
                        },
                        command.payload,
                        selection,
                        view,
                    )
                    .await;
                    admitted.continuity.item_view = view.continuity();
                    match step {
                        ItemMoveStep::Result(outcome, mut deltas) => {
                            // ITEM-LOOP-1: the open container views show the committed tree.
                            if let (ItemMoveOutcome::Moved, Some(views)) =
                                (outcome, container_views.as_mut())
                            {
                                let refreshed = super::container_view::refresh::refresh(
                                    authority,
                                    actor,
                                    admitted.item_fence,
                                    views,
                                    view,
                                )
                                .await;
                                admitted.continuity.item_view = view.continuity();
                                let Ok(delta) = refreshed else {
                                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                                };
                                deltas.extend(delta);
                            }
                            Dispatch::ItemMove(outcome, deltas)
                        }
                        ItemMoveStep::Disconnect => {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        }
                    }
                }
                None => Dispatch::Unregistered,
            }
        } else if command.command_type == COMMAND_TYPE_ITEM_LOOK_INTENT {
            let selection = ItemLookSelection {
                extended: selected_capabilities.contains(CAPABILITY_ITEM_LOOK_V1),
            };
            if !admitted.continuity.look.admit(tokio::time::Instant::now()) {
                Dispatch::Look(
                    ItemLookResult::Refused(ItemLookRefusal::RateLimited),
                    selection,
                )
            } else {
                match decode_item_look_intent_for(command.payload, selection) {
                    Err(_) => Dispatch::Unregistered,
                    Ok(intent) => Dispatch::Look(
                        match intent.target {
                            ItemLookTarget::Creature(entity) => {
                                match known_creature_generation(entity) {
                                    Err(refusal) => ItemLookResult::Refused(refusal),
                                    Ok(_)
                                        if !visibility
                                            .as_ref()
                                            .is_some_and(|view| view.shows(entity)) =>
                                    {
                                        ItemLookResult::Refused(ItemLookRefusal::NotVisible)
                                    }
                                    Ok(_) => {
                                        authority
                                            .look_creature(actor, admitted.game_session_id, entity)
                                            .await
                                    }
                                }
                            }
                            ItemLookTarget::Item(handle) => match item_view.as_ref() {
                                None => ItemLookResult::Refused(ItemLookRefusal::NotAvailable),
                                Some(views) if views.resolve(handle).is_none() => {
                                    ItemLookResult::Refused(ItemLookRefusal::NotAvailable)
                                }
                                Some(views) => {
                                    authority
                                        .look_item(
                                            actor,
                                            admitted.game_session_id,
                                            admitted.item_fence,
                                            views,
                                            handle,
                                        )
                                        .await
                                }
                            },
                        },
                        selection,
                    ),
                }
            }
        } else if super::guild_intent::owns(command.command_type) {
            match authority
                .guild_command(
                    admitted,
                    command.command_id,
                    command.command_type,
                    command.payload,
                )
                .await
            {
                super::guild_intent::GuildReply::Payload(bytes) => Dispatch::Guild(bytes),
                super::guild_intent::GuildReply::Rejected => Dispatch::Unregistered,
                super::guild_intent::GuildReply::Unavailable => {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
            }
        } else if matches!(
            command.command_type,
            oteryn_protocol_oteryn::staff::COMMAND_TYPE_STAFF_REQUEST
                | oteryn_protocol_oteryn::staff::COMMAND_TYPE_STAFF_RECEIPT
        ) {
            match super::staff::dispatch(authority, admitted, command.command_type, command.payload)
                .await
            {
                super::StaffReply::Result(bytes) => Dispatch::Staff(Some(bytes)),
                super::StaffReply::Rejected => Dispatch::Staff(None),
                super::StaffReply::Unknown => {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
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
        } else if (48..=50).contains(&command.command_type) {
            Dispatch::Vip(
                super::vip::dispatch(
                    authority,
                    admitted,
                    command.command_id,
                    command.command_type,
                    command.payload,
                )
                .await,
            )
        } else if matches!(
            command.command_type,
            oteryn_protocol_oteryn::party::COMMAND_TYPE_PARTY_INTENT
                | oteryn_protocol_oteryn::party::COMMAND_TYPE_PARTY_QUERY
        ) {
            match authority
                .party_command(
                    admitted,
                    command.command_id,
                    command.command_type,
                    command.payload,
                    &mut party_view,
                )
                .await
            {
                Some(payload) => Dispatch::Party(payload),
                None => Dispatch::Unregistered,
            }
        } else if command.command_type == COMMAND_TYPE_CHAT_INTENT {
            // CHAT-WIRE-1: spam control is the speaker's limiter inside the authority; a
            // malformed intent is REJECTED with an empty payload like any unregistered command.
            match super::guild_chat::wire::dispatch(
                authority,
                admitted,
                actor,
                command.command_id,
                command.payload,
            )
            .await
            {
                Some(result) => Dispatch::Chat(result),
                None => Dispatch::Unregistered,
            }
        } else if command.command_type == COMMAND_TYPE_NPC_TALK_INTENT {
            // NPC-CMD-1a: a malformed intent is REJECTED, but it still spends the session's
            // NPC0-RL-04 allowance: it reaches the limiter as an intent with no text.
            let intent = decode_npc_talk_intent(command.payload).unwrap_or(NpcTalkIntent {
                npc_actor: WireEntityRef {
                    identity: Default::default(),
                    generation: 0,
                },
                text: String::new(),
            });
            let before_npc = authority.observe(actor).await;
            let outcome = authority
                .npc_talk(
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    intent,
                )
                .await;
            if outcome.unknown {
                return ConnectionEnd::AdmittedThenDisconnected(admitted);
            }
            npc_relocation = newhaven_travel_transport::retire_buffer(
                authority,
                actor,
                before_npc,
                &mut buffered,
            )
            .await;
            Dispatch::Npc(
                outcome.disposition,
                NpcUpdate {
                    conversation: outcome.conversation,
                    window: outcome.window,
                },
                outcome.inventory_changed,
            )
        } else if command.command_type == COMMAND_TYPE_NPC_TRADE_INTENT {
            // NPC-TRADE-2: a malformed intent is REJECTED. A committed trade answers OK and the
            // inventory delta (domain 9) follows when the session shows items; an unknown outcome
            // ends the connection before the CommandId is sequenced, so the resend replays.
            let step = match decode_npc_trade_intent(command.payload) {
                Ok(intent) => {
                    authority
                        .npc_trade(
                            actor,
                            UseCommand {
                                game_session_id: admitted.game_session_id,
                                command_id: command.command_id,
                                item_fence: admitted.item_fence,
                            },
                            intent,
                        )
                        .await
                }
                Err(_) => NpcTradeStep::Refused(super::npc_command::NpcTradeRefusal::only(
                    NpcIntentDisposition::Rejected,
                )),
            };
            match step {
                NpcTradeStep::Traded => {
                    let mut deltas = Vec::new();
                    if let Some(view) = item_view.as_mut() {
                        let Some(inventory) = item_move::observe_inventory(
                            authority,
                            actor,
                            admitted.game_session_id,
                            view,
                        )
                        .await
                        else {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        };
                        let Ok(delta) = view.inventory_committed(inventory) else {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        };
                        admitted.continuity.item_view = view.continuity();
                        deltas.extend(delta);
                    }
                    Dispatch::NpcTrade(NpcIntentDisposition::Ok, NpcUpdate::default(), deltas)
                }
                NpcTradeStep::Refused(refusal) => {
                    Dispatch::NpcTrade(refusal.disposition, refusal.update, Vec::new())
                }
                NpcTradeStep::Unknown => return ConnectionEnd::AdmittedThenDisconnected(admitted),
            }
        } else if command.command_type == COMMAND_TYPE_ATTACK_TARGET_INTENT {
            // ATTACK-1b: over ATTACK0-RL-01 (25 per second per GameSession, sliding window) it is
            // REJECTED with an empty payload before decoding, as is a malformed intent.
            let admitted_rate = admitted
                .continuity
                .combat
                .target_intents
                .admit(tokio::time::Instant::now());
            match decode_attack_target_intent(command.payload) {
                Ok(target) if admitted_rate => Dispatch::Attack(
                    authority
                        .attack_target(actor, admitted.game_session_id, command.command_id, target)
                        .await,
                ),
                _ => Dispatch::Unregistered,
            }
        } else if command.command_type == COMMAND_TYPE_FIGHT_MODES_INTENT {
            // ATTACK-1b: ATTACK0-RL-02, as command 11.
            let admitted_rate = admitted
                .continuity
                .combat
                .mode_intents
                .admit(tokio::time::Instant::now());
            match decode_fight_modes_intent(command.payload) {
                Ok(modes) if admitted_rate => Dispatch::Attack(
                    authority
                        .fight_modes(actor, admitted.game_session_id, modes)
                        .await,
                ),
                _ => Dispatch::Unregistered,
            }
        } else if command.command_type == COMMAND_TYPE_LOGOUT_INTENT {
            // LOGOUT-WIRE-1 (§1.6): a non-empty intent is REJECTED with an empty payload before
            // any authority call. While the logout runs, frames are read and held, in order, for
            // after its result (LOGOUT-RL-01): a second logout among them is a protocol error.
            if decode_logout_intent(command.payload).is_err() {
                Dispatch::Unregistered
            } else {
                // A paced step can already have held the frames that follow this logout; a
                // second logout among them was read before this result too (#2127 Codex P2
                // 4279788741).
                second_logout = held
                    .iter()
                    .any(|(frame, _)| is_logout_command(frame, generation));
                let mut logout = std::pin::pin!(authority.logout(admitted));
                let reply = loop {
                    let reading = !lost_while_logging_out
                        && held.len() < MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP;
                    let mut read = std::pin::pin!(frames.next(stream));
                    let next = std::future::poll_fn(|context| {
                        if let std::task::Poll::Ready(reply) = logout.as_mut().poll(context) {
                            return std::task::Poll::Ready(Ok(reply));
                        }
                        if reading && let std::task::Poll::Ready(read) = read.as_mut().poll(context)
                        {
                            return std::task::Poll::Ready(Err(read));
                        }
                        std::task::Poll::Pending
                    })
                    .await;
                    match next {
                        Ok(reply) => break reply,
                        Err(Ok(frame)) => {
                            second_logout |= is_logout_command(&frame, generation);
                            held.push_back((frame, false));
                        }
                        Err(Err(_)) => lost_while_logging_out = true,
                    }
                };
                match reply {
                    LogoutReply::Result(result) => Dispatch::Logout(result),
                    // §1.6 unknown outcome: no result; the loss lifecycle reconciles the row.
                    LogoutReply::Unknown => {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
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
        } else if command.command_type
            == oteryn_protocol_oteryn::appearance::COMMAND_TYPE_APPEARANCE_CHANGE
        {
            Dispatch::Appearance(
                appearance_change::dispatch(
                    authority,
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    command.payload,
                )
                .await,
            )
        } else if matches!(
            command.command_type,
            oteryn_protocol_oteryn::house::COMMAND_TYPE_HOUSE_INTENT
                | oteryn_protocol_oteryn::house::COMMAND_TYPE_HOUSE_QUERY
        ) {
            Dispatch::House(
                super::house_intent::dispatch(
                    authority,
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    command.payload,
                    command.command_type == oteryn_protocol_oteryn::house::COMMAND_TYPE_HOUSE_QUERY,
                )
                .await,
            )
        } else if matches!(
            command.command_type,
            oteryn_protocol_oteryn::daily_reward::COMMAND_TYPE_DAILY_REWARD_CLAIM
                | oteryn_protocol_oteryn::daily_reward::COMMAND_TYPE_DAILY_REWARD_QUERY
        ) {
            Dispatch::DailyReward(
                super::daily_reward::dispatch(
                    authority,
                    actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: command.command_id,
                        item_fence: admitted.item_fence,
                    },
                    command.command_type,
                    command.payload,
                )
                .await,
            )
        } else if command.command_type == COMMAND_TYPE_HIGHSCORES_QUERY_V1 {
            // HIGHSCORES0 §6: the World and the asking Character are the admitted session's.
            match decode_highscores_query(command.payload) {
                Ok(query) => Dispatch::Highscores(
                    authority
                        .highscores(admitted.world_id, admitted.game_session_id, query)
                        .await,
                ),
                Err(_) => Dispatch::Highscores(HighscoresReply::Rejected),
            }
        } else {
            Dispatch::Unregistered
        };
        // Native deposit/withdraw custody changes use the same inventory refresh as a trade.
        // Typed refusals and balance-only transfers do not manufacture an inventory mutation.
        dispatch = match dispatch {
            Dispatch::Npc(disposition, update, true) => {
                let mut deltas = Vec::new();
                if let Some(view) = item_view.as_mut() {
                    let Some(inventory) = item_move::observe_inventory(
                        authority,
                        actor,
                        admitted.game_session_id,
                        view,
                    )
                    .await
                    else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    let Ok(delta) = view.inventory_committed(inventory) else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    admitted.continuity.item_view = view.continuity();
                    deltas.extend(delta);
                }
                Dispatch::NpcTrade(disposition, update, deltas)
            }
            other => other,
        };
        drop(input_permit);
        if matches!(dispatch, Dispatch::Appearance(None) | Dispatch::House(None)) {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
        if matches!(dispatch, Dispatch::Vip(None)) {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
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
            Dispatch::Vip(Some(payload)) => (CommandStatus::Accepted, payload.clone()),
            Dispatch::Vip(None) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
            Dispatch::House(Some(payload)) => (CommandStatus::Accepted, payload.clone()),
            Dispatch::House(None) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
            Dispatch::DailyReward(payload) => {
                super::daily_reward::result(command.command_type, payload.take())
            }
            Dispatch::Appearance(Some(result)) => match appearance_change::result(*result) {
                Some(result) => result,
                None => return ConnectionEnd::AdmittedThenDisconnected(admitted),
            },
            Dispatch::Appearance(None) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
            Dispatch::Step(outcome, step_wait_ms) => (
                if matches!(
                    outcome.disposition,
                    StepDisposition::Rejected | StepDisposition::TooEarly
                ) {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                encode_step_outcome(
                    selected_capabilities.as_slice(),
                    outcome.disposition,
                    if step_wait_receipts { *step_wait_ms } else { 0 },
                ),
            ),
            Dispatch::Use(outcome) => {
                let (disposition, gate_message_id) = negotiated_use_disposition(
                    UseCapabilities {
                        item_use: selected_capabilities.contains(CAPABILITY_ITEM_USE_V1),
                        world_interaction: selected_capabilities
                            .contains(CAPABILITY_WORLD_INTERACTION_V1),
                        rune_use: selected_capabilities.contains(CAPABILITY_RUNE_USE_V1),
                    },
                    outcome.disposition,
                    outcome.gate_message_id,
                );
                (
                    if disposition == UseDisposition::Rejected {
                        CommandStatus::Rejected
                    } else {
                        CommandStatus::Accepted
                    },
                    encode_use_result_with_gate(disposition, gate_message_id).unwrap_or_else(
                        |_| encode_use_result(UseDisposition::Rejected).unwrap_or_default(),
                    ),
                )
            }
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
                })
                .unwrap_or_default(),
            ),
            Dispatch::ContainerView(outcome, _) => (
                CommandStatus::Accepted,
                encode_container_view_result(*outcome),
            ),
            Dispatch::Depot(reply) => {
                let Ok(payload) = oteryn_protocol_oteryn::depot::encode_depot_reply(reply) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                (CommandStatus::Accepted, payload)
            }
            Dispatch::ItemMove(outcome, _) => {
                let Ok(payload) = encode_item_move_result_for(
                    *outcome,
                    ItemMoveSelection {
                        equip_drop: selected_capabilities.contains(CAPABILITY_ITEM_EQUIP_DROP_V1),
                        world_interaction: selected_capabilities
                            .contains(CAPABILITY_WORLD_INTERACTION_V1),
                        ..ItemMoveSelection::default()
                    },
                ) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                (CommandStatus::Accepted, payload)
            }
            Dispatch::Look(result, selection) => {
                let Ok(payload) = encode_item_look_result_for(result, *selection) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                (CommandStatus::Accepted, payload)
            }
            Dispatch::Party(payload) => (CommandStatus::Accepted, payload.clone()),
            Dispatch::QuestLog(_) => (CommandStatus::Accepted, Vec::new()),
            Dispatch::Staff(result) => match result {
                Some(bytes) => (CommandStatus::Accepted, bytes.clone()),
                None => (CommandStatus::Rejected, Vec::new()),
            },
            Dispatch::Chat(result) => {
                let Ok(payload) = encode_chat_intent_result(result) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                (
                    if result.disposition == oteryn_protocol_oteryn::chat::ChatDisposition::Rejected
                    {
                        CommandStatus::Rejected
                    } else {
                        CommandStatus::Accepted
                    },
                    payload,
                )
            }
            Dispatch::Npc(disposition, _, _) | Dispatch::NpcTrade(disposition, _, _) => (
                if *disposition == NpcIntentDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                encode_npc_intent_result(*disposition),
            ),
            Dispatch::Attack(disposition) => (
                if *disposition == AttackIntentDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                encode_attack_intent_result(*disposition),
            ),
            Dispatch::Logout(result) => {
                let Ok(payload) = encode_logout_result(*result) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                (
                    if result.is_rejected() {
                        CommandStatus::Rejected
                    } else {
                        CommandStatus::Accepted
                    },
                    payload,
                )
            }
            Dispatch::Achievements(AccountAchievementsReply::Page(payload)) => {
                // The page is written once; move it out instead of copying up to 32 KiB.
                (CommandStatus::Accepted, std::mem::take(payload))
            }
            Dispatch::Guild(payload) | Dispatch::Highscores(HighscoresReply::Result(payload)) => {
                (CommandStatus::Accepted, std::mem::take(payload))
            }
            Dispatch::Unregistered
                if command.command_type
                    == oteryn_protocol_oteryn::appearance::COMMAND_TYPE_APPEARANCE_CHANGE
                    && selected_capabilities
                        .contains(oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1) =>
            {
                match appearance_change::result(
                    oteryn_protocol_oteryn::appearance::ChangeResult::Rejected,
                ) {
                    Some(result) => result,
                    None => return ConnectionEnd::AdmittedThenDisconnected(admitted),
                }
            }
            Dispatch::Achievements(
                AccountAchievementsReply::Terminal(_) | AccountAchievementsReply::Rejected,
            )
            | Dispatch::Highscores(HighscoresReply::Terminal(_) | HighscoresReply::Rejected)
            | Dispatch::Unregistered => (CommandStatus::Rejected, Vec::new()),
        };
        let result = if let Dispatch::Achievements(AccountAchievementsReply::Terminal(error))
        | Dispatch::Highscores(HighscoresReply::Terminal(error)) = &dispatch
        {
            // Display contract §3.3, §4.3 and HIGHSCORES0 §6: fail closed with the registered
            // operation-terminal error and nothing else.
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
        if let Dispatch::Logout(LogoutResult::Accepted) = dispatch {
            // §1.6: the release committed before this result. No command is read any more and
            // the transport closes cleanly; a failed write is no control loss either. A second
            // logout read before this result is still the `LOGOUT-RL-01` protocol error, but the
            // committed release keeps this a logout end.
            if write_frame(stream, &result).await.is_ok()
                && (!second_logout
                    || send_error(
                        stream,
                        FoundationProtocolError::TooManyOutstandingCommands,
                        admitted.continuity.connection_generation,
                    )
                    .await
                    .is_ok())
            {
                let _ = tokio::time::timeout(
                    policy.interval,
                    tokio::io::AsyncWriteExt::shutdown(stream),
                )
                .await;
            }
            return ConnectionEnd::AdmittedThenLoggedOut(admitted);
        }
        if write_frame(stream, &result).await.is_err() {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
        if matches!(dispatch, Dispatch::Logout(_)) {
            if second_logout {
                return close_admitted(
                    stream,
                    admitted,
                    FoundationProtocolError::TooManyOutstandingCommands,
                )
                .await;
            }
            if lost_while_logging_out {
                return ConnectionEnd::AdmittedThenDisconnected(admitted);
            }
        }
        match dispatch {
            Dispatch::Step(outcome, _) => {
                if let Some(observation) = outcome.moved_to
                    && moved_step_frames(
                        stream,
                        authority,
                        actor,
                        &mut admitted,
                        visibility.as_mut(),
                        item_view.as_mut(),
                        world_map.as_mut(),
                        generation,
                        &mut sequence,
                        &mut revision,
                        &mut appearances,
                        observation,
                    )
                    .await
                    .is_none()
                {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                if tutorial_notices_transport::emit(stream, authority, &mut admitted, &mut sequence)
                    .await
                    .is_none()
                {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
            }
            Dispatch::Use(outcome) => {
                if command.command_type == COMMAND_TYPE_BED_SLEEP_INTENT
                    && outcome.disposition == UseDisposition::OfflineTraining
                {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                // ITEM-VIEW-1b: a committed `USE` (the chest MINT) may have changed the
                // backpack or open bags; their deltas follow the durable commit when changed.
                // An unreadable backpack fails closed; the resumed snapshot restores it.
                if outcome.disposition == UseDisposition::Committed
                    && let Some(view) = item_view.as_mut()
                    && item_use_view_refresh::refresh(
                        stream,
                        authority,
                        actor,
                        &mut admitted,
                        view,
                        container_views.as_mut(),
                        &mut sequence,
                    )
                    .await
                    .is_none()
                {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                // ITEM-USE-1c: a committed food or potion changed the owner's vitals revision
                // (fed time or health/mana); its ACTOR_VITALS delta follows at once, as a spell's.
                if outcome.disposition == UseDisposition::Committed
                    && admitted.continuity.vitals_revision != 0
                    && let Some((to, value)) = authority
                        .observe_vitals(actor, admitted.game_session_id)
                        .await
                        .filter(|(revision, _)| *revision > admitted.continuity.vitals_revision)
                {
                    let Some((delta_sequence, delta)) = vitals_delta(
                        generation,
                        sequence,
                        admitted.continuity.vitals_revision,
                        to,
                        &value,
                    ) else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    sequence = delta_sequence;
                    admitted.continuity.server_sequence = sequence;
                    if write_frame(stream, &delta).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                    admitted.continuity.vitals_revision = to;
                }
                // WORLD_OBJECT_OVERLAY (domain 2, delta type 1) is Channel-global, not
                // per-connection: its "from" revision is always the committed entry's own
                // revision minus one, since `LocalObjectRuntime::attempt_use` only ever commits
                // by advancing its revision by exactly one.
                if let Some(entry) = outcome.committed {
                    // MAP-WIRE-1 contract §3: a bundle World's object states are tile changes
                    // of domain 17 and it commits no domain 2 entry. One committed for a session
                    // that selected capability 18 has no domain 17 form: fail closed, never send
                    // domain 2 or drop the state.
                    if !overlay_served {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
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
                // revision: a committed cast advances it by exactly one. Revisions start at 1, so
                // 0 means the join carried no ACTOR_VITALS snapshot: there is no base to delta
                // from, and the session keeps its spell result without the domain.
                let observed = authority
                    .observe_vitals(actor, admitted.game_session_id)
                    .await;
                if admitted.continuity.vitals_revision != 0
                    && let Some((to, value)) = observed
                        .filter(|(revision, _)| *revision > admitted.continuity.vitals_revision)
                        .or(outcome.vitals.filter(|(revision, _)| {
                            *revision > admitted.continuity.vitals_revision
                        }))
                {
                    let Some((delta_sequence, delta)) = vitals_delta(
                        generation,
                        sequence,
                        admitted.continuity.vitals_revision,
                        to,
                        &value,
                    ) else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    sequence = delta_sequence;
                    admitted.continuity.server_sequence = sequence;
                    if write_frame(stream, &delta).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                    admitted.continuity.vitals_revision = to;
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
            Dispatch::Depot(reply) => {
                if reply.outcome == oteryn_protocol_oteryn::depot::DepotOutcome::Moved {
                    let Some(view) = item_view.as_mut() else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    if item_use_view_refresh::refresh(
                        stream,
                        authority,
                        actor,
                        &mut admitted,
                        view,
                        container_views.as_mut(),
                        &mut sequence,
                    )
                    .await
                    .is_none()
                    {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
            }
            Dispatch::ItemMove(outcome, deltas) => {
                for delta in deltas {
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
                // ITEM-LOOP-1: a drop or pickup changed a tile in view; domain 17 shows it now.
                if matches!(outcome, ItemMoveOutcome::Moved)
                    && let (Some(view), Some(items)) = (world_map.as_mut(), item_view.as_mut())
                {
                    let update =
                        world_map_delta(authority, actor, generation, sequence, view, items).await;
                    admitted.continuity.item_view = items.continuity();
                    let Some(update) = update else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    if let Some((delta_sequence, frame)) = update {
                        sequence = delta_sequence;
                        admitted.continuity.server_sequence = sequence;
                        admitted.continuity.world_map_revision = view.revision();
                        if write_frame(stream, &frame).await.is_err() {
                            return ConnectionEnd::AdmittedThenDisconnected(admitted);
                        }
                    }
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
            // CHAT-WIRE-1: the lines reach the hearers' domain 12 on their own ticks. CHAT-2B: an
            // opened or closed room follows the result as one domain 12 delta type 2.
            Dispatch::Chat(_) => {
                if let Some(rooms) = authority.chat_rooms_changed(admitted.game_session_id).await {
                    let Some((delta_sequence, frame, revision)) = chat_delta(
                        generation,
                        sequence,
                        admitted.continuity.chat_revision,
                        DELTA_TYPE_CHAT_ROOMS_V1,
                        &encode_chat_rooms(rooms),
                    ) else {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    };
                    sequence = delta_sequence;
                    admitted.continuity.server_sequence = sequence;
                    admitted.continuity.chat_revision = revision;
                    if write_frame(stream, &frame).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
            }
            // NPC-CMD-1a: the whole domain 7 payload, then the domain 8 window (NPC-CMD-1b-read),
            // follow the result, to the speaker alone.
            Dispatch::NpcTrade(_, update, deltas) => {
                for delta in deltas {
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
                let Some(frames) = npc_update_frames(
                    generation,
                    &mut sequence,
                    &mut admitted.continuity.npc,
                    &update,
                    &mut npc_shown,
                ) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                admitted.continuity.server_sequence = sequence;
                for frame in frames {
                    if write_frame(stream, &frame).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
            }
            Dispatch::Npc(_, update, _) => {
                let Some(frames) = npc_update_frames(
                    generation,
                    &mut sequence,
                    &mut admitted.continuity.npc,
                    &update,
                    &mut npc_shown,
                ) else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                admitted.continuity.server_sequence = sequence;
                for frame in frames {
                    if write_frame(stream, &frame).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
            }
            Dispatch::Attack(_) => {
                // The changed target or modes reach domain 10 right after the result.
                let Some(state) = authority
                    .observe_combat_state(actor, admitted.game_session_id)
                    .await
                else {
                    continue;
                };
                let Some(change) = admitted
                    .continuity
                    .combat
                    .delta(generation, sequence, state)
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                if let Some((delta_sequence, frame)) = change {
                    sequence = delta_sequence;
                    admitted.continuity.server_sequence = sequence;
                    if write_frame(stream, &frame).await.is_err() {
                        return ConnectionEnd::AdmittedThenDisconnected(admitted);
                    }
                }
            }
            Dispatch::UseItem(_, None)
            | Dispatch::ContainerView(_, None)
            | Dispatch::Look(..)
            | Dispatch::Achievements(_)
            | Dispatch::Highscores(_)
            | Dispatch::Guild(_)
            | Dispatch::Vip(_)
            | Dispatch::Party(_)
            | Dispatch::Appearance(_)
            | Dispatch::DailyReward(_)
            | Dispatch::Logout(_)
            | Dispatch::House(_)
            | Dispatch::Staff(_)
            | Dispatch::Unregistered => {}
        }
        if newhaven_travel_transport::emit_relocation(
            stream,
            authority,
            actor,
            &mut admitted,
            visibility.as_mut(),
            item_view.as_mut(),
            world_map.as_mut(),
            generation,
            &mut sequence,
            &mut revision,
            &mut appearances,
            npc_relocation,
        )
        .await
        .is_none()
        {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
    }
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
mod tests {
    #[path = "capability_tests.rs"]
    mod capability_tests;

    mod bed_sleep_dispatch_tests {
        include!("bed_sleep_dispatch_tests.rs");
    }

    use super::*;
    use crate::gameplay_transport::world_spatial::encode_step_result;
    use oteryn_protocol_oteryn::world_object::decode_use_result_with_gate;
    use std::cell::{Cell, RefCell};
    use std::error::Error;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

    fn caps(item_use: bool, world_interaction: bool) -> UseCapabilities {
        UseCapabilities {
            item_use,
            world_interaction,
            rune_use: false,
        }
    }

    #[test]
    fn use_dispositions_7_to_10_need_capability_15() {
        use UseDisposition as D;
        let neg = |i, w, d| negotiated_use_disposition(caps(i, w), d, None).0;
        for d in [D::RequirementNotMet, D::Exhausted, D::Full, D::NoTarget] {
            assert_eq!(neg(false, false, d), D::Rejected);
            assert_eq!(neg(true, false, d), d);
        }
        for d in [
            D::Locked,
            D::KeyMismatch,
            D::PzBlocked,
            D::NotMovable,
            D::NoRoom,
            D::NotPossible,
        ] {
            assert_eq!(neg(true, false, d), D::Rejected);
            assert_eq!(neg(true, true, d), d);
        }
        for d in [
            D::Committed,
            D::NothingToUse,
            D::Occupied,
            D::StaleState,
            D::TooFar,
        ] {
            assert_eq!(neg(false, false, d), d);
        }
    }

    /// RUNE-WIRE-1: TARGET_ILLEGAL and PROTECTION_ZONE reach only a session that selected 22;
    /// capability 11 alone does not admit them, and 22 alone does not admit 11 to 17.
    #[test]
    fn use_dispositions_18_and_19_need_capability_21() {
        use UseDisposition as D;
        let all = UseCapabilities {
            item_use: true,
            world_interaction: true,
            rune_use: true,
        };
        let runes_only = UseCapabilities {
            world_interaction: false,
            ..all
        };
        for d in [D::TargetIllegal, D::ProtectionZone] {
            assert_eq!(
                negotiated_use_disposition(caps(true, true), d, None).0,
                D::Rejected
            );
            assert_eq!(negotiated_use_disposition(all, d, None), (d, None));
            assert_eq!(negotiated_use_disposition(runes_only, d, None), (d, None));
            assert_eq!(
                negotiated_use_disposition(all, d, Some(9)),
                (D::Rejected, None)
            );
        }
        assert_eq!(
            negotiated_use_disposition(runes_only, D::Locked, None).0,
            D::Rejected
        );
        for d in [D::RequirementNotMet, D::Exhausted, D::NoTarget, D::TooFar] {
            assert_eq!(negotiated_use_disposition(runes_only, d, None).0, d);
        }
    }

    #[test]
    fn sealed_carries_its_gate_message_id_only_under_capability_11() {
        use UseDisposition as D;
        assert_eq!(
            negotiated_use_disposition(caps(true, true), D::Sealed, Some(9)),
            (D::Sealed, Some(9))
        );
        assert_eq!(
            negotiated_use_disposition(caps(true, false), D::Sealed, Some(9)),
            (D::Rejected, None)
        );
        for bad in [None, Some(0), Some(128)] {
            assert_eq!(
                negotiated_use_disposition(caps(true, true), D::Sealed, bad),
                (D::Rejected, None)
            );
        }
        assert_eq!(
            negotiated_use_disposition(caps(true, true), D::Locked, Some(9)),
            (D::Rejected, None)
        );
        let payload = encode_use_result_with_gate(D::Sealed, Some(9)).expect("sealed");
        assert_eq!(
            decode_use_result_with_gate(&payload).expect("decode"),
            (D::Sealed, Some(9))
        );
    }

    const CHARACTER: [u8; 16] = uuid_v7(0x11);
    const SESSION: [u8; 16] = uuid_v7(0x22);
    const WORLD: [u8; 16] = uuid_v7(0x33);
    const CHANNEL: [u8; 16] = uuid_v7(0x44);

    #[test]
    fn vitals_publication_coalesces_real_changes_from_last_transmitted_revision()
    -> Result<(), Box<dyn Error>> {
        let (sequence, wire) = vitals_delta(2, 11, 40, 43, &SERENE_VITALS).ok_or("delta")?;
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&wire)?;
        let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
        assert_eq!(
            (sequence, delta.base_revision, delta.new_revision),
            (12, 40, 43)
        );
        assert_eq!(
            delta.payload,
            encode_actor_vitals(&SERENE_VITALS).map_err(|_| "vitals")?
        );
        assert!(vitals_delta(2, 11, 0, 43, &SERENE_VITALS).is_none());
        assert!(vitals_delta(2, 11, 43, 43, &SERENE_VITALS).is_none());
        assert!(vitals_delta(2, 11, 44, 43, &SERENE_VITALS).is_none());
        Ok(())
    }

    struct PendingSpellAuthority;
    impl FreshAdmissionAuthority for PendingSpellAuthority {
        async fn admit(
            &self,
            _: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }
        async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }
        async fn observe_vitals(
            &self,
            _: ExactActorRef,
            _: GameSessionId,
        ) -> Option<(u64, ActorVitals)> {
            Some((1, SERENE_VITALS))
        }
        async fn cast_spell_completion(
            &self,
            _: ExactActorRef,
            _: GameSessionId,
            _: u64,
            _: SpellCastIntent,
        ) -> Option<SpellCastOutcome> {
            None
        }
    }

    #[test]
    fn unknown_spell_commit_preserves_original_command_and_emits_no_terminal_result()
    -> Result<(), Box<dyn Error>> {
        use super::super::actor_spell::tests as spell;
        run(async {
            let (_, actor, session) = spell::runtime_with_player(0x73);
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
            let intent = SpellCastIntent {
                spell: std::num::NonZeroU32::new(1).ok_or("index")?,
                target: oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
                aim_at_target: false,
            };
            let payload = super::super::actor_spell::encode_spell_cast_intent(&intent);
            let (mut server, mut client) = tokio::io::duplex(1 << 16);
            write_frame(&mut client, &cast_command(1, &payload)).await?;
            let end =
                serve_admitted(&mut server, admitted, &PendingSpellAuthority, IDLE_LIVENESS).await;
            let ConnectionEnd::AdmittedThenDisconnected(retained) = end else {
                return Err("unexpected end".into());
            };
            assert_eq!(retained.continuity.next_command_id, 1);
            assert_eq!(retained.continuity.server_sequence, 0);
            assert_eq!(retained.continuity.vitals_revision, 1);
            drop(server);
            let mut bytes = Vec::new();
            client.read_to_end(&mut bytes).await?;
            let mut remaining = bytes.as_slice();
            while !remaining.is_empty() {
                let prefix: [u8; 4] = remaining.get(..4).ok_or("prefix")?.try_into()?;
                let length = usize::try_from(u32::from_be_bytes(prefix))?;
                let wire = remaining.get(4..4 + length).ok_or("frame")?;
                let envelope = oteryn_protocol_oteryn::decode_wire_envelope(wire)?;
                assert_ne!(
                    envelope.message_type(),
                    oteryn_protocol_oteryn::MessageType::CommandResult
                );
                remaining = remaining.get(4 + length..).ok_or("tail")?;
            }
            Ok(())
        })
    }

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

    /// Historical result fixture only: it supplies no NPC/current-fence or money authority.
    /// Actual continuation admission/drain authority is qualified separately with native Root/PG.
    struct RetainedBankOutcomeFixture {
        commands: RefCell<Vec<UseCommand>>,
        replacement_steps: RefCell<usize>,
        unknown: bool,
        closed_gate: bool,
    }
    impl FreshAdmissionAuthority for RetainedBankOutcomeFixture {
        async fn enter_input(&self) -> Option<InputPermit<'_>> {
            if self.closed_gate {
                None
            } else {
                Some(InputPermit::ungated())
            }
        }
        async fn admit(
            &self,
            _: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }
        async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }
        async fn step(&self, _: ExactActorRef, _: StepDirection) -> StepOutcome {
            *self.replacement_steps.borrow_mut() += 1;
            StepOutcome::rejected()
        }
        async fn resume_npc_bank(
            &self,
            _: ExactActorRef,
            command: UseCommand,
        ) -> Option<NpcTalkOutcome> {
            self.commands.borrow_mut().push(command);
            Some(NpcTalkOutcome {
                unknown: self.unknown,
                inventory_changed: false,
                disposition: NpcIntentDisposition::Ok,
                conversation: None,
                window: None,
            })
        }
    }

    #[test]
    fn original_bank_command_precedes_replacement_text_npc_and_command_type()
    -> Result<(), Box<dyn Error>> {
        run(async {
            // All replacements keep the original sequenced CommandId. Neither decoding a new
            // talk nor a different/unselected command may orphan its accepted original work.
            for (kind, payload) in [
                (COMMAND_TYPE_NPC_TALK_INTENT, b"no".as_slice()),
                (
                    COMMAND_TYPE_NPC_TALK_INTENT,
                    b"different NPC bytes".as_slice(),
                ),
                (
                    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                    b"replacement step".as_slice(),
                ),
                (999, b"".as_slice()),
            ] {
                let authority = RetainedBankOutcomeFixture {
                    commands: RefCell::new(Vec::new()),
                    replacement_steps: RefCell::new(0),
                    unknown: false,
                    closed_gate: false,
                };
                let mut command = Vec::new();
                scalar(&mut command, 1, 1);
                scalar(&mut command, 2, u64::from(kind));
                bytes(&mut command, 4, payload);
                let (end, frames) = drive_session(
                    &authority,
                    resumed_positioned()?,
                    &[envelope(7, RESUMED_GENERATION, &command)],
                )
                .await?;
                assert_eq!(*authority.replacement_steps.borrow(), 0);
                assert_eq!(authority.commands.borrow().len(), 1);
                assert_eq!(authority.commands.borrow()[0].command_id, 1);
                assert_eq!(
                    authority.commands.borrow()[0].game_session_id,
                    GameSessionId::decode(&SESSION)?
                );
                let mut expected = baseline_at(RESUMED_GENERATION);
                expected.push(encode_command_result(
                    RESUMED_GENERATION,
                    1,
                    1,
                    CommandStatus::Accepted,
                    &encode_npc_intent_result(NpcIntentDisposition::Ok),
                )?);
                assert_eq!(frames, expected);
                assert!(
                    matches!(end,ConnectionEnd::AdmittedThenDisconnected(admitted) if admitted.continuity.next_command_id==2)
                );
            }
            Ok(())
        })
    }

    #[test]
    fn unresolved_original_bank_work_never_sequences_a_replacement_command()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = RetainedBankOutcomeFixture {
                commands: RefCell::new(Vec::new()),
                replacement_steps: RefCell::new(0),
                unknown: true,
                closed_gate: false,
            };
            let (end, frames) = drive_session(
                &authority,
                resumed_positioned()?,
                &[command(
                    RESUMED_GENERATION,
                    1,
                    u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
                    StepDirection::East,
                )],
            )
            .await?;
            assert_eq!(frames, baseline_at(RESUMED_GENERATION));
            assert_eq!(*authority.replacement_steps.borrow(), 0);
            assert!(
                matches!(end,ConnectionEnd::AdmittedThenDisconnected(admitted) if admitted.continuity.next_command_id==1)
            );
            Ok(())
        })
    }

    #[test]
    fn closed_input_gate_holds_unclassified_original_bank_command() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = RetainedBankOutcomeFixture {
                commands: RefCell::new(Vec::new()),
                replacement_steps: RefCell::new(0),
                unknown: false,
                closed_gate: true,
            };
            let (end, frames) = drive_session(
                &authority,
                resumed_positioned()?,
                &[command(
                    RESUMED_GENERATION,
                    1,
                    u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
                    StepDirection::East,
                )],
            )
            .await?;
            assert_eq!(frames, baseline_at(RESUMED_GENERATION));
            assert!(authority.commands.borrow().is_empty());
            assert_eq!(*authority.replacement_steps.borrow(), 0);
            assert!(
                matches!(end,ConnectionEnd::AdmittedThenDisconnected(admitted) if admitted.continuity.next_command_id==1)
            );
            Ok(())
        })
    }

    /// NPC-ECON-1: a fresh admission is the GameSession's first connection, so no original Bank
    /// command can be retained. Its first command dispatches without a Bank journal lookup, even
    /// when that lookup would be unknown (Bank storage unavailable), and a closed input gate
    /// refuses it as usual instead of holding it.
    #[test]
    fn fresh_session_first_command_skips_retained_bank_lookup() -> Result<(), Box<dyn Error>> {
        run(async {
            for closed_gate in [false, true] {
                let authority = RetainedBankOutcomeFixture {
                    commands: RefCell::new(Vec::new()),
                    replacement_steps: RefCell::new(0),
                    unknown: true,
                    closed_gate,
                };
                let (end, frames) = drive_session(
                    &authority,
                    positioned()?,
                    &[command(
                        ADMITTED_GENERATION,
                        1,
                        u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
                        StepDirection::East,
                    )],
                )
                .await?;
                assert!(authority.commands.borrow().is_empty());
                assert_eq!(
                    *authority.replacement_steps.borrow(),
                    usize::from(!closed_gate)
                );
                assert_eq!(frames.len(), baseline().len() + 1);
                assert_eq!(frames[..baseline().len()], baseline()[..]);
                assert!(
                    matches!(end,ConnectionEnd::AdmittedThenDisconnected(admitted) if admitted.continuity.next_command_id==2)
                );
            }
            Ok(())
        })
    }

    include!("step_dispatch_fixture.rs");
    include!("tutorial_session_tests.rs");
    include!("newhaven_travel_transport_tests.rs");
    include!("spectate_session_ingress_tests.rs");

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
        uses: RefCell<Vec<oteryn_protocol_oteryn::world_object::WorldObjectTarget>>,
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
            target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
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
        let intent = oteryn_protocol_oteryn::world_object::encode_use_intent(
            &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
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

    // V1 encodes the empty entry list as an empty payload, while retaining
    // the registered domain, snapshot type and initial revision in the envelope.
    fn empty_overlay_snapshot() -> DomainSnapshot<'static> {
        DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: 0,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &[],
        }
    }

    fn baseline() -> Vec<Vec<u8>> {
        baseline_at(ADMITTED_GENERATION)
    }

    fn baseline_at(generation: u64) -> Vec<Vec<u8>> {
        encode_single_chunk_snapshot(
            generation,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &encode_world_spatial(&at(0)),
                },
                empty_overlay_snapshot(),
            ],
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

    /// The first successor generation of a same-session resume (FND-04B §16).
    const RESUMED_GENERATION: u64 = ADMITTED_GENERATION + 1;

    /// `positioned` resumed on the next connection generation before any result of its first
    /// CommandId was delivered: the continuity an accepted original Bank command is retained in.
    fn resumed_positioned() -> Result<AdmittedSession, Box<dyn Error>> {
        let mut resumed = positioned()?;
        resumed.continuity.connection_generation = RESUMED_GENERATION;
        Ok(resumed)
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
                tutorial_notices: Default::default(),
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

    fn refused() -> Result<AdmittedSession, Box<dyn Error>> {
        Ok(AdmittedSession {
            first_entry: FirstEntryOutcome::RefusedUnavailable,
            ..positioned()?
        })
    }

    #[test]
    fn a_silent_refused_session_with_an_actor_closes_after_the_bound() -> Result<(), Box<dyn Error>>
    {
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
                tutorial_notices: Default::default(),
            };
            let (mut server, mut client) = tokio::io::duplex(1 << 16);
            // The client keeps its transport open but never writes.
            let end = serve_admitted(&mut server, refused()?, &authority, FAST).await;
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            drop(server);
            let mut output = Vec::new();
            client.read_to_end(&mut output).await?;
            assert!(output.is_empty());
            assert!(authority.steps.borrow().is_empty());
            Ok(())
        })
    }

    #[test]
    fn a_frame_before_the_bound_closes_a_refused_session_with_an_error()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
                tutorial_notices: Default::default(),
            };
            let (mut server, mut client) = tokio::io::duplex(1 << 16);
            client.write_all(&framed(&ack(1, 1))).await?;
            let end = serve_admitted(&mut server, refused()?, &authority, FAST).await;
            assert!(matches!(end, ConnectionEnd::AdmittedThenClosed(_, _)));
            Ok(())
        })
    }

    #[test]
    fn a_refused_session_without_an_actor_is_not_bounded() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
                tutorial_notices: Default::default(),
            };
            let admitted = AdmittedSession {
                runtime_actor: None,
                ..refused()?
            };
            let (mut server, _client) = tokio::io::duplex(1 << 16);
            let held = tokio::time::timeout(
                FAST.interval.saturating_mul(FAST.missed_limit * 5),
                serve_admitted(&mut server, admitted, &authority, FAST),
            )
            .await;
            assert!(held.is_err());
            Ok(())
        })
    }

    #[test]
    fn answering_client_keeps_control_and_steps_between_probes() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
                tutorial_notices: Default::default(),
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
                tutorial_notices: Default::default(),
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
                tutorial_notices: Default::default(),
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
        let registry: serde_json::Value =
            serde_json::from_str(oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON)?;
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
                tutorial_notices: Default::default(),
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
                    vitals_revision: 0,
                    achievement_notice_revision: None,
                    selected_capabilities: SelectedCapabilities::NONE,
                    item_view: ItemViewContinuity::default(),
                    quest_log: QuestLogContinuity::FRESH,
                    look: LookWindow::EMPTY,
                    combat: CombatContinuity::FRESH,
                    world_map_revision: 0,
                    chat_revision: 0,
                    npc: NpcContinuity::default(),
                    world_clock_revision: 0,
                    world_weather_revision: 0,
                    tutorial_revision: 0,
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
        /// STEP-PACE-WIRE-1: the rollout switch this fixture reports.
        receipts: bool,
    }

    /// On the 50 ms beat, so a capability 13 session gets it as its `step_wait_ms` receipt.
    const PACE: std::time::Duration = std::time::Duration::from_millis(100);

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

        fn step_wait_receipts(&self) -> bool {
            self.receipts
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
            // is a REJECTED command and nothing moves. STEP-PACE-WIRE-1: only with the rollout
            // switch on AND capability 13 does a MOVED carry the 100 ms step duration (field 2 =
            // 0x64) and the TOO_EARLY the time left until the clock is due (at most PACE,
            // rounded up, well above zero); every other session gets today's exact bytes.
            let receipt = |wait: u8| vec![0x08, 0x04, 0x10, wait];
            let left: Vec<Vec<u8>> = (50..=100).map(receipt).collect();
            for (capabilities, receipts, moved, refusals) in [
                (&[13][..], true, vec![0x08, 0x01, 0x10, 0x64], left),
                (&[13][..], false, vec![0x08, 0x01], vec![vec![0x08, 0x04]]),
                (&[][..], true, vec![0x08, 0x01], vec![vec![0x08, 0x03]]),
                (&[][..], false, vec![0x08, 0x01], vec![vec![0x08, 0x03]]),
            ] {
                let authority = PacedAuthority {
                    steps: RefCell::new(Vec::new()),
                    turn: std::time::Duration::ZERO,
                    receipts,
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
                ]);
                let (refusal, frames) = frames.split_last().ok_or("frames")?;
                assert_eq!(frames, expected, "{capabilities:?} {receipts}");
                let mut matched = false;
                for payload in refusals {
                    matched |= *refusal
                        == encode_command_result(1, 5, 3, CommandStatus::Rejected, &payload)?;
                }
                assert!(matched, "{capabilities:?} {receipts} {refusal:?}");
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
                receipts: true,
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
                receipts: true,
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
                receipts: true,
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
                    &[0x08, 0x01, 0x10, 0x64]
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
                    tutorial_notices: Default::default(),
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

    /// N8: a classified refusal writes exactly one `ProtocolError` (generation 0,
    /// `TRANSPORT_FATAL`, the code, nothing else) and no acceptance.
    #[test]
    fn classified_admission_refusal_writes_one_protocol_error() -> Result<(), Box<dyn Error>> {
        run(async {
            use FoundationProtocolError as E;
            for error in [
                E::AdmissionGrantMalformed,
                E::AdmissionGrantAuthenticationFailed,
                E::AdmissionGrantBindingMismatch,
                E::AdmissionGrantNotYetValid,
                E::AdmissionGrantExpired,
                E::AdmissionGrantReplayed,
                E::AdmissionAttemptReconciliationRequired,
                E::AdmissionGrantSecurityStateRevoked,
                E::AdmissionGrantSecurityEvidenceStale,
                E::AdmissionGrantRouteStale,
                E::AdmissionGrantRuntimeGenerationStale,
                E::AdmissionGrantWorldStale,
                E::AdmissionGrantRevisionUnsupported,
                E::AdmissionAccountCharacterConflict,
                E::AdmissionIncumbentProtected,
                E::AdmissionCapacityExceeded,
                E::AdmissionCapabilityRequired,
            ] {
                let code = error.code();
                let refusal = AdmissionRefusal::Classified(error);
                let authority = Authority::new(Err(refusal));
                let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
                assert_eq!(end, ConnectionEnd::AdmissionRefused(refusal));
                assert_eq!(authority.calls.get(), 1);
                assert_eq!(frames, vec![error_frame(error, 0)]);
                let envelope = decode_wire_envelope(&frames[0])?;
                assert_eq!(envelope.message_type(), MessageType::ProtocolError);
                assert_eq!(envelope.connection_generation(), 0);
                let view = oteryn_protocol_oteryn::decode_protocol_error(envelope.payload())?;
                assert_eq!(view.error_code, code);
                assert_eq!(
                    view.disposition,
                    crate::foundation::ProtocolDisposition::TransportFatal
                );
                assert_eq!(
                    (
                        view.related_command_id,
                        view.expected_command_id,
                        view.expected_server_sequence
                    ),
                    (0, 0, 0)
                );
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
                    gate_message_id: None,
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
                    &encode_use_result(UseDisposition::Committed).unwrap_or_default(),
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
                oteryn_protocol_oteryn::world_object::WorldObjectTarget {
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
    #[test]
    fn admitted_empty_overlay_is_present_and_records_its_delivered_revision()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = UseAuthority {
                uses: RefCell::new(Vec::new()),
                commands: RefCell::new(Vec::new()),
                overlay: None,
                watermark: None,
                outcome: UseOutcome::rejected(),
            };
            let world_id = WorldId::decode(&WORLD)?;
            let channel_id = ChannelId::decode(&CHANNEL)?;
            let mut admitted = AdmittedSession {
                game_session_id: GameSessionId::decode(&SESSION)?,
                world_id,
                channel_id,
                runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
                first_entry: FirstEntryOutcome::Positioned,
                controller: None,
                continuity: SessionContinuity::FRESH,
                item_fence: None,
            };
            let (end, frames) = drive_session(&authority, admitted, &[]).await?;
            let chunk = oteryn_protocol_oteryn::decode_wire_envelope(
                frames.get(1).ok_or("missing snapshot chunk")?,
            )?;
            let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(chunk.payload())?;
            let overlay = domains
                .iter()
                .find(|domain| domain.domain_id == STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
                .ok_or("empty overlay domain was omitted")?;
            assert_eq!(overlay.revision, 0);
            assert_eq!(overlay.snapshot_type, SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1);
            assert!(
                oteryn_protocol_oteryn::world_object::decode_world_object_overlay_snapshot(
                    overlay.payload
                )
                .map_err(|error| format!("empty overlay decode: {error:?}"))?
                .is_empty()
            );
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err(format!("unexpected end {end:?}").into());
            };
            assert_eq!(ended.continuity.overlay_revision, 0);
            admitted.continuity.overlay_revision = 7;
            let (end, frames) = drive_session(&authority, admitted, &[]).await?;
            assert!(
                frames.is_empty(),
                "an unavailable previous overlay must not publish empty"
            );
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err(format!("unexpected end {end:?}").into());
            };
            assert_eq!(ended.continuity.overlay_revision, 7);
            Ok(())
        })
    }

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
                    gate_message_id: None,
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
                    gate_message_id: None,
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
                &encode_use_result(UseDisposition::Committed).unwrap_or_default(),
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
                    gate_message_id: None,
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
                &[
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                        payload: &encode_world_spatial(&at(0)),
                    },
                    empty_overlay_snapshot(),
                ],
            )?
            .into();
            expected.extend([
                encode_command_result(
                    1,
                    1,
                    1,
                    CommandStatus::Accepted,
                    &encode_use_result(UseDisposition::NothingToUse).unwrap_or_default(),
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
        /// The owner publishes no ACTOR_VITALS observation: the join carries no snapshot.
        hide_vitals: bool,
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
            if self.hide_vitals {
                return None;
            }
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
                    empty_overlay_snapshot(),
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
                hide_vitals: false,
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
                    empty_overlay_snapshot(),
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

    /// A session joined without an ACTOR_VITALS snapshot (`vitals_revision` 0) has no base to
    /// delta from: a committed cast answers its result and the session stays connected.
    #[test]
    fn spell_cast_without_a_joined_vitals_snapshot_answers_and_stays_connected()
    -> Result<(), Box<dyn Error>> {
        use super::super::actor_spell::{encode_spell_cast_intent, tests as spell};
        run(async {
            let (runtime, actor, session) = spell::runtime_with_player(0x72);
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
                hide_vitals: true,
            };
            let exura = encode_spell_cast_intent(&spell::exura());
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
                &[cast_command(1, &exura), cast_command(2, &exura)],
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
                    empty_overlay_snapshot(),
                ],
            )?
            .into();
            expected.push(encode_command_result(
                ADMITTED_GENERATION,
                1,
                1,
                CommandStatus::Accepted,
                &encode_spell_cast_result(SpellCastDisposition::Cast),
            )?);
            // The session survived the paid cast: the next command is still answered.
            expected.push(encode_command_result(
                ADMITTED_GENERATION,
                2,
                2,
                CommandStatus::Accepted,
                &encode_spell_cast_result(SpellCastDisposition::CoolingDown),
            )?);
            assert_eq!(frames, expected);
            // Only the peer's close ended the session; no vitals base was ever recorded.
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err("ended".into());
            };
            assert_eq!(ended.continuity.vitals_revision, 0);
            assert_eq!(authority.casts.get(), 2);
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
                hide_vitals: false,
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
                    empty_overlay_snapshot(),
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

    /// A fixture authority for `HIGHSCORES_QUERY_V1` dispatch: every call is recorded and
    /// answered with one canned reply.
    struct HighscoresAuthority {
        calls: RefCell<Vec<(WorldId, GameSessionId, HighscoresQuery)>>,
        reply: HighscoresReply,
    }

    impl FreshAdmissionAuthority for HighscoresAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }

        async fn highscores(
            &self,
            world: WorldId,
            session: GameSessionId,
            query: HighscoresQuery,
        ) -> HighscoresReply {
            self.calls.borrow_mut().push((world, session, query));
            self.reply.clone()
        }
    }

    fn highscores_command(id: u64, query: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, id);
        scalar(&mut payload, 2, u64::from(COMMAND_TYPE_HIGHSCORES_QUERY_V1));
        bytes(&mut payload, 4, query);
        envelope(7, ADMITTED_GENERATION, &payload)
    }

    /// HIGHSCORES0 §6: command type 14 dispatches only under capability 9, with the admitted
    /// session's World and GameSession; a malformed query is REJECTED without a read, and the
    /// snapshot outcomes are FND-02 error results 1200 and 1201 without a payload.
    #[test]
    fn highscores_query_reads_only_the_admitted_session_under_capability_9()
    -> Result<(), Box<dyn Error>> {
        let selected = |capabilities: &[u32]| -> Result<AdmittedSession, Box<dyn Error>> {
            let mut admitted = positioned()?;
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, capabilities)
                    .ok_or("bounded")?;
            Ok(admitted)
        };
        let capability = oteryn_protocol_oteryn::highscores::CAPABILITY_HIGHSCORES_V1;
        run(async {
            let admitted = selected(&[capability])?;
            let (world, session) = (admitted.world_id, admitted.game_session_id);
            let authority = HighscoresAuthority {
                calls: RefCell::new(Vec::new()),
                reply: HighscoresReply::Result(vec![0x08, 0x01]),
            };
            let (_end, frames) = drive_session(
                &authority,
                admitted,
                &[
                    // Category 1, all vocations, page 1, the current snapshot.
                    highscores_command(1, &[0x08, 0x01, 0x18, 0x01]),
                    // Page 0 does not decode.
                    highscores_command(2, &[0x08, 0x01]),
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
                    &[0x08, 0x01],
                )?,
                encode_command_result(ADMITTED_GENERATION, 2, 2, CommandStatus::Rejected, &[])?,
            ]);
            assert_eq!(frames, expected);
            let query = HighscoresQuery {
                category: 1,
                vocation: 0,
                page: 1,
                snapshot_id: 0,
            };
            assert_eq!(*authority.calls.borrow(), [(world, session, query)]);

            // Not selected: the command is not dispatched.
            let (_end, frames) = drive_session(
                &authority,
                selected(&[])?,
                &[highscores_command(1, &[0x08, 0x01, 0x18, 0x01])],
            )
            .await?;
            assert_ne!(
                frames.last(),
                Some(&encode_command_result(
                    ADMITTED_GENERATION,
                    1,
                    1,
                    CommandStatus::Accepted,
                    &[0x08, 0x01]
                )?)
            );
            assert_eq!(authority.calls.borrow().len(), 1);

            for error in [
                FoundationProtocolError::NotSupported,
                FoundationProtocolError::SnapshotExpired,
            ] {
                let failing = HighscoresAuthority {
                    calls: RefCell::new(Vec::new()),
                    reply: HighscoresReply::Terminal(error),
                };
                let (_end, frames) = drive_session(
                    &failing,
                    selected(&[capability])?,
                    &[highscores_command(1, &[0x08, 0x0c, 0x18, 0x01])],
                )
                .await?;
                let terminal = encode_command_error_result(ADMITTED_GENERATION, 1, 1, error)?;
                assert_eq!(frames.last(), Some(&terminal), "{error:?}");
                assert_eq!(failing.calls.borrow().len(), 1);
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
                gate_message_id: None,
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
                empty_overlay_snapshot(),
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
            &encode_use_result(UseDisposition::Committed).unwrap_or_default(),
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
    mod appearance_change_tests;
    use super::*;
    use crate::gameplay_transport::world_spatial::{
        ActorPosition, ChannelEntity, EntityDetail, EntityKind, EntityRef,
        PLACEHOLDER_ACTOR_DIRECTION, PLACEHOLDER_APPEARANCE_REF, PLACEHOLDER_HEALTH_PERCENT,
        WorldSpatialEntitiesDelta, WorldSpatialEntitiesSnapshot, encode_step_intent,
        encode_step_result, encode_visibility_delta, encode_visibility_snapshot,
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
                lights: EntityLights::new(),
                observed_at_ms: 0,
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
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision,
                    snapshot_type,
                    payload: &payload,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &[],
                },
            ],
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
                &[
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        revision: 1,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                        payload: &at_start,
                    },
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                        revision: 0,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                        payload: &[],
                    },
                ],
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
    fn others_moving_send_a_delta_and_a_change_above_256_a_resync_on_resume() -> TestResult {
        run(async {
            let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
            let channel = Channel::new(vec![creature([9; 16], at(102, 100), 0)]);
            let crowd: Vec<[u8; 16]> = (0..257_u16)
                .map(|n| {
                    let mut identity = [0x40; 16];
                    identity[14..].copy_from_slice(&n.to_be_bytes());
                    identity
                })
                .collect();
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
                // changes, so a resync: no mid-session snapshot (the admitted client accepts
                // none), the connection ends at the last delivered sequence and revision.
                *channel.others.borrow_mut() = crowd
                    .iter()
                    .map(|identity| creature(*identity, at(99, 100), 0))
                    .collect();
                Ok::<_, Box<dyn Error>>(())
            };
            let (end, observed) = {
                let (mut serving, mut observing) =
                    (std::pin::pin!(serving), std::pin::pin!(observing));
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
                (end, observed)
            };
            observed.ok_or("observed")??;
            let end = end.ok_or("ended")?;
            drop(server);
            assert!(
                read_one(&mut client).await.is_err(),
                "nothing after the last delta"
            );
            let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
                return Err(format!("unexpected end {end:?}").into());
            };
            assert_eq!(ended.continuity.spatial_revision, 3);
            assert_eq!(ended.continuity.server_sequence, 2);
            // The resume of that continuity is the resync: a snapshot of the own actor and the
            // 255 lowest identities (the degrade disposition) at sequence 2 and revision 3.
            let resumed = SessionContinuity {
                connection_generation: 2,
                ..ended.continuity
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
            let mut nearest = vec![own(at(100, 100))];
            nearest.extend(
                crowd[..255]
                    .iter()
                    .map(|identity| wire(EntityKind::Creature, *identity, at(99, 100))),
            );
            assert_eq!(frames, snapshot_frames(2, 1, 2, 3, &selected, nearest));
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
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

#[cfg(test)]
#[path = "item_look_dispatch_tests.rs"]
mod item_look_dispatch_tests;
