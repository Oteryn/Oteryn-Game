//! One gameplay connection. The transport decodes bounded Foundation frames and
//! hands the validated bootstrap to the owning admission authority; it never
//! decides admission itself and fails closed for every message it does not own.

pub(super) mod appearance_change;
mod authority;
pub use authority::{AchievementNoticeObservation, DepotObservation, FreshAdmissionAuthority};
mod gameplay_idle;
pub use gameplay_idle::{GameplayIdlePolicy, GameplayIdleState};
mod moved_step;
mod routing;
#[cfg(test)]
pub(super) async fn test_analyser_route(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    view: &mut super::party_command::SessionPartyView,
) -> Option<Vec<u8>> {
    let reply = if command.command_type
        == oteryn_protocol_oteryn::analyser_catalogue::COMMAND_TYPE_ANALYSER_NPC_PRICES
    {
        routing::analyser_catalogue::route(owner, admitted, command).await
    } else {
        routing::analyser_party::route(owner, admitted, command, view).await
    };
    match reply {
        routing::Dispatch::Reply(crate::foundation::CommandStatus::Accepted, payload) => {
            Some(payload)
        }
        _ => None,
    }
}

mod serve;
use crate::combat_pickup::GroundPickupError;
use routing::Dispatch;
use serve::serve_admitted_actor_for_join;
#[cfg(test)]
pub(in crate::gameplay_transport) use serve::{
    ItemUseContention, regression_item_use_with_busy_holder, regression_routed_step,
    regression_world_use_with_busy_holder,
};
#[path = "newhaven_travel_transport.rs"]
mod newhaven_travel_transport;
#[path = "server_save_countdown_transport.rs"]
mod server_save_countdown_transport;
#[path = "tutorial_notices_transport.rs"]
mod tutorial_notices_transport;
#[path = "zones_nologout_transport.rs"]
mod zones_nologout_transport;
use crate::durability::item_transfer::{
    CommittedItemTransfer, CurrentCharacterItemFence, ItemTransferError, ItemTransferOutcome,
};
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelId, CharacterId, ExactActorRef, FoundationProtocolError,
    GameSessionId, MessageType, ServerResumeAcceptedValue, WorldId, decode_wire_envelope,
    encode_protocol_error, encode_protocol_error_with_trace, encode_server_accepted,
    encode_server_accepted_with_trace, encode_server_resume_accepted,
};
mod item_use_view_refresh;
pub(in crate::gameplay_transport) use item_use_view_refresh::refresh_newhaven_inventory;

use appearance_change::SessionAppearance;
use moved_step::moved_step_frames;
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};

struct ConnectionTraceContext {
    trace: [u8; 16],
    enabled: std::cell::Cell<bool>,
}

tokio::task_local! {
    static CONNECTION_TRACE: ConnectionTraceContext;
}

