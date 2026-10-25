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

mod appearance_state;
mod depot;
mod domain_dispatch;
mod gated_command;
mod map_transport;
pub use oteryn_protocol_oteryn::depot::{DepotAction, DepotIntent, DepotOutcome, DepotReply};
mod guild_chat;
pub use appearance_state::{AppearanceItemDictionary, AppearanceUpdate};
use oteryn_protocol_oteryn::actor_spell::{self, SpellCastIntent};
/// Spell types a client names when it casts and draws vitals: re-exported so the client needs no
/// direct `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::actor_spell::{ActorVitals, SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::attack::{self, CAPABILITY_ATTACK_V1};
/// The capability-17 attack types a client sends and draws from, re-exported so it needs no
/// direct `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::attack::{
    ActorCombatState, AttackIntentDisposition, ChaseMode, FightMode, FightModes,
};
pub use oteryn_protocol_oteryn::chat::guild_room::{GuildChatLine, GuildRoomState};
use oteryn_protocol_oteryn::chat::{
    self, CAPABILITY_CHAT_V1, COMMAND_TYPE_CHAT_INTENT, STATE_DOMAIN_CHAT,
};
/// The capability-7 chat types a client sends and draws from, re-exported so it needs no direct
/// `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::chat::{
    ChatDisposition, ChatIntent, ChatLine, ChatRoom, ChatRoomSet, ChatSpeaker, ChatSpeechMode,
    ChatWireError, MAX_CHAT_NAME_BYTES, MAX_CHAT_TEXT_BYTES, MAX_CHAT_WAIT_SECONDS,
};
/// The capability-4 item types a client sends and draws from, re-exported so it needs no direct
/// `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::item_view::{
    CharacterInventory, ItemEntry, ItemHandle, ItemMoveDestination, ItemMoveIntent,
    ItemMoveOutcome, MAX_CHARACTER_INVENTORY_ITEMS, MAX_OPEN_CONTAINER_ENTRIES, OpenContainer,
};
use oteryn_protocol_oteryn::logout;
mod tutorial_notices;
/// The capability-21 logout result a client reads, re-exported so it needs no protocol import.
pub use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::world_interaction;
pub use oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry;
use oteryn_protocol_oteryn::world_object::{self, UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{self, CAPABILITY_PACED_MOVEMENT_V1};
/// The step types a client names when it walks and reads the outcome (ADR-0020 section 1).
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
        self, CAPABILITY_ITEM_VIEW_MOVE_V1, COMMAND_TYPE_ITEM_MOVE_INTENT, ItemViewWireError,
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

/// The capabilities this client implements and advertises by default: 4 `ITEM_VIEW_MOVE_V1`
/// (domains 9 and 11, command 9, the USE item target and the item handle on domain-1 entities),
/// 6 `WORLD_SPATIAL_ENTITIES` (the domain-1 type 2 snapshot and delta with every visible entity), 7 `CHAT_V1` (command 13 and
/// domain 12), 13 `PACED_MOVEMENT_V1` (the step result `TOO_EARLY`) and 17 `ATTACK_V1` (command
/// types 11 and 12, domain 10; requires 6) and 21 `LOGOUT_V1` (command type 23). Add an ID here only
/// together with its routing, and keep the set closed under the registry's `requires`.
pub const CLIENT_SUPPORTED_CAPABILITIES: &[u32] = &[
    CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    CAPABILITY_CHAT_V1,
    CAPABILITY_PACED_MOVEMENT_V1,
    CAPABILITY_ATTACK_V1,
    oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
    logout::CAPABILITY_LOGOUT_V1,
    oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1,
    oteryn_protocol_oteryn::tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1,
    chat::guild_room::CAPABILITY_CHAT_GUILD_ROOM_V1,
    oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1,
];

/// Capabilities supported by the raw command/snapshot/delta routes, including opt-in
/// capabilities absent from `CLIENT_SUPPORTED_CAPABILITIES`. Typed routes stay separate.
const CLIENT_GATED_CAPABILITIES: &[u32] = &[
    oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
    oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1,
    oteryn_protocol_oteryn::tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1,
    oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1,
    CAPABILITY_BESTIARY_CHARMS_V1,
    CAPABILITY_ACHIEVEMENT_NOTICES_V1,
    CAPABILITY_ANALYSER_V1,
    CAPABILITY_CONTAINER_TREE_V1,
];

fn gated_routes() -> impl Iterator<Item = (u32, &'static [u32], &'static [u32])> {
    oteryn_protocol_oteryn::capability_tables::GATED
        .iter()
        .copied()
        .filter(|(capability, _, _)| CLIENT_GATED_CAPABILITIES.contains(capability))
}

fn capability_of_domain(domain_id: u32) -> Option<u32> {
    gated_routes()
        .find(|(_, _, domains)| domains.contains(&domain_id))
        .map(|(capability, _, _)| capability)
}

/// Total `step_retrying` attempts and the first backoff (doubled after each `TOO_EARLY`).
pub const STEP_RETRY_ATTEMPTS: u32 = 4;
pub const STEP_RETRY_INITIAL_BACKOFF: Duration = Duration::from_millis(100);

/// CAP-NEG-1: a resume keeps the original selected set and never widens it. The client calls
/// this with the set it holds and the set a `ServerResumeAccepted` carries; a mismatch means the
/// resume must be abandoned for a fresh admission.
pub fn check_resume_selection(original: &[u32], resumed: &[u32]) -> Result<(), SessionError> {
    let same = original.len() == resumed.len() && resumed.iter().all(|id| original.contains(id));
    if same {
        Ok(())
    } else {
        Err(SessionError::ResumeSelectionChanged)
    }
}

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

/// Everything [`Session::admit`] needs to admit with one grant and read its join snapshot.
#[derive(Debug, Clone, Copy)]
pub struct Admission<'a> {
    /// Verified client artifact Item references for appearance transforms, when loaded.
    pub appearance_items: Option<AppearanceItemDictionary<'a>>,
    /// FND-02 `ClientBootstrap.schema_revision`. Any nonzero value the server accepts.
    pub schema_revision: u32,
    pub character_id: CharacterId,
    /// The signed admission grant, as the wire's `admission_material` bytes.
    pub admission_material: &'a [u8],
    pub client_build_id: &'a str,
    /// FND-02 `supported_capability_id`s this client sends in `ClientBootstrap` (CAP-NEG-1). The
    /// server selects the ones it offers; the session keeps only that selection.
    /// [`CLIENT_SUPPORTED_CAPABILITIES`] is the client's default.
    pub supported_capabilities: &'a [u32],
    /// Bounds each individual frame read of the join sequence (`ServerAccepted`, `SnapshotBegin`,
    /// `SnapshotChunk`, `SnapshotCommit`) and every later per-frame read and write. A stalled
    /// server fails with `SessionError::Timeout` naming the stage, rather than hanging.
    pub deadline: Duration,
}

/// `oteryn-protocol-oteryn`'s own domain codecs (state domains 1 `WORLD_SPATIAL` and 2
/// `WORLD_OBJECT_OVERLAY`).
#[derive(Debug, Clone)]
pub struct JoinSnapshot {
    pub game_session_id: GameSessionId,
    pub world_spatial: WorldSpatialObservation,
    /// The `WORLD_SPATIAL_VISIBILITY` domain revision the snapshot carried: the base every later
    /// domain-1 `StateDelta` must name.
    pub world_spatial_revision: u64,
    pub world_object_overlay: Vec<WorldObjectOverlayEntry>,
    /// The `WORLD_OBJECT_OVERLAY` domain revision the snapshot carried: the base every later
    /// domain-2 `StateDelta` must name.
    pub world_object_overlay_revision: u64,
}

