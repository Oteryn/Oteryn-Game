//! Transport-neutral Oteryn Game session (ADR-0020 section 1, lane N1; ADR-0014 layering
//! `protocol-oteryn -> session -> TCP adapter`): admission, envelope validation, sequencing,
//! join-snapshot assembly, the `step`/`use_object`/`cast_spell` command exchange, command status pairing and
//! liveness, all over an abstract byte stream ([`SessionStream`]).
//!
//! The crate names no TLS or TCP type and opens no connection: a caller (the TCP adapter, a later
//! QUIC adapter, or a test double) connects, then hands the established stream to
//! [`Session::admit`]. Transport-level checks that must happen before any Foundation frame is
//! sent (for the TCP adapter: TLS 1.3 and the exact ALPN `oteryn-game/1`) belong to the adapter.
//!
//! Every wire codec used here is `oteryn-protocol-oteryn`'s own (`encode_client_bootstrap`,
//! `encode_client_command`, `encode_liveness_ack`, `decode_wire_envelope`,
//! `WireEnvelopeView::validate`, `decode_server_accepted`, `decode_snapshot_chunk_framing`,
//! `decode_snapshot_body`, `decode_command_result`, `decode_state_delta`,
//! `decode_liveness_probe`, and the `world_spatial`/`world_object` domain codecs): this crate
//! holds no codec of its own, only the glue that ties one admission to its join-snapshot decode
//! and its command/sequence discipline. Every admission or codec error fails closed.

/// Account achievement query and typed result.
pub mod account_achievements;
/// Account-only observer transport; separate from the player Session.
pub mod cast_chat;
pub mod cast_viewer;

mod session_types;
pub use session_types::{Admission, DuplicateOutcome, JoinSnapshot};
use session_types::{MAX_RETAINED_DUPLICATES, SERVER_SILENCE_DEADLINE};

/// Optional authenticated node-capacity queue metadata.
pub use oteryn_protocol_oteryn::admission_queue::{self, AdmissionQueue};
/// Display and control vocabulary for S17.
pub use oteryn_protocol_oteryn::cast;