#[cfg(test)]
tokio::task_local! {
    pub(super) static AFTER_COMMAND_INPUT_GUARDS: std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

pub(super) async fn with_connection_trace<F: Future>(trace: [u8; 16], future: F) -> F::Output {
    with_connection_trace_state(trace, false, future).await
}

pub(super) async fn with_selected_connection_trace<F: Future>(
    trace: [u8; 16],
    future: F,
) -> F::Output {
    with_connection_trace_state(trace, true, future).await
}

async fn with_connection_trace_state<F: Future>(
    trace: [u8; 16],
    enabled: bool,
    future: F,
) -> F::Output {
    CONNECTION_TRACE
        .scope(
            ConnectionTraceContext {
                trace,
                enabled: std::cell::Cell::new(enabled),
            },
            // LOGDBG-C: the durable passes this connection issues carry its trace.
            crate::durability::with_holder_trace(trace, future),
        )
        .await
}

fn connection_trace() -> Option<[u8; 16]> {
    CONNECTION_TRACE
        .try_with(|context| context.enabled.get().then_some(context.trace))
        .ok()
        .flatten()
}

pub(super) fn diagnostic_connection_trace() -> Option<[u8; 16]> {
    CONNECTION_TRACE.try_with(|context| context.trace).ok()
}

fn enable_connection_trace() {
    let _ = CONNECTION_TRACE.try_with(|context| context.enabled.set(true));
}

use super::actor_spell::{
    ActorVitals, DELTA_TYPE_ACTOR_VITALS_V1, SNAPSHOT_TYPE_ACTOR_VITALS_V1,
    STATE_DOMAIN_ACTOR_VITALS, SpellCastDisposition, SpellCastIntent, SpellCastOutcome,
    decode_spell_cast_intent, encode_actor_vitals, encode_spell_cast_result,
};
use super::attack::{COMBAT_REFRESH, CombatContinuity};
use super::capabilities::{
    CAPABILITY_ITEM_LOOK_V1, OfferedCapability, PRODUCTION_OFFERED_CAPABILITIES,
    SelectedCapabilities, with_weather_offer,
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
    decode_step_intent_selected, encode_step_outcome, encode_visibility_delta_lit,
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
use oteryn_protocol_oteryn::account_achievements::decode_account_achievements_query;
use oteryn_protocol_oteryn::achievement_notices::{
    AchievementEarned, AchievementWatermark, CAPABILITY_ACHIEVEMENT_NOTICES_V1,
    DELTA_TYPE_ACHIEVEMENT_EARNED_V1, SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
    STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES, encode_achievement_earned,
    encode_achievement_notices_snapshot,
};
use oteryn_protocol_oteryn::attack::{
    ActorCombatState, AttackIntentDisposition, ChaseMode, FightModes,
    SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1, STATE_DOMAIN_ACTOR_COMBAT_STATE,
    decode_attack_target_intent, decode_fight_modes_intent, encode_attack_intent_result,
};
use oteryn_protocol_oteryn::bed::{
    BedSleepIntent, COMMAND_TYPE_BED_SLEEP_INTENT, decode_bed_sleep_intent,
};
use oteryn_protocol_oteryn::chat::{
    ChatIntent, ChatIntentResult, ChatLine, ChatRoomSet, DELTA_TYPE_CHAT_LINE_V1,
    DELTA_TYPE_CHAT_ROOMS_V1, STATE_DOMAIN_CHAT, encode_chat_intent_result, encode_chat_line,
    encode_chat_rooms,
};
use oteryn_protocol_oteryn::container_tree::CAPABILITY_CONTAINER_TREE_V1;
use oteryn_protocol_oteryn::container_tree::{
    ContainerViewOutcome, STATE_DOMAIN_CONTAINER_VIEWS, decode_container_view_intent,
    encode_container_view_result,
};
use oteryn_protocol_oteryn::encode_command_error_result;
use oteryn_protocol_oteryn::encode_snapshot;
use oteryn_protocol_oteryn::highscores::{HighscoresQuery, decode_highscores_query};
use oteryn_protocol_oteryn::item_look::{
    ItemLookRefusal, ItemLookResult, ItemLookSelection, ItemLookTarget,
    decode_item_look_intent_for, encode_item_look_result_for, known_creature_generation,
};
use oteryn_protocol_oteryn::item_view::{
    CAPABILITY_ITEM_EQUIP_DROP_V1, EquipmentSlot, ItemMoveOutcome, ItemMoveSelection,
    STATE_DOMAIN_CHARACTER_INVENTORY, encode_item_move_result_for,
};
use oteryn_protocol_oteryn::logout::{
    COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_intent, encode_logout_result,
};
use oteryn_protocol_oteryn::npc_service::{
    DELTA_TYPE_NPC_CONVERSATION_V1, DELTA_TYPE_NPC_TRADE_WINDOW_V1, NpcConversation,
    NpcIntentDisposition, NpcTalkIntent, NpcTradeWindow, SNAPSHOT_TYPE_NPC_CONVERSATION_V1,
    SNAPSHOT_TYPE_NPC_TRADE_WINDOW_V1, STATE_DOMAIN_NPC_CONVERSATION,
    STATE_DOMAIN_NPC_TRADE_WINDOW, decode_npc_talk_intent, encode_npc_conversation,
    encode_npc_intent_result, encode_npc_trade_window,
};
use oteryn_protocol_oteryn::quest_log::{
    DELTA_TYPE_QUEST_LOG_V1, SNAPSHOT_TYPE_QUEST_LOG_V1, STATE_DOMAIN_QUEST_LOG,
    decode_quest_log_query,
};
use oteryn_protocol_oteryn::world_clock::{
    CAPABILITY_WORLD_LIGHT_V1, DELTA_TYPE_WORLD_CLOCK_V1, SNAPSHOT_TYPE_WORLD_CLOCK_V1,
    STATE_DOMAIN_WORLD_CLOCK, WorldClock, encode_world_clock,
};
use oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW;
use oteryn_protocol_oteryn::world_object::{
    CAPABILITY_ITEM_USE_V1, CAPABILITY_RUNE_USE_V1, CAPABILITY_WORLD_INTERACTION_V1,
    DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1, MAX_GATE_MESSAGE_ID, SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
    STATE_DOMAIN_WORLD_OBJECT_OVERLAY, UseDisposition, UseIntent, UseTarget,
    WorldObjectOverlayEntry, decode_use_intent_target, encode_use_result,
    encode_world_object_overlay_delta, encode_world_object_overlay_snapshot,
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
#[derive(Clone, Copy)]
pub struct FreshAdmissionAttempt<'a> {
    pub character_id: CharacterId,
    pub admission_material: &'a [u8],
    pub game_session_id: GameSessionId,
    pub transport: AuthenticatedTransportRefV1,
    /// The bootstrap's supported capabilities, untrusted. ADMIT0 §4: the owner checks them
    /// against the Channel's required set only after authenticating the grant.
    pub supported_capabilities: &'a [u32],
}

impl std::fmt::Debug for FreshAdmissionAttempt<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FreshAdmissionAttempt")
            .field("character_id", &self.character_id)
            .field("admission_material", &"[REDACTED]")
            .field("game_session_id", &self.game_session_id)
            .field("transport", &self.transport)
            .field("supported_capabilities", &self.supported_capabilities)
            .finish()
    }
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
    /// Domain 15 cumulative revision, including discards at recovery snapshots.
    pub analyser_revision: u64,
    /// Domain 57 revision and once-per-login MOTD receipt; both survive reconnect/transfer.
    pub server_message_revision: u64,
    pub server_motd_sent: bool,
    /// IDLEKICK: transport traffic and same-session reconnects are not gameplay input.
    pub gameplay_idle: GameplayIdleState,
    pub connection_generation: u64,
    pub next_command_id: u64,
    pub server_sequence: u64,
    pub spatial_revision: u64,
    /// PLAYER_TRADE domain revision last delivered by this GameSession.
    pub player_trade_revision: u64,
    /// Last presentation domain revision delivered, shared by movement effects and sounds.
    pub presentation_revision: u64,
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
    /// Whether this transport still owes the durable first-input receipt for
    /// a House handoff. A replacement cannot clear the old transport's flag.
    /// None preserves the admission authority's guard for other owners.
    pub house_input_pending: Option<bool>,
    /// A join-time durable lookup proved that this exact GameSession has no activity origin.
    pub non_activity_session: Option<GameSessionId>,
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
    /// PARITY-EVENT-CALLBACKS-1: the last domain 40 revision and the (planned reset instant,
    /// minute) of the last countdown notice sent. Used only with capability 75; a resume or
    /// channel transfer carries them, so a reconnect never repeats a minute.
    pub server_save_revision: u64,
    pub server_save_sent: Option<(i64, u8)>,
}