#[derive(Debug)]
pub enum SessionError {
    /// A read or write on the underlying byte stream failed (including an unexpected close).
    Io(io::Error),
    /// A `LivenessProbe`'s `probe_id` did not strictly exceed the last one this session acked
    /// (FND-02 §17: probe ids are monotonic and never reused within a connection generation).
    ProbeIdNotAdvancing {
        last: u64,
        received: u64,
    },
    Protocol(FoundationProtocolError),
    WorldSpatial(world_spatial::WorldSpatialError),
    WorldObject(world_object::WorldObjectError),
    ActorSpell(actor_spell::ActorSpellError),
    /// A chat payload was refused by its codec.
    Chat(ChatWireError),
    /// A capability-17 attack payload was refused by its codec.
    Attack(attack::AttackWireError),
    /// A capability-4 item payload was refused by its codec.
    ItemView(ItemViewWireError),
    /// A capability-21 logout result was refused by its codec.
    Logout(logout::LogoutWireError),
    /// The server closed, or replied with something other than `ServerAccepted`, before
    /// admission completed.
    NotAdmitted(MessageType),
    /// The server refused admission with an N8 code (1100..=1199) in one `ProtocolError`
    /// (ARCH-LOGIN-FIRST-PACKETS-V1 §1.2); `admission_refusal_class` gives its public class.
    AdmissionRefused {
        code: u32,
    },
    /// A later frame's message type did not match what the join sequence expects next.
    UnexpectedMessage {
        expected: MessageType,
        actual: MessageType,
    },
    /// A `SnapshotBegin`/`SnapshotChunk`/`SnapshotCommit` frame's own envelope
    /// `connection_generation` did not match the admitted session's (from `ServerAccepted`).
    ConnectionGenerationMismatch {
        expected: u64,
        actual: u64,
    },
    /// `SnapshotChunk`'s or `SnapshotCommit`'s `snapshot_id` did not match `SnapshotBegin`'s —
    /// the three frames of one join-snapshot transfer no longer correlate.
    SnapshotIdMismatch {
        expected: u64,
        actual: u64,
    },
    /// A join-snapshot domain entry named `PROTOCOL_OTERYN_V1_REGISTRY.json` state domain 1
    /// (`WORLD_SPATIAL_VISIBILITY`) or 2 (`WORLD_OBJECT_OVERLAY`) with a `snapshot_type` other
    /// than the one registered snapshot type (1) for that domain.
    UnregisteredSnapshotType {
        domain_id: u32,
        snapshot_type: u32,
    },
    /// A `SnapshotChunk`'s `chunk_index` did not equal the index this client expected next
    /// (zero-based, in order — `SnapshotBegin`'s declared `chunk_count`, FND-02 §16).
    ChunkIndexMismatch {
        expected: u32,
        actual: u32,
    },
    /// The summed `data` bytes of every received `SnapshotChunk` did not equal `SnapshotBegin`'s
    /// declared `total_encoded_bytes`.
    AssembledLengthMismatch {
        expected: u64,
        actual: u64,
    },
    /// The join snapshot did not carry a domain this client needed.
    MissingDomain(u32),
    /// One frame read/write did not complete within `Admission::deadline`.
    Timeout(&'static str),
    /// A `step`/`use_object` was attempted on a session that an earlier command exchange failed
    /// (protocol violation, timeout or I/O): the peer's state is no longer trusted, so nothing
    /// further is sent on it.
    SessionUnusable,
    /// The next `CommandId` would overflow `u64`.
    CommandIdExhausted,
    /// A `CommandResult` named a `command_id` other than the one just sent.
    CommandIdMismatch {
        expected: u64,
        actual: u64,
    },
    /// A server-sequenced frame (`CommandResult`/`StateDelta`) did not carry exactly the
    /// previous applied `server_sequence` plus one (FND-02 §14: gapless, in order).
    ServerSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    /// A `StateDelta` named a domain other than the one the disposition promised.
    UnexpectedDomain {
        expected: u32,
        actual: u32,
    },
    /// A `StateDelta` named a `delta_type` other than the one registered for its domain (1 for
    /// both `WORLD_SPATIAL` and `WORLD_OBJECT_OVERLAY` in `PROTOCOL_OTERYN_V1_REGISTRY.json`).
    UnregisteredDeltaType {
        domain_id: u32,
        delta_type: u32,
    },
    /// A `StateDelta`'s `base_revision` was not the domain revision this client last applied.
    StateRevisionMismatch {
        domain_id: u32,
        expected_base: u64,
        actual_base: u64,
    },
    /// A delta's `content_generation` was not the one the join snapshot loaded.
    ContentGenerationMismatch {
        domain_id: u32,
    },
    /// A `WORLD_OBJECT_OVERLAY` delta's entry carried a `revision` other than the delta's own
    /// `new_revision`; nothing from it is applied.
    OverlayEntryRevisionMismatch {
        new_revision: u64,
        entry_revision: u64,
    },
    /// A `CommandResult`'s status did not pair with its typed disposition (the server sends
    /// `ACCEPTED` for every disposition except `REJECTED`, which it sends with `REJECTED`), or
    /// it carried a duplicate status for the command just sent (a fresh `CommandId` cannot be a
    /// duplicate).
    InconsistentCommandResult {
        command_id: u64,
        status: CommandStatus,
    },
    /// A duplicate-status `CommandResult` (FND-02 §13.2) named a `CommandId` this session never
    /// sent (below its first, or not yet sent).
    DuplicateForUnsentCommand {
        command_id: u64,
    },
    /// `ServerAccepted` selected a capability this client did not advertise.
    CapabilityNotRequested(u32),
    /// A resume carried a selected set different from the session's original one.
    ResumeSelectionChanged,
    /// A gated command or domain of a capability this session did not select, or one the
    /// registry does not assign to that capability.
    CapabilityNotSelected {
        capability: u32,
    },
    /// The server sent a state domain whose capability the session did not select.
    UnselectedDomain {
        domain_id: u32,
    },
    /// A pushed `StateDelta` named a domain this session keeps no store for (neither domain 1, 2
    /// nor 3), so applying it would silently drop a server revision.
    UnsupportedPushedDomain {
        domain_id: u32,
    },
    /// A capability-6 entity delta contradicts the stored entities: an `enter` of an identity
    /// already stored, an `update` or `leave` of one that is not, more than
    /// `MAX_SNAPSHOT_ENTITIES` stored, or an own actor that is gone or not at the header's
    /// `actor_position`. The session is unusable.
    EntityStoreInconsistent {
        reason: &'static str,
    },
    /// More than [`MAX_QUEUED_EVENTS`] applied deltas were waiting for `take_events`; the session
    /// fails closed rather than growing the queue without bound.
    EventQueueOverflow {
        limit: usize,
    },
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "transport I/O failed: {error}"),
            Self::Protocol(error) => write!(formatter, "FND-02 protocol error: {error}"),
            Self::WorldSpatial(error) => {
                write!(formatter, "WORLD_SPATIAL decode failed: {error:?}")
            }
            Self::WorldObject(error) => {
                write!(formatter, "WORLD_OBJECT_OVERLAY decode failed: {error:?}")
            }
            Self::ActorSpell(error) => {
                write!(
                    formatter,
                    "ACTOR_SPELL/ACTOR_VITALS decode failed: {error:?}"
                )
            }
            Self::Chat(error) => write!(formatter, "CHAT payload refused: {error:?}"),
            Self::Attack(error) => write!(formatter, "ATTACK payload refused: {error:?}"),
            Self::ItemView(error) => write!(formatter, "ITEM payload refused: {error:?}"),
            Self::Logout(error) => write!(formatter, "LOGOUT payload refused: {error:?}"),
            Self::NotAdmitted(message_type) => {
                write!(formatter, "admission refused: server sent {message_type:?}")
            }
            Self::AdmissionRefused { code } => {
                write!(formatter, "admission refused with code {code}")
            }
            Self::UnexpectedMessage { expected, actual } => write!(
                formatter,
                "expected {expected:?} next, server sent {actual:?}"
            ),
            Self::ConnectionGenerationMismatch { expected, actual } => write!(
                formatter,
                "connection generation mismatch: admitted session is {expected}, frame carried {actual}"
            ),
            Self::SnapshotIdMismatch { expected, actual } => write!(
                formatter,
                "snapshot id mismatch: SnapshotBegin was {expected}, frame carried {actual}"
            ),
            Self::UnregisteredSnapshotType {
                domain_id,
                snapshot_type,
            } => write!(
                formatter,
                "domain {domain_id} carried unregistered snapshot_type {snapshot_type}"
            ),
            Self::ChunkIndexMismatch { expected, actual } => write!(
                formatter,
                "chunk index mismatch: expected {expected} next, chunk carried {actual}"
            ),
            Self::AssembledLengthMismatch { expected, actual } => write!(
                formatter,
                "assembled snapshot length mismatch: SnapshotBegin declared {expected} bytes, chunks totaled {actual}"
            ),
            Self::MissingDomain(domain_id) => {
                write!(formatter, "join snapshot did not carry domain {domain_id}")
            }
            Self::Timeout(stage) => write!(formatter, "timed out waiting for {stage}"),
            Self::SessionUnusable => write!(
                formatter,
                "session is unusable after an earlier failed command exchange"
            ),
            Self::CommandIdExhausted => write!(formatter, "CommandId space exhausted"),
            Self::ProbeIdNotAdvancing { last, received } => write!(
                formatter,
                "liveness probe id {received} does not advance past {last}"
            ),
            Self::CommandIdMismatch { expected, actual } => write!(
                formatter,
                "command id mismatch: sent {expected}, CommandResult carried {actual}"
            ),
            Self::ServerSequenceMismatch { expected, actual } => write!(
                formatter,
                "server sequence mismatch: expected {expected} next, frame carried {actual}"
            ),
            Self::UnexpectedDomain { expected, actual } => write!(
                formatter,
                "unexpected state domain: expected a domain {expected} delta, got domain {actual}"
            ),
            Self::UnregisteredDeltaType {
                domain_id,
                delta_type,
            } => write!(
                formatter,
                "domain {domain_id} delta carried unregistered delta_type {delta_type}"
            ),
            Self::StateRevisionMismatch {
                domain_id,
                expected_base,
                actual_base,
            } => write!(
                formatter,
                "domain {domain_id} delta base revision mismatch: applied {expected_base}, delta based on {actual_base}"
            ),
            Self::ContentGenerationMismatch { domain_id } => write!(
                formatter,
                "domain {domain_id} delta content generation differs from the loaded one"
            ),
            Self::OverlayEntryRevisionMismatch {
                new_revision,
                entry_revision,
            } => write!(
                formatter,
                "overlay delta new_revision {new_revision} differs from its entry revision {entry_revision}"
            ),
            Self::InconsistentCommandResult { command_id, status } => write!(
                formatter,
                "command {command_id} result status {status:?} is inconsistent with its disposition"
            ),
            Self::DuplicateForUnsentCommand { command_id } => write!(
                formatter,
                "duplicate result for command {command_id}, which this session never sent"
            ),
            Self::CapabilityNotRequested(capability) => write!(
                formatter,
                "server selected capability {capability}, which this client did not advertise"
            ),
            Self::ResumeSelectionChanged => {
                write!(formatter, "resume changed the selected capability set")
            }
            Self::CapabilityNotSelected { capability } => {
                write!(formatter, "capability {capability} is not selected")
            }
            Self::UnselectedDomain { domain_id } => write!(
                formatter,
                "server sent domain {domain_id} of an unselected capability"
            ),
            Self::UnsupportedPushedDomain { domain_id } => write!(
                formatter,
                "server pushed a delta of domain {domain_id}, which this session keeps no store for"
            ),
            Self::EntityStoreInconsistent { reason } => {
                write!(
                    formatter,
                    "entity delta contradicts the stored entities: {reason}"
                )
            }
            Self::EventQueueOverflow { limit } => write!(
                formatter,
                "more than {limit} pushed deltas were left undrained"
            ),
        }
    }
}

impl StdError for SessionError {}

impl From<io::Error> for SessionError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<FoundationProtocolError> for SessionError {
    fn from(error: FoundationProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<world_spatial::WorldSpatialError> for SessionError {
    fn from(error: world_spatial::WorldSpatialError) -> Self {
        Self::WorldSpatial(error)
    }
}

impl From<world_object::WorldObjectError> for SessionError {
    fn from(error: world_object::WorldObjectError) -> Self {
        Self::WorldObject(error)
    }
}

impl From<actor_spell::ActorSpellError> for SessionError {
    fn from(error: actor_spell::ActorSpellError) -> Self {
        Self::ActorSpell(error)
    }
}

impl From<ChatWireError> for SessionError {
    fn from(error: ChatWireError) -> Self {
        Self::Chat(error)
    }
}

/// One delta a command's outcome carried, already validated and applied to the session's state:
/// its own `server_sequence` and the domain revisions it moved between.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedDelta<T> {
    pub server_sequence: u64,
    pub base_revision: u64,
    pub new_revision: u64,
    pub value: T,
}

/// One command's decoded outcome: the `CommandResult` (its `command_id`, `status`, the typed
/// `disposition` and the `server_sequence` it arrived at) plus, for `Moved` (domain 1) and `Cast`
/// (domain 3), the first delta of that domain applied after the result, by revision alone: the
/// frames carry no command id, so whatever caused it, it is the one reported. A `Committed` use
/// names no domain and returns at its result, so `world_object_overlay_delta` is always `None`;
/// its effects arrive through [`Session::take_events`]. Every other disposition carries no delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutcome<D> {
    pub command_id: u64,
    pub status: CommandStatus,
    pub disposition: D,
    pub result_server_sequence: u64,
    pub world_spatial_delta: Option<AppliedDelta<WorldSpatialObservation>>,
    pub world_object_overlay_delta: Option<AppliedDelta<WorldObjectOverlayEntry>>,
    /// The gate message id (1 to 127) of a `SEALED` use result under capability 11, so a client
    /// can show the gate's own reason; `None` for every other disposition and command.
    pub gate_message_id: Option<u8>,
}

