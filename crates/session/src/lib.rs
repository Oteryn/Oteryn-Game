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

use oteryn_protocol_oteryn::actor_spell::{self, SpellCastIntent};
/// Spell types a client names when it casts and draws vitals: re-exported so the client needs no
/// direct `protocol-oteryn` edge (ADR-0020 section 1).
pub use oteryn_protocol_oteryn::actor_spell::{ActorVitals, SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::world_object::{
    self, UseDisposition, WorldObjectOverlayEntry, WorldObjectTarget,
};
use oteryn_protocol_oteryn::world_spatial::{
    self, CAPABILITY_PACED_MOVEMENT_V1, StepDirection, StepDisposition, WorldSpatialObservation,
};
use oteryn_protocol_oteryn::{
    CharacterId, ClientBootstrapValue, ClientCommandValue, CommandStatus, Direction,
    FoundationProtocolError, FrameLength, GameSessionId, MessageType, decode_command_result,
    decode_liveness_probe, decode_server_accepted, decode_snapshot_begin, decode_snapshot_body,
    decode_snapshot_chunk_framing, decode_snapshot_id, decode_state_delta, decode_wire_envelope,
    encode_client_bootstrap, encode_client_command, encode_liveness_ack,
};
use oteryn_protocol_oteryn::{
    achievement_notices::{
        CAPABILITY_ACHIEVEMENT_NOTICES_V1, STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
    },
    analyser::{CAPABILITY_ANALYSER_V1, STATE_DOMAIN_ACTOR_ANALYSER},
    bestiary::STATE_DOMAIN_CHARACTER_BESTIARY,
    charm::{
        CAPABILITY_BESTIARY_CHARMS_V1, COMMAND_TYPE_CHARM_ASSIGN_INTENT,
        COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT, STATE_DOMAIN_CHARACTER_CHARMS,
    },
    chat::{CAPABILITY_CHAT_V1, COMMAND_TYPE_CHAT_INTENT, STATE_DOMAIN_CHAT},
    container_tree::{
        CAPABILITY_CONTAINER_TREE_V1, COMMAND_TYPE_CONTAINER_VIEW_INTENT,
        STATE_DOMAIN_CONTAINER_VIEWS,
    },
    item_view::{
        CAPABILITY_ITEM_VIEW_MOVE_V1, COMMAND_TYPE_ITEM_MOVE_INTENT,
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

/// The capabilities this client implements and advertises by default: 13 `PACED_MOVEMENT_V1`
/// (the step result `TOO_EARLY`). Add an ID here only together with its routing.
pub const CLIENT_SUPPORTED_CAPABILITIES: &[u32] = &[CAPABILITY_PACED_MOVEMENT_V1];

/// Capability-owned command types and state domains (`PROTOCOL_OTERYN_V1_REGISTRY.json`;
/// mirrors the server's gate table). Capabilities 6, 12 and 13 own none: they extend the core
/// domain 1 and command types 1 and 9, whose codecs gate the extension on the selected set.
const GATED_ROUTES: &[(u32, &[u32], &[u32])] = &[
    (
        CAPABILITY_BESTIARY_CHARMS_V1,
        &[
            COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
            COMMAND_TYPE_CHARM_ASSIGN_INTENT,
        ],
        &[
            STATE_DOMAIN_CHARACTER_BESTIARY,
            STATE_DOMAIN_CHARACTER_CHARMS,
        ],
    ),
    (
        CAPABILITY_ITEM_VIEW_MOVE_V1,
        &[COMMAND_TYPE_ITEM_MOVE_INTENT],
        &[
            STATE_DOMAIN_CHARACTER_INVENTORY,
            STATE_DOMAIN_OPEN_CONTAINER,
        ],
    ),
    (
        CAPABILITY_CHAT_V1,
        &[COMMAND_TYPE_CHAT_INTENT],
        &[STATE_DOMAIN_CHAT],
    ),
    (
        CAPABILITY_ACHIEVEMENT_NOTICES_V1,
        &[],
        &[STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES],
    ),
    (CAPABILITY_ANALYSER_V1, &[], &[STATE_DOMAIN_ACTOR_ANALYSER]),
    (
        CAPABILITY_CONTAINER_TREE_V1,
        &[COMMAND_TYPE_CONTAINER_VIEW_INTENT],
        &[STATE_DOMAIN_CONTAINER_VIEWS],
    ),
];

fn capability_of_domain(domain_id: u32) -> Option<u32> {
    GATED_ROUTES
        .iter()
        .find(|(_, _, domains)| domains.contains(&domain_id))
        .map(|(capability, _, _)| *capability)
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
    /// The server closed, or replied with something other than `ServerAccepted`, before
    /// admission completed.
    NotAdmitted(MessageType),
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
            Self::NotAdmitted(message_type) => {
                write!(formatter, "admission refused: server sent {message_type:?}")
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

/// Most applied-but-undrained pushed deltas [`Session::take_events`] holds. The next one fails the
/// session closed with [`SessionError::EventQueueOverflow`].
pub const MAX_QUEUED_EVENTS: usize = 256;

/// A server-sequenced delta that no command outcome claimed, already validated and applied to the
/// session's domain store (FND-02 §14, §15), in the order it was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    /// Domain 1 `WORLD_SPATIAL_VISIBILITY`.
    WorldSpatial(AppliedDelta<WorldSpatialObservation>),
    /// Domain 2 `WORLD_OBJECT_OVERLAY`.
    WorldObjectOverlay(AppliedDelta<WorldObjectOverlayEntry>),
    /// Domain 3 `ACTOR_VITALS`.
    ActorVitals(AppliedDelta<ActorVitals>),
}

impl SessionEvent {
    #[must_use]
    pub const fn domain_id(&self) -> u32 {
        match self {
            Self::WorldSpatial(_) => world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            Self::WorldObjectOverlay(_) => world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            Self::ActorVitals(_) => actor_spell::STATE_DOMAIN_ACTOR_VITALS,
        }
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
        let mut world_spatial_observation = None;
        let mut world_object_overlay = None;
        let mut actor_vitals = None;
        let (mut spatial_revision, mut overlay_revision, mut vitals_revision) = (0, 0, 0);
        let mut gated_snapshots = Vec::new();
        let mut gated_revisions = Vec::new();
        for domain in decode_snapshot_body(&assembled_body)? {
            match (domain.domain_id, domain.snapshot_type) {
                (
                    world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    world_spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                ) => {
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

    /// Sends a command type owned by `capability` and returns its raw `CommandResult`. Refused
    /// before anything is sent when the capability is not selected or does not own the type.
    pub async fn gated_command(
        &mut self,
        capability: u32,
        command_type: u32,
        payload: &[u8],
    ) -> Result<GatedCommandResult, SessionError> {
        self.ensure_usable()?;
        let owned = GATED_ROUTES
            .iter()
            .any(|(id, commands, _)| *id == capability && commands.contains(&command_type));
        if !owned || !self.is_selected(capability) {
            return Err(SessionError::CapabilityNotSelected { capability });
        }
        let outcome = self.send_and_read_result(command_type, payload).await;
        let result = self.poison_on_error(outcome)?;
        Ok(GatedCommandResult {
            command_id: result.command_id,
            status: result.status,
            server_sequence: result.server_sequence,
            payload: result.payload,
        })
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
        })
    }

    async fn exchange_use(&mut self, payload: &[u8]) -> Result<UseOutcome, SessionError> {
        let result = self
            .send_and_read_result(world_object::COMMAND_TYPE_USE_INTENT, payload)
            .await?;
        let disposition = world_object::decode_use_result(&result.payload)?;
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

    /// The domain store: applies one validated `StateDelta` by its own domain. It must be based
    /// on exactly the stored revision, carry the domain's registered `delta_type` and, where the
    /// domain has one, the loaded `content_generation`. Nothing is stored unless every check and
    /// the typed decode pass.
    fn apply_state_delta(
        &mut self,
        server_sequence: u64,
        payload: &[u8],
    ) -> Result<SessionEvent, SessionError> {
        let delta = decode_state_delta(payload)?;
        let (domain_id, base_revision, new_revision) =
            (delta.domain_id, delta.base_revision, delta.new_revision);
        fn applied<T>(sequence: u64, base: u64, new: u64, value: T) -> AppliedDelta<T> {
            AppliedDelta {
                server_sequence: sequence,
                base_revision: base,
                new_revision: new,
                value,
            }
        }
        match domain_id {
            world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY => {
                check_delta(
                    &delta,
                    world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1,
                    self.spatial_revision,
                )?;
                let observation = world_spatial::decode_world_spatial(delta.payload)?;
                if observation.content_generation != self.world_spatial.content_generation {
                    return Err(SessionError::ContentGenerationMismatch { domain_id });
                }
                self.world_spatial = observation;
                self.spatial_revision = new_revision;
                Ok(SessionEvent::WorldSpatial(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    observation,
                )))
            }
            world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY => {
                check_delta(
                    &delta,
                    world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    self.overlay_revision,
                )?;
                let entry = world_object::decode_world_object_overlay_delta(delta.payload)?;
                if entry.revision != new_revision {
                    return Err(SessionError::OverlayEntryRevisionMismatch {
                        new_revision,
                        entry_revision: entry.revision,
                    });
                }
                if entry.content_generation != self.world_spatial.content_generation {
                    return Err(SessionError::ContentGenerationMismatch { domain_id });
                }
                match self
                    .world_object_overlay
                    .iter_mut()
                    .find(|existing| existing.placement == entry.placement)
                {
                    Some(existing) => *existing = entry.clone(),
                    None => self.world_object_overlay.push(entry.clone()),
                }
                self.overlay_revision = new_revision;
                Ok(SessionEvent::WorldObjectOverlay(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    entry,
                )))
            }
            actor_spell::STATE_DOMAIN_ACTOR_VITALS => {
                check_delta(
                    &delta,
                    actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                    self.vitals_revision,
                )?;
                let vitals = actor_spell::decode_actor_vitals(delta.payload)?;
                self.actor_vitals = Some(vitals);
                self.vitals_revision = new_revision;
                Ok(SessionEvent::ActorVitals(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    vitals,
                )))
            }
            _ => Err(match capability_of_domain(domain_id) {
                Some(capability) if !self.is_selected(capability) => {
                    SessionError::UnselectedDomain { domain_id }
                }
                _ => SessionError::UnsupportedPushedDomain { domain_id },
            }),
        }
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
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::world_object::{
        SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
        WorldObjectOverlayEntry as WireOverlayEntry, encode_world_object_overlay_snapshot,
    };
    use oteryn_protocol_oteryn::world_spatial::{
        ActorPosition, DELTA_TYPE_WORLD_SPATIAL_V1, SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, WorldSpatialObservation as WireSpatialObservation,
        encode_step_result, encode_world_spatial,
    };
    use oteryn_protocol_oteryn::{
        ChannelId, DomainSnapshot, ServerAcceptedValue, WorldId, encode_command_result,
        encode_server_accepted, encode_single_chunk_snapshot, encode_state_delta,
    };
    use tokio::io::DuplexStream;

    type BoxError = Box<dyn StdError + Send + Sync>;

    const CONTENT_GENERATION: [u8; 32] = [0x11; 32];
    const DEADLINE: Duration = Duration::from_secs(5);

    fn uuid_v7(marker: u8) -> [u8; 16] {
        [
            0x01, 0x93, 0x4f, 0x10, 0x7c, 0x00, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33,
            0x44, marker,
        ]
    }

    fn spatial(x: i32) -> Vec<u8> {
        encode_world_spatial(&WireSpatialObservation {
            content_generation: CONTENT_GENERATION,
            actor_position: ActorPosition { x, y: 0, floor: 0 },
        })
    }

    /// A scripted peer over an in-memory duplex: no TLS, no TCP, no socket. Reads the
    /// `ClientBootstrap`, admits, sends the join snapshot, then answers one `step` with `Moved`
    /// and its one `WORLD_SPATIAL` delta.
    async fn scripted_server(mut stream: DuplexStream) -> Result<(), BoxError> {
        let bootstrap = read_frame(&mut stream).await?;
        assert_eq!(
            decode_wire_envelope(&bootstrap)?
                .client_bootstrap()?
                .admission_material,
            b"grant"
        );
        write_frame(
            &mut stream,
            &encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                world_id: WorldId::decode(&uuid_v7(2))?,
                channel_id: ChannelId::decode(&uuid_v7(3))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: &[],
            })?,
        )
        .await?;
        let spatial_payload = spatial(0);
        let overlay_payload = encode_world_object_overlay_snapshot(&[WireOverlayEntry {
            content_generation: CONTENT_GENERATION,
            placement: b"oteryn:cell/entry-door".to_vec(),
            state: b"oteryn:reference.state.closed".to_vec(),
            revision: 0,
        }])
        .map_err(|error| format!("overlay snapshot: {error:?}"))?;
        for frame in encode_single_chunk_snapshot(
            1,
            1,
            40,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 5,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial_payload,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 2,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay_payload,
                },
            ],
        )? {
            write_frame(&mut stream, &frame).await?;
        }

        let command = read_frame(&mut stream).await?;
        let command = decode_wire_envelope(&command)?;
        assert_eq!(command.client_command(1)?.command_id, 7);
        write_frame(
            &mut stream,
            &encode_command_result(
                1,
                41,
                7,
                CommandStatus::Accepted,
                &encode_step_result(StepDisposition::Moved),
            )?,
        )
        .await?;
        write_frame(
            &mut stream,
            &encode_state_delta(
                1,
                42,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                5,
                6,
                DELTA_TYPE_WORLD_SPATIAL_V1,
                &spatial(1),
            )?,
        )
        .await?;
        Ok(())
    }

    fn admission() -> Result<Admission<'static>, BoxError> {
        Ok(Admission {
            schema_revision: 1,
            character_id: CharacterId::decode(&uuid_v7(4))?,
            admission_material: b"grant",
            client_build_id: "oteryn-session-test",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: DEADLINE,
        })
    }

    fn block_on<F: Future>(future: F) -> Result<F::Output, BoxError> {
        Ok(tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(future))
    }

    /// The session runs over any byte stream: admission, join snapshot, and one `step` with its
    /// sequenced delta, with no TLS or TCP anywhere.
    #[test]
    fn admits_joins_and_steps_over_an_in_memory_stream() -> Result<(), BoxError> {
        block_on(async {
            let (client, server) = tokio::io::duplex(64 * 1024);
            let server = tokio::spawn(scripted_server(server));
            let mut session = Session::admit(client, admission()?).await?;
            assert_eq!(session.next_command_id(), 7);
            assert_eq!(session.last_server_sequence(), 40);
            assert_eq!(session.join_snapshot().world_spatial_revision, 5);
            assert_eq!(session.join_snapshot().world_object_overlay_revision, 2);
            let outcome = session.step(StepDirection::East).await?;
            server.await??;
            assert_eq!(outcome.disposition, StepDisposition::Moved);
            assert_eq!(outcome.result_server_sequence, 41);
            let delta = outcome.world_spatial_delta.ok_or("Moved carries a delta")?;
            assert_eq!((delta.server_sequence, delta.new_revision), (42, 6));
            assert_eq!(session.world_spatial().actor_position.x, 1);
            assert_eq!(session.last_server_sequence(), 42);
            Ok(())
        })?
    }

    fn vitals(health: u32, mana: u32) -> ActorVitals {
        ActorVitals {
            health,
            max_health: 150,
            mana,
            max_mana: 55,
            soul: 100,
            harmony: 0,
            serene: false,
        }
    }

    /// Admits, sends a join snapshot that carries `ACTOR_VITALS` (revision 3) and then answers two
    /// casts: `Cast` with its vitals delta, then `Rejected` (the closed server gate) with none.
    async fn send_vitals_join(stream: &mut DuplexStream) -> Result<(), BoxError> {
        read_frame(&mut *stream).await?;
        write_frame(
            &mut *stream,
            &encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                world_id: WorldId::decode(&uuid_v7(2))?,
                channel_id: ChannelId::decode(&uuid_v7(3))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: &[],
            })?,
        )
        .await?;
        let overlay = encode_world_object_overlay_snapshot(&[])
            .map_err(|error| format!("overlay snapshot: {error:?}"))?;
        let vitals_payload = actor_spell::encode_actor_vitals(&vitals(150, 55))
            .map_err(|error| format!("vitals: {error:?}"))?;
        for frame in encode_single_chunk_snapshot(
            1,
            1,
            40,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 5,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial(0),
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 2,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
                DomainSnapshot {
                    domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    revision: 3,
                    snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                    payload: &vitals_payload,
                },
            ],
        )? {
            write_frame(&mut *stream, &frame).await?;
        }
        Ok(())
    }

    async fn cast_server(mut stream: DuplexStream) -> Result<(), BoxError> {
        send_vitals_join(&mut stream).await?;
        let command = read_frame(&mut stream).await?;
        let command = decode_wire_envelope(&command)?.client_command(1)?;
        assert_eq!(
            (command.command_id, command.command_type),
            (7, actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT)
        );
        assert_eq!(
            actor_spell::decode_spell_cast_intent(command.payload),
            Ok(SpellCastIntent {
                spell: NonZeroU32::new(2).ok_or("nonzero")?,
                target: SpellTarget::AttackTarget,
                aim_at_target: true,
            })
        );
        write_frame(
            &mut stream,
            &encode_command_result(
                1,
                41,
                7,
                CommandStatus::Accepted,
                &actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
            )?,
        )
        .await?;
        write_frame(
            &mut stream,
            &encode_state_delta(
                1,
                42,
                actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                3,
                4,
                actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                &actor_spell::encode_actor_vitals(&vitals(150, 30))
                    .map_err(|error| format!("vitals: {error:?}"))?,
            )?,
        )
        .await?;
        read_frame(&mut stream).await?;
        write_frame(
            &mut stream,
            &encode_command_result(
                1,
                43,
                8,
                CommandStatus::Rejected,
                &actor_spell::encode_spell_cast_result(SpellCastDisposition::Rejected),
            )?,
        )
        .await?;
        Ok(())
    }

    #[test]
    fn a_cast_applies_the_vitals_delta_and_a_rejected_cast_changes_nothing() -> Result<(), BoxError>
    {
        block_on(async {
            let (client, server) = tokio::io::duplex(64 * 1024);
            let server = tokio::spawn(cast_server(server));
            let mut session = Session::admit(client, admission()?).await?;
            assert_eq!(session.actor_vitals(), Some(&vitals(150, 55)));
            let spell = NonZeroU32::new(2).ok_or("nonzero")?;
            let cast = session
                .cast_spell(spell, SpellTarget::AttackTarget, true)
                .await?;
            assert_eq!(cast.disposition, SpellCastDisposition::Cast);
            let delta = cast.actor_vitals_delta.ok_or("Cast carries a delta")?;
            assert_eq!((delta.server_sequence, delta.base_revision), (42, 3));
            assert_eq!(session.actor_vitals(), Some(&vitals(150, 30)));
            let rejected = session.cast_spell(spell, SpellTarget::None, false).await?;
            server.await??;
            assert_eq!(rejected.status, CommandStatus::Rejected);
            assert_eq!(rejected.disposition, SpellCastDisposition::Rejected);
            assert_eq!(rejected.actor_vitals_delta, None);
            assert_eq!(session.actor_vitals(), Some(&vitals(150, 30)));
            assert_eq!(session.last_server_sequence(), 43);
            // The session stays usable after a rejection.
            assert_eq!(session.next_command_id(), 9);
            Ok(())
        })?
    }

    /// A peer that closes before `ServerAccepted` fails admission closed: no session.
    async fn vitals_pair() -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
        let (client, mut peer) = tokio::io::duplex(64 * 1024);
        let joining = tokio::spawn(async move {
            send_vitals_join(&mut peer).await?;
            Ok::<_, BoxError>(peer)
        });
        let session = Session::admit(client, admission()?).await?;
        Ok((session, joining.await??))
    }

    fn vitals_frame(
        generation: u64,
        sequence: u64,
        domain: u32,
        base: u64,
        new: u64,
        delta_type: u32,
        payload: &[u8],
    ) -> Result<Vec<u8>, BoxError> {
        Ok(encode_state_delta(
            generation, sequence, domain, base, new, delta_type, payload,
        )?)
    }

    #[test]
    fn idle_applies_sequenced_actor_vitals_before_liveness_ack() -> Result<(), BoxError> {
        block_on(async {
            let (mut session, mut peer) = vitals_pair().await?;
            let payload = actor_spell::encode_actor_vitals(&vitals(140, 55))
                .map_err(|error| format!("vitals: {error:?}"))?;
            write_frame(&mut peer, &vitals_frame(1, 41, 3, 3, 4, 1, &payload)?).await?;
            write_frame(
                &mut peer,
                &oteryn_protocol_oteryn::encode_liveness_probe(1, 9)?,
            )
            .await?;
            session.service_liveness(Duration::from_millis(10)).await?;
            assert_eq!(session.actor_vitals(), Some(&vitals(140, 55)));
            assert_eq!(session.last_server_sequence(), 41);
            assert_eq!(session.vitals_revision, 4);
            let ack = read_frame(&mut peer).await?;
            let ack = decode_wire_envelope(&ack)?.liveness_ack(1)?;
            assert_eq!(ack.last_applied_server_sequence, Some(41));
            Ok::<_, BoxError>(())
        })?
    }

    #[test]
    fn asynchronous_vitals_before_command_result_preserves_mandated_cast_delta()
    -> Result<(), BoxError> {
        block_on(async {
            let (mut session, mut peer) = vitals_pair().await?;
            let responder = tokio::spawn(async move {
                let command_frame = read_frame(&mut peer).await?;
                let command = decode_wire_envelope(&command_frame)?.client_command(1)?;
                let payload = actor_spell::encode_actor_vitals(&vitals(140, 55))
                    .map_err(|error| format!("vitals: {error:?}"))?;
                write_frame(&mut peer, &vitals_frame(1, 41, 3, 3, 4, 1, &payload)?).await?;
                write_frame(
                    &mut peer,
                    &encode_command_result(
                        1,
                        42,
                        command.command_id,
                        CommandStatus::Accepted,
                        &actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
                    )?,
                )
                .await?;
                let paid = actor_spell::encode_actor_vitals(&vitals(145, 30))
                    .map_err(|error| format!("vitals: {error:?}"))?;
                write_frame(&mut peer, &vitals_frame(1, 43, 3, 4, 5, 1, &paid)?).await?;
                Ok::<_, BoxError>(())
            });
            let cast = session
                .cast_spell(
                    NonZeroU32::new(1).ok_or("spell index")?,
                    SpellTarget::None,
                    false,
                )
                .await?;
            let applied = cast.actor_vitals_delta.ok_or("mandatory cast delta")?;
            assert_eq!(
                (
                    applied.server_sequence,
                    applied.base_revision,
                    applied.new_revision
                ),
                (43, 4, 5)
            );
            assert_eq!(session.actor_vitals(), Some(&vitals(145, 30)));
            assert_eq!(session.last_server_sequence(), 43);
            responder.await??;
            Ok::<_, BoxError>(())
        })?
    }

    #[test]
    fn invalid_unsolicited_vitals_poison_idle_and_command_without_advancing_state()
    -> Result<(), BoxError> {
        block_on(async {
            let payload = actor_spell::encode_actor_vitals(&vitals(140, 55))
                .map_err(|error| format!("vitals: {error:?}"))?;
            let mut nonadvancing = vitals_frame(1, 41, 3, 3, 4, 1, &payload)?;
            let revision = nonadvancing
                .windows(4)
                .position(|bytes| bytes == [24, 4, 32, 1])
                .ok_or("revision field in existing encoded envelope")?;
            nonadvancing[revision + 1] = 3;
            // A frame that fails the sequence check records nothing; one with a valid sequence
            // records its sequence before the domain store refuses it. Either poisons the session
            // and leaves the vitals unchanged.
            let cases = [
                ("generation", 40, vitals_frame(2, 41, 3, 3, 4, 1, &payload)?),
                (
                    "stale sequence",
                    40,
                    vitals_frame(1, 40, 3, 3, 4, 1, &payload)?,
                ),
                (
                    "sequence gap",
                    40,
                    vitals_frame(1, 42, 3, 3, 4, 1, &payload)?,
                ),
                ("domain", 41, vitals_frame(1, 41, 1, 3, 4, 1, &payload)?),
                ("delta type", 41, vitals_frame(1, 41, 3, 3, 4, 2, &payload)?),
                ("stale base", 41, vitals_frame(1, 41, 3, 2, 4, 1, &payload)?),
                ("nonadvancing revision", 41, nonadvancing),
                (
                    "malformed vitals",
                    41,
                    vitals_frame(1, 41, 3, 3, 4, 1, &[8, 255])?,
                ),
            ];
            for command_wait in [false, true] {
                for (name, sequence, frame) in &cases {
                    let (mut session, mut peer) = vitals_pair().await?;
                    write_frame(&mut peer, frame).await?;
                    let failed = if command_wait {
                        session
                            .cast_spell(
                                NonZeroU32::new(1).ok_or("index")?,
                                SpellTarget::None,
                                false,
                            )
                            .await
                            .map(|_| ())
                    } else {
                        session.service_liveness(Duration::from_millis(10)).await
                    };
                    assert!(
                        failed.is_err(),
                        "invalid {name}; command_wait={command_wait}"
                    );
                    assert_eq!(session.last_server_sequence(), *sequence, "{name}");
                    assert_eq!(session.vitals_revision, 3, "{name}");
                    assert_eq!(session.actor_vitals(), Some(&vitals(150, 55)), "{name}");
                    assert!(matches!(
                        session.service_liveness(Duration::ZERO).await,
                        Err(SessionError::SessionUnusable)
                    ));
                    let next = session.next_command_id();
                    assert!(matches!(
                        session.step(StepDirection::East).await,
                        Err(SessionError::SessionUnusable)
                    ));
                    assert_eq!(session.next_command_id(), next);
                }
            }
            Ok::<_, BoxError>(())
        })?
    }

    #[test]
    fn expired_idle_window_does_not_start_queued_delta_but_finishes_started_frame()
    -> Result<(), BoxError> {
        block_on(async {
            let (mut session, mut peer) = vitals_pair().await?;
            let payload = actor_spell::encode_actor_vitals(&vitals(140, 55))
                .map_err(|error| format!("vitals: {error:?}"))?;
            write_frame(&mut peer, &vitals_frame(1, 41, 3, 3, 4, 1, &payload)?).await?;
            session.service_liveness(Duration::ZERO).await?;
            assert_eq!(session.last_server_sequence(), 40);
            session.service_liveness(Duration::from_millis(10)).await?;
            assert_eq!(session.last_server_sequence(), 41);
            let body = vitals_frame(1, 42, 3, 4, 5, 1, &payload)?;
            let mut frame = u32::try_from(body.len())?.to_be_bytes().to_vec();
            frame.extend_from_slice(&body);
            peer.write_all(&frame[..1]).await?;
            let finishing = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(30)).await;
                peer.write_all(&frame[1..]).await?;
                Ok::<_, BoxError>(peer)
            });
            session.service_liveness(Duration::from_millis(10)).await?;
            assert_eq!(session.last_server_sequence(), 42);
            assert_eq!(session.vitals_revision, 5);
            let _peer = finishing.await??;
            Ok::<_, BoxError>(())
        })?
    }

    #[test]
    fn a_stream_closed_before_admission_fails_closed() -> Result<(), BoxError> {
        block_on(async {
            let (client, server) = tokio::io::duplex(1024);
            drop(server);
            let result = Session::admit(client, admission()?).await;
            assert!(matches!(result, Err(SessionError::Io(_))));
            Ok(())
        })?
    }
    const CHAT_DOMAIN: u32 = STATE_DOMAIN_CHAT;
    const CHAT_COMMAND: u32 = COMMAND_TYPE_CHAT_INTENT;

    /// Reads the bootstrap (asserting the advertised set), admits with `selected`, and sends a
    /// join snapshot carrying the two core domains plus `extra` (domain, revision) payload `b"snap"`.
    async fn join_peer(
        stream: &mut DuplexStream,
        advertised: &[u32],
        selected: &[u32],
        extra: &[(u32, u64)],
    ) -> Result<(), BoxError> {
        let bootstrap = read_frame(stream).await?;
        assert_eq!(
            decode_wire_envelope(&bootstrap)?
                .client_bootstrap()?
                .supported_capabilities,
            advertised
        );
        write_frame(
            stream,
            &encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                world_id: WorldId::decode(&uuid_v7(2))?,
                channel_id: ChannelId::decode(&uuid_v7(3))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: selected,
            })?,
        )
        .await?;
        let spatial_payload = spatial(0);
        let overlay_payload = encode_world_object_overlay_snapshot(&[])
            .map_err(|error| format!("overlay snapshot: {error:?}"))?;
        let mut domains = vec![
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 5,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial_payload,
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 2,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay_payload,
            },
        ];
        for &(domain_id, revision) in extra {
            domains.push(DomainSnapshot {
                domain_id,
                revision,
                snapshot_type: 1,
                payload: b"snap",
            });
        }
        for frame in encode_single_chunk_snapshot(1, 1, 40, &domains)? {
            write_frame(stream, &frame).await?;
        }
        Ok(())
    }

    fn admission_with(supported: &'static [u32]) -> Result<Admission<'static>, BoxError> {
        Ok(Admission {
            supported_capabilities: supported,
            ..admission()?
        })
    }

    #[test]
    fn a_too_early_step_is_retried_with_backoff_and_needs_capability_13() -> Result<(), BoxError> {
        block_on(async {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                join_peer(&mut server, &[13], &[13], &[]).await?;
                for (id, sequence) in [(7, 41), (8, 42)] {
                    let command = read_frame(&mut server).await?;
                    assert_eq!(
                        decode_wire_envelope(&command)?
                            .client_command(1)?
                            .command_id,
                        id
                    );
                    write_frame(
                        &mut server,
                        &encode_command_result(
                            1,
                            sequence,
                            id,
                            CommandStatus::Rejected,
                            &encode_step_result(StepDisposition::TooEarly),
                        )?,
                    )
                    .await?;
                }
                read_frame(&mut server).await?;
                write_frame(
                    &mut server,
                    &encode_command_result(
                        1,
                        43,
                        9,
                        CommandStatus::Accepted,
                        &encode_step_result(StepDisposition::Blocked),
                    )?,
                )
                .await?;
                Ok::<(), BoxError>(())
            });
            let mut session = Session::admit(client, admission()?).await?;
            assert_eq!(session.selected_capabilities(), &[13]);
            let started = std::time::Instant::now();
            let outcome = session.step_retrying(StepDirection::East).await?;
            peer.await??;
            assert_eq!(outcome.disposition, StepDisposition::Blocked);
            assert_eq!(outcome.command_id, 9);
            assert!(started.elapsed() >= STEP_RETRY_INITIAL_BACKOFF * 3);
            Ok(())
        })?
    }

    #[test]
    fn too_early_without_capability_13_fails_closed() -> Result<(), BoxError> {
        block_on(async {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                join_peer(&mut server, &[13], &[], &[]).await?;
                read_frame(&mut server).await?;
                write_frame(
                    &mut server,
                    &encode_command_result(
                        1,
                        41,
                        7,
                        CommandStatus::Rejected,
                        &encode_step_result(StepDisposition::TooEarly),
                    )?,
                )
                .await?;
                Ok::<(), BoxError>(())
            });
            let mut session = Session::admit(client, admission()?).await?;
            assert!(session.selected_capabilities().is_empty());
            assert!(matches!(
                session.step(StepDirection::East).await,
                Err(SessionError::WorldSpatial(_))
            ));
            peer.await??;
            Ok(())
        })?
    }

    #[test]
    fn a_selection_outside_the_advertised_set_is_refused() -> Result<(), BoxError> {
        block_on(async {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move { join_peer(&mut server, &[13], &[7], &[]).await });
            let result = Session::admit(client, admission()?).await;
            let _ = peer.await?;
            assert!(matches!(
                result,
                Err(SessionError::CapabilityNotRequested(7))
            ));
            Ok(())
        })?
    }

    #[test]
    fn gated_domains_and_commands_route_by_selected_capability() -> Result<(), BoxError> {
        block_on(async {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                join_peer(
                    &mut server,
                    &[7, 13],
                    &[7, 13],
                    &[(CHAT_DOMAIN, 3), (4242, 1)],
                )
                .await?;
                let command = read_frame(&mut server).await?;
                assert_eq!(
                    decode_wire_envelope(&command)?
                        .client_command(1)?
                        .command_type,
                    CHAT_COMMAND
                );
                write_frame(
                    &mut server,
                    &encode_command_result(1, 41, 7, CommandStatus::Accepted, b"ok")?,
                )
                .await?;
                write_frame(
                    &mut server,
                    &encode_state_delta(1, 42, CHAT_DOMAIN, 3, 4, 1, b"delta")?,
                )
                .await?;
                Ok::<(), BoxError>(())
            });
            let mut session = Session::admit(client, admission_with(&[7, 13])?).await?;
            // The unregistered domain 4242 is ignored; the selected chat domain is kept raw.
            let snapshot = session.gated_snapshot(CHAT_DOMAIN).ok_or("chat snapshot")?;
            assert_eq!(
                (snapshot.revision, snapshot.payload.as_slice()),
                (3, &b"snap"[..])
            );
            assert!(session.gated_snapshot(4242).is_none());
            // A command another capability owns is refused before anything is sent.
            assert!(matches!(
                session.gated_command(13, CHAT_COMMAND, b"x").await,
                Err(SessionError::CapabilityNotSelected { capability: 13 })
            ));
            let result = session.gated_command(7, CHAT_COMMAND, b"x").await?;
            assert_eq!(
                (result.command_id, result.payload.as_slice()),
                (7, &b"ok"[..])
            );
            let delta = session.read_gated_delta(CHAT_DOMAIN, 1).await?;
            peer.await??;
            assert_eq!((delta.base_revision, delta.new_revision), (3, 4));
            assert_eq!(delta.payload, b"delta");
            Ok(())
        })?
    }

    #[test]
    fn unselected_gated_commands_and_domains_are_refused() -> Result<(), BoxError> {
        block_on(async {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                join_peer(&mut server, &[13], &[13], &[]).await?;
                Ok::<(), BoxError>(())
            });
            let mut session = Session::admit(client, admission()?).await?;
            peer.await??;
            assert!(matches!(
                session.gated_command(7, CHAT_COMMAND, b"x").await,
                Err(SessionError::CapabilityNotSelected { capability: 7 })
            ));
            assert!(matches!(
                session.read_gated_delta(CHAT_DOMAIN, 1).await,
                Err(SessionError::CapabilityNotSelected { capability: 7 })
            ));
            // Nothing was sent, so the session stays usable.
            assert_eq!(session.next_command_id(), 7);

            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                join_peer(&mut server, &[13], &[13], &[(CHAT_DOMAIN, 3)]).await
            });
            let result = Session::admit(client, admission()?).await;
            let _ = peer.await?;
            assert!(matches!(
                result,
                Err(SessionError::UnselectedDomain {
                    domain_id: CHAT_DOMAIN
                })
            ));
            Ok(())
        })?
    }

    #[test]
    fn a_resume_never_changes_the_selected_set() {
        assert!(check_resume_selection(&[7, 13], &[13, 7]).is_ok());
        assert!(check_resume_selection(&[], &[]).is_ok());
        assert!(matches!(
            check_resume_selection(&[13], &[7, 13]),
            Err(SessionError::ResumeSelectionChanged)
        ));
        assert!(matches!(
            check_resume_selection(&[7, 13], &[13]),
            Err(SessionError::ResumeSelectionChanged)
        ));
    }

    // --- SESSION-PUSH-1: server-initiated deltas, the domain store and the bounded event queue.

    enum Step {
        Send(Vec<u8>),
        ReadCommand,
    }

    /// Joins (core domains only, no selected capability) and plays `script`, then holds the
    /// stream open until the client drops it.
    fn script_peer(
        script: Vec<Step>,
    ) -> (DuplexStream, tokio::task::JoinHandle<Result<(), BoxError>>) {
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            join_peer(&mut server, &[13], &[], &[]).await?;
            for step in script {
                match step {
                    Step::Send(frame) => write_frame(&mut server, &frame).await?,
                    Step::ReadCommand => {
                        read_frame(&mut server).await?;
                    }
                }
            }
            let mut rest = Vec::new();
            let _ = server.read_to_end(&mut rest).await;
            Ok::<(), BoxError>(())
        });
        (client, peer)
    }

    fn result_frame(
        sequence: u64,
        id: u64,
        disposition: StepDisposition,
    ) -> Result<Step, BoxError> {
        Ok(Step::Send(encode_command_result(
            1,
            sequence,
            id,
            CommandStatus::Accepted,
            &encode_step_result(disposition),
        )?))
    }

    fn spatial_push(sequence: u64, base: u64, new: u64, x: i32) -> Result<Step, BoxError> {
        Ok(Step::Send(encode_state_delta(
            1,
            sequence,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            base,
            new,
            DELTA_TYPE_WORLD_SPATIAL_V1,
            &spatial(x),
        )?))
    }

    fn vitals_push(sequence: u64, base: u64, new: u64, health: u32) -> Result<Step, BoxError> {
        vitals_push_typed(
            sequence,
            base,
            new,
            health,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
        )
    }

    fn vitals_push_typed(
        sequence: u64,
        base: u64,
        new: u64,
        health: u32,
        delta_type: u32,
    ) -> Result<Step, BoxError> {
        Ok(Step::Send(encode_state_delta(
            1,
            sequence,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            base,
            new,
            delta_type,
            &actor_spell::encode_actor_vitals(&vitals(health, 55))
                .map_err(|error| format!("vitals: {error:?}"))?,
        )?))
    }

    fn door_entry(revision: u64) -> WireOverlayEntry {
        WireOverlayEntry {
            content_generation: CONTENT_GENERATION,
            placement: b"oteryn:cell/entry-door".to_vec(),
            state: b"oteryn:reference.state.open".to_vec(),
            revision,
        }
    }

    fn overlay_push(sequence: u64, base: u64, new: u64) -> Result<Step, BoxError> {
        Ok(Step::Send(encode_state_delta(
            1,
            sequence,
            STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            base,
            new,
            world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
            &world_object::encode_world_object_overlay_delta(&door_entry(new))
                .map_err(|error| format!("overlay delta: {error:?}"))?,
        )?))
    }

    fn vitals_event(sequence: u64, base: u64, new: u64, health: u32) -> SessionEvent {
        SessionEvent::ActorVitals(AppliedDelta {
            server_sequence: sequence,
            base_revision: base,
            new_revision: new,
            value: vitals(health, 55),
        })
    }

    fn spatial_event(sequence: u64, base: u64, new: u64, x: i32) -> SessionEvent {
        SessionEvent::WorldSpatial(AppliedDelta {
            server_sequence: sequence,
            base_revision: base,
            new_revision: new,
            value: WireSpatialObservation {
                content_generation: CONTENT_GENERATION,
                actor_position: ActorPosition { x, y: 0, floor: 0 },
            },
        })
    }

    #[test]
    fn a_pushed_vitals_delta_during_idle_applies_and_is_queued_once() -> Result<(), BoxError> {
        block_on(async {
            let (client, peer) = script_peer(vec![vitals_push(41, 0, 1, 120)?]);
            let mut session = Session::admit(client, admission()?).await?;
            session.service_liveness(Duration::from_millis(100)).await?;
            assert_eq!(session.actor_vitals(), Some(&vitals(120, 55)));
            assert_eq!(session.last_server_sequence(), 41);
            assert_eq!(session.take_events(), vec![vitals_event(41, 0, 1, 120)]);
            assert!(session.take_events().is_empty());
            drop(session);
            peer.await??;
            Ok(())
        })?
    }

    #[test]
    fn a_delta_pushed_before_the_result_applies_and_the_steps_own_delta_still_applies()
    -> Result<(), BoxError> {
        block_on(async {
            let (client, peer) = script_peer(vec![
                Step::ReadCommand,
                vitals_push(41, 0, 1, 120)?,
                result_frame(42, 7, StepDisposition::Moved)?,
                spatial_push(43, 5, 6, 1)?,
            ]);
            let mut session = Session::admit(client, admission()?).await?;
            let outcome = session.step(StepDirection::East).await?;
            assert_eq!(outcome.result_server_sequence, 42);
            let delta = outcome
                .world_spatial_delta
                .ok_or("Moved carries its delta")?;
            assert_eq!((delta.server_sequence, delta.new_revision), (43, 6));
            assert_eq!(session.take_events(), vec![vitals_event(41, 0, 1, 120)]);
            drop(session);
            peer.await??;
            Ok(())
        })?
    }

    #[test]
    fn a_delta_between_the_result_and_the_steps_own_delta_applies_as_pushed() -> Result<(), BoxError>
    {
        block_on(async {
            let (client, peer) = script_peer(vec![
                Step::ReadCommand,
                result_frame(41, 7, StepDisposition::Moved)?,
                vitals_push(42, 0, 1, 120)?,
                spatial_push(43, 5, 6, 1)?,
            ]);
            let mut session = Session::admit(client, admission()?).await?;
            let outcome = session.step(StepDirection::East).await?;
            let delta = outcome
                .world_spatial_delta
                .ok_or("Moved carries its delta")?;
            assert_eq!((delta.server_sequence, delta.base_revision), (43, 5));
            assert_eq!(session.world_spatial().actor_position.x, 1);
            assert_eq!(session.take_events(), vec![vitals_event(42, 0, 1, 120)]);
            drop(session);
            peer.await??;
            Ok(())
        })?
    }

    #[test]
    fn a_cast_claims_its_vitals_delta_after_a_pushed_spatial_delta() -> Result<(), BoxError> {
        block_on(async {
            let (client, peer) = script_peer(vec![
                Step::ReadCommand,
                Step::Send(encode_command_result(
                    1,
                    41,
                    7,
                    CommandStatus::Accepted,
                    &actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
                )?),
                spatial_push(42, 5, 6, 1)?,
                vitals_push(43, 0, 1, 90)?,
            ]);
            let mut session = Session::admit(client, admission()?).await?;
            let spell = NonZeroU32::new(2).ok_or("nonzero")?;
            let cast = session.cast_spell(spell, SpellTarget::None, false).await?;
            let delta = cast.actor_vitals_delta.ok_or("Cast carries its delta")?;
            assert_eq!((delta.server_sequence, delta.value), (43, vitals(90, 55)));
            assert_eq!(session.take_events(), vec![spatial_event(42, 5, 6, 1)]);
            drop(session);
            peer.await??;
            Ok(())
        })?
    }

    #[test]
    fn two_consecutive_spatial_deltas_end_the_step_at_the_first_and_nothing_is_lost()
    -> Result<(), BoxError> {
        block_on(async {
            let (client, peer) = script_peer(vec![
                Step::ReadCommand,
                result_frame(41, 7, StepDisposition::Moved)?,
                spatial_push(42, 5, 6, 1)?,
                spatial_push(43, 6, 7, 2)?,
                // The second step's exchange reads the leftover delta before its own result.
                Step::ReadCommand,
                result_frame(44, 8, StepDisposition::Blocked)?,
            ]);
            let mut session = Session::admit(client, admission()?).await?;
            let first = session.step(StepDirection::East).await?;
            let delta = first.world_spatial_delta.ok_or("Moved carries its delta")?;
            assert_eq!(delta.new_revision, 6);
            assert_eq!(session.last_server_sequence(), 42);
            assert!(session.take_events().is_empty());
            let second = session.step(StepDirection::East).await?;
            assert_eq!(second.disposition, StepDisposition::Blocked);
            assert_eq!(session.world_spatial().actor_position.x, 2);
            assert_eq!(session.take_events(), vec![spatial_event(43, 6, 7, 2)]);
            drop(session);
            peer.await??;
            Ok(())
        })?
    }

    #[test]
    fn a_leftover_delta_applies_on_the_next_idle_read() -> Result<(), BoxError> {
        block_on(async {
            let (client, peer) = script_peer(vec![
                Step::ReadCommand,
                result_frame(41, 7, StepDisposition::Moved)?,
                spatial_push(42, 5, 6, 1)?,
                spatial_push(43, 6, 7, 2)?,
            ]);
            let mut session = Session::admit(client, admission()?).await?;
            session.step(StepDirection::East).await?;
            session.service_liveness(Duration::from_millis(100)).await?;
            assert_eq!(session.take_events(), vec![spatial_event(43, 6, 7, 2)]);
            drop(session);
            peer.await??;
            Ok(())
        })?
    }

    #[test]
    fn a_committed_use_returns_at_its_result_and_its_domain_2_delta_arrives_as_an_event()
    -> Result<(), BoxError> {
        block_on(async {
            let committed = encode_command_result(
                1,
                41,
                7,
                CommandStatus::Accepted,
                &world_object::encode_use_result(UseDisposition::Committed),
            )?;
            let (client, peer) = script_peer(vec![
                Step::ReadCommand,
                Step::Send(committed),
                overlay_push(42, 2, 3)?,
            ]);
            let mut session = Session::admit(client, admission()?).await?;
            let outcome = session.use_object(b"oteryn:cell/entry-door", 2).await?;
            assert_eq!(outcome.disposition, UseDisposition::Committed);
            assert_eq!(outcome.world_object_overlay_delta, None);
            assert_eq!(outcome.world_spatial_delta, None);
            // Returned at the result: the delta behind it is still unread.
            assert_eq!(session.last_server_sequence(), 41);
            assert!(session.world_object_overlay().is_empty());
            session.service_liveness(Duration::from_millis(100)).await?;
            assert_eq!(session.world_object_overlay().len(), 1);
            let events = session.take_events();
            assert!(matches!(
                events.as_slice(),
                [SessionEvent::WorldObjectOverlay(delta)]
                    if (delta.server_sequence, delta.base_revision, delta.new_revision) == (42, 2, 3)
            ));
            drop(session);
            peer.await??;
            Ok(())
        })?
    }

    #[test]
    fn a_committed_use_with_no_delta_returns_at_its_result() -> Result<(), BoxError> {
        block_on(async {
            let committed = encode_command_result(
                1,
                41,
                7,
                CommandStatus::Accepted,
                &world_object::encode_use_result(UseDisposition::Committed),
            )?;
            let (client, peer) = script_peer(vec![Step::ReadCommand, Step::Send(committed)]);
            let mut session = Session::admit(client, admission()?).await?;
            let outcome = session.use_object(b"oteryn:cell/entry-door", 2).await?;
            assert_eq!(outcome.disposition, UseDisposition::Committed);
            assert_eq!(outcome.world_object_overlay_delta, None);
            assert!(session.take_events().is_empty());
            drop(session);
            peer.await??;
            Ok(())
        })?
    }

    /// Each of these frames fails closed on an idle read and leaves the session unusable.
    async fn assert_idle_push_poisons(
        push: Step,
        check: impl FnOnce(&SessionError) -> bool,
    ) -> Result<(), BoxError> {
        let (client, peer) = script_peer(vec![push]);
        let mut session = Session::admit(client, admission()?).await?;
        let error = session
            .service_liveness(Duration::from_millis(200))
            .await
            .err()
            .ok_or("the push must fail closed")?;
        assert!(check(&error), "{error:?}");
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(SessionError::SessionUnusable)
        ));
        drop(session);
        peer.await??;
        Ok(())
    }

    #[test]
    fn a_base_mismatch_an_unselected_domain_and_an_unregistered_type_poison_the_session()
    -> Result<(), BoxError> {
        block_on(async {
            assert_idle_push_poisons(spatial_push(41, 4, 6, 1)?, |error| {
                matches!(
                    error,
                    SessionError::StateRevisionMismatch {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        expected_base: 5,
                        actual_base: 4
                    }
                )
            })
            .await?;
            assert_idle_push_poisons(
                Step::Send(encode_state_delta(1, 41, CHAT_DOMAIN, 0, 1, 1, b"line")?),
                |error| {
                    matches!(
                        error,
                        SessionError::UnselectedDomain {
                            domain_id: CHAT_DOMAIN
                        }
                    )
                },
            )
            .await?;
            assert_idle_push_poisons(vitals_push_typed(41, 0, 1, 90, 2)?, |error| {
                matches!(
                    error,
                    SessionError::UnregisteredDeltaType {
                        domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                        delta_type: 2
                    }
                )
            })
            .await?;
            // A domain the session keeps no store for is not silently dropped either.
            assert_idle_push_poisons(
                Step::Send(encode_state_delta(1, 41, 4242, 0, 1, 1, b"x")?),
                |error| {
                    matches!(
                        error,
                        SessionError::UnsupportedPushedDomain { domain_id: 4242 }
                    )
                },
            )
            .await
        })?
    }

    #[test]
    fn a_pushed_delta_mismatch_during_an_exchange_poisons_the_session() -> Result<(), BoxError> {
        block_on(async {
            let (client, peer) = script_peer(vec![
                Step::ReadCommand,
                spatial_push(41, 4, 6, 1)?,
                result_frame(42, 7, StepDisposition::Blocked)?,
            ]);
            let mut session = Session::admit(client, admission()?).await?;
            assert!(matches!(
                session.step(StepDirection::East).await,
                Err(SessionError::StateRevisionMismatch { .. })
            ));
            assert!(matches!(
                session.step(StepDirection::East).await,
                Err(SessionError::SessionUnusable)
            ));
            drop(session);
            peer.await??;
            Ok(())
        })?
    }

    fn queue_script(count: usize) -> Result<Vec<Step>, BoxError> {
        (0..count)
            .map(|index| {
                let index = u64::try_from(index)?;
                vitals_push(41 + index, index, index + 1, 100)
            })
            .collect()
    }

    #[test]
    fn the_event_queue_holds_exactly_the_limit_and_fails_closed_one_past_it() -> Result<(), BoxError>
    {
        block_on(async {
            let (client, peer) = script_peer(queue_script(MAX_QUEUED_EVENTS)?);
            let mut session = Session::admit(client, admission()?).await?;
            session.service_liveness(Duration::from_millis(500)).await?;
            assert_eq!(session.take_events().len(), MAX_QUEUED_EVENTS);
            drop(session);
            peer.await??;

            let (client, peer) = script_peer(queue_script(MAX_QUEUED_EVENTS + 1)?);
            let mut session = Session::admit(client, admission()?).await?;
            assert!(matches!(
                session.service_liveness(Duration::from_millis(500)).await,
                Err(SessionError::EventQueueOverflow {
                    limit: MAX_QUEUED_EVENTS
                })
            ));
            assert!(matches!(
                session.step(StepDirection::East).await,
                Err(SessionError::SessionUnusable)
            ));
            drop(session);
            peer.await??;
            Ok(())
        })?
    }
}
