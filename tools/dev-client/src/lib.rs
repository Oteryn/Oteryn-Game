//! Dev/qualification-only native Oteryn Game client (ADR-0011 §6, ADR-0020 lane N1): a thin
//! harness over the transport-neutral session crate (`oteryn-session`) and the TLS/TCP adapter
//! (`oteryn-session-tcp`). It connects to a real game server over rustls TLS 1.3 with ALPN
//! `oteryn-game/1` (the adapter), admits with a caller-supplied grant (the same `ClientBootstrap`
//! mechanism the shipped server accepts), decodes the join snapshot's state domains, and then
//! drives the admitted session with the two registered gameplay commands: `step`
//! (FIRST-CONTROL-WIRE-V1 command type 1) and `use_object` (USE-WIRE-V1 command type 2), decoding
//! each `CommandResult` disposition and the server-sequenced `WORLD_SPATIAL` /
//! `WORLD_OBJECT_OVERLAY` deltas that follow it (the session crate).
//!
//! This crate holds no codec and no session logic of its own: it keeps the public
//! `JoinRequest`/`connect_and_join`/`connect_session`/`DevClientSession`/`DevClientError` test
//! surface and delegates to the two crates above.
//!
//! Not a production client entry. The shipped native client stays fail-closed behind
//! `PreNativeProtocol` (ADR-0011 §3/§5); this dev harness is the explicit exception ADR-0011 §6
//! allows, and it is kept out of both production closures (`oteryn-client`, `oteryn-game-server`)
//! by `workspace-boundaries.toml`.

use oteryn_protocol_oteryn::actor_spell::{ActorSpellError, ActorVitals, SpellTarget};
use oteryn_protocol_oteryn::item_view::ItemViewWireError;
use oteryn_protocol_oteryn::logout::LogoutWireError;
use oteryn_protocol_oteryn::world_object::{self, WorldObjectOverlayEntry};
use oteryn_protocol_oteryn::world_spatial::{self, StepDirection, WorldSpatialObservation};
use oteryn_protocol_oteryn::{CharacterId, FoundationProtocolError, MessageType};
use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, SessionError};
pub use oteryn_session::{
    AppliedDelta, CastOutcome, CharacterInventory, ChatDisposition, ChatIntent, ChatLine, ChatLog,
    ChatOutcome, ChatRoom, ChatRoomSet, ChatSpeaker, ChatSpeechMode, ChatWireError, CommandOutcome,
    DuplicateOutcome, EntityDetail, EntityKind, EntityRef, GuildChatLine, GuildRoomState,
    ItemEntry, ItemHandle, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome,
    ItemMoveOutcomeResult, JoinSnapshot, MAX_CHARACTER_INVENTORY_ITEMS, MAX_CHAT_LOG_LINES,
    MAX_CHAT_NAME_BYTES, MAX_CHAT_TEXT_BYTES, MAX_CHAT_WAIT_SECONDS, MAX_OPEN_CONTAINER_ENTRIES,
    MAX_QUEUED_EVENTS, OpenContainer, SessionEvent, StepOutcome, UseOutcome, WorldEntities,
    WorldSpatialEntitiesDelta, WorldSpatialEntity,
};
use oteryn_session_tcp::{TcpAdapterError, TcpConnect, TcpTlsStream};
use rustls::pki_types::CertificateDer;
use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::time::Duration;

/// Everything `connect_and_join` needs to admit with one grant and read its join snapshot.
#[derive(Debug, Clone, Copy)]
pub struct JoinRequest<'a> {
    /// The server's real loopback (or routable) TCP address.
    pub address: SocketAddr,
    /// The TLS server name to verify the presented certificate against.
    pub server_name: &'a str,
    /// The single trust root the connection's certificate must chain to.
    pub root_certificate: &'a CertificateDer<'static>,
    /// FND-02 `ClientBootstrap.schema_revision`. Any nonzero value the server accepts.
    pub schema_revision: u32,
    pub character_id: CharacterId,
    /// The signed admission grant (e.g. a WP5 fixture grant JWT), as the wire's
    /// `admission_material` bytes.
    pub admission_material: &'a [u8],
    pub client_build_id: &'a str,
    /// Bounds the TCP connect, the TLS handshake, and each individual frame read of the join
    /// sequence (`ServerAccepted`, `SnapshotBegin`, `SnapshotChunk`, `SnapshotCommit`) —
    /// consistent with the neighbouring seam-qualification TLS-connect (30s) and
    /// read-to-completion (20s) timeouts in `qualification.rs`. A stalled server fails with
    /// `DevClientError::Timeout` naming the stage, rather than hanging.
    pub deadline: Duration,
}