impl SessionContinuity {
    /// A fresh admission: generation 1, first CommandId 1, no sequenced output yet, the
    /// baseline spatial revision 1, and no overlay observed yet (0, the door's own initial
    /// revision).
    pub const FRESH: Self = Self {
        analyser_revision: 0,
        server_message_revision: 0,
        server_motd_sent: false,
        gameplay_idle: GameplayIdleState::FRESH,
        connection_generation: ADMITTED_GENERATION,
        next_command_id: 1,
        server_sequence: 0,
        spatial_revision: 1,
        player_trade_revision: 0,
        presentation_revision: 0,
        overlay_revision: 0,
        vitals_revision: 0,
        achievement_notice_revision: None,
        selected_capabilities: SelectedCapabilities::NONE,
        house_input_pending: None,
        non_activity_session: None,
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
        server_save_revision: 0,
        server_save_sent: None,
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
    /// Positioned and otherwise initialized; the connection or resume retries final spell repair.
    AwaitingSpellRepair,
    /// The Channel owner refused the write; nothing was written.
    RefusedByChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionRefusal {
    ChannelSwitch(oteryn_protocol_oteryn::channel_switch::SwitchStatus),
    /// PREMCHAN: authenticated destination entry requires current Premium. Uses the existing
    /// silent rejection path; no new public error code is allocated.
    PremiumRequired,
    /// The material or current owner facts do not admit this attempt.
    Rejected,
    /// The owning authority could not decide; nothing was admitted.
    Unavailable,
    /// Refused with an N8 admission refusal code (ARCH-LOGIN-FIRST-PACKETS-V1 §1.2); nothing
    /// was admitted. The client receives the code in one `ProtocolError` before the close.
    Classified(FoundationProtocolError),
    /// Authenticated capacity refusal with a bounded server-owned FIFO position.
    Queued(oteryn_protocol_oteryn::admission_queue::AdmissionQueue),
    /// ADMIT0 §4: an authenticated recovery whose support lacks the Channel's required set.
    /// Sent as `RECOVERY_CAPABILITY_REQUIRED` (1118, CP D1187); no claim is released and no
    /// recovery grant is consumed.
    RecoveryCapabilityRequired,
}

/// N8: the one pre-admission refusal frame is written and flushed within this bound, or dropped.
const ADMISSION_REFUSAL_WRITE_BOUND: std::time::Duration = std::time::Duration::from_secs(2);

// The supported client waits 20 seconds for its first response. Reserve time for the
// refusal write and give fresh admission three ordinary root recovery windows.
const FRESH_ADMISSION_BOUND: std::time::Duration =
    crate::durability::ROOT_RECOVERY_WINDOW.saturating_mul(3);

pub(super) struct PendingJoin {
    frame: Vec<u8>,
    deadline: tokio::time::Instant,
}

#[derive(Clone, Copy)]
enum JoinCause {
    HolderBusy,
    Deadline,
    Unavailable,
    Unprepared,
}

impl JoinCause {
    fn name(self) -> &'static str {
        match self {
            Self::HolderBusy => "holder_busy",
            Self::Deadline => "deadline",
            Self::Unavailable => "authority_unavailable",
            Self::Unprepared => "unprepared",
        }
    }
}

#[derive(Clone, Copy)]
struct JoinRefusal {
    phase: &'static str,
    cause: JoinCause,
}

impl JoinRefusal {
    fn unprepared(phase: &'static str) -> Self {
        Self {
            phase,
            cause: JoinCause::Unprepared,
        }
    }