/// Outcome of `Session::step` (FIRST-CONTROL-WIRE-V1, command type 1).
pub type StepOutcome = CommandOutcome<StepDisposition>;
/// Outcome of `Session::use_object` (USE-WIRE-V1, command type 2).
pub type UseOutcome = CommandOutcome<UseDisposition>;

/// Outcome of `Session::cast_spell` (SPELL wire contract, command type 3). A `Cast` is followed by
/// exactly one `ACTOR_VITALS` delta (the cast pays its cost); every other disposition, including
/// `Rejected` while the server gate is closed, changes nothing and carries no delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    pub disposition: SpellCastDisposition,
    pub result_server_sequence: u64,
    pub actor_vitals_delta: Option<AppliedDelta<ActorVitals>>,
}

/// The decoded `CommandResult` of a capability-21 `LOGOUT_INTENT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogoutOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    /// `None` is a `REJECTED` with an empty payload: the server refused the command before its
    /// logout authority (a capability or rate-limit refusal).
    pub result: Option<LogoutResult>,
    pub result_server_sequence: u64,
}

/// Outcome of `Session::attack_target` and `Session::set_fight_modes` (ATTACK-0, command types
/// 11 and 12). The resulting `ACTOR_COMBAT_STATE` delta, if any, goes through the event queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    pub disposition: AttackIntentDisposition,
    pub result_server_sequence: u64,
}

/// Most applied-but-undrained pushed deltas [`Session::take_events`] holds. The next one fails the
/// session closed with [`SessionError::EventQueueOverflow`].
pub const MAX_QUEUED_EVENTS: usize = 256;

/// A server-sequenced delta that no command outcome claimed, already validated and applied to the
/// session's domain store (FND-02 §14, §15), in the order it was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    Appearance(AppliedDelta<AppearanceUpdate>),
    TutorialNotice(AppliedDelta<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice>),
    /// Domain 1 `WORLD_SPATIAL_VISIBILITY`.
    WorldSpatial(AppliedDelta<WorldSpatialObservation>),
    /// Domain 1 with capability 6 selected: the entities that entered, updated and left, and the
    /// own-actor position. [`Session::world_entities`] already holds the result.
    WorldSpatialEntities(AppliedDelta<WorldSpatialEntitiesDelta>),
    /// Domain 2 `WORLD_OBJECT_OVERLAY`.
    WorldObjectOverlay(AppliedDelta<WorldObjectOverlayEntry>),
    /// Domain 17: validated bundle map changes, bound to the admitted content and reset epoch.
    WorldMap(AppliedDelta<oteryn_protocol_oteryn::world_map::WorldMapViewDelta>),
    /// Domain 3 `ACTOR_VITALS`.
    ActorVitals(AppliedDelta<ActorVitals>),
    /// Domain 10 `ACTOR_COMBAT_STATE`: the own actor's attack target, modes and in-fight flag,
    /// replaced whole. [`Session::combat_state`] already holds it.
    ActorCombatState(AppliedDelta<ActorCombatState>),
    /// Domain 12 `CHAT`, delta type 1: one line. [`Session::chat_log`] already holds it.
    ChatLine(AppliedDelta<ChatLine>),
    /// Domain 12 `CHAT`, delta type 2: the open-room set, replaced whole.
    ChatRooms(AppliedDelta<ChatRoomSet>),
    /// Domain 9 `CHARACTER_INVENTORY`: the inventory, replaced whole.
    /// [`Session::inventory`] already holds it.
    Inventory(AppliedDelta<CharacterInventory>),
    /// Domain 11 `OPEN_CONTAINER`: the open corpse, replaced whole (the default means closed).
    /// [`Session::open_container`] already holds it.
    OpenContainer(AppliedDelta<OpenContainer>),
}

impl SessionEvent {
    #[must_use]
    pub const fn domain_id(&self) -> u32 {
        match self {
            Self::Appearance(_) => oteryn_protocol_oteryn::appearance::STATE_DOMAIN_APPEARANCE,
            Self::TutorialNotice(_) => 23,
            Self::WorldSpatial(_) | Self::WorldSpatialEntities(_) => {
                world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
            }
            Self::WorldObjectOverlay(_) => world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            Self::WorldMap(_) => oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
            Self::ActorVitals(_) => actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            Self::ActorCombatState(_) => attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
            Self::ChatLine(_) | Self::ChatRooms(_) => STATE_DOMAIN_CHAT,
            Self::Inventory(_) => STATE_DOMAIN_CHARACTER_INVENTORY,
            Self::OpenContainer(_) => STATE_DOMAIN_OPEN_CONTAINER,
        }
    }
}

/// Most chat lines [`ChatLog`] keeps: the oldest is dropped past it (the `CHAT0-RL-11` egress
/// queue bound sets the size).
pub const MAX_CHAT_LOG_LINES: usize = 64;

/// The chat state of a session that selected capability 7: the open rooms and a ring of the last
/// [`MAX_CHAT_LOG_LINES`] lines, oldest first. `Dropped` markers are lines like any other.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatLog {
    rooms: ChatRoomSet,
    guild: GuildRoomState,
    lines: VecDeque<ChatLine>,
}

impl ChatLog {
    #[must_use]
    pub const fn rooms(&self) -> ChatRoomSet {
        self.rooms
    }

    /// The kept lines, oldest first.
    pub fn lines(&self) -> impl ExactSizeIterator<Item = &ChatLine> {
        self.lines.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    fn push(&mut self, line: ChatLine) {
        if self.lines.len() >= MAX_CHAT_LOG_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }
}

/// Outcome of `Session::chat` (CHAT-1b, command type 13). The command returns at its result: the
/// line it produced (the sender's own echo included) and a room change arrive as pushed deltas,
/// through [`Session::take_events`] and [`Session::chat_log`]. A `Muted` or `Exhausted` result
/// carries `wait_seconds` and changes no local state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    pub disposition: ChatDisposition,
    pub wait_seconds: u32,
    pub result_server_sequence: u64,
}

/// Outcome of `Session::move_item` (ITEM-MOVE-WIRE-1, command type 9). The command returns at its
/// result: a `Moved` result's domain 9 and domain 11 deltas arrive as pushed deltas, through
/// [`Session::take_events`], [`Session::inventory`] and [`Session::open_container`]. Every other
/// outcome (`Stale` included) changes no local state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMoveOutcomeResult {
    pub command_id: u64,
    pub status: CommandStatus,
    pub outcome: ItemMoveOutcome,
    pub result_server_sequence: u64,
}

/// The visible entities of a session that selected capability 6, keyed by `EntityRef` (identity
/// and generation). Identities are unique, at most [`MAX_SNAPSHOT_ENTITIES`] are stored, and the
/// own actor is always a stored `Player` at the own-actor position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntities {
    own_identity: [u8; world_spatial_entities::ENTITY_IDENTITY_BYTES],
    entities: BTreeMap<EntityRef, WorldSpatialEntity>,
}

impl WorldEntities {
    /// The own actor's identity.
    #[must_use]
    pub const fn own_identity(&self) -> &[u8; world_spatial_entities::ENTITY_IDENTITY_BYTES] {
        &self.own_identity
    }

    #[must_use]
    pub fn get(&self, entity: &EntityRef) -> Option<&WorldSpatialEntity> {
        self.entities.get(entity)
    }

    /// Every stored entity, ordered by `EntityRef`.
    pub fn iter(&self) -> impl Iterator<Item = &WorldSpatialEntity> {
        self.entities.values()
    }