/// The dev client's error: the TLS/TCP adapter's failures plus every `SessionError` variant,
/// flattened so the dev client's public matching surface is unchanged by the extraction.
#[derive(Debug)]
pub enum DevClientError {
    Tls(rustls::Error),
    Io(io::Error),
    InvalidServerName,
    /// The negotiated TLS ALPN protocol was not exactly `oteryn-game/1`, including when no ALPN
    /// was negotiated at all (FND-02: an ALPN mismatch terminates the connection).
    AlpnMismatch,
    /// A `LivenessProbe`'s `probe_id` did not strictly exceed the last one this session acked
    /// (FND-02 §17: probe ids are monotonic and never reused within a connection generation).
    ProbeIdNotAdvancing {
        last: u64,
        received: u64,
    },
    Protocol(FoundationProtocolError),
    WorldSpatial(world_spatial::WorldSpatialError),
    WorldObject(world_object::WorldObjectError),
    ActorSpell(ActorSpellError),
    /// A chat intent or chat payload failed its codec.
    Chat(ChatWireError),
    /// The attack codec failed (the same error type as chat, kept distinct by the variant).
    Attack(ChatWireError),
    /// A capability-4 item payload failed its codec.
    ItemView(ItemViewWireError),
    /// A capability-21 logout result failed its codec.
    Logout(LogoutWireError),
    /// The server closed, or replied with something other than `ServerAccepted`, before
    /// admission completed.
    NotAdmitted(MessageType),
    /// The server refused admission with an N8 code (1100..=1199).
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
    /// A join-snapshot domain entry named state domain 1 (`WORLD_SPATIAL_VISIBILITY`) or 2
    /// (`WORLD_OBJECT_OVERLAY`) with a `snapshot_type` other than the one registered (1).
    UnregisteredSnapshotType {
        domain_id: u32,
        snapshot_type: u32,
    },
    /// A `SnapshotChunk`'s `chunk_index` did not equal the index expected next.
    ChunkIndexMismatch {
        expected: u32,
        actual: u32,
    },
    /// The summed chunk `data` bytes did not equal `SnapshotBegin`'s `total_encoded_bytes`.
    AssembledLengthMismatch {
        expected: u64,
        actual: u64,
    },
    /// The join snapshot did not carry a domain this client needed.
    MissingDomain(u32),
    /// The TCP connect, TLS handshake, or one frame read/write did not complete within
    /// `JoinRequest::deadline`.
    Timeout(&'static str),
    /// A command was attempted on a session that an earlier exchange failed.
    SessionUnusable,
    /// The next `CommandId` would overflow `u64`.
    CommandIdExhausted,
    /// A `CommandResult` named a `command_id` other than the one just sent.
    CommandIdMismatch {
        expected: u64,
        actual: u64,
    },
    /// A server-sequenced frame did not carry exactly the previous applied sequence plus one.
    ServerSequenceMismatch {
        expected: u64,
        actual: u64,
    },
    /// A `StateDelta` named a domain other than the one the disposition promised.
    UnexpectedDomain {
        expected: u32,
        actual: u32,
    },
    /// A `StateDelta` named a `delta_type` other than the one registered for its domain.
    UnregisteredDeltaType {
        domain_id: u32,
        delta_type: u32,
    },
    /// A `StateDelta`'s `base_revision` was not the domain revision last applied.
    StateRevisionMismatch {
        domain_id: u32,
        expected_base: u64,
        actual_base: u64,
    },
    /// A delta's `content_generation` was not the one the join snapshot loaded.
    ContentGenerationMismatch {
        domain_id: u32,
    },
    /// A `WORLD_OBJECT_OVERLAY` delta's entry `revision` differed from the delta's `new_revision`.
    OverlayEntryRevisionMismatch {
        new_revision: u64,
        entry_revision: u64,
    },
    /// A `CommandResult`'s status did not pair with its typed disposition.
    InconsistentCommandResult {
        command_id: u64,
        status: oteryn_protocol_oteryn::CommandStatus,
    },
    /// A duplicate-status `CommandResult` named a `CommandId` this session never sent.
    DuplicateForUnsentCommand {
        command_id: u64,
    },
    CapabilityNotRequested(u32),
    ResumeSelectionChanged,
    CapabilityNotSelected {
        capability: u32,
    },
    UnselectedDomain {
        domain_id: u32,
    },
    /// A pushed `StateDelta` named a domain the session keeps no store for.
    UnsupportedPushedDomain {
        domain_id: u32,
    },
    /// A capability-6 entity delta contradicted the stored entities.
    EntityStoreInconsistent {
        reason: &'static str,
    },
    /// More pushed deltas than `MAX_QUEUED_EVENTS` were left undrained.
    EventQueueOverflow {
        limit: usize,
    },
}

/// Every `SessionError` arm is listed (no wildcard), so a variant added to the session crate
/// fails this build until the dev client maps it.
impl From<SessionError> for DevClientError {
    fn from(error: SessionError) -> Self {
        match error {
            SessionError::Io(error) => Self::Io(error),
            SessionError::ProbeIdNotAdvancing { last, received } => {
                Self::ProbeIdNotAdvancing { last, received }
            }
            SessionError::Protocol(error) => Self::Protocol(error),
            SessionError::WorldSpatial(error) => Self::WorldSpatial(error),
            SessionError::WorldObject(error) => Self::WorldObject(error),
            SessionError::ActorSpell(error) => Self::ActorSpell(error),
            SessionError::Chat(error) => Self::Chat(error),
            SessionError::Attack(error) => Self::Attack(error),
            SessionError::ItemView(error) => Self::ItemView(error),
            SessionError::Logout(error) => Self::Logout(error),
            SessionError::NotAdmitted(message_type) => Self::NotAdmitted(message_type),
            SessionError::AdmissionRefused { code } => Self::AdmissionRefused { code },
            SessionError::UnexpectedMessage { expected, actual } => {
                Self::UnexpectedMessage { expected, actual }
            }
            SessionError::ConnectionGenerationMismatch { expected, actual } => {
                Self::ConnectionGenerationMismatch { expected, actual }
            }
            SessionError::SnapshotIdMismatch { expected, actual } => {
                Self::SnapshotIdMismatch { expected, actual }
            }
            SessionError::UnregisteredSnapshotType {
                domain_id,
                snapshot_type,
            } => Self::UnregisteredSnapshotType {
                domain_id,
                snapshot_type,
            },
            SessionError::ChunkIndexMismatch { expected, actual } => {
                Self::ChunkIndexMismatch { expected, actual }
            }
            SessionError::AssembledLengthMismatch { expected, actual } => {
                Self::AssembledLengthMismatch { expected, actual }
            }
            SessionError::MissingDomain(domain_id) => Self::MissingDomain(domain_id),
            SessionError::Timeout(stage) => Self::Timeout(stage),
            SessionError::SessionUnusable => Self::SessionUnusable,
            SessionError::CommandIdExhausted => Self::CommandIdExhausted,
            SessionError::CommandIdMismatch { expected, actual } => {
                Self::CommandIdMismatch { expected, actual }
            }
            SessionError::ServerSequenceMismatch { expected, actual } => {
                Self::ServerSequenceMismatch { expected, actual }
            }
            SessionError::UnexpectedDomain { expected, actual } => {
                Self::UnexpectedDomain { expected, actual }
            }
            SessionError::UnregisteredDeltaType {
                domain_id,
                delta_type,
            } => Self::UnregisteredDeltaType {
                domain_id,
                delta_type,
            },
            SessionError::StateRevisionMismatch {
                domain_id,
                expected_base,
                actual_base,
            } => Self::StateRevisionMismatch {
                domain_id,
                expected_base,
                actual_base,
            },
            SessionError::ContentGenerationMismatch { domain_id } => {
                Self::ContentGenerationMismatch { domain_id }
            }
            SessionError::OverlayEntryRevisionMismatch {
                new_revision,
                entry_revision,
            } => Self::OverlayEntryRevisionMismatch {
                new_revision,
                entry_revision,
            },
            SessionError::InconsistentCommandResult { command_id, status } => {
                Self::InconsistentCommandResult { command_id, status }
            }
            SessionError::DuplicateForUnsentCommand { command_id } => {
                Self::DuplicateForUnsentCommand { command_id }
            }
            SessionError::CapabilityNotRequested(capability) => {
                Self::CapabilityNotRequested(capability)
            }
            SessionError::ResumeSelectionChanged => Self::ResumeSelectionChanged,
            SessionError::CapabilityNotSelected { capability } => {
                Self::CapabilityNotSelected { capability }
            }
            SessionError::UnselectedDomain { domain_id } => Self::UnselectedDomain { domain_id },
            SessionError::UnsupportedPushedDomain { domain_id } => {
                Self::UnsupportedPushedDomain { domain_id }
            }
            SessionError::EntityStoreInconsistent { reason } => {
                Self::EntityStoreInconsistent { reason }
            }
            SessionError::EventQueueOverflow { limit } => Self::EventQueueOverflow { limit },
        }
    }
}

impl From<TcpAdapterError> for DevClientError {
    fn from(error: TcpAdapterError) -> Self {
        match error {
            TcpAdapterError::Tls(error) => Self::Tls(error),
            TcpAdapterError::Io(error) => Self::Io(error),
            TcpAdapterError::InvalidServerName => Self::InvalidServerName,
            TcpAdapterError::AlpnMismatch => Self::AlpnMismatch,
            TcpAdapterError::Timeout(stage) => Self::Timeout(stage),
        }
    }
}

impl From<io::Error> for DevClientError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<FoundationProtocolError> for DevClientError {
    fn from(error: FoundationProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<world_spatial::WorldSpatialError> for DevClientError {
    fn from(error: world_spatial::WorldSpatialError) -> Self {
        Self::WorldSpatial(error)
    }
}

impl From<world_object::WorldObjectError> for DevClientError {
    fn from(error: world_object::WorldObjectError) -> Self {
        Self::WorldObject(error)
    }
}

impl fmt::Display for DevClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tls(error) => write!(formatter, "TLS setup failed: {error}"),
            Self::Io(error) => write!(formatter, "transport I/O failed: {error}"),
            Self::InvalidServerName => write!(formatter, "invalid TLS server name"),
            Self::AlpnMismatch => write!(
                formatter,
                "TLS ALPN mismatch: server did not negotiate oteryn-game/1"
            ),
            Self::Protocol(error) => write!(formatter, "FND-02 protocol error: {error}"),
            Self::Chat(error) => write!(formatter, "chat codec failed: {error:?}"),
            Self::Attack(error) => write!(formatter, "attack codec failed: {error:?}"),
            Self::ItemView(error) => write!(formatter, "item codec failed: {error:?}"),
            Self::Logout(error) => write!(formatter, "logout codec failed: {error:?}"),
            Self::WorldSpatial(error) => {
                write!(formatter, "WORLD_SPATIAL decode failed: {error:?}")
            }
            Self::ActorSpell(error) => {
                write!(
                    formatter,
                    "ACTOR_SPELL/ACTOR_VITALS decode failed: {error:?}"
                )
            }
            Self::WorldObject(error) => {
                write!(formatter, "WORLD_OBJECT_OVERLAY decode failed: {error:?}")
            }
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
            Self::UnsupportedPushedDomain { domain_id } => write!(
                formatter,
                "server pushed a delta of domain {domain_id}, which this session keeps no store for"
            ),
            Self::EntityStoreInconsistent { reason } => write!(
                formatter,
                "entity delta contradicts the stored entities: {reason}"
            ),
            Self::EventQueueOverflow { limit } => write!(
                formatter,
                "more than {limit} pushed deltas were left undrained"
            ),
            Self::UnselectedDomain { domain_id } => write!(
                formatter,
                "server sent domain {domain_id} of an unselected capability"
            ),
        }
    }
}