#[path = "achievement_notices.rs"]
mod achievement_notices_transport;
pub use oteryn_protocol_oteryn::achievement_notices;
mod activity_handoff;
/// Server-committed shared activity scope receipts.
pub use oteryn_protocol_oteryn::activity_scope;
pub use oteryn_protocol_oteryn::analyser_catalogue;
pub use oteryn_protocol_oteryn::analyser_party;
pub use oteryn_protocol_oteryn::battle_metadata;
pub mod analyser_transport;
mod appearance_state;
pub use oteryn_protocol_oteryn::presentation::{
    ActorCooldown, ActorCooldownKind, ActorCooldowns, decode_actor_cooldowns,
};
#[path = "features/0083-channel-list/protocol.rs"]
pub mod channel_list;
mod depot;
#[path = "features/0037-depot/protocol.rs"]
mod depot_containers;
mod domain_dispatch;
mod entities;
mod error;
mod events;
mod framing;
mod pvp_expert;
pub use events::SessionEvent;
mod gated_command;
pub use gated_command::world_context;
/// Server-owned highscores query and result codecs.
pub use oteryn_protocol_oteryn::highscores;
/// The own Platform sanction record (capability 497).
pub use oteryn_protocol_oteryn::sanction_record;
pub mod container_tree;
mod hotkey_equip_transport;
#[path = "features/0004-item-view-move/protocol.rs"]
pub mod item_movement;
mod join;
mod map_transport;
mod outcomes;
mod player_trade_transport;
mod presentation;
mod quest_log_transport;
mod refusal_details;
mod replacement_snapshot;
mod session_access;
pub use oteryn_protocol_oteryn::refusal_detail;
mod session_commands;
mod session_exchange;
mod session_transport;
mod snapshot_install;
mod snapshot_state;
mod snapshot_transfer;
#[path = "features/0287-actor-spell-catalog/protocol.rs"]
pub mod spell_catalog;
pub use oteryn_protocol_oteryn::presentation::{
    PresentationColor, PresentationEvent, PresentationEventKind, PresentationSource,
    PresentationValue, PresentationValueKind,
};
mod recovery;
use join::JoinAcceptance;
pub use oteryn_protocol_oteryn::coin_market;
pub use oteryn_protocol_oteryn::coin_transfer;
pub use oteryn_protocol_oteryn::depot::{
    DepotAction, DepotEntry, DepotIntent, DepotOutcome, DepotReply,
};
/// Guild command and query wire values for the selected client feature.
pub use oteryn_protocol_oteryn::guild;
/// Hireling wire values for the selected client feature.
pub use oteryn_protocol_oteryn::hireling;
pub use oteryn_protocol_oteryn::hotkey_equip;
/// Market wire values for the selected client feature.
pub use oteryn_protocol_oteryn::market;
pub use oteryn_protocol_oteryn::market_details;
/// Party wire values for the selected client feature.
pub use oteryn_protocol_oteryn::party;
pub use oteryn_protocol_oteryn::quest_log;
pub use oteryn_protocol_oteryn::world_clock;
#[path = "features/0019-world-light/protocol.rs"]
mod world_light;
pub use oteryn_protocol_oteryn::world_weather;
#[path = "features/0026-world-weather/protocol.rs"]
mod weather;
pub use oteryn_protocol_oteryn::world_event;
/// Live map snapshot and delta value types and codecs.
pub use oteryn_protocol_oteryn::world_map;
pub use oteryn_protocol_oteryn::{
    animus_mastery, browse_field, daily_boost, daily_reward, depot_search, item_forge, item_view,
    store,
};
pub use oteryn_protocol_oteryn::{atelier, prey, wheel};
pub use oteryn_protocol_oteryn::{creature_push, target_move};
/// Player-trade wire values for the selected native-client feature.
pub use oteryn_protocol_oteryn::{player_trade, player_typing};
pub use recovery::{Recovery, RecoveryState, check_resume_selection};
mod chat_extended;
mod guild_chat;
mod private_channels;
pub use appearance_state::{
    AppearanceItemDictionary, AppearanceUpdate, EntityDisplayName, Look, VisibleAppearance,
};
pub use chat_extended::{
    ChannelListExtension, ChannelNotice, ChannelNoticeKind, ChannelParticipants, LineExtra,
    NoticeEntry,
};
use oteryn_protocol_oteryn::actor_spell::{
    self, AimedSpellCastIntent, SpellAimMode, SpellCastIntent,
};
/// Spell types a client names when it casts and draws vitals: re-exported so the client needs no
/// direct `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::actor_spell::{
    ActorVitals, CAPABILITY_SPELL_AIM_MODE_V1, SpellCastDisposition, SpellTarget,
    SpellTargetPosition,
};
use oteryn_protocol_oteryn::attack::{self, CAPABILITY_ATTACK_V1};
/// The capability-17 attack types a client sends and draws from, re-exported so it needs no
/// direct `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::attack::{
    ActorCombatState, AttackIntentDisposition, ChaseMode, FightMode, FightModes,
};
pub use oteryn_protocol_oteryn::chat::guild_room::{GuildChatLine, GuildRoomState};
/// Capability-80 channel list and command values used by typed client adapters.
pub use oteryn_protocol_oteryn::chat::private_channel::{
    ChatChannelList, InvitedChannel, PrivateChannelIntent, PrivateChannelLine,
};
use oteryn_protocol_oteryn::chat::{self, STATE_DOMAIN_CHAT};
pub use oteryn_protocol_oteryn::chat::{CAPABILITY_CHAT_V1, COMMAND_TYPE_CHAT_INTENT};
/// The capability-7 chat types a client sends and draws from, re-exported so it needs no direct
/// `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::chat::{
    ChatDisposition, ChatIntent, ChatLine, ChatRoom, ChatRoomSet, ChatSpeaker, ChatSpeechMode,
    ChatWireError, MAX_CHAT_NAME_BYTES, MAX_CHAT_TEXT_BYTES, MAX_CHAT_WAIT_SECONDS,
    console as chat_console, encode_chat_intent,
};
/// The capability-4 item types a client sends and draws from, re-exported so it needs no direct
/// `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::item_view::{
    CharacterInventory, ItemEntry, ItemHandle, ItemMoveDestination, ItemMoveIntent,
    ItemMoveOutcome, MAX_CHARACTER_INVENTORY_ITEMS, MAX_OPEN_CONTAINER_ENTRIES, OpenContainer,
    decode_character_inventory_with_equip_drop, encode_character_inventory_with_equip_drop,
};
use oteryn_protocol_oteryn::logout;
mod server_messages;
mod server_save_countdown;
mod tutorial_notices;
pub use oteryn_protocol_oteryn::server_save_countdown::ServerSaveCountdown;
pub use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
pub use oteryn_protocol_oteryn::{
    arena, boss_difficulty, boss_rewards, character_info, cyclopedia_character, cyclopedia_tracker,
    premium_overview, weapon_proficiency, weapon_proficiency_state, zones_nologout::ServerMessage,
};
mod vip_presence;
pub use oteryn_protocol_oteryn::achievement_notices::{AchievementEarned, AchievementWatermark};
pub use oteryn_protocol_oteryn::action_bar_cooldowns;
pub use oteryn_protocol_oteryn::battle_instinct_status;
pub use oteryn_protocol_oteryn::condition_status;
pub use oteryn_protocol_oteryn::death_protection;
pub use oteryn_protocol_oteryn::exiva;
/// Imbuement shrine wire values used by the native client adapter.
pub use oteryn_protocol_oteryn::imbuement;
/// The capability-21 logout result a client reads, re-exported so it needs no protocol import.
pub use oteryn_protocol_oteryn::logout::LogoutResult;
pub use oteryn_protocol_oteryn::loyalty_skills;
pub use oteryn_protocol_oteryn::mechanics_status;
pub use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTradeSide};
pub use oteryn_protocol_oteryn::npc_shop;
pub use oteryn_protocol_oteryn::party_effects;
pub use oteryn_protocol_oteryn::potion_durations;
pub use oteryn_protocol_oteryn::rest_status;
pub use oteryn_protocol_oteryn::skill_display;
pub use oteryn_protocol_oteryn::skill_progress;
/// Prey slot wire values for the selected client feature.
pub use oteryn_protocol_oteryn::vip_presence::{
    PresenceBatch as VipPresenceBatch, PresenceEntry as VipPresenceEntry,
    Status as VipPresenceStatus,
};
use oteryn_protocol_oteryn::world_interaction;
pub use oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry;
use oteryn_protocol_oteryn::world_object::{self, UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{self, CAPABILITY_PACED_MOVEMENT_V1};
pub use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, StepDirection, StepDisposition, WorldSpatialObservation,
};
use oteryn_protocol_oteryn::world_spatial_entities::{
    self, CAPABILITY_WORLD_SPATIAL_ENTITIES, MAX_SNAPSHOT_ENTITIES,
};
/// The capability-6 entity types a client draws from, re-exported so it needs no direct
/// `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, EntityKind, EntityRef, WorldSpatialEntitiesDelta, WorldSpatialEntity,
    decode_world_spatial_entities_delta_with_item_handles,
    encode_world_spatial_entities_delta_with_item_handles,
};
/// N8 admission refusal classes, re-exported so the client maps a refused code to its FND-04A
/// public class without a direct `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::{
    ADMISSION_REFUSAL_CODES, AdmissionProgression, AdmissionPublicClass, admission_refusal_class,
};
pub use oteryn_protocol_oteryn::{CharacterId, CommandStatus};
use oteryn_protocol_oteryn::{
    ClientBootstrapValue, ClientCommandValue, Direction, FoundationProtocolError, FrameLength,
    GameSessionId, MessageType, ProtocolDisposition, decode_command_result, decode_liveness_probe,
    decode_protocol_error, decode_server_accepted, decode_snapshot_begin, decode_snapshot_body,
    decode_snapshot_chunk_framing, decode_snapshot_id, decode_state_delta, decode_wire_envelope,
    encode_client_bootstrap, encode_client_command, encode_liveness_ack,
};
use oteryn_protocol_oteryn::{
    achievement_notices::CAPABILITY_ACHIEVEMENT_NOTICES_V1,
    analyser::CAPABILITY_ANALYSER_V1,
    charm::CAPABILITY_BESTIARY_CHARMS_V1,
    container_tree::CAPABILITY_CONTAINER_TREE_V1,
    item_view::{
        CAPABILITY_ITEM_VIEW_MOVE_V1, COMMAND_TYPE_ITEM_MOVE_INTENT, ItemViewWireError,
        STATE_DOMAIN_CHARACTER_INVENTORY, STATE_DOMAIN_OPEN_CONTAINER,
    },
};
/// CHARM-5 view and command types (registered under capability 1 `BESTIARY_CHARMS_V1`, not yet
/// routed by this crate), re-exported for the client views the same way.
pub use oteryn_protocol_oteryn::{
    bestiary::BestiaryRaceProgress,
    charm::{
        CharmAssignDisposition, CharmAssignIntent, CharmKind, CharmState, CharmUnlockDisposition,
        CharmUnlockStageIntent, CharmView,
    },
};
pub use oteryn_protocol_oteryn::{inspection, quick_loot, team_finder, team_finder_members};
pub use oteryn_protocol_oteryn::{npc_service, staff};
use std::collections::{BTreeMap, VecDeque};
use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::io;
use std::num::NonZeroU32;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The ALPN protocol id an adapter must negotiate (or otherwise guarantee) before a session is
/// admitted over its stream. Re-exported so adapters need no direct `protocol-oteryn` edge.
pub use oteryn_protocol_oteryn::ALPN_OTERYN_GAME_V1;