    #[track_caller]
    fn report(self, admitted: AdmittedSession) {
        report_serve_end(
            admitted,
            &format!(
                "join_refused phase={} cause={}",
                self.phase,
                self.cause.name()
            ),
        );
    }
}

#[track_caller]
fn report_serve_end(admitted: AdmittedSession, reason: &str) {
    super::operator_event(&format!(
        "event=serve_end reason={reason} source={} next_command_id={} server_sequence={}",
        std::panic::Location::caller(),
        admitted.continuity.next_command_id,
        admitted.continuity.server_sequence,
    ));
}

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static BEFORE_JOIN_SNAPSHOT:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

struct AdmissionWait {
    deadline: tokio::time::Instant,
    retry: std::cell::Cell<Option<JoinRefusal>>,
}

tokio::task_local! {
    static ADMISSION_WAIT: AdmissionWait;
}

pub(super) fn admission_wait_deadline() -> Option<tokio::time::Instant> {
    ADMISSION_WAIT.try_with(|wait| wait.deadline).ok()
}

pub(super) fn record_admission_retry(phase: &'static str, holder_busy: bool) {
    let cause = if holder_busy {
        JoinCause::HolderBusy
    } else {
        JoinCause::Unavailable
    };
    let _ = ADMISSION_WAIT.try_with(|wait| {
        wait.retry.set(Some(JoinRefusal { phase, cause }));
        super::operator_event(&format!(
            "event=admission_retry phase={phase} cause={} remaining_ms={}",
            cause.name(),
            wait.deadline
                .saturating_duration_since(tokio::time::Instant::now())
                .as_millis(),
        ));
    });
}

pub(super) fn clear_admission_retry() {
    let _ = ADMISSION_WAIT.try_with(|wait| wait.retry.set(None));
}

fn join_refusal(phase: &'static str) -> JoinRefusal {
    ADMISSION_WAIT
        .try_with(|wait| wait.retry.get())
        .ok()
        .flatten()
        .unwrap_or_else(|| JoinRefusal::unprepared(phase))
}

async fn admission_stage<T>(
    deadline: Option<tokio::time::Instant>,
    phase: &'static str,
    wait_on: &'static str,
    work: impl Future<Output = T>,
) -> Result<T, JoinRefusal> {
    let Some(deadline) = deadline else {
        return Ok(work.await);
    };
    let started = tokio::time::Instant::now();
    let result = ADMISSION_WAIT
        .scope(
            AdmissionWait {
                deadline,
                retry: std::cell::Cell::new(None),
            },
            async {
                tokio::time::timeout_at(deadline, work).await.map_err(|_| {
                    ADMISSION_WAIT
                        .with(|wait| wait.retry.get())
                        .unwrap_or(JoinRefusal {
                            phase,
                            cause: JoinCause::Deadline,
                        })
                })
            },
        )
        .await;
    super::record_admission_stage(
        phase,
        wait_on,
        if result.is_ok() {
            "completed"
        } else {
            "deadline"
        },
        started.elapsed(),
    );
    result
}

/// DEFLAKE-2: the closed-schema operator line of a silent fresh-admission or resume refusal.
/// It names the refusal kind alone, never a session, account, character or transport.
#[cfg(test)]
pub(super) fn admission_refused_event(refusal: AdmissionRefusal) -> Option<&'static str> {
    match refusal {
        AdmissionRefusal::Unavailable => Some("event=admission_refused kind=unavailable"),
        // PREMCHAN: a Premium refusal takes the silent rejection path.
        AdmissionRefusal::Rejected | AdmissionRefusal::PremiumRequired => {
            Some("event=admission_refused kind=rejected")
        }
        _ => None,
    }
}

pub(super) fn report_admission_refusal(refusal: AdmissionRefusal) {
    let (kind, reason) = match refusal {
        AdmissionRefusal::PremiumRequired | AdmissionRefusal::Rejected => ("rejected", None),
        AdmissionRefusal::ChannelSwitch(status) => {
            use oteryn_protocol_oteryn::channel_switch::SwitchLock;
            (
                match status.lock {
                    SwitchLock::ChannelFull => "channel_full",
                    SwitchLock::Cooldown => "channel_cooldown",
                    SwitchLock::Combat => "channel_combat",
                    SwitchLock::Skull => "channel_skull",
                    SwitchLock::Transition => "channel_transition",
                    _ => "channel_locked",
                },
                None,
            )
        }
        AdmissionRefusal::Unavailable => ("unavailable", None),
        AdmissionRefusal::Classified(error) => ("classified", Some(error)),
        AdmissionRefusal::Queued(_) => ("queued", None),
        AdmissionRefusal::RecoveryCapabilityRequired => ("recovery_capability", None),
    };
    let trace = diagnostic_connection_trace().map(super::format_connection_trace);
    let mut line = oteryn_error_codes::Line::new(
        oteryn_error_codes::Level::Warn,
        "admission",
        "admission_refused",
    )
    .field("kind", oteryn_error_codes::FieldValue::Token(kind));
    if let Some(error) = reason {
        line = line.field(
            "reason_code",
            oteryn_error_codes::FieldValue::U64(u64::from(error.code())),
        );
    }
    if let Some(trace) = trace.as_deref() {
        line = line.trace(trace);
    }
    oteryn_error_codes::emit("oteryn-game-server", &line);
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
    /// Immutable reward-mint event ID for an admitted treasure claim.
    pub reward_granted: Option<[u8; 16]>,
    pub committed: Option<oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry>,
    /// ACH-NOTIFY-1/2: what the grant of this `USE` earned, after its commit. The loop acts on
    /// it only for a session that selected capability 8.
    pub earned: EarnedNotice,
    /// The gate message id of a `SEALED` outcome (1 to 127): carried to the result under
    /// capability 11, required by the codec for `SEALED` and absent for every other disposition.
    pub gate_message_id: Option<u8>,
    pub potion_utterance: Option<oteryn_protocol_oteryn::world_object::PotionUtterance>,
    /// PARITY-REFUSAL-DETAIL-1: the refusal detail the disposition alone does not name (the
    /// floor of a target out of reach). `None` sends the disposition's own detail under
    /// capability 66; nothing is sent without it.
    pub refusal_detail: Option<oteryn_protocol_oteryn::refusal_detail::RefusalDetail>,
}