impl StdError for DevClientError {}
/// Connects to `request.address` over rustls TLS 1.3 with ALPN `oteryn-game/1` (the TCP adapter
/// rejects any other or absent negotiated ALPN before sending anything), admits with the
/// request's grant and decodes the join snapshot (see `Session::admit` for every check it makes).
/// The TCP connect, the TLS handshake, and every frame read are bounded by `request.deadline`.
///
/// Returns the decoded snapshot only and drops the connection; use `connect_session` to keep the
/// admitted session and drive it with `step`/`use_object`.
pub async fn connect_and_join(request: JoinRequest<'_>) -> Result<JoinSnapshot, DevClientError> {
    connect_session(request)
        .await
        .map(DevClientSession::into_join_snapshot)
}

/// `connect_and_join`'s full join, but keeps the admitted connection: see
/// `oteryn_session::Session` for the sequence, revision and liveness discipline it carries.
pub async fn connect_session(request: JoinRequest<'_>) -> Result<DevClientSession, DevClientError> {
    let stream = oteryn_session_tcp::connect(TcpConnect {
        address: request.address,
        server_name: request.server_name,
        root_certificate: request.root_certificate,
        deadline: request.deadline,
    })
    .await?;
    let session = Session::admit(
        stream,
        Admission {
            appearance_items: None,
            schema_revision: request.schema_revision,
            character_id: request.character_id,
            admission_material: request.admission_material,
            client_build_id: request.client_build_id,
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: request.deadline,
        },
    )
    .await?;
    Ok(DevClientSession { session })
}

