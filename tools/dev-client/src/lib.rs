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
use oteryn_protocol_oteryn::world_object::{self, WorldObjectOverlayEntry};
use oteryn_protocol_oteryn::world_spatial::{self, StepDirection, WorldSpatialObservation};
use oteryn_protocol_oteryn::{CharacterId, FoundationProtocolError, MessageType};
use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, SessionError};
pub use oteryn_session::{
    AppliedDelta, CastOutcome, CommandOutcome, DuplicateOutcome, JoinSnapshot, MAX_QUEUED_EVENTS,
    SessionEvent, StepOutcome, UseOutcome,
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
            SessionError::NotAdmitted(message_type) => Self::NotAdmitted(message_type),
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

    /// The overlay entries after every delta applied so far.
    pub fn world_object_overlay(&self) -> &[WorldObjectOverlayEntry] {
        self.session.world_object_overlay()
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
mod tests {
    use super::*;
    // The fake servers below frame and decode with the same codecs the session crate uses; these
    // were `super::*` items before the session extraction, so only this import list changed.
    use oteryn_protocol_oteryn::world_object::{
        SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
        WorldObjectOverlayEntry as WireOverlayEntry, encode_world_object_overlay_snapshot,
    };
    use oteryn_protocol_oteryn::world_spatial::{
        ActorPosition, SNAPSHOT_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        WorldSpatialObservation as WireSpatialObservation, encode_world_spatial,
    };
    use oteryn_protocol_oteryn::{
        ALPN_OTERYN_GAME_V1, CommandStatus, FrameLength, GameSessionId,
        decode_snapshot_chunk_framing, decode_wire_envelope,
    };
    use oteryn_protocol_oteryn::{
        ChannelId, DomainSnapshot, ServerAcceptedValue, WorldId, encode_server_accepted,
        encode_single_chunk_snapshot,
    };
    use oteryn_session::{read_frame, write_frame};
    use rustls::pki_types::PrivatePkcs8KeyDer;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::net::TcpStream;
    use world_object::{UseDisposition, WorldObjectTarget};
    use world_spatial::StepDisposition;

    const DOOR_PLACEMENT: &str = "oteryn:cell/entry-door";
    const DOOR_STATE: &str = "oteryn:reference.state.closed";
    const CLIENT_BUILD_ID: &str = "oteryn-dev-client-test";
    /// Generous enough that it never fires in the happy-path/negative tests below (all
    /// same-process, no real network latency); the stall test uses its own short deadline.
    const TEST_DEADLINE: Duration = Duration::from_secs(5);

    fn test_uuid_v7(marker: u8) -> [u8; 16] {
        [
            0x01, 0x93, 0x4f, 0x10, 0x7c, 0x00, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33,
            0x44, marker,
        ]
    }

    /// One committed-players-free fake server: accept one TLS connection, decode the incoming
    /// `ClientBootstrap` back through `oteryn-protocol-oteryn`'s own server-direction decode
    /// (proving the encoder `connect_and_join` drives is exactly what a real server ingests),
    /// then reply with `ServerAccepted` and a join snapshot carrying both state domains — the
    /// native entry door overlay (`oteryn:cell/entry-door`, closed, revision 0) among them.
    async fn run_fake_server(
        listener: TcpListener,
        acceptor: tokio_rustls::TlsAcceptor,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;

        let mut prefix = [0_u8; 4];
        stream.read_exact(&mut prefix).await?;
        let length = FrameLength::from_prefix(&prefix)?;
        let mut body = vec![0_u8; length.get() as usize];
        stream.read_exact(&mut body).await?;
        let envelope = decode_wire_envelope(&body)?;
        let bootstrap = envelope.client_bootstrap()?;
        assert_eq!(bootstrap.client_build_id, CLIENT_BUILD_ID);
        assert_eq!(bootstrap.admission_material, b"fixture-grant");

        let accepted = encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
            world_id: WorldId::decode(&test_uuid_v7(2))?,
            channel_id: ChannelId::decode(&test_uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: &[],
        })?;
        write_frame(&mut stream, &accepted).await?;

        let content_generation = [0x11_u8; 32];
        let spatial_payload = encode_world_spatial(&WireSpatialObservation {
            content_generation,
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        });
        let overlay_payload = encode_world_object_overlay_snapshot(&[WireOverlayEntry {
            content_generation,
            placement: DOOR_PLACEMENT.as_bytes().to_vec(),
            state: DOOR_STATE.as_bytes().to_vec(),
            revision: 0,
        }])
        .map_err(|error| format!("encode_world_object_overlay_snapshot: {error:?}"))?;
        let snapshot = encode_single_chunk_snapshot(
            1,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial_payload,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay_payload,
                },
            ],
        )?;
        for frame in snapshot {
            write_frame(&mut stream, &frame).await?;
        }
        stream.flush().await?;
        Ok(())
    }

    /// Builds a manual multi-thread runtime rather than relying on `#[tokio::test]`, which needs
    /// tokio's `macros` feature; this workspace pins `tokio` to a vendored, offline-built path
    /// dependency (root `Cargo.toml` `[patch.crates-io]`) that does not carry `tokio-macros`.
    /// `apps/game-server`'s own qualification tests build their runtime the same way.
    fn block_on<F: std::future::Future>(
        future: F,
    ) -> Result<F::Output, Box<dyn StdError + Send + Sync>> {
        Ok(tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?
            .block_on(future))
    }

    /// End to end against a same-process fake server: TLS 1.3 + ALPN handshake, `ClientBootstrap`
    /// admission, and a decoded join snapshot whose `WORLD_OBJECT_OVERLAY` entry is the native
    /// entry door — by placement `oteryn:cell/entry-door` and its state key, per #162 comment
    /// 5875470550 option 3.
    #[test]
    fn connect_and_join_decodes_join_snapshot_with_entry_door_overlay()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(run_join_snapshot_case())?
    }

    async fn run_join_snapshot_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let certificate: CertificateDer<'static> = generated.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());

        let mut server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())?;
        server_config.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(run_fake_server(listener, acceptor));

        let character_id = CharacterId::decode(&test_uuid_v7(4))?;
        let snapshot = connect_and_join(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: TEST_DEADLINE,
        })
        .await
        .map_err(|error| format!("connect_and_join: {error}"))?;

        server.await??;

        assert_eq!(
            snapshot.game_session_id,
            GameSessionId::decode(&test_uuid_v7(1))?
        );
        assert_eq!(
            snapshot.world_spatial.actor_position,
            ActorPosition {
                x: 0,
                y: 0,
                floor: 0
            }
        );
        assert_eq!(snapshot.world_object_overlay.len(), 1);
        let door = &snapshot.world_object_overlay[0];
        assert_eq!(door.placement, DOOR_PLACEMENT.as_bytes());
        assert_eq!(door.state, DOOR_STATE.as_bytes());
        assert_eq!(door.revision, 0);
        assert_eq!(snapshot.world_spatial_revision, 1);
        assert_eq!(snapshot.world_object_overlay_revision, 0);
        Ok(())
    }

    #[test]
    fn connect_and_join_reports_refusal_when_server_closes_before_admission()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(run_server_closes_before_admission_case())?
    }

    async fn run_server_closes_before_admission_case() -> Result<(), Box<dyn StdError + Send + Sync>>
    {
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let certificate: CertificateDer<'static> = generated.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
        let mut server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())?;
        server_config.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        tokio::spawn(async move {
            if let Ok((tcp, _)) = listener.accept().await
                && let Ok(mut stream) = acceptor.accept(tcp).await
            {
                let _ = stream.shutdown().await;
            }
        });

        let character_id = CharacterId::decode(&test_uuid_v7(4))?;
        let result = connect_and_join(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: TEST_DEADLINE,
        })
        .await;
        assert!(result.is_err());
        Ok(())
    }

    /// Reads one framed `ClientBootstrap` off `stream` and discards it — every fixture below
    /// needs a well-formed request to answer, but none inspects its contents.
    async fn read_and_discard_bootstrap(
        stream: &mut tokio_rustls::server::TlsStream<TcpStream>,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let mut prefix = [0_u8; 4];
        stream.read_exact(&mut prefix).await?;
        let length = FrameLength::from_prefix(&prefix)?;
        let mut body = vec![0_u8; length.get() as usize];
        stream.read_exact(&mut body).await?;
        decode_wire_envelope(&body)?.client_bootstrap()?;
        Ok(())
    }

    fn fake_accepted_frame() -> Result<Vec<u8>, Box<dyn StdError + Send + Sync>> {
        Ok(encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
            world_id: WorldId::decode(&test_uuid_v7(2))?,
            channel_id: ChannelId::decode(&test_uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: &[],
        })?)
    }

    async fn tls_test_listener() -> Result<
        (
            CertificateDer<'static>,
            tokio_rustls::TlsAcceptor,
            TcpListener,
            SocketAddr,
        ),
        Box<dyn StdError + Send + Sync>,
    > {
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let certificate: CertificateDer<'static> = generated.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
        let mut server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())?;
        server_config.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        Ok((certificate, acceptor, listener, address))
    }

    /// Codex P2 finding 2 (#162 review on `1efc029`): a `SnapshotChunk` whose `snapshot_id`
    /// does not match `SnapshotBegin`'s must be rejected, not silently accepted.
    #[test]
    fn connect_and_join_rejects_a_snapshot_chunk_with_mismatched_snapshot_id()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(run_mismatched_snapshot_id_case())?
    }

    async fn run_mismatched_snapshot_id_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
        let (certificate, acceptor, listener, address) = tls_test_listener().await?;
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            read_and_discard_bootstrap(&mut stream).await?;
            write_frame(&mut stream, &fake_accepted_frame()?).await?;

            let payload = [0_u8];
            let domain = [DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &payload,
            }];
            // SnapshotBegin carries snapshot_id 1; SnapshotChunk carries a mismatched 2.
            let begin = encode_single_chunk_snapshot(1, 1, 0, &domain)?;
            let mismatched = encode_single_chunk_snapshot(1, 2, 0, &domain)?;
            write_frame(&mut stream, &begin[0]).await?;
            write_frame(&mut stream, &mismatched[1]).await?;
            stream.flush().await?;
            Ok::<(), Box<dyn StdError + Send + Sync>>(())
        });

        let character_id = CharacterId::decode(&test_uuid_v7(4))?;
        let result = connect_and_join(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: TEST_DEADLINE,
        })
        .await;
        assert!(matches!(
            result,
            Err(DevClientError::SnapshotIdMismatch {
                expected: 1,
                actual: 2
            })
        ));
        server.await??;
        Ok(())
    }

    /// Codex P2 finding 3 (#162 review on `1efc029`): a server that admits and then stalls
    /// (never sends `SnapshotBegin`) must fail with a bounded diagnostic, not hang.
    #[test]
    fn connect_and_join_times_out_when_the_server_stalls_after_admission()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(run_stalls_after_admission_case())?
    }

    async fn run_stalls_after_admission_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
        let (certificate, acceptor, listener, address) = tls_test_listener().await?;
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            read_and_discard_bootstrap(&mut stream).await?;
            write_frame(&mut stream, &fake_accepted_frame()?).await?;
            // Stall: never send SnapshotBegin. Held well past the client's short deadline below.
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok::<(), Box<dyn StdError + Send + Sync>>(())
        });

        let character_id = CharacterId::decode(&test_uuid_v7(4))?;
        let result = connect_and_join(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: Duration::from_millis(200),
        })
        .await;
        assert!(matches!(
            result,
            Err(DevClientError::Timeout("SnapshotBegin"))
        ));
        server.abort();
        Ok(())
    }

    /// Codex P2 finding 4 (#162 review on `1efc029`): `PROTOCOL_OTERYN_V1_REGISTRY.json`
    /// registers exactly one `snapshot_type` (1) for domains 1 and 2; anything else naming one
    /// of those domains must be rejected, not silently treated as an unknown domain.
    #[test]
    fn connect_and_join_rejects_an_unregistered_snapshot_type_for_a_known_domain()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(run_unregistered_snapshot_type_case())?
    }

    async fn run_unregistered_snapshot_type_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
        let (certificate, acceptor, listener, address) = tls_test_listener().await?;
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            read_and_discard_bootstrap(&mut stream).await?;
            write_frame(&mut stream, &fake_accepted_frame()?).await?;

            let payload = [0_u8];
            let frames = encode_single_chunk_snapshot(
                1,
                1,
                0,
                &[DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    // Not registered for domain 1 (only snapshot_type 1 is).
                    snapshot_type: 2,
                    payload: &payload,
                }],
            )?;
            for frame in frames {
                write_frame(&mut stream, &frame).await?;
            }
            stream.flush().await?;
            Ok::<(), Box<dyn StdError + Send + Sync>>(())
        });

        let character_id = CharacterId::decode(&test_uuid_v7(4))?;
        let result = connect_and_join(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: TEST_DEADLINE,
        })
        .await;
        assert!(matches!(
            result,
            Err(DevClientError::UnregisteredSnapshotType {
                domain_id: 1,
                snapshot_type: 2
            })
        ));
        server.await??;
        Ok(())
    }

    /// Like `tls_test_listener`, but deliberately configures no ALPN protocol: a trusted server
    /// (valid, chain-verifiable certificate) whose handshake completes without negotiating one.
    async fn tls_test_listener_without_alpn() -> Result<
        (
            CertificateDer<'static>,
            tokio_rustls::TlsAcceptor,
            TcpListener,
            SocketAddr,
        ),
        Box<dyn StdError + Send + Sync>,
    > {
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let certificate: CertificateDer<'static> = generated.cert.der().clone();
        let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
        let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.clone()], key.into())?;
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        Ok((certificate, acceptor, listener, address))
    }

    /// Codex P2 finding 1 (#162 review on PR #1147, `f0f27df`): a trusted TLS server that never
    /// negotiates ALPN must be rejected before anything is sent, not merely proceed unauthenticated.
    #[test]
    fn connect_and_join_rejects_a_connection_with_no_negotiated_alpn()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(run_no_alpn_case())?
    }

    async fn run_no_alpn_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
        let (certificate, acceptor, listener, address) = tls_test_listener_without_alpn().await?;
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            // The client must detect the ALPN mismatch and send nothing: a bounded read
            // observing EOF (or a reset) rather than any bytes proves it.
            let mut buffer = [0_u8; 1];
            match tokio::time::timeout(Duration::from_millis(300), stream.read(&mut buffer)).await {
                Ok(Ok(0)) | Ok(Err(_)) => {}
                Ok(Ok(_)) => return Err("client sent bytes after an ALPN mismatch".into()),
                Err(_) => return Err("client did not close after an ALPN mismatch".into()),
            }
            Ok::<(), Box<dyn StdError + Send + Sync>>(())
        });

        let character_id = CharacterId::decode(&test_uuid_v7(4))?;
        let result = connect_and_join(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: TEST_DEADLINE,
        })
        .await;
        assert!(matches!(result, Err(DevClientError::AlpnMismatch)));
        server.await??;
        Ok(())
    }

    // --- Minimal test-only wire construction for the 3 fixtures below, which need a
    // `SnapshotBegin` declaring more than the one chunk `encode_single_chunk_snapshot` can ever
    // produce. This is fixture-only: it simulates a hypothetical/adversarial *server*, never
    // anything `connect_and_join` itself sends or trusts without checking.

    fn push_test_varint(output: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            output.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        output.push(value as u8);
    }

    fn push_test_scalar(output: &mut Vec<u8>, field: u64, value: u64) {
        if value != 0 {
            push_test_varint(output, field << 3);
            push_test_varint(output, value);
        }
    }

    fn push_test_bytes_field(output: &mut Vec<u8>, field: u64, value: &[u8]) {
        push_test_varint(output, (field << 3) | 2);
        push_test_varint(output, value.len() as u64);
        output.extend_from_slice(value);
    }

    fn test_server_frame(
        message_type: MessageType,
        connection_generation: u64,
        payload: &[u8],
    ) -> Vec<u8> {
        test_server_frame_with_sequence(message_type, connection_generation, 0, payload)
    }

    /// Like `test_server_frame`, but also sets the envelope's `server_sequence` (field 3) —
    /// needed only by the fixture proving `WireEnvelopeView::validate` rejects a nonzero one on
    /// an unsequenced (`Sequencing::None`) message type such as `SnapshotBegin`.
    fn test_server_frame_with_sequence(
        message_type: MessageType,
        connection_generation: u64,
        server_sequence: u64,
        payload: &[u8],
    ) -> Vec<u8> {
        let mut output = Vec::new();
        push_test_scalar(&mut output, 1, message_type as u64);
        push_test_scalar(&mut output, 2, connection_generation);
        push_test_scalar(&mut output, 3, server_sequence);
        push_test_bytes_field(&mut output, 4, payload);
        output
    }

    fn test_snapshot_begin(snapshot_id: u64, chunk_count: u64, total_bytes: u64) -> Vec<u8> {
        let mut payload = Vec::new();
        push_test_scalar(&mut payload, 1, snapshot_id);
        push_test_scalar(&mut payload, 2, chunk_count);
        push_test_scalar(&mut payload, 3, total_bytes);
        test_server_frame(MessageType::SnapshotBegin, 1, &payload)
    }

    fn test_snapshot_chunk(snapshot_id: u64, chunk_index: u64, data: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        push_test_scalar(&mut payload, 1, snapshot_id);
        push_test_scalar(&mut payload, 2, chunk_index);
        push_test_bytes_field(&mut payload, 3, data);
        test_server_frame(MessageType::SnapshotChunk, 1, &payload)
    }

    fn test_snapshot_commit(snapshot_id: u64) -> Vec<u8> {
        let mut payload = Vec::new();
        push_test_scalar(&mut payload, 1, snapshot_id);
        test_server_frame(MessageType::SnapshotCommit, 1, &payload)
    }

    /// Drives `connect_and_join` against a fixture that sends exactly `frames` after
    /// `ServerAccepted`, and asserts the join fails with `expected`.
    async fn assert_join_fails_with(
        frames: Vec<Vec<u8>>,
        expected: impl Fn(&DevClientError) -> bool + Send + 'static,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        let (certificate, acceptor, listener, address) = tls_test_listener().await?;
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            read_and_discard_bootstrap(&mut stream).await?;
            write_frame(&mut stream, &fake_accepted_frame()?).await?;
            for frame in &frames {
                write_frame(&mut stream, frame).await?;
            }
            stream.flush().await?;
            Ok::<(), Box<dyn StdError + Send + Sync>>(())
        });

        let character_id = CharacterId::decode(&test_uuid_v7(4))?;
        let result = connect_and_join(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: TEST_DEADLINE,
        })
        .await;
        let Err(error) = &result else {
            return Err(format!("expected a join failure, got {result:?}").into());
        };
        if !expected(error) {
            return Err(format!("unexpected error variant: {error:?}").into());
        }
        server.await??;
        Ok(())
    }

    /// Codex P2 finding 2 (#162 review on PR #1147, `f0f27df`): `SnapshotBegin` declares 2
    /// chunks, but the server sends only 1 before the commit — rejected (the commit arrives
    /// where a 2nd `SnapshotChunk` was expected).
    #[test]
    fn connect_and_join_rejects_fewer_chunks_than_snapshot_begin_declared()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(assert_join_fails_with(
            vec![
                test_snapshot_begin(1, 2, 0),
                test_snapshot_chunk(1, 0, &[]),
                test_snapshot_commit(1),
            ],
            |error| {
                matches!(
                    error,
                    DevClientError::UnexpectedMessage {
                        expected: MessageType::SnapshotChunk,
                        actual: MessageType::SnapshotCommit
                    }
                )
            },
        ))?
    }

    /// Codex P2 finding 2: a `SnapshotChunk` whose `chunk_index` does not match the index
    /// expected next is rejected.
    #[test]
    fn connect_and_join_rejects_a_wrong_chunk_index() -> Result<(), Box<dyn StdError + Send + Sync>>
    {
        block_on(assert_join_fails_with(
            vec![test_snapshot_begin(1, 1, 0), test_snapshot_chunk(1, 1, &[])],
            |error| {
                matches!(
                    error,
                    DevClientError::ChunkIndexMismatch {
                        expected: 0,
                        actual: 1
                    }
                )
            },
        ))?
    }

    /// Codex P2 finding 2: the summed `SnapshotChunk` `data` bytes must equal `SnapshotBegin`'s
    /// declared `total_encoded_bytes`; a mismatch is rejected before the commit is trusted.
    #[test]
    fn connect_and_join_rejects_an_assembled_length_mismatch()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(assert_join_fails_with(
            vec![
                // Declares 100 encoded bytes but the one chunk sent carries an empty (valid,
                // zero-domain) body: 0 actual bytes assembled.
                test_snapshot_begin(1, 1, 100),
                test_snapshot_chunk(1, 0, &[]),
                test_snapshot_commit(1),
            ],
            |error| {
                matches!(
                    error,
                    DevClientError::AssembledLengthMismatch {
                        expected: 100,
                        actual: 0
                    }
                )
            },
        ))?
    }

    /// C1b fix round 3 finding 1 (#1147 review): a valid 2-chunk transfer whose assembled
    /// `SnapshotBody` protobuf bytes are split across the chunk boundary at an arbitrary byte
    /// offset — not necessarily a field boundary (FND-02 §16) — must still succeed. Proves every
    /// chunk's raw `data` is concatenated, in `chunk_index` order, before the body is decoded
    /// once, rather than each chunk's `data` being decoded on its own.
    #[test]
    fn connect_and_join_assembles_a_two_chunk_snapshot_split_mid_field()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(run_two_chunk_split_case())?
    }

    async fn run_two_chunk_split_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
        let content_generation = [0x22_u8; 32];
        let spatial_payload = encode_world_spatial(&WireSpatialObservation {
            content_generation,
            actor_position: ActorPosition {
                x: 7,
                y: 9,
                floor: 1,
            },
        });
        let overlay_payload = encode_world_object_overlay_snapshot(&[WireOverlayEntry {
            content_generation,
            placement: DOOR_PLACEMENT.as_bytes().to_vec(),
            state: DOOR_STATE.as_bytes().to_vec(),
            revision: 3,
        }])
        .map_err(|error| format!("encode_world_object_overlay_snapshot: {error:?}"))?;
        let domains = [
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial_payload,
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 3,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay_payload,
            },
        ];
        // A single-chunk transfer's one chunk's `data` *is* the complete, validly-encoded
        // `SnapshotBody` (`encode_single_chunk_snapshot`'s own contract) — reused here only to
        // obtain real assembled-body bytes to split in two, never sent as a single chunk.
        let single = encode_single_chunk_snapshot(1, 7, 0, &domains)?;
        let single_chunk_envelope = decode_wire_envelope(&single[1])?;
        let (_, whole_body) = decode_snapshot_chunk_framing(single_chunk_envelope.payload())?;
        let body_len = whole_body.len() as u64;
        let split_at = whole_body.len() / 2;
        assert!(
            split_at > 0 && split_at < whole_body.len(),
            "fixture body too small to split"
        );
        let first_half = whole_body[..split_at].to_vec();
        let second_half = whole_body[split_at..].to_vec();

        let (certificate, acceptor, listener, address) = tls_test_listener().await?;
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            read_and_discard_bootstrap(&mut stream).await?;
            write_frame(&mut stream, &fake_accepted_frame()?).await?;
            write_frame(&mut stream, &test_snapshot_begin(7, 2, body_len)).await?;
            write_frame(&mut stream, &test_snapshot_chunk(7, 0, &first_half)).await?;
            write_frame(&mut stream, &test_snapshot_chunk(7, 1, &second_half)).await?;
            write_frame(&mut stream, &test_snapshot_commit(7)).await?;
            stream.flush().await?;
            Ok::<(), Box<dyn StdError + Send + Sync>>(())
        });

        let character_id = CharacterId::decode(&test_uuid_v7(4))?;
        let snapshot = connect_and_join(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: TEST_DEADLINE,
        })
        .await
        .map_err(|error| format!("connect_and_join: {error}"))?;
        server.await??;

        assert_eq!(
            snapshot.world_spatial.actor_position,
            ActorPosition {
                x: 7,
                y: 9,
                floor: 1
            }
        );
        assert_eq!(snapshot.world_object_overlay.len(), 1);
        assert_eq!(snapshot.world_object_overlay[0].revision, 3);
        assert_eq!(snapshot.world_spatial_revision, 1);
        assert_eq!(snapshot.world_object_overlay_revision, 3);
        Ok(())
    }

    /// C1b fix round 3 finding 2 (#1147 review): `ServerAccepted`'s own envelope
    /// `connection_generation` must be absent (0) pre-admission (FND-02 §8/§11) — a server that
    /// sends a nonzero one is rejected by `WireEnvelopeView::validate` before the payload
    /// (which carries the *session's* new generation in a different field) is even decoded.
    #[test]
    fn connect_and_join_rejects_a_server_accepted_with_a_nonzero_envelope_generation()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(run_server_accepted_nonzero_generation_case())?
    }

    async fn run_server_accepted_nonzero_generation_case()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        let accepted_payload = decode_wire_envelope(&fake_accepted_frame()?)?
            .payload()
            .to_vec();
        let tampered = test_server_frame(MessageType::ServerAccepted, 5, &accepted_payload);

        let (certificate, acceptor, listener, address) = tls_test_listener().await?;
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            read_and_discard_bootstrap(&mut stream).await?;
            write_frame(&mut stream, &tampered).await?;
            stream.flush().await?;
            Ok::<(), Box<dyn StdError + Send + Sync>>(())
        });

        let character_id = CharacterId::decode(&test_uuid_v7(4))?;
        let result = connect_and_join(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: TEST_DEADLINE,
        })
        .await;
        assert!(matches!(
            result,
            Err(DevClientError::Protocol(
                FoundationProtocolError::MalformedEnvelope
            ))
        ));
        server.await??;
        Ok(())
    }

    /// C1b fix round 3 finding 2: `SnapshotBegin` (and every other snapshot-transfer frame) is
    /// `Sequencing::None` (FND-02 §14: transfer-control frames carry no `server_sequence`); a
    /// server that sets a nonzero one is rejected by `WireEnvelopeView::validate`.
    #[test]
    fn connect_and_join_rejects_a_snapshot_begin_with_a_nonzero_server_sequence()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(assert_join_fails_with(
            vec![{
                let mut payload = Vec::new();
                push_test_scalar(&mut payload, 1, 1);
                push_test_scalar(&mut payload, 2, 1);
                push_test_scalar(&mut payload, 3, 0);
                test_server_frame_with_sequence(MessageType::SnapshotBegin, 1, 7, &payload)
            }],
            |error| {
                matches!(
                    error,
                    DevClientError::Protocol(FoundationProtocolError::MalformedEnvelope)
                )
            },
        ))?
    }

    // --- `step` / `use_object` against a scripted fake server. Every command the client sends is
    // decoded by the fake server through `oteryn-protocol-oteryn`'s own server-side ingress
    // (`WireEnvelopeView::client_command`), and every reply is built with the crate's own server
    // encoders. The join deliberately starts away from the fresh-admission constants (first
    // CommandId 7, snapshot target sequence 40, spatial revision 5, overlay revision 2) so the
    // client is proven to take them from the join, not assume them.

    type BoxError = Box<dyn StdError + Send + Sync>;
    type ServerStream = tokio_rustls::server::TlsStream<TcpStream>;

    const GENERATION: u64 = 1;
    const FIRST_COMMAND_ID: u64 = 7;
    const JOIN_SEQUENCE: u64 = 40;
    const JOIN_SPATIAL_REVISION: u64 = 5;
    const JOIN_DOOR_REVISION: u64 = 2;
    const CONTENT_GENERATION: [u8; 32] = [0x11; 32];
    const OTHER_GENERATION: [u8; 32] = [0x99; 32];
    const DOOR_OPEN_STATE: &str = "oteryn:reference.state.open";

    fn spatial_payload(x: i32, y: i32, content_generation: [u8; 32]) -> Vec<u8> {
        encode_world_spatial(&WireSpatialObservation {
            content_generation,
            actor_position: ActorPosition { x, y, floor: 0 },
        })
    }

    fn door_entry(state: &str, revision: u64, content_generation: [u8; 32]) -> WireOverlayEntry {
        WireOverlayEntry {
            content_generation,
            placement: DOOR_PLACEMENT.as_bytes().to_vec(),
            state: state.as_bytes().to_vec(),
            revision,
        }
    }

    fn command_result_frame(
        sequence: u64,
        command_id: u64,
        status: CommandStatus,
        payload: &[u8],
    ) -> Result<Vec<u8>, BoxError> {
        Ok(oteryn_protocol_oteryn::encode_command_result(
            GENERATION, sequence, command_id, status, payload,
        )?)
    }

    fn step_result_frame(
        sequence: u64,
        command_id: u64,
        disposition: StepDisposition,
    ) -> Result<Vec<u8>, BoxError> {
        let status = if disposition == StepDisposition::Rejected {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        };
        command_result_frame(
            sequence,
            command_id,
            status,
            &world_spatial::encode_step_result(disposition),
        )
    }

    fn use_result_frame(
        sequence: u64,
        command_id: u64,
        disposition: UseDisposition,
    ) -> Result<Vec<u8>, BoxError> {
        command_result_frame(
            sequence,
            command_id,
            CommandStatus::Accepted,
            &world_object::encode_use_result(disposition),
        )
    }

    fn state_delta_frame(
        sequence: u64,
        domain_id: u32,
        base: u64,
        new: u64,
        delta_type: u32,
        payload: &[u8],
    ) -> Result<Vec<u8>, BoxError> {
        Ok(oteryn_protocol_oteryn::encode_state_delta(
            GENERATION, sequence, domain_id, base, new, delta_type, payload,
        )?)
    }

    fn spatial_delta_frame(sequence: u64, base: u64, x: i32, y: i32) -> Result<Vec<u8>, BoxError> {
        state_delta_frame(
            sequence,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            base,
            base + 1,
            world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1,
            &spatial_payload(x, y, CONTENT_GENERATION),
        )
    }

    fn overlay_delta_frame(sequence: u64, base: u64, state: &str) -> Result<Vec<u8>, BoxError> {
        let payload = world_object::encode_world_object_overlay_delta(&door_entry(
            state,
            base + 1,
            CONTENT_GENERATION,
        ))
        .map_err(|error| format!("encode_world_object_overlay_delta: {error:?}"))?;
        state_delta_frame(
            sequence,
            STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            base,
            base + 1,
            world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
            &payload,
        )
    }

    async fn read_command(stream: &mut ServerStream) -> Result<(u64, u32, Vec<u8>), BoxError> {
        let body = read_frame(stream).await?;
        let command = decode_wire_envelope(&body)?.client_command(GENERATION)?;
        Ok((
            command.command_id,
            command.command_type,
            command.payload.to_vec(),
        ))
    }

    async fn expect_step(
        stream: &mut ServerStream,
        command_id: u64,
        direction: StepDirection,
    ) -> Result<(), BoxError> {
        let (id, command_type, payload) = read_command(stream).await?;
        assert_eq!(
            (id, command_type),
            (
                command_id,
                world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT
            )
        );
        assert_eq!(
            world_spatial::decode_step_intent(&payload),
            Ok(direction),
            "command {command_id}"
        );
        Ok(())
    }

    async fn expect_use(
        stream: &mut ServerStream,
        command_id: u64,
        placement: &[u8],
        expected_revision: u64,
    ) -> Result<(), BoxError> {
        let (id, command_type, payload) = read_command(stream).await?;
        assert_eq!(
            (id, command_type),
            (command_id, world_object::COMMAND_TYPE_USE_INTENT)
        );
        assert_eq!(
            world_object::decode_use_intent(&payload),
            Ok(WorldObjectTarget {
                placement: placement.to_vec(),
                expected_revision,
            }),
            "command {command_id}"
        );
        Ok(())
    }

    async fn send(stream: &mut ServerStream, frames: &[Vec<u8>]) -> Result<(), BoxError> {
        for frame in frames {
            write_frame(stream, frame).await?;
        }
        stream.flush().await?;
        Ok(())
    }

    type ServerTask = tokio::task::JoinHandle<Result<(), BoxError>>;

    /// Admits one client over real TLS, sends `ServerAccepted` (`next_command_id`
    /// `FIRST_COMMAND_ID`, `current_server_sequence` `JOIN_SEQUENCE`) and a join snapshot (target
    /// sequence `JOIN_SEQUENCE`, spatial domain revision `JOIN_SPATIAL_REVISION` at (0,0,0), the
    /// closed door at revision `JOIN_DOOR_REVISION`), then hands the stream to `script`.
    async fn joined_session<F, Fut>(
        deadline: Duration,
        script: F,
    ) -> Result<(DevClientSession, ServerTask), BoxError>
    where
        F: FnOnce(ServerStream) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), BoxError>> + Send + 'static,
    {
        let (certificate, acceptor, listener, address) = tls_test_listener().await?;
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            read_and_discard_bootstrap(&mut stream).await?;
            let accepted = encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
                world_id: WorldId::decode(&test_uuid_v7(2))?,
                channel_id: ChannelId::decode(&test_uuid_v7(3))?,
                connection_generation: GENERATION,
                current_server_sequence: JOIN_SEQUENCE,
                next_command_id: FIRST_COMMAND_ID,
                schema_revision: 1,
                selected_capabilities: &[],
            })?;
            write_frame(&mut stream, &accepted).await?;
            let overlay = encode_world_object_overlay_snapshot(&[door_entry(
                DOOR_STATE,
                JOIN_DOOR_REVISION,
                CONTENT_GENERATION,
            )])
            .map_err(|error| format!("encode_world_object_overlay_snapshot: {error:?}"))?;
            let spatial = spatial_payload(0, 0, CONTENT_GENERATION);
            let snapshot = encode_single_chunk_snapshot(
                GENERATION,
                1,
                JOIN_SEQUENCE,
                &[
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        revision: JOIN_SPATIAL_REVISION,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                        payload: &spatial,
                    },
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                        revision: JOIN_DOOR_REVISION,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                        payload: &overlay,
                    },
                ],
            )?;
            send(&mut stream, &snapshot).await?;
            script(stream).await
        });
        let session = connect_session(JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id: CharacterId::decode(&test_uuid_v7(4))?,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline,
        })
        .await
        .map_err(|error| format!("connect_session: {error}"))?;
        Ok((session, server))
    }

    /// Waits (bounded) for the client to close, so the scripted server never ends first.
    async fn wait_for_client_close(stream: &mut ServerStream) {
        let mut buffer = [0_u8; 1];
        let _ = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer)).await;
    }

    fn door_use_outcome(
        command_id: u64,
        result_server_sequence: u64,
        disposition: UseDisposition,
        overlay: Option<AppliedDelta<WireOverlayEntry>>,
    ) -> UseOutcome {
        CommandOutcome {
            command_id,
            status: CommandStatus::Accepted,
            disposition,
            result_server_sequence,
            world_spatial_delta: None,
            world_object_overlay_delta: overlay,
        }
    }

    fn step_outcome(
        command_id: u64,
        result_server_sequence: u64,
        disposition: StepDisposition,
        spatial: Option<AppliedDelta<WireSpatialObservation>>,
    ) -> StepOutcome {
        CommandOutcome {
            command_id,
            status: CommandStatus::Accepted,
            disposition,
            result_server_sequence,
            world_spatial_delta: spatial,
            world_object_overlay_delta: None,
        }
    }

    fn moved_to(
        server_sequence: u64,
        base_revision: u64,
        x: i32,
        y: i32,
    ) -> Option<AppliedDelta<WireSpatialObservation>> {
        Some(AppliedDelta {
            server_sequence,
            base_revision,
            new_revision: base_revision + 1,
            value: WireSpatialObservation {
                content_generation: CONTENT_GENERATION,
                actor_position: ActorPosition { x, y, floor: 0 },
            },
        })
    }

    fn door_delta(
        server_sequence: u64,
        base_revision: u64,
        state: &str,
    ) -> Option<AppliedDelta<WireOverlayEntry>> {
        Some(AppliedDelta {
            server_sequence,
            base_revision,
            new_revision: base_revision + 1,
            value: door_entry(state, base_revision + 1, CONTENT_GENERATION),
        })
    }

    /// The full door scenario (`use_wire`'s command mix), one command at a time: every
    /// disposition, the `CommandId` and `server_sequence` discipline, the domain-1 and domain-2
    /// revision chains, and the session's own mirrored state after every applied delta.
    #[test]
    fn step_and_use_follow_command_id_sequence_and_revision_discipline() -> Result<(), BoxError> {
        block_on(run_door_scenario_case())?
    }

    async fn run_door_scenario_case() -> Result<(), BoxError> {
        let door = DOOR_PLACEMENT.as_bytes();
        let (mut session, server) = joined_session(TEST_DEADLINE, move |mut stream| async move {
            // ids 7..=15, sequences 41..=54: east(Moved), use open(Committed), north(Moved),
            // use(Occupied), south(Moved), use close(Committed), north(Blocked), use with a stale
            // revision, use of an unknown placement.
            expect_step(&mut stream, 7, StepDirection::East).await?;
            send(
                &mut stream,
                &[
                    step_result_frame(41, 7, StepDisposition::Moved)?,
                    spatial_delta_frame(42, 5, 1, 0)?,
                ],
            )
            .await?;
            expect_use(&mut stream, 8, DOOR_PLACEMENT.as_bytes(), 2).await?;
            send(
                &mut stream,
                &[
                    use_result_frame(43, 8, UseDisposition::Committed)?,
                    overlay_delta_frame(44, 2, DOOR_OPEN_STATE)?,
                ],
            )
            .await?;
            expect_step(&mut stream, 9, StepDirection::North).await?;
            send(
                &mut stream,
                &[
                    step_result_frame(45, 9, StepDisposition::Moved)?,
                    spatial_delta_frame(46, 6, 1, -1)?,
                ],
            )
            .await?;
            expect_use(&mut stream, 10, DOOR_PLACEMENT.as_bytes(), 3).await?;
            send(
                &mut stream,
                &[use_result_frame(47, 10, UseDisposition::Occupied)?],
            )
            .await?;
            expect_step(&mut stream, 11, StepDirection::South).await?;
            send(
                &mut stream,
                &[
                    step_result_frame(48, 11, StepDisposition::Moved)?,
                    spatial_delta_frame(49, 7, 1, 0)?,
                ],
            )
            .await?;
            expect_use(&mut stream, 12, DOOR_PLACEMENT.as_bytes(), 3).await?;
            send(
                &mut stream,
                &[
                    use_result_frame(50, 12, UseDisposition::Committed)?,
                    overlay_delta_frame(51, 3, DOOR_STATE)?,
                ],
            )
            .await?;
            expect_step(&mut stream, 13, StepDirection::North).await?;
            send(
                &mut stream,
                &[step_result_frame(52, 13, StepDisposition::Blocked)?],
            )
            .await?;
            expect_use(&mut stream, 14, DOOR_PLACEMENT.as_bytes(), 2).await?;
            send(
                &mut stream,
                &[use_result_frame(53, 14, UseDisposition::StaleState)?],
            )
            .await?;
            expect_use(&mut stream, 15, b"oteryn:cell/unknown", 4).await?;
            send(
                &mut stream,
                &[use_result_frame(54, 15, UseDisposition::NothingToUse)?],
            )
            .await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;

        let joined = session.join_snapshot();
        assert_eq!(joined.world_spatial_revision, JOIN_SPATIAL_REVISION);
        assert_eq!(joined.world_object_overlay_revision, JOIN_DOOR_REVISION);
        assert_eq!(session.next_command_id(), FIRST_COMMAND_ID);
        assert_eq!(session.last_server_sequence(), JOIN_SEQUENCE);

        assert_eq!(
            session.step(StepDirection::East).await?,
            step_outcome(7, 41, StepDisposition::Moved, moved_to(42, 5, 1, 0))
        );
        assert_eq!(
            session.use_object(door, 2).await?,
            door_use_outcome(8, 43, UseDisposition::Committed, None)
        );
        // A use returns at its result: the door delta behind it is read by the next exchange.
        assert_eq!(
            session.world_object_overlay(),
            &[door_entry(
                DOOR_STATE,
                JOIN_DOOR_REVISION,
                CONTENT_GENERATION
            )]
        );
        assert_eq!(
            session.step(StepDirection::North).await?,
            step_outcome(9, 45, StepDisposition::Moved, moved_to(46, 6, 1, -1))
        );
        assert_eq!(
            session.world_object_overlay(),
            &[door_entry(DOOR_OPEN_STATE, 3, CONTENT_GENERATION)]
        );
        assert_eq!(
            session.take_events(),
            vec![SessionEvent::WorldObjectOverlay(
                door_delta(44, 2, DOOR_OPEN_STATE).ok_or("door delta")?
            )]
        );
        assert_eq!(
            session.use_object(door, 3).await?,
            door_use_outcome(10, 47, UseDisposition::Occupied, None)
        );
        assert_eq!(
            session.step(StepDirection::South).await?,
            step_outcome(11, 48, StepDisposition::Moved, moved_to(49, 7, 1, 0))
        );
        assert_eq!(
            session.use_object(door, 3).await?,
            door_use_outcome(12, 50, UseDisposition::Committed, None)
        );
        assert_eq!(
            session.step(StepDirection::North).await?,
            step_outcome(13, 52, StepDisposition::Blocked, None)
        );
        assert_eq!(
            session.take_events(),
            vec![SessionEvent::WorldObjectOverlay(
                door_delta(51, 3, DOOR_STATE).ok_or("door delta")?
            )]
        );
        assert_eq!(
            session.use_object(door, 2).await?,
            door_use_outcome(14, 53, UseDisposition::StaleState, None)
        );
        assert_eq!(
            session.use_object(b"oteryn:cell/unknown", 4).await?,
            door_use_outcome(15, 54, UseDisposition::NothingToUse, None)
        );
        assert_eq!(session.next_command_id(), 16);
        assert_eq!(session.last_server_sequence(), 54);
        assert_eq!(
            session.world_spatial().actor_position,
            ActorPosition {
                x: 1,
                y: 0,
                floor: 0
            }
        );
        assert_eq!(
            session.world_object_overlay(),
            &[door_entry(DOOR_STATE, 4, CONTENT_GENERATION)]
        );
        // The original join snapshot is untouched by later deltas.
        assert_eq!(session.join_snapshot().world_object_overlay[0].revision, 2);
        drop(session);
        server.await??;
        Ok(())
    }

    /// A `REJECTED` step (an ineligible actor, `StepDisposition::Rejected`) carries no delta and
    /// leaves the session usable for the next command.
    #[test]
    fn a_rejected_step_carries_no_delta_and_the_session_stays_usable() -> Result<(), BoxError> {
        block_on(run_rejected_step_case())?
    }

    async fn run_rejected_step_case() -> Result<(), BoxError> {
        let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
            expect_step(&mut stream, 7, StepDirection::West).await?;
            send(
                &mut stream,
                &[step_result_frame(41, 7, StepDisposition::Rejected)?],
            )
            .await?;
            expect_step(&mut stream, 8, StepDirection::East).await?;
            send(
                &mut stream,
                &[
                    step_result_frame(42, 8, StepDisposition::Moved)?,
                    spatial_delta_frame(43, 5, 1, 0)?,
                ],
            )
            .await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
        let rejected = session.step(StepDirection::West).await?;
        assert_eq!(rejected.status, CommandStatus::Rejected);
        assert_eq!(rejected.disposition, StepDisposition::Rejected);
        assert!(rejected.world_spatial_delta.is_none());
        assert_eq!(
            session.step(StepDirection::East).await?,
            step_outcome(8, 42, StepDisposition::Moved, moved_to(43, 5, 1, 0))
        );
        drop(session);
        server.await??;
        Ok(())
    }

    /// A `LivenessProbe` that arrives while a command's reply is being read (before its
    /// `CommandResult`, or between it and its delta) is answered with a `LivenessAck` for that probe (carrying the last applied `server_sequence`)
    /// and does not disturb the command flow or the sequence chain.
    #[test]
    fn liveness_probes_are_acked_and_do_not_disturb_the_command_flow() -> Result<(), BoxError> {
        block_on(run_liveness_case())?
    }

    async fn run_liveness_case() -> Result<(), BoxError> {
        let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
            expect_step(&mut stream, 7, StepDirection::East).await?;
            send(
                &mut stream,
                &[
                    oteryn_protocol_oteryn::encode_liveness_probe(GENERATION, 3)?,
                    step_result_frame(41, 7, StepDisposition::Moved)?,
                    oteryn_protocol_oteryn::encode_liveness_probe(GENERATION, 4)?,
                    spatial_delta_frame(42, 5, 1, 0)?,
                ],
            )
            .await?;
            for (probe_id, last_applied) in [(3, JOIN_SEQUENCE), (4, 41)] {
                let body = read_frame(&mut stream).await?;
                let ack = decode_wire_envelope(&body)?.liveness_ack(GENERATION)?;
                assert_eq!(
                    ack,
                    oteryn_protocol_oteryn::LivenessAckView {
                        probe_id,
                        last_applied_server_sequence: Some(last_applied),
                    }
                );
            }
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
        assert_eq!(
            session.step(StepDirection::East).await?,
            step_outcome(7, 41, StepDisposition::Moved, moved_to(42, 5, 1, 0))
        );
        drop(session);
        server.await??;
        Ok(())
    }

    /// A placement over the accepted key bound is refused before anything is sent: the
    /// `CommandId` is not consumed and the session stays usable.
    #[test]
    fn an_unencodable_use_is_refused_locally_without_consuming_a_command_id() -> Result<(), BoxError>
    {
        block_on(run_unencodable_use_case())?
    }

    async fn run_unencodable_use_case() -> Result<(), BoxError> {
        let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
            expect_step(&mut stream, 7, StepDirection::East).await?;
            send(
                &mut stream,
                &[step_result_frame(41, 7, StepDisposition::Blocked)?],
            )
            .await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
        let oversized = vec![b'x'; 513];
        assert!(matches!(
            session.use_object(&oversized, 0).await,
            Err(DevClientError::WorldObject(
                world_object::WorldObjectError::LimitExceeded
            ))
        ));
        assert_eq!(session.next_command_id(), FIRST_COMMAND_ID);
        assert_eq!(
            session.step(StepDirection::East).await?,
            step_outcome(7, 41, StepDisposition::Blocked, None)
        );
        drop(session);
        server.await??;
        Ok(())
    }

    enum Action {
        Step,
        Use,
    }

    /// Drives one command against a server that answers it with exactly `reply`, asserts the
    /// client fails with `expected`, and that the session is then unusable.
    async fn assert_command_fails(
        action: Action,
        reply: Vec<Vec<u8>>,
        expected: impl Fn(&DevClientError) -> bool,
    ) -> Result<(), BoxError> {
        let (mut session, server) = joined_session(TEST_DEADLINE, move |mut stream| async move {
            let (id, _, _) = read_command(&mut stream).await?;
            assert_eq!(id, FIRST_COMMAND_ID);
            send(&mut stream, &reply).await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
        let result = match action {
            Action::Step => session.step(StepDirection::East).await.map(|_| ()),
            // A use returns at its result, so a bad delta behind it fails the next idle read.
            Action::Use => match session
                .use_object(DOOR_PLACEMENT.as_bytes(), JOIN_DOOR_REVISION)
                .await
            {
                Ok(_) => session.service_liveness(Duration::from_millis(200)).await,
                Err(error) => Err(error),
            },
        };
        match &result {
            Err(error) if expected(error) => {}
            other => return Err(format!("unexpected command result: {other:?}").into()),
        }
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(DevClientError::SessionUnusable)
        ));
        drop(session);
        server.await??;
        Ok(())
    }

    #[test]
    fn a_command_result_for_another_command_id_is_rejected() -> Result<(), BoxError> {
        block_on(assert_command_fails(
            Action::Step,
            vec![step_result_frame(41, 8, StepDisposition::Blocked)?],
            |error| {
                matches!(
                    error,
                    DevClientError::CommandIdMismatch {
                        expected: 7,
                        actual: 8
                    }
                )
            },
        ))?
    }

    #[test]
    fn a_server_sequence_gap_or_replay_is_rejected() -> Result<(), BoxError> {
        // A gap (42), a replay of the join's own sequence (40) and a reset (1).
        for wrong in [42, 40, 1] {
            block_on(assert_command_fails(
                Action::Step,
                vec![step_result_frame(wrong, 7, StepDisposition::Blocked)?],
                move |error| {
                    matches!(
                        error,
                        DevClientError::ServerSequenceMismatch {
                            expected: 41,
                            actual
                        } if *actual == wrong
                    )
                },
            ))??;
        }
        Ok(())
    }

    #[test]
    fn a_frame_for_another_connection_generation_is_rejected() -> Result<(), BoxError> {
        let stale = oteryn_protocol_oteryn::encode_command_result(
            2,
            41,
            7,
            CommandStatus::Accepted,
            &world_spatial::encode_step_result(StepDisposition::Blocked),
        )?;
        block_on(assert_command_fails(Action::Step, vec![stale], |error| {
            matches!(
                error,
                DevClientError::ConnectionGenerationMismatch {
                    expected: 1,
                    actual: 2
                }
            )
        }))?
    }

    /// `CommandResult` is a server-sequenced message: an unsequenced one (`server_sequence` 0) is
    /// refused by `WireEnvelopeView::validate` before its payload is consumed.
    #[test]
    fn an_unsequenced_command_result_is_rejected_by_envelope_validation() -> Result<(), BoxError> {
        let unsequenced = test_server_frame_with_sequence(
            MessageType::CommandResult,
            GENERATION,
            0,
            &[0x08, 0x07, 0x10, 0x01, 0x2a, 0x02, 0x08, 0x02],
        );
        block_on(assert_command_fails(
            Action::Step,
            vec![unsequenced],
            |error| {
                matches!(
                    error,
                    DevClientError::Protocol(FoundationProtocolError::MalformedEnvelope)
                )
            },
        ))?
    }

    #[test]
    fn a_protocol_error_in_place_of_a_command_result_is_reported() -> Result<(), BoxError> {
        let gap = oteryn_protocol_oteryn::encode_command_protocol_error(
            FoundationProtocolError::CommandSequenceGap,
            GENERATION,
            7,
            6,
        )?;
        block_on(assert_command_fails(Action::Step, vec![gap], |error| {
            matches!(
                error,
                DevClientError::UnexpectedMessage {
                    expected: MessageType::CommandResult,
                    actual: MessageType::ProtocolError
                }
            )
        }))?
    }

    #[test]
    fn a_moved_step_without_its_delta_is_rejected() -> Result<(), BoxError> {
        block_on(assert_command_fails(
            Action::Step,
            vec![
                step_result_frame(41, 7, StepDisposition::Moved)?,
                step_result_frame(42, 8, StepDisposition::Blocked)?,
            ],
            |error| {
                matches!(
                    error,
                    DevClientError::UnexpectedMessage {
                        expected: MessageType::StateDelta,
                        actual: MessageType::CommandResult
                    }
                )
            },
        ))?
    }

    #[test]
    fn a_spatial_delta_naming_the_wrong_domain_or_type_or_base_is_rejected() -> Result<(), BoxError>
    {
        let moved = step_result_frame(41, 7, StepDisposition::Moved)?;
        let spatial = spatial_payload(1, 0, CONTENT_GENERATION);
        // A domain the session keeps no store for.
        block_on(assert_command_fails(
            Action::Step,
            vec![moved.clone(), state_delta_frame(42, 4242, 0, 1, 1, b"x")?],
            |error| {
                matches!(
                    error,
                    DevClientError::UnsupportedPushedDomain { domain_id: 4242 }
                )
            },
        ))??;
        // Unregistered delta type.
        block_on(assert_command_fails(
            Action::Step,
            vec![moved.clone(), state_delta_frame(42, 1, 5, 6, 2, &spatial)?],
            |error| {
                matches!(
                    error,
                    DevClientError::UnregisteredDeltaType {
                        domain_id: 1,
                        delta_type: 2
                    }
                )
            },
        ))??;
        // Base revision that is not the applied one (5).
        block_on(assert_command_fails(
            Action::Step,
            vec![moved.clone(), spatial_delta_frame(42, 6, 1, 0)?],
            |error| {
                matches!(
                    error,
                    DevClientError::StateRevisionMismatch {
                        domain_id: 1,
                        expected_base: 5,
                        actual_base: 6
                    }
                )
            },
        ))??;
        // A content generation other than the loaded one.
        block_on(assert_command_fails(
            Action::Step,
            vec![
                moved.clone(),
                state_delta_frame(42, 1, 5, 6, 1, &spatial_payload(1, 0, OTHER_GENERATION))?,
            ],
            |error| {
                matches!(
                    error,
                    DevClientError::ContentGenerationMismatch { domain_id: 1 }
                )
            },
        ))??;
        // The delta itself is sequenced: a gap after the result is rejected too.
        block_on(assert_command_fails(
            Action::Step,
            vec![moved, spatial_delta_frame(43, 5, 1, 0)?],
            |error| {
                matches!(
                    error,
                    DevClientError::ServerSequenceMismatch {
                        expected: 42,
                        actual: 43
                    }
                )
            },
        ))?
    }

    #[test]
    fn an_overlay_delta_naming_the_wrong_domain_or_base_or_generation_is_rejected()
    -> Result<(), BoxError> {
        let committed = use_result_frame(41, 7, UseDisposition::Committed)?;
        // A spatial delta based on the wrong revision.
        block_on(assert_command_fails(
            Action::Use,
            vec![committed.clone(), spatial_delta_frame(42, 4, 1, 0)?],
            |error| {
                matches!(
                    error,
                    DevClientError::StateRevisionMismatch {
                        domain_id: 1,
                        expected_base: 5,
                        actual_base: 4
                    }
                )
            },
        ))??;
        // Base revision that is not the applied one (2).
        block_on(assert_command_fails(
            Action::Use,
            vec![
                committed.clone(),
                overlay_delta_frame(42, 1, DOOR_OPEN_STATE)?,
            ],
            |error| {
                matches!(
                    error,
                    DevClientError::StateRevisionMismatch {
                        domain_id: 2,
                        expected_base: 2,
                        actual_base: 1
                    }
                )
            },
        ))??;
        // A content generation other than the loaded one.
        let foreign = world_object::encode_world_object_overlay_delta(&door_entry(
            DOOR_OPEN_STATE,
            3,
            OTHER_GENERATION,
        ))
        .map_err(|error| format!("encode_world_object_overlay_delta: {error:?}"))?;
        block_on(assert_command_fails(
            Action::Use,
            vec![
                committed,
                state_delta_frame(
                    42,
                    2,
                    2,
                    3,
                    world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    &foreign,
                )?,
            ],
            |error| {
                matches!(
                    error,
                    DevClientError::ContentGenerationMismatch { domain_id: 2 }
                )
            },
        ))?
    }

    /// A server that receives the command and never answers fails with a bounded diagnostic
    /// naming the stage, and the session is then unusable.
    #[test]
    fn a_stalled_command_result_times_out_and_poisons_the_session() -> Result<(), BoxError> {
        block_on(run_command_stall_case())?
    }

    async fn run_command_stall_case() -> Result<(), BoxError> {
        let (mut session, server) =
            joined_session(Duration::from_millis(200), |mut stream| async move {
                read_command(&mut stream).await?;
                wait_for_client_close(&mut stream).await;
                Ok(())
            })
            .await?;
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(DevClientError::Timeout("CommandResult"))
        ));
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(DevClientError::SessionUnusable)
        ));
        drop(session);
        server.await??;
        Ok(())
    }

    /// Codex P2: an overlay delta whose decoded entry `revision` differs from the delta's own
    /// `new_revision` is refused and nothing from it is installed.
    #[test]
    fn an_overlay_entry_revision_differing_from_new_revision_is_not_installed()
    -> Result<(), BoxError> {
        block_on(run_overlay_entry_revision_case())?
    }

    async fn run_overlay_entry_revision_case() -> Result<(), BoxError> {
        // base 2 -> new 3, but the entry claims revision 4.
        let entry = world_object::encode_world_object_overlay_delta(&door_entry(
            DOOR_OPEN_STATE,
            4,
            CONTENT_GENERATION,
        ))
        .map_err(|error| format!("encode_world_object_overlay_delta: {error:?}"))?;
        let frames = vec![
            use_result_frame(41, 7, UseDisposition::Committed)?,
            state_delta_frame(
                42,
                STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                2,
                3,
                world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                &entry,
            )?,
        ];
        let (mut session, server) = joined_session(TEST_DEADLINE, move |mut stream| async move {
            read_command(&mut stream).await?;
            send(&mut stream, &frames).await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
        session
            .use_object(DOOR_PLACEMENT.as_bytes(), JOIN_DOOR_REVISION)
            .await?;
        let result = session.service_liveness(Duration::from_millis(200)).await;
        assert!(matches!(
            result,
            Err(DevClientError::OverlayEntryRevisionMismatch {
                new_revision: 3,
                entry_revision: 4
            })
        ));
        assert_eq!(
            session.world_object_overlay(),
            &[door_entry(
                DOOR_STATE,
                JOIN_DOOR_REVISION,
                CONTENT_GENERATION
            )]
        );
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(DevClientError::SessionUnusable)
        ));
        drop(session);
        server.await??;
        Ok(())
    }

    /// Codex P2: a `CommandResult` status that does not pair with its disposition fails closed
    /// (the server sends `ACCEPTED` for everything except `Rejected`, which gets `REJECTED`).
    #[test]
    fn an_inconsistent_status_and_disposition_pairing_is_rejected() -> Result<(), BoxError> {
        let step = |disposition| world_spatial::encode_step_result(disposition);
        let use_ = |disposition| world_object::encode_use_result(disposition);
        for (action, status, payload) in [
            (
                Action::Step,
                CommandStatus::Accepted,
                step(StepDisposition::Rejected),
            ),
            (
                Action::Step,
                CommandStatus::Rejected,
                step(StepDisposition::Blocked),
            ),
            (
                Action::Step,
                CommandStatus::Rejected,
                step(StepDisposition::Moved),
            ),
            (
                Action::Use,
                CommandStatus::Rejected,
                use_(UseDisposition::Committed),
            ),
            (
                Action::Use,
                CommandStatus::Rejected,
                use_(UseDisposition::StaleState),
            ),
            (
                Action::Use,
                CommandStatus::Accepted,
                use_(UseDisposition::Rejected),
            ),
        ] {
            block_on(assert_command_fails(
                action,
                vec![command_result_frame(41, 7, status, &payload)?],
                move |error| {
                    matches!(
                        error,
                        DevClientError::InconsistentCommandResult { command_id: 7, status: actual }
                            if *actual == status
                    )
                },
            ))??;
        }
        Ok(())
    }

    /// Codex P2: a probe that arrives while the session is idle is acked by `service_liveness`,
    /// which then returns when the window ends; the session stays usable.
    #[test]
    fn service_liveness_acks_probes_that_arrive_while_idle() -> Result<(), BoxError> {
        block_on(run_idle_liveness_case())?
    }

    async fn run_idle_liveness_case() -> Result<(), BoxError> {
        let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
            for probe_id in [5, 6] {
                send(
                    &mut stream,
                    &[oteryn_protocol_oteryn::encode_liveness_probe(
                        GENERATION, probe_id,
                    )?],
                )
                .await?;
                let body = read_frame(&mut stream).await?;
                assert_eq!(
                    decode_wire_envelope(&body)?.liveness_ack(GENERATION)?,
                    oteryn_protocol_oteryn::LivenessAckView {
                        probe_id,
                        last_applied_server_sequence: Some(JOIN_SEQUENCE),
                    }
                );
            }
            expect_step(&mut stream, 7, StepDirection::East).await?;
            send(
                &mut stream,
                &[step_result_frame(41, 7, StepDisposition::Blocked)?],
            )
            .await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
        let started = tokio::time::Instant::now();
        session.service_liveness(Duration::from_millis(400)).await?;
        assert!(started.elapsed() >= Duration::from_millis(350));
        assert_eq!(
            session.step(StepDirection::East).await?,
            step_outcome(7, 41, StepDisposition::Blocked, None)
        );
        drop(session);
        server.await??;
        Ok(())
    }

    /// `service_liveness` fails closed on a reused probe ID, and the session is then unusable.
    #[test]
    fn service_liveness_rejects_a_reused_probe_id() -> Result<(), BoxError> {
        block_on(run_idle_reused_probe_case())?
    }

    async fn run_idle_reused_probe_case() -> Result<(), BoxError> {
        let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
            for _ in 0..2 {
                send(
                    &mut stream,
                    &[oteryn_protocol_oteryn::encode_liveness_probe(
                        GENERATION, 5,
                    )?],
                )
                .await?;
            }
            let _first_ack = read_frame(&mut stream).await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
        assert!(matches!(
            session.service_liveness(Duration::from_secs(2)).await,
            Err(DevClientError::ProbeIdNotAdvancing {
                last: 5,
                received: 5
            })
        ));
        assert!(matches!(
            session.service_liveness(Duration::from_millis(10)).await,
            Err(DevClientError::SessionUnusable)
        ));
        drop(session);
        server.await??;
        Ok(())
    }

    #[test]
    fn service_liveness_fails_closed_on_an_unexpected_frame() -> Result<(), BoxError> {
        block_on(run_idle_unexpected_frame_case())?
    }

    async fn run_idle_unexpected_frame_case() -> Result<(), BoxError> {
        let (mut session, server) = joined_session(TEST_DEADLINE, |mut stream| async move {
            send(
                &mut stream,
                &[step_result_frame(41, 7, StepDisposition::Blocked)?],
            )
            .await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
        assert!(matches!(
            session.service_liveness(Duration::from_secs(2)).await,
            Err(DevClientError::UnexpectedMessage {
                expected: MessageType::LivenessProbe,
                actual: MessageType::CommandResult
            })
        ));
        assert!(matches!(
            session.service_liveness(Duration::from_millis(10)).await,
            Err(DevClientError::SessionUnusable)
        ));
        drop(session);
        server.await??;
        Ok(())
    }

    /// Codex P1: FND-02 §13.2 duplicate results for earlier CommandIds this session sent are
    /// surfaced as distinct outcomes (no delta), do not end the wait for the current command's
    /// own result, and do not poison the session.
    #[test]
    fn duplicate_results_for_sent_commands_are_surfaced_and_do_not_poison_the_session()
    -> Result<(), BoxError> {
        block_on(run_duplicate_outcome_case())?
    }

    async fn run_duplicate_outcome_case() -> Result<(), BoxError> {
        let replayed = world_spatial::encode_step_result(StepDisposition::Blocked);
        let replayed_frames = replayed.clone();
        let (mut session, server) = joined_session(TEST_DEADLINE, move |mut stream| async move {
            expect_step(&mut stream, 7, StepDirection::West).await?;
            send(
                &mut stream,
                &[step_result_frame(41, 7, StepDisposition::Blocked)?],
            )
            .await?;
            expect_step(&mut stream, 8, StepDirection::East).await?;
            send(
                &mut stream,
                &[
                    command_result_frame(42, 7, CommandStatus::DuplicateReplay, &replayed_frames)?,
                    command_result_frame(43, 7, CommandStatus::DuplicateOutcomeExpired, &[])?,
                    step_result_frame(44, 8, StepDisposition::Moved)?,
                    spatial_delta_frame(45, 5, 1, 0)?,
                ],
            )
            .await?;
            expect_step(&mut stream, 9, StepDirection::West).await?;
            send(
                &mut stream,
                &[step_result_frame(46, 9, StepDisposition::Blocked)?],
            )
            .await?;
            wait_for_client_close(&mut stream).await;
            Ok(())
        })
        .await?;
        session.step(StepDirection::West).await?;
        assert!(session.take_duplicate_outcomes().is_empty());
        assert_eq!(
            session.step(StepDirection::East).await?,
            step_outcome(8, 44, StepDisposition::Moved, moved_to(45, 5, 1, 0))
        );
        assert_eq!(
            session.take_duplicate_outcomes(),
            vec![
                DuplicateOutcome {
                    command_id: 7,
                    status: CommandStatus::DuplicateReplay,
                    server_sequence: 42,
                    payload: replayed,
                },
                DuplicateOutcome {
                    command_id: 7,
                    status: CommandStatus::DuplicateOutcomeExpired,
                    server_sequence: 43,
                    payload: Vec::new(),
                },
            ]
        );
        assert!(session.take_duplicate_outcomes().is_empty());
        assert_eq!(
            session.step(StepDirection::West).await?,
            step_outcome(9, 46, StepDisposition::Blocked, None)
        );
        drop(session);
        server.await??;
        Ok(())
    }

    /// A duplicate for a CommandId this session never sent (below its first, or not yet sent), or
    /// for the command just sent, fails closed.
    #[test]
    fn a_duplicate_for_an_unsent_command_id_fails_closed() -> Result<(), BoxError> {
        for unsent in [6, 8, 99] {
            block_on(assert_command_fails(
                Action::Step,
                vec![command_result_frame(
                    41,
                    unsent,
                    CommandStatus::DuplicateReplay,
                    &[],
                )?],
                move |error| {
                    matches!(
                        error,
                        DevClientError::DuplicateForUnsentCommand { command_id }
                            if *command_id == unsent
                    )
                },
            ))??;
        }
        block_on(assert_command_fails(
            Action::Step,
            vec![command_result_frame(
                41,
                7,
                CommandStatus::DuplicateOutcomeExpired,
                &[],
            )?],
            |error| {
                matches!(
                    error,
                    DevClientError::InconsistentCommandResult {
                        command_id: 7,
                        status: CommandStatus::DuplicateOutcomeExpired
                    }
                )
            },
        ))?
    }
}