/// The byte-stream boundary a [`Session`] runs over: an ordered, reliable, bidirectional byte
/// stream that is already established and, where the transport requires it, already secured and
/// negotiated. Any tokio `AsyncRead + AsyncWrite` type qualifies; adapters (TCP, later QUIC)
/// produce one after their own connect/handshake checks.
pub trait SessionStream: AsyncRead + AsyncWrite + Unpin {}

impl<T: AsyncRead + AsyncWrite + Unpin> SessionStream for T {}

mod capability_support;
pub use capability_support::CLIENT_SUPPORTED_CAPABILITIES;
use capability_support::{capability_of_domain, gated_routes};

/// Total `step_retrying` attempts and the first backoff (doubled after each `TOO_EARLY`).
pub const STEP_RETRY_ATTEMPTS: u32 = 4;
pub const STEP_RETRY_INITIAL_BACKOFF: Duration = Duration::from_millis(100);

/// A state domain of a selected capability from the join snapshot, payload undecoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatedSnapshot {
    pub domain_id: u32,
    pub snapshot_type: u32,
    pub revision: u64,
    pub payload: Vec<u8>,
}

/// A `CommandResult` of a capability-gated command, payload undecoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatedCommandResult {
    pub command_id: u64,
    pub status: CommandStatus,
    pub server_sequence: u64,
    pub payload: Vec<u8>,
}