/// An admitted, joined game session over the TLS/TCP adapter: `oteryn_session::Session` with the
/// dev client's error type. Dropping it closes the connection.
#[derive(Debug)]
pub struct DevClientSession {
    session: Session<TcpTlsStream>,
}

impl DevClientSession {
    /// The join snapshot exactly as decoded, before any command.
    pub fn join_snapshot(&self) -> &JoinSnapshot {
        self.session.join_snapshot()
    }

    pub fn into_join_snapshot(self) -> JoinSnapshot {
        self.session.into_join_snapshot()
    }

    /// The own-actor position after every delta applied so far.
    pub fn world_spatial(&self) -> &WorldSpatialObservation {
        self.session.world_spatial()
    }

    /// The visible entities after every delta applied so far; `Some` exactly when the server
    /// selected capability 6 `WORLD_SPATIAL_ENTITIES`.
    pub fn world_entities(&self) -> Option<&WorldEntities> {
        self.session.world_entities()
    }

    /// The overlay entries after every delta applied so far.
    pub fn world_object_overlay(&self) -> &[WorldObjectOverlayEntry] {
        self.session.world_object_overlay()
    }

    /// The open rooms and the last `MAX_CHAT_LOG_LINES` lines; `Some` exactly when the server
    /// selected capability 7 `CHAT_V1`.
    pub fn chat_log(&self) -> Option<&ChatLog> {
        self.session.chat_log()
    }