    /// Every stored entity but the own actor, ordered by `EntityRef`.
    pub fn others(&self) -> impl Iterator<Item = &WorldSpatialEntity> {
        self.entities
            .values()
            .filter(|entity| entity.entity.identity != self.own_identity)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    fn from_snapshot(snapshot: &world_spatial_entities::WorldSpatialEntitiesSnapshot) -> Self {
        Self {
            own_identity: snapshot.own_identity,
            entities: snapshot
                .entities
                .iter()
                .map(|entity| (entity.entity, *entity))
                .collect(),
        }
    }

    fn has_identity(&self, identity: &[u8; world_spatial_entities::ENTITY_IDENTITY_BYTES]) -> bool {
        self.entities
            .keys()
            .any(|stored| stored.identity == *identity)
    }

    /// The store after `delta`, or the reason the delta contradicts it. Nothing changes here.
    fn after(&self, delta: &WorldSpatialEntitiesDelta) -> Result<Self, SessionError> {
        let inconsistent = |reason| SessionError::EntityStoreInconsistent { reason };
        let mut next = self.clone();
        for reference in &delta.leave {
            if next.entities.remove(reference).is_none() {
                return Err(inconsistent("leave of an entity that is not stored"));
            }
        }
        for entity in &delta.update {
            match next.entities.get_mut(&entity.entity) {
                Some(stored) => *stored = *entity,
                None => return Err(inconsistent("update of an entity that is not stored")),
            }
        }
        for entity in &delta.enter {
            if next.has_identity(&entity.entity.identity) {
                return Err(inconsistent("enter of an identity that is already stored"));
            }
            next.entities.insert(entity.entity, *entity);
        }
        if next.entities.len() > MAX_SNAPSHOT_ENTITIES {
            return Err(inconsistent("more entities than the snapshot bound"));
        }
        let own_at_header = next.entities.values().any(|entity| {
            entity.kind == EntityKind::Player
                && entity.entity.identity == next.own_identity
                && entity.position == delta.actor_position
        });
        if !own_at_header {
            return Err(inconsistent(
                "own actor is gone or not at the header position",
            ));
        }
        Ok(next)
    }
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
/// Liveness: the session runs no background task. A `LivenessProbe` that arrives while a
/// command's reply is being read is answered with a `LivenessAck` and otherwise ignored, but
/// nothing reads (so nothing answers) while the session is idle. A caller that idles longer than
/// the server's probe cadence (5 s out of combat, three unanswered probes lose control) must
/// call `service_liveness` for the idle time.
///
/// Dropping the session closes the connection.
#[derive(Debug)]
pub struct Session<S> {
    stream: S,
    deadline: Duration,
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
    /// `Some` exactly when capability 7 is selected.
    chat: Option<ChatLog>,
}

/// A duplicate-status `CommandResult` (FND-02 §13.2) for an earlier `CommandId` of this session:
/// the server did not execute that command again. `payload` is the replayed typed result for
/// `DuplicateReplay` (undecoded; it belongs to the original command's type) and empty for
/// `DuplicateOutcomeExpired`. It promises no delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateOutcome {
    pub command_id: u64,
    pub status: CommandStatus,
    pub server_sequence: u64,
    pub payload: Vec<u8>,
}

/// Most duplicate results retained between `take_duplicate_outcomes` calls; more fails closed.
const MAX_RETAINED_DUPLICATES: usize = 64;

impl<S: SessionStream> Session<S> {
    /// Admits over the established `stream` with `admission` and decodes the join snapshot the
    /// server sends right after `ServerAccepted` (`SnapshotBegin`, `SnapshotChunk`,
    /// `SnapshotCommit` - FND-02 section 16), keeping the admitted connection: the returned
    /// session carries the admitted `connection_generation`, the first `CommandId`
    /// (`ServerAccepted.next_command_id`), the last applied `server_sequence`
    /// (`SnapshotBegin.target_server_sequence`) and both domain revisions the join snapshot
    /// established, which `step`/`use_object` then extend under the same envelope, sequence and
    /// revision discipline.
    ///
    /// Every inbound frame is checked with `WireEnvelopeView::validate` (direction, phase,
    /// sequencing and - pre- vs post-admission - the envelope `connection_generation` presence
    /// rule) before its payload is consumed at all, then correlated to `SnapshotBegin`'s full
    /// declaration and the admitted session: `SnapshotBegin`/`SnapshotChunk`/`SnapshotCommit`
    /// must each carry the admitted `connection_generation`; `SnapshotChunk`'s and
    /// `SnapshotCommit`'s `snapshot_id` must equal `SnapshotBegin`'s; exactly `SnapshotBegin`'s
    /// declared `chunk_count` chunks are read, each with the expected `chunk_index` in order; and
    /// their concatenated `data` bytes must equal `SnapshotBegin`'s declared
    /// `total_encoded_bytes`. The assembled `SnapshotBody` is protobuf-decoded exactly once, only
    /// after every chunk and the matching `SnapshotCommit` have validated - never per chunk, since
    /// a multi-chunk transfer may split a body field at any byte offset. Every frame read is
    /// bounded by `admission.deadline`.
    pub async fn admit(mut stream: S, admission: Admission<'_>) -> Result<Self, SessionError> {
        let bootstrap = encode_client_bootstrap(&ClientBootstrapValue {
            schema_revision: admission.schema_revision,
            character_id: admission.character_id,
            admission_material: admission.admission_material,
            client_build_id: admission.client_build_id,
            supported_capabilities: admission.supported_capabilities,
        })?;
        write_frame(&mut stream, &bootstrap).await?;

        let accepted = bounded(
            admission.deadline,
            "ServerAccepted",
            read_frame(&mut stream),
        )
        .await?;
        let accepted_envelope = decode_wire_envelope(&accepted)?;
        // Pre-admission server traffic: direction, phase (Bootstrap), sequencing (unsequenced) and a
        // zero envelope `connection_generation` (FND-02 §8/§11/§12/§14) are checked before this
        // frame's payload is consumed at all.
        accepted_envelope.validate(Direction::ServerToClient, false)?;
        if accepted_envelope.message_type() == MessageType::ProtocolError
            && let Ok(refusal) = decode_protocol_error(accepted_envelope.payload())
            && ADMISSION_REFUSAL_CODES.contains(&refusal.error_code)
            // N8 fixes the refusal shape; any other disposition or correlation fails closed below.
            && refusal.disposition == ProtocolDisposition::TransportFatal
            && refusal.related_command_id == 0
            && refusal.expected_command_id == 0
            && refusal.expected_server_sequence == 0
        {
            return Err(SessionError::AdmissionRefused {
                code: refusal.error_code,
            });
        }
        if accepted_envelope.message_type() != MessageType::ServerAccepted {
            return Err(SessionError::NotAdmitted(accepted_envelope.message_type()));
        }
        let accepted_fields = decode_server_accepted(accepted_envelope.payload())?;
        let session_generation = accepted_fields.connection_generation;
        // CAP-NEG-1: the server selects only from the advertised set; anything else is a violation.
        if let Some(&capability) = accepted_fields
            .selected_capabilities
            .iter()
            .find(|id| !admission.supported_capabilities.contains(id))
        {
            return Err(SessionError::CapabilityNotRequested(capability));
        }
        let selected_capabilities = accepted_fields.selected_capabilities.clone();

        // SnapshotBegin's full declaration: every chunk read below, and the commit that follows
        // them, is checked against it before being trusted.
        let begin_frame =
            bounded(admission.deadline, "SnapshotBegin", read_frame(&mut stream)).await?;
        let begin_envelope = decode_wire_envelope(&begin_frame)?;
        // Post-admission server traffic: direction, phase, sequencing (unsequenced — FND-02 §14
        // snapshot transfer-control frames carry no `server_sequence`) and a nonzero envelope
        // `connection_generation` are checked before this frame's payload is consumed.
        begin_envelope.validate(Direction::ServerToClient, true)?;
        if begin_envelope.message_type() != MessageType::SnapshotBegin {
            return Err(SessionError::UnexpectedMessage {
                expected: MessageType::SnapshotBegin,
                actual: begin_envelope.message_type(),
            });
        }
        if begin_envelope.connection_generation() != session_generation {
            return Err(SessionError::ConnectionGenerationMismatch {
                expected: session_generation,
                actual: begin_envelope.connection_generation(),
            });
        }
        let begin = decode_snapshot_begin(begin_envelope.payload())?;

        // Exactly `begin.chunk_count` `SnapshotChunk` frames, strictly in order: a short transfer
        // (the server stops early) surfaces here as `UnexpectedMessage` (the next frame is
        // `SnapshotCommit` instead) or as a bounded `Timeout`/`Io` (the connection stalls or closes).
        // A multi-chunk transfer may split a `SnapshotBody` protobuf field at any byte offset, not
        // necessarily a field boundary (FND-02 §16), so every chunk's raw `data` is only ever
        // concatenated here, in `chunk_index` order, into one owned buffer — never decoded on its
        // own. `assembled_bytes` is checked against `SnapshotBegin`'s declared
        // `total_encoded_bytes` with checked arithmetic, and rejected, *before* the chunk's bytes are
        // appended (before growing the assembly), so an over-declaring or over-sending peer cannot
        // grow the buffer past the declared bound first and get rejected only afterwards.
        let mut assembled_body: Vec<u8> = Vec::new();
        let mut assembled_bytes: u64 = 0;
        for expected_index in 0..begin.chunk_count {
            let chunk_frame =
                bounded(admission.deadline, "SnapshotChunk", read_frame(&mut stream)).await?;
            let chunk_envelope = decode_wire_envelope(&chunk_frame)?;
            // Post-admission server traffic, same as `SnapshotBegin` above.
            chunk_envelope.validate(Direction::ServerToClient, true)?;
            if chunk_envelope.message_type() != MessageType::SnapshotChunk {
                return Err(SessionError::UnexpectedMessage {
                    expected: MessageType::SnapshotChunk,
                    actual: chunk_envelope.message_type(),
                });
            }
            if chunk_envelope.connection_generation() != session_generation {
                return Err(SessionError::ConnectionGenerationMismatch {
                    expected: session_generation,
                    actual: chunk_envelope.connection_generation(),
                });
            }
            let chunk_snapshot_id = decode_snapshot_id(chunk_envelope.payload())?;
            if chunk_snapshot_id != begin.snapshot_id {
                return Err(SessionError::SnapshotIdMismatch {
                    expected: begin.snapshot_id,
                    actual: chunk_snapshot_id,
                });
            }
            let (chunk_index, chunk_data) =
                decode_snapshot_chunk_framing(chunk_envelope.payload())?;
            if chunk_index != expected_index {
                return Err(SessionError::ChunkIndexMismatch {
                    expected: expected_index,
                    actual: chunk_index,
                });
            }
            let chunk_len = u64::try_from(chunk_data.len()).unwrap_or(u64::MAX);
            assembled_bytes = assembled_bytes
                .checked_add(chunk_len)
                .filter(|total| *total <= begin.total_encoded_bytes)
                .ok_or(SessionError::AssembledLengthMismatch {
                    expected: begin.total_encoded_bytes,
                    actual: assembled_bytes.saturating_add(chunk_len),
                })?;
            assembled_body.extend_from_slice(chunk_data);
        }
        if assembled_bytes != begin.total_encoded_bytes {
            return Err(SessionError::AssembledLengthMismatch {
                expected: begin.total_encoded_bytes,
                actual: assembled_bytes,
            });
        }

        // The commit is accepted only once every declared chunk arrived, in order, and the
        // assembled length matched.
        let commit_snapshot_id = read_snapshot_marker(
            &mut stream,
            admission.deadline,
            "SnapshotCommit",
            MessageType::SnapshotCommit,
            session_generation,
        )
        .await?;
        if commit_snapshot_id != begin.snapshot_id {
            return Err(SessionError::SnapshotIdMismatch {
                expected: begin.snapshot_id,
                actual: commit_snapshot_id,
            });
        }

        // Only now — every declared chunk arrived in order, the assembled length matched, and the
        // matching `SnapshotCommit` validated — is the assembled `SnapshotBody` decoded, exactly
        // once (FND-02 §16: "protobuf decode occurs only after a full bounded body is assembled" and
        // "apply is atomic only after all chunks and matching SnapshotCommit validate").
        let entities_selected = selected_capabilities.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES);
        let mut world_spatial_observation = None;
        let mut entities = None;
        let mut world_object_overlay = None;
        let mut actor_vitals = None;
        let mut combat_state = None;
        let mut combat_revision = 0;
        let attack_selected = selected_capabilities.contains(&CAPABILITY_ATTACK_V1);
        let (mut spatial_revision, mut overlay_revision, mut vitals_revision) = (0, 0, 0);
        guild_chat::check_selection(&selected_capabilities)?;
        let chat_selected = selected_capabilities.contains(&CAPABILITY_CHAT_V1);
        let mut chat = chat_selected.then(ChatLog::default);
        let mut chat_revision = 0;
        let item_selected = selected_capabilities.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1);
        let mut inventory = item_selected.then(CharacterInventory::default);
        let mut open_container = item_selected.then(OpenContainer::default);
        let (mut inventory_revision, mut container_revision) = (0, 0);
        let mut gated_snapshots = Vec::new();
        let mut gated_revisions = Vec::new();
        for domain in decode_snapshot_body(&assembled_body)? {
            match (domain.domain_id, domain.snapshot_type) {
                (
                    world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                ) if entities_selected => {
                    let snapshot = if item_selected {
                        world_spatial_entities::decode_world_spatial_entities_snapshot_with_item_handles(
                            domain.payload,
                        )?
                    } else {
                        world_spatial_entities::decode_world_spatial_entities_snapshot(
                            domain.payload,
                        )?
                    };
                    world_spatial_observation = Some(WorldSpatialObservation {
                        content_generation: snapshot.content_generation,
                        actor_position: snapshot.actor_position,
                    });
                    entities = Some(WorldEntities::from_snapshot(&snapshot));
                    spatial_revision = domain.revision;
                }
                (
                    world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    world_spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                ) if !entities_selected => {
                    world_spatial_observation =
                        Some(world_spatial::decode_world_spatial(domain.payload)?);
                    spatial_revision = domain.revision;
                }
                (
                    world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                ) => {
                    world_object_overlay = Some(
                        world_object::decode_world_object_overlay_snapshot(domain.payload)?,
                    );
                    overlay_revision = domain.revision;
                }
                (
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                ) => {
                    actor_vitals = Some(actor_spell::decode_actor_vitals(domain.payload)?);
                    vitals_revision = domain.revision;
                }
                (
                    attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                    attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
                ) if attack_selected => {
                    combat_state = Some(
                        attack::decode_actor_combat_state(domain.payload)
                            .map_err(SessionError::Attack)?,
                    );
                    combat_revision = domain.revision;
                }
                (attack::STATE_DOMAIN_ACTOR_COMBAT_STATE, snapshot_type) if attack_selected => {
                    return Err(SessionError::UnregisteredSnapshotType {
                        domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                        snapshot_type,
                    });
                }
                (attack::STATE_DOMAIN_ACTOR_COMBAT_STATE, _) => {
                    return Err(SessionError::UnselectedDomain {
                        domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                    });
                }
                (
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                ) if item_selected => {
                    inventory = Some(
                        item_view::decode_character_inventory(domain.payload)
                            .map_err(SessionError::ItemView)?,
                    );
                    inventory_revision = domain.revision;
                }
                (STATE_DOMAIN_OPEN_CONTAINER, item_view::SNAPSHOT_TYPE_OPEN_CONTAINER_V1)
                    if item_selected =>
                {
                    open_container = Some(
                        item_view::decode_open_container(domain.payload)
                            .map_err(SessionError::ItemView)?,
                    );
                    container_revision = domain.revision;
                }
                (
                    domain_id @ (STATE_DOMAIN_CHARACTER_INVENTORY | STATE_DOMAIN_OPEN_CONTAINER),
                    snapshot_type,
                ) if item_selected => {
                    return Err(SessionError::UnregisteredSnapshotType {
                        domain_id,
                        snapshot_type,
                    });
                }
                (
                    domain_id @ (STATE_DOMAIN_CHARACTER_INVENTORY | STATE_DOMAIN_OPEN_CONTAINER),
                    _,
                ) => {
                    return Err(SessionError::UnselectedDomain { domain_id });
                }
                (STATE_DOMAIN_CHAT, snapshot_type) if chat_selected => {
                    chat = Some(guild_chat::load_snapshot(
                        snapshot_type,
                        domain.payload,
                        &selected_capabilities,
                    )?);
                    chat_revision = domain.revision;
                }
                (STATE_DOMAIN_CHAT, _) => {
                    return Err(SessionError::UnselectedDomain {
                        domain_id: STATE_DOMAIN_CHAT,
                    });
                }
                // PROTOCOL_OTERYN_V1_REGISTRY.json registers exactly one snapshot_type (1) for
                // domains 1, 2 and 3; anything else naming one of those domains is a registry
                // violation, not a domain this client merely doesn't need.
                (
                    domain_id @ world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    snapshot_type,
                )
                | (domain_id @ world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY, snapshot_type)
                | (domain_id @ actor_spell::STATE_DOMAIN_ACTOR_VITALS, snapshot_type) => {
                    return Err(SessionError::UnregisteredSnapshotType {
                        domain_id,
                        snapshot_type,
                    });
                }
                // A gated domain is kept raw for its capability's own packet. Unselected is a server
                // violation; an unregistered domain is ignored.
                (domain_id, snapshot_type) => match capability_of_domain(domain_id) {
                    Some(capability) if selected_capabilities.contains(&capability) => {
                        gated_revisions.push((domain_id, domain.revision));
                        gated_snapshots.push(GatedSnapshot {
                            domain_id,
                            snapshot_type,
                            revision: domain.revision,
                            payload: domain.payload.to_vec(),
                        });
                    }
                    Some(_) => return Err(SessionError::UnselectedDomain { domain_id }),
                    None => {}
                },
            }
        }