/// A `StateDelta` of a selected capability's domain, payload undecoded; its revision is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatedDelta {
    pub server_sequence: u64,
    pub base_revision: u64,
    pub new_revision: u64,
    pub payload: Vec<u8>,
}

/// An admitted, joined game session that can issue the registered gameplay commands.
///
/// Discipline (FND-02): every `ClientCommand` carries the admitted `connection_generation` and
/// the next `CommandId` (starting at `ServerAccepted.next_command_id`, strictly increasing,
/// consumed even when the exchange later fails); every inbound frame is envelope-validated
/// (`WireEnvelopeView::validate`), must carry the admitted generation, and every
/// `CommandResult`/`StateDelta` must arrive at exactly the previous applied `server_sequence`
/// plus one; a `CommandResult` must correlate to the command just sent; a `StateDelta` must name
/// the domain and registered `delta_type` the disposition promised, be based on exactly the
/// domain revision last applied, and carry the loaded `content_generation`. The session never
/// predicts the domain of the next frame: every `StateDelta`, in idle or around a command, goes
/// through one domain store by its own domain and revision, and is either claimed by the command
/// that waits for that domain or queued for [`Session::take_events`] (at most
/// [`MAX_QUEUED_EVENTS`]). A `CommandResult`'s
/// status must pair with its disposition (`REJECTED` only with `Rejected`). A duplicate-status
/// result (FND-02 §13.2) for a `CommandId` this session sent is not a violation: it is
/// recorded and returned by `take_duplicate_outcomes`, never followed by a delta. Any other
/// violation, timeout or I/O failure makes the session unusable (`SessionUnusable`): the
/// server's state is no longer known, so nothing further is sent.
///
/// Replacement snapshots are committed atomically and advance the applied sequence; domains
/// absent from a replacement retain their current state.
///
/// Liveness: the session runs no background task. A `LivenessProbe` that arrives while a
/// command's reply is being read is answered with a `LivenessAck` and otherwise ignored, but
/// nothing reads (so nothing answers) while the session is idle. A caller that idles longer than
/// the server's probe cadence (5 s out of combat, three unanswered probes lose control) must
/// call `service_liveness` for the idle time.
///
/// Dropping the session closes the connection.
#[derive(Debug)]
pub struct Session<S> {
    bulletin: Option<String>,
    stream: S,
    deadline: Duration,
    last_received: tokio::time::Instant,
    connection_generation: u64,
    next_command_id: u64,
    last_server_sequence: u64,
    spatial_revision: u64,
    overlay_revision: u64,
    vitals_revision: u64,
    combat_revision: u64,
    /// `Some` exactly when capability 17 is selected and the server sent domain 10.
    combat_state: Option<ActorCombatState>,
    /// Own-actor vitals; `None` until the server sends the `ACTOR_VITALS` domain (it is optional
    /// in the join snapshot while the server cast gate is closed).
    actor_vitals: Option<ActorVitals>,
    world_spatial: WorldSpatialObservation,
    world_object_overlay: Vec<WorldObjectOverlayEntry>,
    join_snapshot: JoinSnapshot,
    unusable: bool,
    last_refusal_detail: Option<item_movement::RefusalDetail>,
    first_command_id: u64,
    duplicates: Vec<DuplicateOutcome>,
    last_probe_id: u64,
    selected_capabilities: Vec<u32>,
    gated_snapshots: Vec<GatedSnapshot>,
    gated_revisions: Vec<(u32, u64)>,
    events: Vec<SessionEvent>,
    /// `Some` exactly when capability 6 is selected.
    entities: Option<WorldEntities>,
    appearance: Option<appearance_state::AppearanceState>,
    chat_revision: u64,
    map_header: Option<oteryn_protocol_oteryn::world_map::MapViewHeader>,
    inventory_revision: u64,
    container_revision: u64,
    inventory: Option<CharacterInventory>,
    open_container: Option<OpenContainer>,
    container_views: Option<container_tree::ContainerViews>,
    /// `Some` exactly when capability 7 is selected.
    chat: Option<ChatLog>,
    vip_presence: Option<vip_presence::VipPresenceStore>,
}

#[cfg(test)]
mod tests;

/// VIP request and result codec used by session-layer clients.
pub use oteryn_protocol_oteryn::vip;

#[cfg(test)]
#[path = "guild_chat_tests.rs"]
mod guild_chat_tests;

/// Task Board wire values exposed to admitted client features.
pub use oteryn_protocol_oteryn::task_board;

/// Server-owned stash wire types exposed to client feature adapters.
pub use oteryn_protocol_oteryn::stash;

/// Existing property and services command codecs for native feature adapters.
pub mod property_services;
pub use entities::WorldEntities;
pub use error::SessionError;
use framing::*;
pub use framing::{read_frame, write_frame};
pub use outcomes::*;

/// Optional server-executed Gem modifier presentation.
pub use oteryn_protocol_oteryn::gem_presentation;

/// Optional authoritative Wheel graph and advisory draft preview.
pub use oteryn_protocol_oteryn::wheel_presentation;
