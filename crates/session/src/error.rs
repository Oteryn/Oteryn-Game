//! Session error.
use super::*;

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
    AdmissionQueued {
        code: u32,
        queue: AdmissionQueue,
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
    /// A complete committed scope switch canceled this source-session request.
    /// The destination remains usable; do not retry the request automatically.
    ActivityScopeChanged,
    /// The server replaced the source session while its local request was in flight.
    /// No outcome is inferred and the fresh session remains usable at command one.
    ScopeChanged,
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
            Self::ActivityScopeChanged => {
                write!(formatter, "activity scope changed; source request canceled")
            }
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
            Self::AdmissionQueued { code, queue } => write!(
                formatter,
                "admission queued with code {code}, position {}",
                queue.position
            ),
            Self::AdmissionRefused { code } => {
                write!(formatter, "admission refused with code {code}")
            }
            Self::UnexpectedMessage { expected, actual } => write!(
                formatter,
                "expected {expected:?} next, server sent {actual:?}"
            ),
            Self::ConnectionGenerationMismatch { expected, actual } => write!(
                formatter,
                "connection generation mismatch: admitted session is {expected}, \
                frame carried {actual}"
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
                "assembled snapshot length mismatch: SnapshotBegin declared \
                {expected} bytes, chunks totaled {actual}"
            ),
            Self::MissingDomain(domain_id) => {
                write!(formatter, "join snapshot did not carry domain {domain_id}")
            }
            Self::Timeout(stage) => write!(formatter, "timed out waiting for {stage}"),
            Self::ScopeChanged => write!(
                formatter,
                "the activity scope changed; the source request was cancelled"
            ),
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
                "domain {domain_id} delta base revision mismatch: applied \
                {expected_base}, delta based on {actual_base}"
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
                "overlay delta new_revision {new_revision} differs from its entry \
                revision {entry_revision}"
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