        let appearance = appearance_state::from_snapshots(
            &selected_capabilities,
            &gated_snapshots,
            world_spatial_observation
                .as_ref()
                .ok_or(SessionError::MissingDomain(1))?
                .content_generation,
            admission.appearance_items,
        )?;
        appearance_state::validate_entities(appearance.as_ref(), entities.as_ref())?;
        if attack_selected && combat_state.is_none() {
            return Err(SessionError::MissingDomain(
                attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
            ));
        }
        tutorial_notices::validate_join(&selected_capabilities, &gated_snapshots)?;
        let map_header = map_transport::validate_join(
            &selected_capabilities,
            &gated_snapshots,
            world_spatial_observation
                .as_ref()
                .ok_or(SessionError::MissingDomain(1))?
                .content_generation,
        )?;
        if map_header.is_some() {
            if world_object_overlay.is_some() {
                return Err(SessionError::UnselectedDomain { domain_id: 2 });
            }
            world_object_overlay = Some(Vec::new());
        }
        let snapshot = JoinSnapshot {
            game_session_id: accepted_fields.game_session_id,
            world_spatial: world_spatial_observation.ok_or(SessionError::MissingDomain(
                world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            ))?,
            world_spatial_revision: spatial_revision,
            world_object_overlay: world_object_overlay.ok_or(SessionError::MissingDomain(
                world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            ))?,
            world_object_overlay_revision: overlay_revision,
        };
        Ok(Self {
            stream,
            deadline: admission.deadline,
            connection_generation: session_generation,
            first_command_id: accepted_fields.next_command_id,
            next_command_id: accepted_fields.next_command_id,
            last_server_sequence: begin.target_server_sequence,
            spatial_revision,
            overlay_revision,
            vitals_revision,
            combat_revision,
            combat_state,
            actor_vitals,
            world_spatial: snapshot.world_spatial,
            world_object_overlay: snapshot.world_object_overlay.clone(),
            join_snapshot: snapshot,
            unusable: false,
            duplicates: Vec::new(),
            last_probe_id: 0,
            selected_capabilities,
            gated_snapshots,
            gated_revisions,
            events: Vec::new(),
            entities,
            appearance,
            chat_revision,
            map_header,
            inventory_revision,
            container_revision,
            inventory,
            open_container,
            chat,
        })
    }

    /// The capabilities the server selected at admission (never widened by a resume).
    pub fn selected_capabilities(&self) -> &[u32] {
        &self.selected_capabilities
    }

    pub fn is_selected(&self, capability: u32) -> bool {
        self.selected_capabilities.contains(&capability)
    }

    /// The join snapshot of a selected capability's `domain_id`, if the server sent it.
    pub fn gated_snapshot(&self, domain_id: u32) -> Option<&GatedSnapshot> {
        self.gated_snapshots
            .iter()
            .find(|snapshot| snapshot.domain_id == domain_id)
    }

    /// Reads the next `StateDelta` of the selected capability's `domain_id`/`delta_type`, checks
    /// it is based on the applied revision, and advances that revision; the caller decodes the
    /// payload.
    pub async fn read_gated_delta(
        &mut self,
        domain_id: u32,
        delta_type: u32,
    ) -> Result<GatedDelta, SessionError> {
        self.ensure_usable()?;
        let capability = capability_of_domain(domain_id)
            .filter(|capability| self.is_selected(*capability))
            .ok_or(SessionError::CapabilityNotSelected {
                capability: capability_of_domain(domain_id).unwrap_or(0),
            })?;
        debug_assert!(self.is_selected(capability));
        let applied = self
            .gated_revisions
            .iter()
            .find(|(id, _)| *id == domain_id)
            .map_or(0, |(_, revision)| *revision);
        let outcome = self.read_delta(domain_id, delta_type, applied).await;
        let delta = self.poison_on_error(outcome)?;
        match self
            .gated_revisions
            .iter_mut()
            .find(|(id, _)| *id == domain_id)
        {
            Some(entry) => entry.1 = delta.new_revision,
            None => self.gated_revisions.push((domain_id, delta.new_revision)),
        }
        Ok(GatedDelta {
            server_sequence: delta.server_sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            payload: delta.payload,
        })
    }

    pub fn join_snapshot(&self) -> &JoinSnapshot {
        &self.join_snapshot
    }

    pub fn into_join_snapshot(self) -> JoinSnapshot {
        self.join_snapshot
    }

    /// The visible entities after every delta applied so far; `Some` exactly when capability 6 is
    /// selected.
    pub fn world_entities(&self) -> Option<&WorldEntities> {
        self.entities.as_ref()
    }

    /// The open rooms and the last lines after every delta applied so far; `Some` exactly when
    /// capability 7 is selected.
    pub fn chat_log(&self) -> Option<&ChatLog> {
        self.chat.as_ref()
    }

    /// The inventory after every delta applied so far; `Some` exactly when capability 4 is
    /// selected.
    pub fn inventory(&self) -> Option<&CharacterInventory> {
        self.inventory.as_ref()
    }

    /// The open corpse after every delta applied so far (the default means nothing is open);
    /// `Some` exactly when capability 4 is selected.
    pub fn open_container(&self) -> Option<&OpenContainer> {
        self.open_container.as_ref()
    }

    /// The own-actor position after every delta applied so far.
    pub fn world_spatial(&self) -> &WorldSpatialObservation {
        &self.world_spatial
    }

    /// The overlay entries after every delta applied so far.
    pub fn world_object_overlay(&self) -> &[WorldObjectOverlayEntry] {
        &self.world_object_overlay
    }

    /// The own-actor vitals after every delta applied so far, if the server has sent any.
    pub fn actor_vitals(&self) -> Option<&ActorVitals> {
        self.actor_vitals.as_ref()
    }

    /// The own-actor combat state after every delta applied so far; `Some` once a session that
    /// selected capability 17 has received domain 10.
    pub fn combat_state(&self) -> Option<&ActorCombatState> {
        self.combat_state.as_ref()
    }

    /// The `CommandId` the next `step`/`use_object`/`cast_spell` will send.
    pub fn next_command_id(&self) -> u64 {
        self.next_command_id
    }

    /// The `server_sequence` of the last frame applied (initially the snapshot's target).
    pub fn last_server_sequence(&self) -> u64 {
        self.last_server_sequence
    }

    /// Returns (and clears) the duplicate-status results received while reading command results.
    pub fn take_duplicate_outcomes(&mut self) -> Vec<DuplicateOutcome> {
        std::mem::take(&mut self.duplicates)
    }

    /// Returns (and clears) the pushed deltas applied since the last call, in order.
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }

    /// Keeps an otherwise idle session alive: for up to `duration`, reads frames, answers each
    /// `LivenessProbe` with a `LivenessAck` (last applied `server_sequence`) and applies each
    /// pushed `StateDelta`, queueing it for `take_events`. Returns `Ok` when
    /// the time elapses. Any other frame, a failed validation, a closed connection or an I/O
    /// error fails closed and makes the session unusable. Runs on the caller's task only.
    pub async fn service_liveness(&mut self, duration: Duration) -> Result<(), SessionError> {
        self.ensure_usable()?;
        let until = tokio::time::Instant::now() + duration;
        let outcome = self.serve_liveness_until(until).await;
        self.poison_on_error(outcome)
    }

    async fn serve_liveness_until(
        &mut self,
        until: tokio::time::Instant,
    ) -> Result<(), SessionError> {
        loop {
            // Tokio may poll a ready read before an expired timeout. A continuously
            // readable peer must not extend the caller's idle window indefinitely.
            if tokio::time::Instant::now() >= until {
                return Ok(());
            }
            // Waiting for the first byte is cancel-safe (a plain `read` either returns data or
            // consumes nothing), so the idle window can end without losing part of a frame; the
            // rest of a started frame is then read under the ordinary per-frame deadline.
            let mut first = [0_u8; 1];
            match tokio::time::timeout_at(until, self.stream.read(&mut first)).await {
                Err(_elapsed) => return Ok(()),
                Ok(Err(error)) => return Err(error.into()),
                Ok(Ok(0)) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
                Ok(Ok(_)) => {}
            }
            let frame = bounded(
                self.deadline,
                "idle frame",
                read_frame_after(&mut self.stream, first[0]),
            )
            .await?;
            let envelope = decode_wire_envelope(&frame)?;
            envelope.validate(Direction::ServerToClient, true)?;
            self.check_generation(&envelope)?;
            match envelope.message_type() {
                MessageType::LivenessProbe => self.answer_probe(envelope.payload()).await?,
                MessageType::StateDelta => {
                    let server_sequence = self.next_sequence(&envelope)?;
                    let event = self.apply_state_delta(server_sequence, envelope.payload())?;
                    self.queue_event(event)?;
                }
                actual => {
                    return Err(SessionError::UnexpectedMessage {
                        expected: MessageType::LivenessProbe,
                        actual,
                    });
                }
            }
        }
    }

    fn check_generation(
        &self,
        envelope: &oteryn_protocol_oteryn::WireEnvelopeView<'_>,
    ) -> Result<(), SessionError> {
        if envelope.connection_generation() != self.connection_generation {
            return Err(SessionError::ConnectionGenerationMismatch {
                expected: self.connection_generation,
                actual: envelope.connection_generation(),
            });
        }
        Ok(())
    }

    async fn answer_probe(&mut self, payload: &[u8]) -> Result<(), SessionError> {
        let probe_id = decode_liveness_probe(payload)?;
        if probe_id <= self.last_probe_id {
            return Err(SessionError::ProbeIdNotAdvancing {
                last: self.last_probe_id,
                received: probe_id,
            });
        }
        self.last_probe_id = probe_id;
        let ack = encode_liveness_ack(
            self.connection_generation,
            probe_id,
            self.last_server_sequence,
        )?;
        bounded(
            self.deadline,
            "LivenessAck write",
            write_frame(&mut self.stream, &ack),
        )
        .await
    }

    /// Sends the FND-02 `ClientCommand` type 1 `WORLD_ACTOR_STEP_INTENT` for `direction` and
    /// decodes its `CommandResult` and, when it `Moved`, the one `WORLD_SPATIAL` delta.
    pub async fn step(&mut self, direction: StepDirection) -> Result<StepOutcome, SessionError> {
        self.ensure_usable()?;
        let payload = world_spatial::encode_step_intent(direction);
        let outcome = self.exchange_step(&payload).await;
        self.poison_on_error(outcome)
    }

    /// `step`, retried with doubling backoff while the server answers `TooEarly` (capability 13),
    /// at most [`STEP_RETRY_ATTEMPTS`] sends; returns the last outcome.
    pub async fn step_retrying(
        &mut self,
        direction: StepDirection,
    ) -> Result<StepOutcome, SessionError> {
        let mut backoff = STEP_RETRY_INITIAL_BACKOFF;
        let mut attempt = 1;
        loop {
            let outcome = self.step(direction).await?;
            if outcome.disposition != StepDisposition::TooEarly || attempt >= STEP_RETRY_ATTEMPTS {
                return Ok(outcome);
            }
            tokio::time::sleep(backoff).await;
            backoff = backoff.saturating_mul(2);
            attempt += 1;
        }
    }

    /// Sends the FND-02 `ClientCommand` type 2 `USE_INTENT` (USE-WIRE-V1) for the object at
    /// `placement`, naming the overlay revision this client believes current
    /// (`expected_revision`), and decodes its `CommandResult` and, when `Committed`, the one
    /// `WORLD_OBJECT_OVERLAY` delta.
    pub async fn use_object(
        &mut self,
        placement: &[u8],
        expected_revision: u64,
    ) -> Result<UseOutcome, SessionError> {
        self.ensure_usable()?;
        let payload = world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.to_vec(),
            expected_revision,
        })?;
        let outcome = self.exchange_use(&payload).await;
        self.poison_on_error(outcome)
    }

    /// Sends the FND-02 `ClientCommand` type 2 `USE_INTENT` with the field 2 item target
    /// (capability 4), which opens a corpse. Refused before anything is sent when capability 4 is
    /// not selected. It returns at its result; the domain 11 and 9 deltas it causes go through
    /// the domain store and the event queue.
    pub async fn use_item(&mut self, handle: ItemHandle) -> Result<UseOutcome, SessionError> {
        self.ensure_usable()?;
        self.ensure_item_view()?;
        let payload = world_object::encode_use_item_intent(handle);
        let outcome = self.exchange_use(&payload).await;
        self.poison_on_error(outcome)
    }

    /// Sends the FND-02 `ClientCommand` type 9 `ITEM_MOVE_INTENT` (capability 4) and decodes its
    /// `CommandResult`. Only the capability-4 destination (the main backpack) is encodable: the
    /// codec refuses the others before anything is sent. Every outcome but `Moved` (`Stale`
    /// included) is a normal result that changes no local state.
    pub async fn move_item(
        &mut self,
        intent: &ItemMoveIntent,
    ) -> Result<ItemMoveOutcomeResult, SessionError> {
        self.ensure_usable()?;
        self.ensure_item_view()?;
        let payload = item_view::encode_item_move_intent(intent).map_err(SessionError::ItemView)?;
        let outcome = self.exchange_move(&payload).await;
        self.poison_on_error(outcome)
    }

    fn ensure_item_view(&self) -> Result<(), SessionError> {
        if self.is_selected(CAPABILITY_ITEM_VIEW_MOVE_V1) {
            Ok(())
        } else {
            Err(SessionError::CapabilityNotSelected {
                capability: CAPABILITY_ITEM_VIEW_MOVE_V1,
            })
        }
    }

    async fn exchange_move(
        &mut self,
        payload: &[u8],
    ) -> Result<ItemMoveOutcomeResult, SessionError> {
        let result = self
            .send_and_read_result(COMMAND_TYPE_ITEM_MOVE_INTENT, payload)
            .await?;
        let outcome =
            item_view::decode_item_move_result(&result.payload).map_err(SessionError::ItemView)?;
        check_status_pairing(
            result.command_id,
            result.status,
            outcome == ItemMoveOutcome::Rejected,
        )?;
        Ok(ItemMoveOutcomeResult {
            command_id: result.command_id,
            status: result.status,
            outcome,
            result_server_sequence: result.server_sequence,
        })
    }

    /// Sends the FND-02 `ClientCommand` type 3 `WORLD_ACTOR_SPELL_CAST_INTENT` for the 1-based
    /// `spell` index, and decodes its `CommandResult` and, when it `Cast`, the one `ACTOR_VITALS`
    /// delta. A `Rejected` result (the closed server gate) is a normal outcome, not an error.
    pub async fn cast_spell(
        &mut self,
        spell: NonZeroU32,
        target: SpellTarget,
        aim_at_target: bool,
    ) -> Result<CastOutcome, SessionError> {
        self.ensure_usable()?;
        let payload = actor_spell::encode_spell_cast_intent(&SpellCastIntent {
            spell,
            target,
            aim_at_target,
        });
        let outcome = self.exchange_cast(&payload).await;
        self.poison_on_error(outcome)
    }

    /// Sends the FND-02 `ClientCommand` type 11 `ATTACK_TARGET_INTENT` (capability 17): attack
    /// `target`, or stop attacking with `None`. Refused before anything is sent when capability
    /// 17 is not selected. Every disposition is a normal outcome; the `ACTOR_COMBAT_STATE` delta
    /// it causes goes through the domain store and the event queue.
    pub async fn attack_target(
        &mut self,
        target: Option<&EntityRef>,
    ) -> Result<AttackOutcome, SessionError> {
        self.ensure_usable()?;
        self.ensure_attack()?;
        let payload = attack::encode_attack_target_intent(target).map_err(SessionError::Attack)?;
        let outcome = self
            .exchange_attack(attack::COMMAND_TYPE_ATTACK_TARGET_INTENT, &payload)
            .await;
        self.poison_on_error(outcome)
    }

    /// Sends the FND-02 `ClientCommand` type 12 `FIGHT_MODES_INTENT` (capability 17), like
    /// [`Self::attack_target`].
    pub async fn set_fight_modes(
        &mut self,
        modes: &FightModes,
    ) -> Result<AttackOutcome, SessionError> {
        self.ensure_usable()?;
        self.ensure_attack()?;
        let payload = attack::encode_fight_modes_intent(modes);
        let outcome = self
            .exchange_attack(attack::COMMAND_TYPE_FIGHT_MODES_INTENT, &payload)
            .await;
        self.poison_on_error(outcome)
    }

    fn ensure_attack(&self) -> Result<(), SessionError> {
        if self.is_selected(CAPABILITY_ATTACK_V1) {
            Ok(())
        } else {
            Err(SessionError::CapabilityNotSelected {
                capability: CAPABILITY_ATTACK_V1,
            })
        }
    }

    async fn exchange_attack(
        &mut self,
        command_type: u32,
        payload: &[u8],
    ) -> Result<AttackOutcome, SessionError> {
        let result = self.send_and_read_result(command_type, payload).await?;
        // The server's rate limit answers `Rejected` with an empty payload: a normal refusal.
        let disposition = if result.status == CommandStatus::Rejected && result.payload.is_empty() {
            AttackIntentDisposition::Rejected
        } else {
            attack::decode_attack_intent_result(&result.payload).map_err(SessionError::Attack)?
        };
        check_status_pairing(
            result.command_id,
            result.status,
            disposition == AttackIntentDisposition::Rejected,
        )?;
        Ok(AttackOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition,
            result_server_sequence: result.server_sequence,
        })
    }

    /// Sends the FND-02 `ClientCommand` type 23 `LOGOUT_INTENT` (capability 21) and decodes its
    /// `CommandResult`. Refused before anything is sent when capability 21 is not selected.
    /// `IN_FIGHT` and `BUSY` are normal refusals that leave the session playing. After
    /// `ACCEPTED` the server has ended the session and closes the transport, so this session
    /// sends nothing further.
    pub async fn logout(&mut self) -> Result<LogoutOutcome, SessionError> {
        self.ensure_usable()?;
        if !self.is_selected(logout::CAPABILITY_LOGOUT_V1) {
            return Err(SessionError::CapabilityNotSelected {
                capability: logout::CAPABILITY_LOGOUT_V1,
            });
        }
        let outcome = self.exchange_logout().await;
        if matches!(
            outcome,
            Ok(LogoutOutcome {
                result: Some(LogoutResult::Accepted),
                ..
            })
        ) {
            self.unusable = true;
        }
        self.poison_on_error(outcome)
    }

    async fn exchange_logout(&mut self) -> Result<LogoutOutcome, SessionError> {
        let result = self
            .send_and_read_result(
                logout::COMMAND_TYPE_LOGOUT_INTENT,
                &logout::encode_logout_intent(),
            )
            .await?;
        let decoded = if result.status == CommandStatus::Rejected && result.payload.is_empty() {
            None
        } else {
            Some(logout::decode_logout_result(&result.payload).map_err(SessionError::Logout)?)
        };
        check_status_pairing(
            result.command_id,
            result.status,
            decoded.is_none_or(LogoutResult::is_rejected),
        )?;
        Ok(LogoutOutcome {
            command_id: result.command_id,
            status: result.status,
            result: decoded,
            result_server_sequence: result.server_sequence,
        })
    }

    /// Sends the FND-02 `ClientCommand` type 13 `CHAT_INTENT` (capability 7) and decodes its
    /// `CommandResult`. Refused before anything is sent when capability 7 is not selected or the
    /// intent breaks a codec bound (empty or oversized text or name). A `Muted` or `Exhausted`
    /// result is a normal outcome that changes no local state; the command returns at its result
    /// and every resulting delta goes through the domain store and the event queue.
    pub async fn chat(&mut self, intent: &ChatIntent) -> Result<ChatOutcome, SessionError> {
        self.ensure_usable()?;
        if !self.is_selected(CAPABILITY_CHAT_V1) {
            return Err(SessionError::CapabilityNotSelected {
                capability: CAPABILITY_CHAT_V1,
            });
        }
        guild_chat::check_intent(intent, &self.selected_capabilities)?;
        let payload = chat::encode_chat_intent(intent)?;
        let outcome = self.exchange_chat(&payload).await;
        self.poison_on_error(outcome)
    }

    async fn exchange_chat(&mut self, payload: &[u8]) -> Result<ChatOutcome, SessionError> {
        let result = self
            .send_and_read_result(COMMAND_TYPE_CHAT_INTENT, payload)
            .await?;
        let decoded = chat::decode_chat_intent_result(&result.payload)?;
        guild_chat::check_result(decoded.disposition, &self.selected_capabilities)?;
        check_status_pairing(
            result.command_id,
            result.status,
            decoded.disposition == ChatDisposition::Rejected,
        )?;
        Ok(ChatOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition: decoded.disposition,
            wait_seconds: decoded.wait_seconds,
            result_server_sequence: result.server_sequence,
        })
    }

    fn ensure_usable(&self) -> Result<(), SessionError> {
        if self.unusable {
            Err(SessionError::SessionUnusable)
        } else {
            Ok(())
        }
    }

    fn poison_on_error<T>(&mut self, outcome: Result<T, SessionError>) -> Result<T, SessionError> {
        if outcome.is_err() {
            self.unusable = true;
        }
        outcome
    }

    async fn exchange_step(&mut self, payload: &[u8]) -> Result<StepOutcome, SessionError> {
        let result = self
            .send_and_read_result(world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, payload)
            .await?;
        let disposition = world_spatial::decode_step_result_paced(
            &result.payload,
            self.is_selected(CAPABILITY_PACED_MOVEMENT_V1),
        )?;
        check_status_pairing(
            result.command_id,
            result.status,
            matches!(
                disposition,
                StepDisposition::Rejected | StepDisposition::TooEarly
            ),
        )?;
        let world_spatial_delta =
            if result.status == CommandStatus::Accepted && disposition == StepDisposition::Moved {
                match self
                    .read_claimed_delta(world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY)
                    .await?
                {
                    SessionEvent::WorldSpatial(delta) => Some(delta),
                    SessionEvent::WorldSpatialEntities(delta) => {
                        // The outcome carries only the observation; the entity enter/update/leave
                        // lists reach consumers through the event queue.
                        let observation = AppliedDelta {
                            server_sequence: delta.server_sequence,
                            base_revision: delta.base_revision,
                            new_revision: delta.new_revision,
                            value: WorldSpatialObservation {
                                content_generation: delta.value.content_generation,
                                actor_position: delta.value.actor_position,
                            },
                        };
                        self.queue_event(SessionEvent::WorldSpatialEntities(delta))?;
                        Some(observation)
                    }
                    other => {
                        return Err(SessionError::UnexpectedDomain {
                            expected: world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                            actual: other.domain_id(),
                        });
                    }
                }
            } else {
                None
            };
        Ok(CommandOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition,
            result_server_sequence: result.server_sequence,
            world_spatial_delta,
            world_object_overlay_delta: None,
            gate_message_id: None,
        })
    }

    async fn exchange_use(&mut self, payload: &[u8]) -> Result<UseOutcome, SessionError> {
        let result = self
            .send_and_read_result(world_object::COMMAND_TYPE_USE_INTENT, payload)
            .await?;
        let (disposition, gate_message_id) =
            world_object::decode_use_result_with_gate(&result.payload)?;
        // Dispositions 11 to 17 and the gate message id exist only under capability 11, and
        // dispositions 18 and 19 only under capability 22.
        check_use_result_negotiated(
            disposition,
            gate_message_id,
            self.is_selected(world_interaction::CAPABILITY_WORLD_INTERACTION_V1),
            self.is_selected(world_object::CAPABILITY_RUNE_USE_V1),
        )?;
        check_status_pairing(
            result.command_id,
            result.status,
            disposition == UseDisposition::Rejected,
        )?;
        // A `Committed` use names no domain (it can change 2, 9, 11, several or none), so the
        // exchange returns at its result; its deltas go through the store and the event queue.
        Ok(CommandOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition,
            result_server_sequence: result.server_sequence,
            world_spatial_delta: None,
            world_object_overlay_delta: None,
            gate_message_id,
        })
    }

    async fn exchange_cast(&mut self, payload: &[u8]) -> Result<CastOutcome, SessionError> {
        let result = self
            .send_and_read_result(
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                payload,
            )
            .await?;
        let disposition = actor_spell::decode_spell_cast_result(&result.payload)?;
        check_status_pairing(
            result.command_id,
            result.status,
            disposition == SpellCastDisposition::Rejected,
        )?;
        let actor_vitals_delta = if result.status == CommandStatus::Accepted
            && disposition == SpellCastDisposition::Cast
        {
            match self
                .read_claimed_delta(actor_spell::STATE_DOMAIN_ACTOR_VITALS)
                .await?
            {
                SessionEvent::ActorVitals(delta) => Some(delta),
                other => {
                    return Err(SessionError::UnexpectedDomain {
                        expected: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                        actual: other.domain_id(),
                    });
                }
            }
        } else {
            None
        };
        Ok(CastOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition,
            result_server_sequence: result.server_sequence,
            actor_vitals_delta,
        })
    }

    /// Sends one command under the next `CommandId` and reads its correlated `CommandResult`.
    async fn send_and_read_result(
        &mut self,
        command_type: u32,
        payload: &[u8],
    ) -> Result<ReceivedResult, SessionError> {
        let command_id = self.next_command_id;
        // Consumed before the write: an attempted (possibly half-written) command's id is never
        // reused.
        self.next_command_id = command_id
            .checked_add(1)
            .ok_or(SessionError::CommandIdExhausted)?;
        let frame = encode_client_command(
            self.connection_generation,
            &ClientCommandValue {
                command_id,
                command_type,
                payload,
            },
        )?;
        bounded(
            self.deadline,
            "ClientCommand write",
            write_frame(&mut self.stream, &frame),
        )
        .await?;

        loop {
            let (message_type, server_sequence, result_payload) =
                self.read_sequenced_push("CommandResult").await?;
            if message_type == MessageType::StateDelta {
                // Pushed before the result (the session never predicts the next frame's domain).
                let event = self.apply_state_delta(server_sequence, &result_payload)?;
                self.queue_event(event)?;
                continue;
            }
            let result = decode_command_result(&result_payload)?;
            if matches!(
                result.status,
                CommandStatus::DuplicateReplay | CommandStatus::DuplicateOutcomeExpired
            ) {
                // FND-02 §13.2: a duplicate names an already reserved lower CommandId and is
                // never executed again, so it is only ever valid for a CommandId this session
                // sent before the current one; the current command's own result is never a
                // duplicate. It carries no delta and does not end the wait for the real result.
                if result.command_id == command_id {
                    return Err(SessionError::InconsistentCommandResult {
                        command_id,
                        status: result.status,
                    });
                }
                if result.command_id < self.first_command_id || result.command_id > command_id {
                    return Err(SessionError::DuplicateForUnsentCommand {
                        command_id: result.command_id,
                    });
                }
                if self.duplicates.len() >= MAX_RETAINED_DUPLICATES {
                    return Err(FoundationProtocolError::PayloadLimitExceeded.into());
                }
                self.duplicates.push(DuplicateOutcome {
                    command_id: result.command_id,
                    status: result.status,
                    server_sequence,
                    payload: result.payload.to_vec(),
                });
                continue;
            }
            if result.command_id != command_id {
                return Err(SessionError::CommandIdMismatch {
                    expected: command_id,
                    actual: result.command_id,
                });
            }
            return Ok(ReceivedResult {
                command_id,
                status: result.status,
                server_sequence,
                payload: result.payload.to_vec(),
            });
        }
    }

    /// Reads `StateDelta` frames until one names `domain_id` and returns it as the claimed delta:
    /// the first delta of that domain applied after the command's result, by revision alone.
    /// Deltas of other domains apply and queue on the way. `ServerSequence` and revision checks
    /// run for each frame.
    async fn read_claimed_delta(&mut self, domain_id: u32) -> Result<SessionEvent, SessionError> {
        loop {
            let (server_sequence, payload) = self
                .read_sequenced("StateDelta", MessageType::StateDelta)
                .await?;
            let event = self.apply_state_delta(server_sequence, &payload)?;
            if event.domain_id() == domain_id {
                return Ok(event);
            }
            self.queue_event(event)?;
        }
    }

    fn queue_event(&mut self, event: SessionEvent) -> Result<(), SessionError> {
        if self.events.len() >= MAX_QUEUED_EVENTS {
            return Err(SessionError::EventQueueOverflow {
                limit: MAX_QUEUED_EVENTS,
            });
        }
        self.events.push(event);
        Ok(())
    }

    /// Reads one server-sequenced `StateDelta` and checks it names `domain_id`, its registered
    /// `delta_type`, and is based on exactly `applied_revision`. The caller decodes the typed
    /// payload and only then commits the new revision.
    async fn read_delta(
        &mut self,
        domain_id: u32,
        delta_type: u32,
        applied_revision: u64,
    ) -> Result<RawDelta, SessionError> {
        let (server_sequence, payload) = self
            .read_sequenced("StateDelta", MessageType::StateDelta)
            .await?;
        let delta = decode_state_delta(&payload)?;
        if delta.domain_id != domain_id {
            return Err(SessionError::UnexpectedDomain {
                expected: domain_id,
                actual: delta.domain_id,
            });
        }
        check_delta(&delta, delta_type, applied_revision)?;
        Ok(RawDelta {
            server_sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            payload: delta.payload.to_vec(),
        })
    }

    /// Reads the next post-admission frame that is not a `LivenessProbe` (each probe is answered
    /// with a `LivenessAck` first). The envelope is validated as post-admission server traffic
    /// and must carry the admitted `connection_generation`.
    async fn read_post_admission(&mut self, label: &'static str) -> Result<Vec<u8>, SessionError> {
        loop {
            let frame = bounded(self.deadline, label, read_frame(&mut self.stream)).await?;
            let envelope = decode_wire_envelope(&frame)?;
            envelope.validate(Direction::ServerToClient, true)?;
            self.check_generation(&envelope)?;
            if envelope.message_type() != MessageType::LivenessProbe {
                return Ok(frame);
            }
            self.answer_probe(envelope.payload()).await?;
        }
    }

    /// Reads the next non-probe frame, requires it to be `expected`, and requires its
    /// `server_sequence` to be exactly the last applied one plus one. Returns that sequence and
    /// the frame's payload.
    async fn read_sequenced(
        &mut self,
        label: &'static str,
        expected: MessageType,
    ) -> Result<(u64, Vec<u8>), SessionError> {
        let (message_type, server_sequence, payload) = self.read_sequenced_push(label).await?;
        if message_type != expected {
            return Err(SessionError::UnexpectedMessage {
                expected,
                actual: message_type,
            });
        }
        Ok((server_sequence, payload))
    }

    /// `read_sequenced` for a frame that is either a `CommandResult` or a pushed `StateDelta`;
    /// any other type is a protocol error naming `CommandResult`.
    async fn read_sequenced_push(
        &mut self,
        label: &'static str,
    ) -> Result<(MessageType, u64, Vec<u8>), SessionError> {
        let frame = self.read_post_admission(label).await?;
        let envelope = decode_wire_envelope(&frame)?;
        let message_type = envelope.message_type();
        if !matches!(
            message_type,
            MessageType::CommandResult | MessageType::StateDelta
        ) {
            return Err(SessionError::UnexpectedMessage {
                expected: MessageType::CommandResult,
                actual: message_type,
            });
        }
        let server_sequence = self.next_sequence(&envelope)?;
        Ok((message_type, server_sequence, envelope.payload().to_vec()))
    }

    /// Requires `envelope`'s `server_sequence` to be exactly the last applied one plus one and
    /// records it.
    fn next_sequence(
        &mut self,
        envelope: &oteryn_protocol_oteryn::WireEnvelopeView<'_>,
    ) -> Result<u64, SessionError> {
        let expected_sequence = self
            .last_server_sequence
            .checked_add(1)
            .ok_or(FoundationProtocolError::ServerSequenceGap)?;
        if envelope.server_sequence() != expected_sequence {
            return Err(SessionError::ServerSequenceMismatch {
                expected: expected_sequence,
                actual: envelope.server_sequence(),
            });
        }
        self.last_server_sequence = expected_sequence;
        Ok(expected_sequence)
    }
}