impl UseOutcome {
    pub const fn rejected() -> Self {
        Self {
            disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Rejected,
            committed: None,
            earned: EarnedNotice::NoneEarned,
            reward_granted: None,
            gate_message_id: None,
            potion_utterance: None,
            refusal_detail: None,
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
#[cfg(test)]
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
    admit_frame_inner(stream, frame, authority, identifiers, false)
        .await
        .map(|(admitted, _)| admitted)
}

/// The live listener can delay fresh Channel acceptance until it has built the join snapshot.
#[allow(
    clippy::result_large_err,
    reason = "preserve the committed controller for the listener's terminal lifecycle"
)]
pub(super) async fn admit_frame_for_join<S, A, I>(
    stream: &mut S,
    frame: &[u8],
    authority: &A,
    identifiers: &I,
) -> Result<(AdmittedSession, Option<PendingJoin>), ConnectionEnd>
where
    S: AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
{
    admit_frame_inner(stream, frame, authority, identifiers, true).await
}

#[allow(
    clippy::result_large_err,
    reason = "preserve the committed controller for the listener's terminal lifecycle"
)]
async fn admit_frame_inner<S, A, I>(
    stream: &mut S,
    frame: &[u8],
    authority: &A,
    identifiers: &I,
    defer_fresh_acceptance: bool,
) -> Result<(AdmittedSession, Option<PendingJoin>), ConnectionEnd>
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
            let mut resumed = match resumed {
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
                Err(refusal) => {
                    report_admission_refusal(refusal);
                    return Err(ConnectionEnd::ResumeUnavailable);
                }
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
            // A real activity receipt/current owner supplies the exact scope;
            // no House acknowledgement or unsolicited command result is used.
            let resume_deadline =
                tokio::time::Instant::now() + crate::durability::ROOT_RECOVERY_WINDOW;
            let activity_scope = admission_stage(
                Some(resume_deadline),
                "resume_activity_scope",
                "durability_holder",
                async {
                    authority
                        .activity_scope_snapshot(&mut resumed)
                        .await
                        .map_err(|()| join_refusal("resume_activity_scope"))
                },
            )
            .await;
            let activity_scope = match activity_scope {
                Ok(Ok(scope)) => scope,
                Ok(Err(refusal)) | Err(refusal) => {
                    super::operator_event(&format!(
                        "event=resume_bootstrap_failed phase=activity_scope code={}",
                        FoundationProtocolError::SessionJoinUnavailable.code()
                    ));
                    return Err(join_unavailable(stream, resumed, 0, refusal).await);
                }
            };
            let Ok(accepted) = encode_server_resume_accepted(&ServerResumeAcceptedValue {
                game_session_id: resumed.game_session_id,
                connection_generation: continuity.connection_generation,
                current_server_sequence: continuity.server_sequence,
                next_command_id: continuity.next_command_id,
                schema_revision: SERVER_SCHEMA_REVISION,
                selected_capabilities: continuity.selected_capabilities.as_slice(),
            }) else {
                return Err(join_unavailable(
                    stream,
                    resumed,
                    0,
                    JoinRefusal::unprepared("resume_acceptance_encode"),
                )
                .await);
            };
            write_frame(stream, &accepted)
                .await
                .map_err(|_| ConnectionEnd::AdmittedThenDisconnected(resumed))?;
            let generation = continuity.connection_generation;
            let house_ack = admission_stage(
                Some(resume_deadline),
                "resume_house_scope",
                "durability_holder",
                async {
                    authority
                        .house_scope_ack(resumed)
                        .await
                        .map_err(|()| join_refusal("resume_house_scope"))
                },
            )
            .await;
            let house_ack = match house_ack {
                Ok(Ok(ack)) => ack,
                Ok(Err(refusal)) | Err(refusal) => {
                    return Err(join_unavailable(stream, resumed, generation, refusal).await);
                }
            };
            if let Some((command, payload)) = house_ack {
                if command == 0 {
                    // The native owner admits zero only for an authenticated
                    // fresh House restoration. It is server provenance, not a
                    // client command: the House scope is published by the
                    // existing domain-51 join/resync snapshot instead.
                    return Ok((resumed, None));
                }
                let Some(Ok(ack)) = continuity.server_sequence.checked_add(1).map(|sequence| {
                    encode_command_result(
                        generation,
                        sequence,
                        command,
                        CommandStatus::Accepted,
                        &payload,
                    )
                }) else {
                    return Err(join_unavailable(
                        stream,
                        resumed,
                        generation,
                        JoinRefusal::unprepared("resume_house_ack_encode"),
                    )
                    .await);
                };
                write_frame(stream, &ack)
                    .await
                    .map_err(|_| ConnectionEnd::AdmittedThenDisconnected(resumed))?;
                resumed.continuity.server_sequence += 1;
                return Ok((resumed, None));
            }
            if activity_scope.is_none()
                && (resumed.game_session_id != resume.game_session_id
                    || resumed.item_fence.is_some_and(|f| {
                        matches!(
                            f.runtime_scope,
                            crate::foundation::RuntimeScopeRefV1::Instance { .. }
                        )
                    }))
            {
                return Err(ConnectionEnd::AdmittedThenDisconnected(resumed));
            }
            return Ok((resumed, None));
        }
        _ => Err(FoundationProtocolError::MalformedEnvelope),
    };
    let bootstrap = match bootstrap {
        Ok(bootstrap) => bootstrap,
        Err(error) => return Err(reject(stream, error, 0).await),
    };
    let deadline = tokio::time::Instant::now() + FRESH_ADMISSION_BOUND;
    let (Some(game_session_id), Some(transport)) =
        (identifiers.game_session_id(), identifiers.transport_ref())
    else {
        if defer_fresh_acceptance {
            send_fresh_unavailable(stream).await;
        }
        return Err(ConnectionEnd::AdmissionRefused(
            AdmissionRefusal::Unavailable,
        ));
    };
    // CAP-NEG-1: the client's supported capabilities the server offers, with their `requires`.
    // WEATHER-1: capability 26 is offered only while the authority holds a weather source.
    let Ok(Some(selected)) = admission_stage(
        defer_fresh_acceptance.then_some(deadline),
        "capability_selection",
        "runtime_lock",
        fresh_capability_selection(authority, bootstrap.supported_capabilities),
    )
    .await
    else {
        if defer_fresh_acceptance {
            send_fresh_unavailable(stream).await;
        }
        return Err(ConnectionEnd::AdmissionRefused(
            AdmissionRefusal::Unavailable,
        ));
    };
    if selected.contains(oteryn_protocol_oteryn::connection_trace::CAPABILITY_CONNECTION_TRACE_V1) {
        enable_connection_trace();
    }
    // ADMIT0 §4: no capability refusal precedes the owner's grant authentication; the owner
    // checks the Channel's required set itself and answers ADMISSION_CAPABILITY_REQUIRED.
    let attempt = FreshAdmissionAttempt {
        character_id: bootstrap.character_id,
        admission_material: bootstrap.admission_material,
        game_session_id,
        transport,
        supported_capabilities: bootstrap.supported_capabilities,
    };
    let mut decision = std::pin::pin!(authority.admit(attempt));
    let decision = match admission_stage(
        defer_fresh_acceptance.then_some(deadline),
        "root_pass",
        "durability_holder_pg_runtime",
        decision.as_mut(),
    )
    .await
    {
        Ok(decision) => decision,
        Err(reason) => {
            send_fresh_unavailable(stream).await;
            // A durable commit may be in flight. Keep polling its owning future and return
            // any resulting controller to the listener's unpublished-join cleanup.
            return Err(match decision.await {
                Ok(admitted) => {
                    reason.report(admitted);
                    ConnectionEnd::AdmittedThenClosed(
                        admitted,
                        FoundationProtocolError::SessionJoinUnavailable,
                    )
                }
                Err(refusal) => ConnectionEnd::AdmissionRefused(refusal),
            });
        }
    };
    let mut admitted = match decision {
        Ok(admitted) => admitted,
        Err(refusal) => {
            report_admission_refusal(refusal);
            if defer_fresh_acceptance && refusal == AdmissionRefusal::Unavailable {
                send_fresh_unavailable(stream).await;
            }
            if let AdmissionRefusal::Classified(error) = refusal {
                // N8: exactly one ProtocolError at generation 0, then close. A slow or
                // failed peer write changes nothing: the refusal already stands.
                let _ = tokio::time::timeout(
                    ADMISSION_REFUSAL_WRITE_BOUND,
                    send_error(stream, error, 0),
                )
                .await;
            }
            if let AdmissionRefusal::ChannelSwitch(status) = refusal
                && status.lock == oteryn_protocol_oteryn::channel_switch::SwitchLock::ChannelFull
            {
                let frame = oteryn_protocol_oteryn::admission_queue::encode_channel_full();
                let _ = tokio::time::timeout(
                    ADMISSION_REFUSAL_WRITE_BOUND,
                    write_frame(stream, &frame),
                )
                .await;
            }
            if let AdmissionRefusal::Queued(queue) = refusal
                && let Ok(frame) =
                    oteryn_protocol_oteryn::admission_queue::encode_queue_refusal(queue)
            {
                let _ = tokio::time::timeout(
                    ADMISSION_REFUSAL_WRITE_BOUND,
                    write_frame(stream, &frame),
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
    let accepted_value = crate::foundation::ServerAcceptedValue {
        game_session_id: admitted.game_session_id,
        world_id: admitted.world_id,
        channel_id: admitted.channel_id,
        connection_generation: ADMITTED_GENERATION,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: SERVER_SCHEMA_REVISION,
        selected_capabilities: selected.as_slice(),
    };
    let Ok(accepted) = (match connection_trace().filter(|_| {
        selected.contains(oteryn_protocol_oteryn::connection_trace::CAPABILITY_CONNECTION_TRACE_V1)
    }) {
        Some(trace) => encode_server_accepted_with_trace(&accepted_value, &trace),
        None => encode_server_accepted(&accepted_value),
    }) else {
        return Err(join_unavailable(
            stream,
            admitted,
            0,
            JoinRefusal::unprepared("acceptance_encode"),
        )
        .await);
    };
    let bulletin = admission_stage(
        defer_fresh_acceptance.then_some(deadline),
        "bulletin",
        "owner_read",
        authority.client_bulletin(),
    )
    .await;
    let bulletin = match bulletin {
        Ok(bulletin) => bulletin,
        Err(refusal) => {
            send_join_unavailable(stream, 0).await;
            refusal.report(admitted);
            return Err(ConnectionEnd::AdmittedThenClosed(
                admitted,
                FoundationProtocolError::SessionJoinUnavailable,
            ));
        }
    };
    let accepted = if let Some(document) = bulletin {
        oteryn_protocol_oteryn::bulletin::append_to_acceptance(&accepted, &document)
            .unwrap_or(accepted)
    } else {
        accepted
    };
    // Resume and House handoffs have their own acceptance/sidecar ordering. The
    // pending spell-repair protocol also uses liveness before it can prepare a join.
    if defer_fresh_acceptance
        && admitted.runtime_actor.is_some()
        && admitted.first_entry != FirstEntryOutcome::AwaitingSpellRepair
        && admitted.item_fence.is_none_or(|fence| {
            matches!(
                fence.runtime_scope,
                crate::foundation::RuntimeScopeRefV1::Channel { .. }
            )
        })
    {
        return Ok((
            admitted,
            Some(PendingJoin {
                frame: accepted,
                deadline,
            }),
        ));
    }
    write_frame(stream, &accepted)
        .await
        .map_err(|_| ConnectionEnd::AdmittedThenDisconnected(admitted))?;
    Ok((admitted, None))
}

/// The existing fresh wire selection, shared with native saved-House admission.
/// It is data only; the owner independently authenticates the grant and current
/// required capabilities before any committed scope transition.
pub(in crate::gameplay_transport) async fn fresh_capability_selection<
    A: FreshAdmissionAuthority,
>(
    authority: &A,
    supported: &[u32],
) -> Option<SelectedCapabilities> {
    let offered = with_weather_offer(
        authority.offered_capabilities(),
        authority.world_weather().is_some(),
    );
    let offered = mechanic_offers(authority, offered);
    let offered = super::native_cue_offer::offered(
        &offered,
        authority.native_world_presentation_available().await,
    );
    SelectedCapabilities::select(&offered, supported)
}

/// Test observation of a current actor's optional offer. Production Fresh/
/// House selection remains before its immutable acknowledgement; this helper
/// neither installs a selection nor widens a transfer/resume.
#[cfg(test)]
pub(in crate::gameplay_transport) async fn fresh_current_actor_capability_selection<
    A: FreshAdmissionAuthority,
>(
    authority: &A,
    supported: &[u32],
    admitted: AdmittedSession,
) -> Option<SelectedCapabilities> {
    let (Some(actor), Some(_)) = (admitted.runtime_actor, admitted.controller) else {
        return fresh_capability_selection(authority, supported).await;
    };
    let offered = with_weather_offer(
        authority.offered_capabilities(),
        authority.world_weather().is_some(),
    );
    let offered = mechanic_offers(authority, offered);
    let offered = super::native_cue_offer::offered(
        &offered,
        authority
            .native_current_actor_map_cue_frame(actor, admitted.game_session_id)
            .await
            .is_some(),
    );
    SelectedCapabilities::select(&offered, supported)
}

/// No gameplay command, state or liveness semantics are allocated to the seam:
/// any post-admission input ends the connection without mutation.
pub async fn hold_admitted<S>(stream: &mut S, admitted: AdmittedSession) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let frame = match read_frame(stream).await {
        Ok(frame) => frame,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
            return close_admitted(stream, admitted, FoundationProtocolError::MalformedFrame).await;
        }
        Err(_) => return ConnectionEnd::AdmittedThenDisconnected(admitted),
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
/// actor is held at most `policy.interval * policy.missed_limit`; then the connection reports
/// `NOT_SUPPORTED` before closing, so a silent client cannot keep its committed GameSession and slot. A frame
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
        Err(_) => close_admitted(stream, admitted, FoundationProtocolError::NotSupported).await,
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

#[cfg(test)]
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
    serve_admitted_for_join(stream, admitted, authority, policy, &mut None).await
}

pub(super) async fn serve_admitted_for_join<S, A>(
    stream: &mut S,
    admitted: AdmittedSession,
    authority: &A,
    policy: LivenessPolicy,
    acceptance: &mut Option<PendingJoin>,
) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    #[cfg(test)]
    super::monk_save::postgres_tests::maintain_authority_ready(authority).await;
    let end = Box::pin(serve_admitted_actor_for_join(
        stream, admitted, authority, policy, acceptance,
    ))
    .await;
    let ended = match end {
        ConnectionEnd::AdmittedThenClosed(current, _)
        | ConnectionEnd::AdmittedThenDisconnected(current)
        | ConnectionEnd::AdmittedThenControlLost(current)
        | ConnectionEnd::AdmittedThenLoggedOut(current) => current,
        _ => admitted,
    };
    let ended = authority.current_activity_connection(ended);
    let (actor, session) = (ended.runtime_actor, ended.game_session_id);
    // CHASE-1 (`RANGED0-RL-03`): the ended connection drives no more chase, so its waiting
    // search leaves the Channel's ordering now rather than holding a place through the grace.
    if let Some(actor) = actor {
        let scoped = authority.resolve_native_scope(ended).await.ok().flatten();
        scoped
            .as_ref()
            .unwrap_or(authority)
            .yield_chase_to_client(actor, session)
            .await;
    }
    #[cfg(test)]
    super::monk_save::postgres_tests::maintain_authority_ready(authority).await;
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

/// MAP-WIRE-2: the domain 17 update at the sequence after `sequence`, `Some(None)` when
/// unchanged, or `None` when the map is unserved or cannot encode. A replacement uses the same
/// snapshot producer as admission. Callers write all frames before any later sequenced output,
/// preserving the FND-02 §16 publication barrier and the current connection generation.
async fn world_map_frames<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    generation: u64,
    sequence: u64,
    view: &mut SessionMapView,
    items: &mut SessionItemView,
) -> Option<Option<(u64, Vec<Vec<u8>>)>> {
    match authority.observe_world_map(actor, view, items).await? {
        Ok(None) => Some(None),
        Ok(Some(MapUpdate::Delta(delta))) => {
            let (sequence, frame) = item_view_delta(generation, sequence, &delta)?;
            Some(Some((sequence, vec![frame])))
        }
        Ok(Some(MapUpdate::Snapshot(snapshot))) => {
            // Admission reserves snapshot ID 1. Sequence boundaries also identify the
            // existing scope-barrier snapshots, so reserve at least boundary 2 here.
            let target = sequence.checked_add(1)?.max(2);
            let frames = encode_snapshot(
                generation,
                target,
                target,
                &[DomainSnapshot {
                    domain_id: snapshot.domain_id,
                    revision: snapshot.revision,
                    snapshot_type: snapshot.snapshot_type,
                    payload: &snapshot.payload,
                }],
            )
            .ok()?;
            Some(Some((target, frames)))
        }
        Err(_) => None,
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
fn npc_update_frames<A: FreshAdmissionAuthority>(
    authority: &A,
    viewer: (GameSessionId, SelectedCapabilities),
    generation: u64,
    sequence: &mut u64,
    continuity: &mut NpcContinuity,
    update: &NpcUpdate,
    npc_shown: &mut Option<WireEntityRef>,
) -> Option<Vec<Vec<u8>>> {
    let (game_session_id, selected) = viewer;
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
        let details =
            selected.contains(oteryn_protocol_oteryn::trade_details::CAPABILITY_TRADE_DETAILS_V1);
        let window = if details {
            authority
                .npc_quote_window(game_session_id, window)
                .unwrap_or_else(|| window.clone())
        } else {
            window.clone()
        };
        let delta_sequence = next.checked_add(1)?;
        let to = window_revision.checked_add(1)?;
        let frame = encode_state_delta(
            generation,
            delta_sequence,
            STATE_DOMAIN_NPC_TRADE_WINDOW,
            window_revision,
            to,
            DELTA_TYPE_NPC_TRADE_WINDOW_V1,
            &oteryn_protocol_oteryn::npc_service::encode_npc_trade_window_for(&window, details)
                .ok()?,
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

/// FND-04C §4.1 (owner 2026-11-10): an admitted or resumed session whose join could not be
/// prepared or sent reports `SESSION_JOIN_UNAVAILABLE` once before the transport closes, at
/// `generation` (0 before the client was told it was admitted). A slow or failed write changes
/// nothing: the end stays the same.
async fn send_join_unavailable<S: AsyncWrite + Unpin>(stream: &mut S, generation: u64) {
    let _ = tokio::time::timeout(
        ADMISSION_REFUSAL_WRITE_BOUND,
        send_error(
            stream,
            FoundationProtocolError::SessionJoinUnavailable,
            generation,
        ),
    )
    .await;
}

async fn send_fresh_unavailable<S: AsyncWrite + Unpin>(stream: &mut S) {
    let _ = tokio::time::timeout(
        ADMISSION_REFUSAL_WRITE_BOUND,
        send_error(
            stream,
            FoundationProtocolError::AdmissionAttemptReconciliationRequired,
            0,
        ),
    )
    .await;
}

/// `AdmittedThenDisconnected` after [`send_join_unavailable`].
async fn join_unavailable<S: AsyncWrite + Unpin>(
    stream: &mut S,
    admitted: AdmittedSession,
    generation: u64,
    refusal: JoinRefusal,
) -> ConnectionEnd {
    send_join_unavailable(stream, generation).await;
    refusal.report(admitted);
    ConnectionEnd::AdmittedThenDisconnected(admitted)
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
    let frame = match connection_trace() {
        Some(trace) => encode_protocol_error_with_trace(error, generation, &trace),
        None => encode_protocol_error(error, generation),
    }
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
pub(in crate::gameplay_transport) use visibility_tests::{
    prime_player_status_clock, regression_player_status,
};

#[cfg(test)]
#[path = "item_look_dispatch_tests.rs"]
mod item_look_dispatch_tests;

include!(concat!(env!("OUT_DIR"), "/mechanic_offers.rs"));

#[cfg(test)]
use super::actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT;

#[cfg(test)]
use oteryn_protocol_oteryn::account_achievements::COMMAND_TYPE_ACCOUNT_ACHIEVEMENTS_QUERY;

#[cfg(test)]
use oteryn_protocol_oteryn::highscores::COMMAND_TYPE_HIGHSCORES_QUERY_V1;

#[cfg(test)]
use oteryn_protocol_oteryn::npc_service::COMMAND_TYPE_NPC_TALK_INTENT;

#[cfg(test)]
use oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT;