    /// The inventory after every delta applied so far; `Some` exactly when the server selected
    /// capability 4 `ITEM_VIEW_MOVE_V1`.
    pub fn inventory(&self) -> Option<&CharacterInventory> {
        self.session.inventory()
    }

    /// The open corpse after every delta applied so far; `Some` exactly when capability 4 is
    /// selected.
    pub fn open_container(&self) -> Option<&OpenContainer> {
        self.session.open_container()
    }

    /// The own-actor vitals after every delta applied so far, if the server has sent any.
    pub fn actor_vitals(&self) -> Option<&ActorVitals> {
        self.session.actor_vitals()
    }

    /// The `CommandId` the next `step`/`use_object`/`cast_spell` will send.
    pub fn next_command_id(&self) -> u64 {
        self.session.next_command_id()
    }

    /// The `server_sequence` of the last frame applied (initially the snapshot's target).
    pub fn last_server_sequence(&self) -> u64 {
        self.session.last_server_sequence()
    }

    /// Returns (and clears) the duplicate-status results received while reading command results.
    pub fn take_duplicate_outcomes(&mut self) -> Vec<DuplicateOutcome> {
        self.session.take_duplicate_outcomes()
    }

    /// Returns (and clears) the pushed deltas applied since the last call. See
    /// `Session::take_events`.
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        self.session.take_events()
    }

    /// See `Session::service_liveness`.
    pub async fn service_liveness(&mut self, duration: Duration) -> Result<(), DevClientError> {
        Ok(self.session.service_liveness(duration).await?)
    }

    /// See `Session::step`.
    pub async fn step(&mut self, direction: StepDirection) -> Result<StepOutcome, DevClientError> {
        Ok(self.session.step(direction).await?)
    }

    /// See `Session::step_retrying`.
    pub async fn step_retrying(
        &mut self,
        direction: StepDirection,
    ) -> Result<StepOutcome, DevClientError> {
        Ok(self.session.step_retrying(direction).await?)
    }

    /// See `Session::selected_capabilities`.
    pub fn selected_capabilities(&self) -> &[u32] {
        self.session.selected_capabilities()
    }

    /// See `Session::cast_spell`.
    pub async fn cast_spell(
        &mut self,
        spell: NonZeroU32,
        target: SpellTarget,
        aim_at_target: bool,
    ) -> Result<CastOutcome, DevClientError> {
        Ok(self
            .session
            .cast_spell(spell, target, aim_at_target)
            .await?)
    }

    /// See `Session::use_item`.
    pub async fn use_item(&mut self, handle: ItemHandle) -> Result<UseOutcome, DevClientError> {
        Ok(self.session.use_item(handle).await?)
    }

    /// See `Session::move_item`.
    pub async fn move_item(
        &mut self,
        intent: &ItemMoveIntent,
    ) -> Result<ItemMoveOutcomeResult, DevClientError> {
        Ok(self.session.move_item(intent).await?)
    }

    /// See `Session::chat`.
    pub async fn chat(&mut self, intent: &ChatIntent) -> Result<ChatOutcome, DevClientError> {
        Ok(self.session.chat(intent).await?)
    }

    /// See `Session::use_object`.
    pub async fn use_object(
        &mut self,
        placement: &[u8],
        expected_revision: u64,
    ) -> Result<UseOutcome, DevClientError> {
        Ok(self
            .session
            .use_object(placement, expected_revision)
            .await?)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
