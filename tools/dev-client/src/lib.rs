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

use oteryn_protocol_oteryn::actor_spell::{ActorVitals, SpellTarget};
use oteryn_protocol_oteryn::world_object::{self, WorldObjectOverlayEntry};
use oteryn_protocol_oteryn::world_spatial::{self, StepDirection, WorldSpatialObservation};
use oteryn_protocol_oteryn::{CharacterId, FoundationProtocolError};
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
    /// `DevClientError::Timeout` (transport) or a wrapped `SessionError::Timeout`
    /// (session), naming the stalled stage.
    pub deadline: Duration,
}

/// The dev client's TLS/TCP adapter failures and transport-neutral session errors.
#[derive(Debug)]
pub enum DevClientError {
    Tls(rustls::Error),
    Io(io::Error),
    InvalidServerName,
    /// The negotiated TLS ALPN protocol was not exactly `oteryn-game/1`.
    AlpnMismatch,
    /// The TCP connect or TLS handshake exceeded `JoinRequest::deadline`.
    Timeout(&'static str),
    Session(SessionError),
}

impl From<SessionError> for DevClientError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
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
        Self::Session(SessionError::from(error))
    }
}

impl From<world_spatial::WorldSpatialError> for DevClientError {
    fn from(error: world_spatial::WorldSpatialError) -> Self {
        Self::Session(SessionError::from(error))
    }
}

impl From<world_object::WorldObjectError> for DevClientError {
    fn from(error: world_object::WorldObjectError) -> Self {
        Self::Session(SessionError::from(error))
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
            Self::Timeout(stage) => write!(formatter, "timed out waiting for {stage}"),
            Self::Session(error) => fmt::Display::fmt(error, formatter),
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