/// A validated, correlated `CommandResult` with its typed payload still undecoded.
struct ReceivedResult {
    command_id: u64,
    status: CommandStatus,
    server_sequence: u64,
    payload: Vec<u8>,
}

/// Checks a decoded `StateDelta` carries the registered `delta_type` and is based on exactly the
/// stored revision.
fn check_delta(
    delta: &oteryn_protocol_oteryn::StateDeltaView<'_>,
    delta_type: u32,
    applied_revision: u64,
) -> Result<(), SessionError> {
    if delta.delta_type != delta_type {
        return Err(SessionError::UnregisteredDeltaType {
            domain_id: delta.domain_id,
            delta_type: delta.delta_type,
        });
    }
    if delta.base_revision != applied_revision {
        return Err(SessionError::StateRevisionMismatch {
            domain_id: delta.domain_id,
            expected_base: applied_revision,
            actual_base: delta.base_revision,
        });
    }
    Ok(())
}

/// A validated `StateDelta` with its domain-typed payload still undecoded.
struct RawDelta {
    server_sequence: u64,
    base_revision: u64,
    new_revision: u64,
    payload: Vec<u8>,
}

/// Reads one bounded frame expected to be `expected` (`SnapshotBegin` or `SnapshotCommit`),
/// checks its envelope `connection_generation` matches the admitted session's, and returns its
/// `snapshot_id` (`decode_snapshot_id`) for the caller to correlate against `SnapshotBegin`'s.
async fn read_snapshot_marker<S: SessionStream>(
    stream: &mut S,
    deadline: Duration,
    label: &'static str,
    expected: MessageType,
    session_generation: u64,
) -> Result<u64, SessionError> {
    let frame = bounded(deadline, label, read_frame(stream)).await?;
    let envelope = decode_wire_envelope(&frame)?;
    // Post-admission server traffic, same rules as `SnapshotBegin`/`SnapshotChunk`.
    envelope.validate(Direction::ServerToClient, true)?;
    if envelope.message_type() != expected {
        return Err(SessionError::UnexpectedMessage {
            expected,
            actual: envelope.message_type(),
        });
    }
    if envelope.connection_generation() != session_generation {
        return Err(SessionError::ConnectionGenerationMismatch {
            expected: session_generation,
            actual: envelope.connection_generation(),
        });
    }
    Ok(decode_snapshot_id(envelope.payload())?)
}

