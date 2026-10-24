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
    let Some(mut commands) =
        super::session_ingress::SessionCommandStream::open(authority.session_commands(), &admitted)
    else {
        return ConnectionEnd::AdmittedThenDisconnected(admitted);
    };
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
        let buffered_original = matches!(&next, Next::StepDue);
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
        match commands.begin(command.command_id, generation, sequence, buffered_original) {
            Err((error, expected)) => {
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
            Ok(super::session_ingress::CommandAction::Replay(frame)) => {
                sequence += 1; // begin checked the successor before encoding the replay.
                admitted.continuity.server_sequence = sequence;
                if write_frame(stream, &frame).await.is_err() {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
                continue;
            }
            Ok(super::session_ingress::CommandAction::Pending) => continue,
            Ok(super::session_ingress::CommandAction::Execute) => {}
        }
        admitted.continuity.next_command_id = commands.next_command_id();
        // Unknown command types and malformed payloads have no effect. The result payload
        // belongs to the command type, so an unregistered type gets none. USE_INTENT (command
        // type 2, USE-WIRE-V1 #162 5868482467) is dispatched under the same FND-02 CommandId
        // session ingress above for every command. A reserved CommandId never becomes a
        // second operation, including across a same-GameSession connection replacement.
        // Spell retries likewise never cast or pay a second time (SPELL-D3).
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
            HouseMoveOut(Vec<u8>),
            Timed(Vec<u8>, Vec<ItemViewDelta>),
            Appearance(Option<oteryn_protocol_oteryn::appearance::ChangeResult>),
            DailyReward(Option<Vec<u8>>),
            WorldEvent(Result<Vec<u8>, ()>),
            Training(oteryn_protocol_oteryn::loop_training::TrainingResult),
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
        if input_permit.is_some()
            && !authority
                .training_activity(admitted, command.command_type)
                .await
        {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
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
        } else if matches!(command.command_type, 66 | 67) {
            let Some(result) = super::loop_training::wire::dispatch(
                authority,
                admitted,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                command.command_type,
                command.payload,
                item_view.as_ref(),
            )
            .await
            else {
                return ConnectionEnd::AdmittedThenControlLost(admitted);
            };
            Dispatch::Training(result)
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
                    match super::mining::dispatch_map_use(authority, actor, UseCommand {
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
                // Creature and unrouted position targets still fail closed here.
                Ok(UseIntent {
                    use_with: Some(_), ..
                })
                // WORLDINT-WIRE-1: map targets decode only under capability 11; the
                // mechanic owner above keeps unsupported tool actions closed.
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
                        (_, Some(key), _) if consumable => {
                            let Some(outcome) = tokio::time::timeout(
                                std::time::Duration::from_secs(10),
                                authority.use_item_completion(
                                    actor,
                                    UseCommand {
                                        game_session_id: admitted.game_session_id,
                                        command_id: command.command_id,
                                        item_fence: admitted.item_fence,
                                    },
                                    key,
                                ),
                            ).await.ok().flatten() else {
                                return ConnectionEnd::AdmittedThenDisconnected(admitted);
                            };
                            Dispatch::Use(outcome)
                        },
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
        } else if command.command_type
            == oteryn_protocol_oteryn::timed_item::COMMAND_TYPE_TIMED_ITEM_INTENT
        {
            let Some((payload, deltas)) = super::timed_item_wire::dispatch(
                authority,
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                command.payload,
                item_view.as_mut(),
            )
            .await
            else {
                return ConnectionEnd::AdmittedThenDisconnected(admitted);
            };
            if let Some(view) = item_view.as_ref() {
                admitted.continuity.item_view = view.continuity();
            }
            Dispatch::Timed(payload, deltas)
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
        } else if super::house_move_out::owns(command.command_type) {
            match authority
                .house_move_out_command(
                    admitted,
                    command.command_id,
                    command.command_type,
                    command.payload,
                )
                .await
            {
                super::house_move_out::HouseMoveOutReply::Payload(bytes) => {
                    Dispatch::HouseMoveOut(bytes)
                }
                super::house_move_out::HouseMoveOutReply::Rejected => Dispatch::Unregistered,
                super::house_move_out::HouseMoveOutReply::Unavailable => {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                }
            }
        } else if matches!(command.command_type, 72 | 73) {
            match authority
                .cyclopedia_command(
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
        } else if command.command_type
            == oteryn_protocol_oteryn::world_event::COMMAND_TYPE_WORLD_EVENT_QUERY
        {
            match super::world_event::dispatch(
                authority,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: command.command_id,
                    item_fence: admitted.item_fence,
                },
                command.payload,
            )
            .await
            {
                Some(reply) => Dispatch::WorldEvent(reply),
                None => return ConnectionEnd::AdmittedThenDisconnected(admitted),
            }
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
        let Some(result_sequence) = sequence.checked_add(1) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        sequence = result_sequence;
        admitted.continuity.server_sequence = sequence;
        let training_logged_out =
            matches!(&dispatch,Dispatch::Training(result) if result.disposition==3);
        let (status, result_payload) = match &mut dispatch {
            Dispatch::Vip(Some(payload)) => (CommandStatus::Accepted, payload.clone()),
            Dispatch::Vip(None) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
            Dispatch::House(Some(payload)) => (CommandStatus::Accepted, payload.clone()),
            Dispatch::House(None) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
            Dispatch::DailyReward(payload) => {
                super::daily_reward::result(command.command_type, payload.take())
            }
            Dispatch::Training(result) => {
                let Ok(payload) =
                    oteryn_protocol_oteryn::loop_training::encode_training_result(*result)
                else {
                    return ConnectionEnd::AdmittedThenDisconnected(admitted);
                };
                (
                    if result.disposition == 4 {
                        CommandStatus::Rejected
                    } else {
                        CommandStatus::Accepted
                    },
                    payload,
                )
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
            Dispatch::WorldEvent(Ok(payload)) => (CommandStatus::Accepted, std::mem::take(payload)),
            Dispatch::WorldEvent(Err(())) => (CommandStatus::Rejected, Vec::new()),
            Dispatch::Timed(payload, _) => (
                if super::timed_item_wire::accepted(payload) {
                    CommandStatus::Accepted
                } else {
                    CommandStatus::Rejected
                },
                std::mem::take(payload),
            ),
            Dispatch::HouseMoveOut(payload)
            | Dispatch::Guild(payload)
            | Dispatch::Highscores(HighscoresReply::Result(payload)) => {
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
        if commands.terminalize(command.command_id, &result).is_none() {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
        admitted.continuity.next_command_id = commands.next_command_id();
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
            Dispatch::Timed(_, deltas) => {
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
            Dispatch::Guild(_)
                if command.command_type == 73 && command.payload.first() == Some(&4) =>
            {
                if let Some(view) = item_view.as_mut()
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
            }
            Dispatch::UseItem(_, None)
            | Dispatch::ContainerView(_, None)
            | Dispatch::Look(..)
            | Dispatch::Achievements(_)
            | Dispatch::Highscores(_)
            | Dispatch::HouseMoveOut(_)
            | Dispatch::Guild(_)
            | Dispatch::Vip(_)
            | Dispatch::Party(_)
            | Dispatch::WorldEvent(_)
            | Dispatch::Appearance(_)
            | Dispatch::DailyReward(_)
            | Dispatch::Training(_)
            | Dispatch::Logout(_)
            | Dispatch::House(_)
            | Dispatch::Staff(_)
            | Dispatch::Unregistered => {}
        }
        if training_logged_out {
            return ConnectionEnd::AdmittedThenLoggedOut(admitted);
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
mod tests;

/// VIS-3 (MOVE-RL-11 §4): domain 1 composed from the interest set on the connection.
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod visibility_tests;

#[cfg(test)]
#[path = "item_look_dispatch_tests.rs"]
mod item_look_dispatch_tests;