/// Bounds `future` by `deadline`, mapping an elapsed deadline to `SessionError::Timeout(label)`
/// and any inner error through `SessionError`'s existing `From` conversions.
async fn bounded<T, E, F>(
    deadline: Duration,
    label: &'static str,
    future: F,
) -> Result<T, SessionError>
where
    F: Future<Output = Result<T, E>>,
    SessionError: From<E>,
{
    match tokio::time::timeout(deadline, future).await {
        Ok(result) => result.map_err(SessionError::from),
        Err(_elapsed) => Err(SessionError::Timeout(label)),
    }
}

/// Writes one length-prefixed frame. Generic over the stream type (client or server side of any
/// transport) so a fake peer can frame with the exact same code the session uses.
pub async fn write_frame<S: AsyncWrite + Unpin>(
    stream: &mut S,
    body: &[u8],
) -> Result<(), SessionError> {
    let length = FrameLength::new(
        u32::try_from(body.len()).map_err(|_error| FoundationProtocolError::FrameTooLarge)?,
    )?;
    stream.write_all(&length.to_prefix()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

/// Reads one length-prefixed frame body (bounded by `FrameLength`).
pub async fn read_frame<S: AsyncRead + Unpin>(stream: &mut S) -> Result<Vec<u8>, SessionError> {
    let mut prefix = [0_u8; 4];
    stream.read_exact(&mut prefix).await?;
    read_frame_body(stream, prefix).await
}

/// `read_frame` for a frame whose first length-prefix byte was already read.
async fn read_frame_after<S: AsyncRead + Unpin>(
    stream: &mut S,
    first: u8,
) -> Result<Vec<u8>, SessionError> {
    let mut prefix = [first, 0, 0, 0];
    stream.read_exact(&mut prefix[1..]).await?;
    read_frame_body(stream, prefix).await
}

async fn read_frame_body<S: AsyncRead + Unpin>(
    stream: &mut S,
    prefix: [u8; 4],
) -> Result<Vec<u8>, SessionError> {
    let length = FrameLength::from_prefix(&prefix)?;
    let mut body = vec![0_u8; length.get() as usize];
    stream.read_exact(&mut body).await?;
    Ok(body)
}

/// The server sends `ACCEPTED` for every typed disposition except `REJECTED`, which it sends
/// with `REJECTED` (`connection.rs` `serve_admitted`, both `STEP` and `USE_INTENT`); any other
/// pairing is a violation.
/// A server that sends a capability 11 disposition (11 to 17) or a gate message id to a session
/// that did not select capability 11 broke the negotiation: the result fails closed. So does a
/// capability 22 disposition (18 or 19) to a session that did not select capability 22.
fn check_use_result_negotiated(
    disposition: UseDisposition,
    gate_message_id: Option<u8>,
    world_interaction_selected: bool,
    rune_use_selected: bool,
) -> Result<(), SessionError> {
    if ((disposition.needs_world_interaction() || gate_message_id.is_some())
        && !world_interaction_selected)
        || (disposition.needs_rune_use() && !rune_use_selected)
    {
        return Err(SessionError::WorldObject(
            world_object::WorldObjectError::Malformed,
        ));
    }
    Ok(())
}

fn check_status_pairing(
    command_id: u64,
    status: CommandStatus,
    disposition_rejected: bool,
) -> Result<(), SessionError> {
    let consistent = match status {
        CommandStatus::Accepted => !disposition_rejected,
        CommandStatus::Rejected => disposition_rejected,
        CommandStatus::DuplicateReplay | CommandStatus::DuplicateOutcomeExpired => false,
    };
    if consistent {
        Ok(())
    } else {
        Err(SessionError::InconsistentCommandResult { command_id, status })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "guild_chat_tests.rs"]
mod guild_chat_tests;
