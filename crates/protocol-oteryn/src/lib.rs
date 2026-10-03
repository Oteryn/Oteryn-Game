//! FND-02 wire codecs shared by the Oteryn game server and the future native client.
//!
//! Extracted from `apps/game-server/src/foundation/protocol.rs` and
//! `apps/game-server/src/gameplay_transport/{world_spatial,world_object}.rs` per ADR-0011 §2
//! (move exactly the real items that have a consumer, no speculative API) and the #162 A6 owner
//! acceptance of task `OTV2-20260928-protocol-oteryn-crate-c1a`. This crate is the FND-02 wire
//! boundary: bounded frame length-prefixing, the top-level envelope message types and their
//! encode/decode, the ALPN identifier `oteryn-game/1`, and the typed FIRST-CONTROL-WIRE-V1
//! (`world_spatial`) and USE-WIRE-V1 (`world_object`) payload codecs. Server-only concerns
//! (authority, admission, connection handling, durable command-lifecycle bookkeeping) stay in
//! `oteryn-game-server`; the game server re-exports what it needs from here.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FoundationProtocolError {
    MalformedFrame = 1001,
    FrameTooLarge = 1002,
    MalformedEnvelope = 1003,
    UnknownMessageType = 1004,
    ProtocolMajorMismatch = 1005,
    TransportProfileMismatch = 1006,
    CapabilityMismatch = 1007,
    InvalidWireIdentifier = 1008,
    PayloadLimitExceeded = 1009,
    StaleConnectionGeneration = 1010,
    CommandOutcomeExpired = 1020,
    CommandSequenceGap = 1021,
    TooManyOutstandingCommands = 1022,
    ServerSequenceGap = 1030,
    StateRevisionMismatch = 1031,
    SnapshotAssemblyInvalid = 1032,
    SnapshotLimitExceeded = 1033,
    BootstrapLimitExceeded = 1040,
    InvalidCapabilitySet = 1041,
    /// An account's stored data contradicts the server's own content (an Achievement fact under a
    /// key the catalogue lacks): an internal integrity fault. The client learns only that its
    /// account data needs support; the diagnostic stays in the operator event.
    AccountDataIntegrity = 1050,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ProtocolDisposition {
    OperationTerminal = 1,
    ResyncRequired = 2,
    SessionFatal = 3,
    TransportFatal = 4,
}

impl FoundationProtocolError {
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }

    #[must_use]
    pub const fn disposition(self) -> ProtocolDisposition {
        match self {
            Self::MalformedFrame
            | Self::FrameTooLarge
            | Self::MalformedEnvelope
            | Self::StaleConnectionGeneration => ProtocolDisposition::TransportFatal,
            Self::UnknownMessageType
            | Self::ProtocolMajorMismatch
            | Self::TransportProfileMismatch
            | Self::CapabilityMismatch
            | Self::InvalidWireIdentifier
            | Self::SnapshotLimitExceeded
            | Self::BootstrapLimitExceeded
            | Self::InvalidCapabilitySet => ProtocolDisposition::SessionFatal,
            Self::PayloadLimitExceeded
            | Self::TooManyOutstandingCommands
            | Self::AccountDataIntegrity => ProtocolDisposition::OperationTerminal,
            Self::CommandOutcomeExpired
            | Self::CommandSequenceGap
            | Self::ServerSequenceGap
            | Self::StateRevisionMismatch
            | Self::SnapshotAssemblyInvalid => ProtocolDisposition::ResyncRequired,
        }
    }
}

impl Display for FoundationProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MalformedFrame => "malformed protocol frame",
            Self::FrameTooLarge => "protocol frame exceeds hard limit",
            Self::MalformedEnvelope => "malformed protocol envelope",
            Self::UnknownMessageType => "unknown foundation message type",
            Self::ProtocolMajorMismatch => "protocol major mismatch",
            Self::TransportProfileMismatch => "transport profile mismatch",
            Self::CapabilityMismatch => "capability mismatch",
            Self::InvalidWireIdentifier => "invalid wire identifier",
            Self::PayloadLimitExceeded => "payload exceeds hard limit",
            Self::StaleConnectionGeneration => "connection generation is stale",
            Self::CommandOutcomeExpired => "command outcome is no longer retained",
            Self::CommandSequenceGap => "command sequence contains a gap",
            Self::TooManyOutstandingCommands => "too many commands are outstanding",
            Self::ServerSequenceGap => "server sequence contains a gap",
            Self::StateRevisionMismatch => "state revision mismatch",
            Self::SnapshotAssemblyInvalid => "snapshot assembly is invalid",
            Self::SnapshotLimitExceeded => "snapshot exceeds hard limit",
            Self::BootstrapLimitExceeded => "bootstrap payload exceeds hard limit",
            Self::InvalidCapabilitySet => "invalid capability set",
            Self::AccountDataIntegrity => "account data needs support",
        })
    }
}

impl Error for FoundationProtocolError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameLength(u32);

impl FrameLength {
    pub fn new(value: u32) -> Result<Self, FoundationProtocolError> {
        if value == 0 {
            return Err(FoundationProtocolError::MalformedFrame);
        }
        if value > MAX_WIRE_FRAME_BYTES {
            return Err(FoundationProtocolError::FrameTooLarge);
        }
        Ok(Self(value))
    }

    pub fn from_prefix(prefix: &[u8]) -> Result<Self, FoundationProtocolError> {
        let bytes: [u8; 4] = prefix
            .try_into()
            .map_err(|_error| FoundationProtocolError::MalformedFrame)?;
        Self::new(u32::from_be_bytes(bytes))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn to_prefix(self) -> [u8; 4] {
        self.0.to_be_bytes()
    }
}
pub const MAX_WIRE_FRAME_BYTES: u32 = 1_048_576;
pub const PROTOCOL_MAJOR_V1: u32 = 1;
pub const TRANSPORT_PROFILE_TCP_TLS13_V1: u32 = 1;
pub const ALPN_OTERYN_GAME_V1: &str = "oteryn-game/1";
pub const MAX_BOOTSTRAP_PAYLOAD_BYTES: usize = 65_536;
pub const MAX_ADMISSION_MATERIAL_BYTES: usize = 16_384;
pub const MAX_RECONNECT_MATERIAL_BYTES: usize = 16_384;
pub const MAX_CLIENT_BUILD_ID_BYTES: usize = 128;
pub const MAX_CAPABILITY_COUNT: usize = 128;
pub const MAX_ORDINARY_REPEATED_ENTRIES: usize = 4_096;
pub const MAX_COMMAND_EXPECTED_REVISIONS: usize = 64;
pub const MAX_COMMAND_PAYLOAD_BYTES: usize = 65_536;
pub const MAX_COMMAND_RESULT_PAYLOAD_BYTES: usize = 65_536;
pub const MAX_STATE_DOMAINS_PER_SYNC: usize = 256;
pub const MAX_STATE_DELTA_PAYLOAD_BYTES: usize = 262_144;
pub const MAX_SNAPSHOT_CHUNKS: u32 = 256;
pub const MAX_SNAPSHOT_CHUNK_BYTES: usize = 524_288;
pub const MAX_SNAPSHOT_ASSEMBLED_BYTES: u64 = 16_777_216;

// The optional capabilities PROTOCOL_OTERYN_V1_REGISTRY.json registers: 1 BESTIARY_CHARMS_V1
// (CHARM-5, not offered before CHARM-6), 6 WORLD_SPATIAL_ENTITIES (VIS-2, not offered before the
// server composes it), 7 CHAT_V1 (CHAT-1, not offered before CHAT-1b-2 composes it) and 10
// ANALYSER_V1 (ANALYSER-WIRE-1, not offered before ANALYSER-EMIT-1). Registered is not offered:
// the server selects none today.
// Keep this sorted when a later owning gate allocates an additive capability ID.
const REGISTERED_CAPABILITY_IDS_V1: &[u32] = &[1, 6, 7, 10];

fn decode_uuid_v7(input: &[u8]) -> Result<[u8; 16], FoundationProtocolError> {
    let value: [u8; 16] = input
        .try_into()
        .map_err(|_| FoundationProtocolError::InvalidWireIdentifier)?;
    if value.iter().all(|byte| *byte == 0) || value[6] >> 4 != 7 || value[8] & 0xc0 != 0x80 {
        return Err(FoundationProtocolError::InvalidWireIdentifier);
    }
    Ok(value)
}

macro_rules! foundation_uuid_v7_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 16]);
        impl $name {
            pub fn decode(input: &[u8]) -> Result<Self, FoundationProtocolError> {
                decode_uuid_v7(input).map(Self)
            }
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 16] {
                &self.0
            }
        }
    };
}
foundation_uuid_v7_id!(CharacterId);
foundation_uuid_v7_id!(WorldId);
foundation_uuid_v7_id!(ChannelId);
foundation_uuid_v7_id!(NodeId);
foundation_uuid_v7_id!(GameSessionId);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    ClientToServer,
    ServerToClient,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Bootstrap,
    PostAdmission,
    Any,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sequencing {
    None,
    CommandId,
    ServerSequenced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MessageType {
    ClientBootstrap = 1,
    ServerAccepted = 2,
    ClientResume = 3,
    ServerResumeAccepted = 4,
    LivenessProbe = 5,
    LivenessAck = 6,
    ClientCommand = 7,
    CommandResult = 8,
    StateDelta = 9,
    ResyncRequest = 10,
    SnapshotBegin = 11,
    SnapshotChunk = 12,
    SnapshotCommit = 13,
    ProtocolError = 14,
}
impl MessageType {
    pub const fn direction(self) -> Direction {
        match self {
            Self::ClientBootstrap
            | Self::ClientResume
            | Self::LivenessAck
            | Self::ClientCommand
            | Self::ResyncRequest => Direction::ClientToServer,
            _ => Direction::ServerToClient,
        }
    }
    pub const fn phase(self) -> Phase {
        match self {
            Self::ClientBootstrap
            | Self::ServerAccepted
            | Self::ClientResume
            | Self::ServerResumeAccepted => Phase::Bootstrap,
            Self::ProtocolError => Phase::Any,
            _ => Phase::PostAdmission,
        }
    }
    pub const fn sequencing(self) -> Sequencing {
        match self {
            Self::ClientCommand => Sequencing::CommandId,
            Self::CommandResult | Self::StateDelta => Sequencing::ServerSequenced,
            _ => Sequencing::None,
        }
    }
}
impl TryFrom<u32> for MessageType {
    type Error = FoundationProtocolError;
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        Ok(match v {
            1 => Self::ClientBootstrap,
            2 => Self::ServerAccepted,
            3 => Self::ClientResume,
            4 => Self::ServerResumeAccepted,
            5 => Self::LivenessProbe,
            6 => Self::LivenessAck,
            7 => Self::ClientCommand,
            8 => Self::CommandResult,
            9 => Self::StateDelta,
            10 => Self::ResyncRequest,
            11 => Self::SnapshotBegin,
            12 => Self::SnapshotChunk,
            13 => Self::SnapshotCommit,
            14 => Self::ProtocolError,
            _ => return Err(FoundationProtocolError::UnknownMessageType),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireEnvelopeView<'a> {
    message_type: MessageType,
    connection_generation: u64,
    server_sequence: u64,
    payload: &'a [u8],
    bootstrap: Option<BootstrapIngressView<'a>>,
    liveness_ack: Option<LivenessAckView>,
}

// Fixed-size validated metadata preserves the envelope's Copy contract without
// allocating from peer-controlled counts. Material/build strings borrow input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BootstrapIngressView<'a> {
    schema_revision: u32,
    identity: [u8; 16],
    material: &'a [u8],
    build_id: &'a str,
    capabilities: [u32; MAX_CAPABILITY_COUNT],
    capability_count: usize,
    last_applied_sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientBootstrapView<'a> {
    pub protocol_major: u32,
    pub transport_profile: u32,
    pub schema_revision: u32,
    pub character_id: CharacterId,
    pub admission_material: &'a [u8],
    pub client_build_id: &'a str,
    pub supported_capabilities: &'a [u32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientResumeView<'a> {
    pub protocol_major: u32,
    pub transport_profile: u32,
    pub schema_revision: u32,
    pub game_session_id: GameSessionId,
    pub reconnect_material: &'a [u8],
    pub client_build_id: &'a str,
    pub supported_capabilities: &'a [u32],
    pub last_applied_server_sequence: u64,
}

// Payload values alone do not establish current transport authority or receipt time.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LivenessAckView {
    pub probe_id: u64,
    pub last_applied_server_sequence: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct ServerAcceptedValue<'a> {
    pub game_session_id: GameSessionId,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub connection_generation: u64,
    pub current_server_sequence: u64,
    pub next_command_id: u64,
    pub schema_revision: u32,
    pub selected_capabilities: &'a [u32],
}

// Resume acknowledgement for the seam's resume path.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct ServerResumeAcceptedValue<'a> {
    pub game_session_id: GameSessionId,
    pub connection_generation: u64,
    pub current_server_sequence: u64,
    pub next_command_id: u64,
    pub schema_revision: u32,
    pub selected_capabilities: &'a [u32],
}

/// `ClientBootstrap` fields a client supplies (FND-02 §9): the client-direction counterpart of
/// `ClientBootstrapView`, for constructing the message instead of decoding it.
/// `protocol_major`/`transport_profile` are not fields here: `encode_client_bootstrap` always
/// sends this crate's own pinned `PROTOCOL_MAJOR_V1`/`TRANSPORT_PROFILE_TCP_TLS13_V1`, so a
/// caller cannot send anything else.
#[derive(Debug, Clone, Copy)]
pub struct ClientBootstrapValue<'a> {
    pub schema_revision: u32,
    pub character_id: CharacterId,
    pub admission_material: &'a [u8],
    pub client_build_id: &'a str,
    pub supported_capabilities: &'a [u32],
}

/// Encodes one framed-message-ready `ClientBootstrap` envelope (FND-02 §9): the wire format
/// `validate_bootstrap_ingress` (via `decode_wire_envelope`/`WireEnvelopeView::client_bootstrap`)
/// already accepts, matching `encode_server_accepted`'s sibling shape on the server-direction
/// side. Length- and content-bounds mirror the ones ingress enforces, so this side never
/// produces a payload a conforming decoder would reject.
pub fn encode_client_bootstrap(
    value: &ClientBootstrapValue<'_>,
) -> Result<Vec<u8>, FoundationProtocolError> {
    if value.schema_revision == 0 || value.client_build_id.is_empty() {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if value.admission_material.is_empty() {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if value.admission_material.len() > MAX_ADMISSION_MATERIAL_BYTES {
        return Err(FoundationProtocolError::BootstrapLimitExceeded);
    }
    if value.client_build_id.len() > MAX_CLIENT_BUILD_ID_BYTES {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    let mut count = 0usize;
    let mut previous = None;
    for &capability in value.supported_capabilities {
        validate_capability(u64::from(capability), &mut count, &mut previous, false)?;
    }
    let mut payload = Vec::new();
    push_scalar(&mut payload, 1, u64::from(PROTOCOL_MAJOR_V1));
    push_scalar(&mut payload, 2, u64::from(TRANSPORT_PROFILE_TCP_TLS13_V1));
    push_scalar(&mut payload, 3, u64::from(value.schema_revision));
    for &capability in value.supported_capabilities {
        push_scalar(&mut payload, 4, u64::from(capability));
    }
    push_bytes(&mut payload, 5, value.admission_material);
    push_bytes(&mut payload, 6, value.character_id.as_bytes());
    push_bytes(&mut payload, 7, value.client_build_id.as_bytes());
    let mut output = vec![8, MessageType::ClientBootstrap as u8];
    push_bytes(&mut output, 4, &payload);
    Ok(output)
}

fn push_varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn push_scalar(output: &mut Vec<u8>, field: u32, value: u64) {
    if value != 0 {
        push_varint(output, u64::from(field) << 3);
        push_varint(output, value);
    }
}

fn push_bytes(output: &mut Vec<u8>, field: u32, value: &[u8]) {
    push_varint(output, (u64::from(field) << 3) | 2);
    push_varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

fn validate_acceptance_value(
    generation: u64,
    command: u64,
    schema: u32,
    capabilities: &[u32],
) -> Result<(), FoundationProtocolError> {
    if generation == 0 || command == 0 || schema == 0 {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if capabilities.len() > MAX_CAPABILITY_COUNT {
        return Err(FoundationProtocolError::BootstrapLimitExceeded);
    }
    let mut count = 0;
    let mut previous = None;
    for &capability in capabilities {
        validate_capability(u64::from(capability), &mut count, &mut previous, true)?;
    }
    Ok(())
}

pub fn encode_server_accepted(
    value: &ServerAcceptedValue<'_>,
) -> Result<Vec<u8>, FoundationProtocolError> {
    validate_acceptance_value(
        value.connection_generation,
        value.next_command_id,
        value.schema_revision,
        value.selected_capabilities,
    )?;
    let mut payload = Vec::new();
    push_bytes(&mut payload, 1, value.game_session_id.as_bytes());
    push_bytes(&mut payload, 2, value.world_id.as_bytes());
    push_bytes(&mut payload, 3, value.channel_id.as_bytes());
    push_scalar(&mut payload, 4, value.connection_generation);
    push_scalar(&mut payload, 5, value.current_server_sequence);
    push_scalar(&mut payload, 6, value.next_command_id);
    push_scalar(&mut payload, 7, u64::from(PROTOCOL_MAJOR_V1));
    push_scalar(&mut payload, 8, u64::from(TRANSPORT_PROFILE_TCP_TLS13_V1));
    push_scalar(&mut payload, 9, u64::from(value.schema_revision));
    for &capability in value.selected_capabilities {
        push_scalar(&mut payload, 10, u64::from(capability));
    }
    let mut output = vec![8, 2];
    push_bytes(&mut output, 4, &payload);
    Ok(output)
}

#[allow(dead_code)]
pub fn encode_server_resume_accepted(
    value: &ServerResumeAcceptedValue<'_>,
) -> Result<Vec<u8>, FoundationProtocolError> {
    validate_acceptance_value(
        value.connection_generation,
        value.next_command_id,
        value.schema_revision,
        value.selected_capabilities,
    )?;
    let mut payload = Vec::new();
    push_bytes(&mut payload, 1, value.game_session_id.as_bytes());
    push_scalar(&mut payload, 2, value.connection_generation);
    push_scalar(&mut payload, 3, value.current_server_sequence);
    push_scalar(&mut payload, 4, value.next_command_id);
    push_scalar(&mut payload, 5, u64::from(value.schema_revision));
    for &capability in value.selected_capabilities {
        push_scalar(&mut payload, 6, u64::from(capability));
    }
    let mut output = vec![8, 4];
    push_bytes(&mut output, 4, &payload);
    Ok(output)
}

// Stateless wire construction. The current owner must enforce monotonic probe
// IDs and terminal exhaustion before calling this encoder.
#[allow(dead_code)]
pub fn encode_liveness_probe(
    connection_generation: u64,
    probe_id: u64,
) -> Result<Vec<u8>, FoundationProtocolError> {
    if connection_generation == 0 || probe_id == 0 {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    let mut payload = Vec::new();
    push_scalar(&mut payload, 1, probe_id);
    let mut output = vec![8, MessageType::LivenessProbe as u8];
    push_scalar(&mut output, 2, connection_generation);
    push_bytes(&mut output, 4, &payload);
    Ok(output)
}

/// `CommandStatus` values of FND-02 `CommandResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CommandStatus {
    Accepted = 1,
    // Constructed by the gameplay transport, which path-included test crates omit.
    #[allow(dead_code)]
    Rejected = 2,
    /// `COMMAND_STATUS_DUPLICATE_REPLAY` (FND-02 §13.2): the retained terminal result of an
    /// already reserved lower `CommandId`, replayed; the command is never executed again.
    DuplicateReplay = 3,
    /// `COMMAND_STATUS_DUPLICATE_OUTCOME_EXPIRED` (FND-02 §13.2): an already reserved lower
    /// `CommandId` whose terminal result is no longer retained; reconciliation is required.
    DuplicateOutcomeExpired = 4,
}

/// One post-admission `ClientCommand` (FND-02): its command identity, registered type and the
/// typed payload owned by that type. Expected revisions are validated and bounded but unused by
/// the first registered command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientCommandView<'a> {
    pub command_id: u64,
    pub command_type: u32,
    pub payload: &'a [u8],
}

fn server_frame(
    message_type: MessageType,
    generation: u64,
    sequence: u64,
    payload: &[u8],
) -> Vec<u8> {
    let mut output = Vec::with_capacity(payload.len() + 24);
    push_scalar(&mut output, 1, message_type as u64);
    push_scalar(&mut output, 2, generation);
    push_scalar(&mut output, 3, sequence);
    push_bytes(&mut output, 4, payload);
    output
}

/// Server-sequenced `CommandResult`. The typed payload is owned by the command type.
pub fn encode_command_result(
    connection_generation: u64,
    server_sequence: u64,
    command_id: u64,
    status: CommandStatus,
    payload: &[u8],
) -> Result<Vec<u8>, FoundationProtocolError> {
    if connection_generation == 0 || server_sequence == 0 || command_id == 0 {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if payload.len() > MAX_COMMAND_RESULT_PAYLOAD_BYTES {
        return Err(FoundationProtocolError::PayloadLimitExceeded);
    }
    let mut body = Vec::with_capacity(payload.len() + 16);
    push_scalar(&mut body, 1, command_id);
    push_scalar(&mut body, 2, status as u64);
    push_bytes(&mut body, 5, payload);
    Ok(server_frame(
        MessageType::CommandResult,
        connection_generation,
        server_sequence,
        &body,
    ))
}

/// Server-sequenced `REJECTED` `CommandResult` carrying an `OPERATION_TERMINAL` registered error
/// in `error_code` and no typed payload (FND-02 §18): the command ends, the session does not.
/// Any other disposition is refused, since it is not the outcome of one command.
pub fn encode_command_error_result(
    connection_generation: u64,
    server_sequence: u64,
    command_id: u64,
    error: FoundationProtocolError,
) -> Result<Vec<u8>, FoundationProtocolError> {
    if connection_generation == 0 || server_sequence == 0 || command_id == 0 {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if error.disposition() != ProtocolDisposition::OperationTerminal {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    let mut body = Vec::with_capacity(16);
    push_scalar(&mut body, 1, command_id);
    push_scalar(&mut body, 2, CommandStatus::Rejected as u64);
    push_scalar(&mut body, 3, u64::from(error.code()));
    Ok(server_frame(
        MessageType::CommandResult,
        connection_generation,
        server_sequence,
        &body,
    ))
}

/// Server-sequenced `StateDelta` from `base_revision` to `new_revision` of one domain.
#[allow(clippy::too_many_arguments)]
pub fn encode_state_delta(
    connection_generation: u64,
    server_sequence: u64,
    domain_id: u32,
    base_revision: u64,
    new_revision: u64,
    delta_type: u32,
    payload: &[u8],
) -> Result<Vec<u8>, FoundationProtocolError> {
    if connection_generation == 0
        || server_sequence == 0
        || domain_id == 0
        || delta_type == 0
        || new_revision <= base_revision
    {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if payload.len() > MAX_STATE_DELTA_PAYLOAD_BYTES {
        return Err(FoundationProtocolError::PayloadLimitExceeded);
    }
    let mut body = Vec::with_capacity(payload.len() + 24);
    push_scalar(&mut body, 1, u64::from(domain_id));
    push_scalar(&mut body, 2, base_revision);
    push_scalar(&mut body, 3, new_revision);
    push_scalar(&mut body, 4, u64::from(delta_type));
    push_bytes(&mut body, 5, payload);
    Ok(server_frame(
        MessageType::StateDelta,
        connection_generation,
        server_sequence,
        &body,
    ))
}

/// One domain of a `SnapshotBody`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainSnapshot<'a> {
    pub domain_id: u32,
    pub revision: u64,
    pub snapshot_type: u32,
    pub payload: &'a [u8],
}

/// The three unsequenced frames of one single-chunk snapshot transfer (FND-02 §16):
/// `SnapshotBegin`, `SnapshotChunk[0]` and `SnapshotCommit`. The body must fit one chunk.
pub fn encode_single_chunk_snapshot(
    connection_generation: u64,
    snapshot_id: u64,
    target_server_sequence: u64,
    domains: &[DomainSnapshot<'_>],
) -> Result<[Vec<u8>; 3], FoundationProtocolError> {
    if connection_generation == 0 || snapshot_id == 0 || domains.is_empty() {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    // The receiver's SnapshotBody validation refuses these; never emit them.
    if domains.len() > MAX_STATE_DOMAINS_PER_SYNC {
        return Err(FoundationProtocolError::PayloadLimitExceeded);
    }
    let mut body = Vec::new();
    for (index, domain) in domains.iter().enumerate() {
        if domain.domain_id == 0
            || domain.snapshot_type == 0
            || domains[..index]
                .iter()
                .any(|earlier| earlier.domain_id == domain.domain_id)
        {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        let mut entry = Vec::with_capacity(domain.payload.len() + 24);
        push_scalar(&mut entry, 1, u64::from(domain.domain_id));
        push_scalar(&mut entry, 2, domain.revision);
        push_scalar(&mut entry, 3, u64::from(domain.snapshot_type));
        push_bytes(&mut entry, 4, domain.payload);
        push_bytes(&mut body, 1, &entry);
    }
    if body.len() > MAX_SNAPSHOT_CHUNK_BYTES {
        return Err(FoundationProtocolError::PayloadLimitExceeded);
    }
    let mut begin = Vec::new();
    push_scalar(&mut begin, 1, snapshot_id);
    push_scalar(&mut begin, 2, 1);
    push_scalar(&mut begin, 3, body.len() as u64);
    push_scalar(&mut begin, 4, target_server_sequence);
    let mut chunk = Vec::with_capacity(body.len() + 16);
    push_scalar(&mut chunk, 1, snapshot_id);
    push_bytes(&mut chunk, 3, &body);
    let mut commit = Vec::new();
    push_scalar(&mut commit, 1, snapshot_id);
    Ok([
        server_frame(MessageType::SnapshotBegin, connection_generation, 0, &begin),
        server_frame(MessageType::SnapshotChunk, connection_generation, 0, &chunk),
        server_frame(
            MessageType::SnapshotCommit,
            connection_generation,
            0,
            &commit,
        ),
    ])
}

pub fn encode_protocol_error(
    error: FoundationProtocolError,
    generation: u64,
) -> Result<Vec<u8>, FoundationProtocolError> {
    encode_command_protocol_error(error, generation, 0, 0)
}

/// `ProtocolError` with its optional command correlation (zero means not applicable): the
/// offending `related_command_id` and, for a gap, the `expected_command_id` (FND-02 §13.2).
pub fn encode_command_protocol_error(
    error: FoundationProtocolError,
    generation: u64,
    related_command_id: u64,
    expected_command_id: u64,
) -> Result<Vec<u8>, FoundationProtocolError> {
    let mut payload = Vec::new();
    push_scalar(&mut payload, 1, u64::from(error.code()));
    push_scalar(&mut payload, 2, u64::from(error.disposition() as u32));
    push_scalar(&mut payload, 3, related_command_id);
    push_scalar(&mut payload, 4, expected_command_id);
    let mut output = vec![8, 14];
    push_scalar(&mut output, 2, generation);
    push_bytes(&mut output, 4, &payload);
    Ok(output)
}

/// One `ClientCommand` a client sends (FND-02 §13): the client-direction counterpart of
/// `ClientCommandView`, for constructing the message instead of decoding it. Expected revisions
/// are not a field: no registered command type consumes them (`ClientCommandView` documents the
/// same), so `encode_client_command` never emits `expected_revision`.
#[derive(Debug, Clone, Copy)]
pub struct ClientCommandValue<'a> {
    pub command_id: u64,
    pub command_type: u32,
    pub payload: &'a [u8],
}

/// Encodes one framed-message-ready post-admission `ClientCommand` envelope carrying the current
/// non-zero `connection_generation`. Bounds mirror `WireEnvelopeView::client_command`, so this
/// side never produces a command a conforming server ingress would reject.
pub fn encode_client_command(
    connection_generation: u64,
    value: &ClientCommandValue<'_>,
) -> Result<Vec<u8>, FoundationProtocolError> {
    if connection_generation == 0 || value.command_id == 0 || value.command_type == 0 {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if value.payload.len() > MAX_COMMAND_PAYLOAD_BYTES {
        return Err(FoundationProtocolError::PayloadLimitExceeded);
    }
    let mut body = Vec::with_capacity(value.payload.len() + 16);
    push_scalar(&mut body, 1, value.command_id);
    push_scalar(&mut body, 2, u64::from(value.command_type));
    push_bytes(&mut body, 4, value.payload);
    let mut output = Vec::with_capacity(body.len() + 16);
    push_scalar(&mut output, 1, MessageType::ClientCommand as u64);
    push_scalar(&mut output, 2, connection_generation);
    push_bytes(&mut output, 4, &body);
    Ok(output)
}

/// Encodes one post-admission `LivenessAck` answering the server's `LivenessProbe` numbered
/// `probe_id`. `last_applied_server_sequence` is always emitted, including an explicit zero
/// (the diagnostic hint stays distinct from omission, as `validate_liveness_ack_ingress` reads it).
pub fn encode_liveness_ack(
    connection_generation: u64,
    probe_id: u64,
    last_applied_server_sequence: u64,
) -> Result<Vec<u8>, FoundationProtocolError> {
    if connection_generation == 0 || probe_id == 0 {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    let mut payload = Vec::new();
    push_scalar(&mut payload, 1, probe_id);
    push_varint(&mut payload, 2 << 3);
    push_varint(&mut payload, last_applied_server_sequence);
    let mut output = vec![8, MessageType::LivenessAck as u8];
    push_scalar(&mut output, 2, connection_generation);
    push_bytes(&mut output, 4, &payload);
    Ok(output)
}

/// Decodes a `LivenessProbe` message payload's `probe_id` (field 1; zero or absent is invalid,
/// `foundation.proto`: "Monotonic within one connection_generation; zero is invalid"), the
/// client-direction counterpart of `encode_liveness_probe`.
pub fn decode_liveness_probe(payload: &[u8]) -> Result<u64, FoundationProtocolError> {
    let mut cursor = 0usize;
    let mut probe_id = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            1 => read_singular_varint(payload, &mut cursor, wire, &mut probe_id)?,
            _ => skip_field(payload, &mut cursor, wire)?,
        }
    }
    probe_id
        .filter(|id| *id != 0)
        .ok_or(FoundationProtocolError::MalformedEnvelope)
}

/// Decoded `CommandResult` (FND-02 §13): the client-direction counterpart of
/// `encode_command_result`. `payload` is the typed result owned by the command type (empty for a
/// `Rejected` unregistered command). `authoritative_revision` entries are validated by
/// `decode_wire_envelope`'s ingress but not surfaced: no registered command emits them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandResultView<'a> {
    pub command_id: u64,
    pub status: CommandStatus,
    pub error_code: u32,
    pub payload: &'a [u8],
}

/// Decodes one `CommandResult` message payload — a `WireEnvelopeView::payload()` whose
/// `message_type()` is `MessageType::CommandResult`. Reuses `validate_command_result_ingress`,
/// then recovers the typed fields. A zero/absent `command_id`, or an absent, unspecified or
/// unregistered `status`, fails closed; the four registered statuses (`ACCEPTED`, `REJECTED`,
/// `DUPLICATE_REPLAY`, `DUPLICATE_OUTCOME_EXPIRED`) decode.
pub fn decode_command_result(
    payload: &[u8],
) -> Result<CommandResultView<'_>, FoundationProtocolError> {
    validate_command_result_ingress(payload)?;
    let mut cursor = 0usize;
    let (mut command_id, mut status, mut error_code, mut result_payload) = (None, None, None, None);
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            1 => read_singular_varint(payload, &mut cursor, wire, &mut command_id)?,
            2 => read_singular_varint(payload, &mut cursor, wire, &mut status)?,
            3 => read_singular_varint(payload, &mut cursor, wire, &mut error_code)?,
            5 => read_singular_bytes(
                payload,
                &mut cursor,
                wire,
                &mut result_payload,
                MAX_COMMAND_RESULT_PAYLOAD_BYTES,
                FoundationProtocolError::PayloadLimitExceeded,
            )?,
            _ => skip_field(payload, &mut cursor, wire)?,
        }
    }
    let status = match status {
        Some(1) => CommandStatus::Accepted,
        Some(2) => CommandStatus::Rejected,
        Some(3) => CommandStatus::DuplicateReplay,
        Some(4) => CommandStatus::DuplicateOutcomeExpired,
        _ => return Err(FoundationProtocolError::MalformedEnvelope),
    };
    Ok(CommandResultView {
        command_id: command_id
            .filter(|id| *id != 0)
            .ok_or(FoundationProtocolError::MalformedEnvelope)?,
        status,
        error_code: u32::try_from(error_code.unwrap_or(0))
            .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
        payload: result_payload.unwrap_or(&[]),
    })
}

/// Decoded `StateDelta` (FND-02 §15): the client-direction counterpart of `encode_state_delta`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateDeltaView<'a> {
    pub domain_id: u32,
    pub base_revision: u64,
    pub new_revision: u64,
    pub delta_type: u32,
    pub payload: &'a [u8],
}

/// Decodes one `StateDelta` message payload — a `WireEnvelopeView::payload()` whose
/// `message_type()` is `MessageType::StateDelta`. Reuses `validate_state_delta_ingress`, then
/// applies the same semantic bounds `encode_state_delta` enforces: a non-zero `domain_id` and
/// `delta_type`, and `new_revision` strictly greater than `base_revision`.
pub fn decode_state_delta(payload: &[u8]) -> Result<StateDeltaView<'_>, FoundationProtocolError> {
    validate_state_delta_ingress(payload)?;
    let mut cursor = 0usize;
    let (mut domain_id, mut base_revision, mut new_revision, mut delta_type, mut delta_payload) =
        (None, None, None, None, None);
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            1 => read_singular_varint(payload, &mut cursor, wire, &mut domain_id)?,
            2 => read_singular_varint(payload, &mut cursor, wire, &mut base_revision)?,
            3 => read_singular_varint(payload, &mut cursor, wire, &mut new_revision)?,
            4 => read_singular_varint(payload, &mut cursor, wire, &mut delta_type)?,
            5 => read_singular_bytes(
                payload,
                &mut cursor,
                wire,
                &mut delta_payload,
                MAX_STATE_DELTA_PAYLOAD_BYTES,
                FoundationProtocolError::PayloadLimitExceeded,
            )?,
            _ => skip_field(payload, &mut cursor, wire)?,
        }
    }
    let domain_id = domain_id
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value != 0);
    let delta_type = delta_type
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value != 0);
    let (base_revision, new_revision) = (base_revision.unwrap_or(0), new_revision.unwrap_or(0));
    match (domain_id, delta_type) {
        (Some(domain_id), Some(delta_type)) if new_revision > base_revision => Ok(StateDeltaView {
            domain_id,
            base_revision,
            new_revision,
            delta_type,
            payload: delta_payload.unwrap_or(&[]),
        }),
        _ => Err(FoundationProtocolError::MalformedEnvelope),
    }
}

/// Decoded `ServerAccepted` (FND-02 §11): the client-direction counterpart of
/// `ServerAcceptedValue`/`encode_server_accepted`, for a client decoding what the server sent
/// instead of encoding it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerAcceptedFields {
    pub game_session_id: GameSessionId,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub connection_generation: u64,
    pub current_server_sequence: u64,
    pub next_command_id: u64,
    pub schema_revision: u32,
    pub selected_capabilities: Vec<u32>,
}

/// Decodes one `ServerAccepted` message payload — a `WireEnvelopeView::payload()` whose
/// `message_type()` is `MessageType::ServerAccepted` (obtained from `decode_wire_envelope` /
/// `decode_framed_envelope`, which already frame- and length-check the input). Reuses
/// `validate_server_acceptance_ingress`, the same structural and semantic validation
/// `decode_wire_envelope` itself applies to every server-direction message, then makes one
/// further pass over the now-validated payload to recover the typed fields.
pub fn decode_server_accepted(
    payload: &[u8],
) -> Result<ServerAcceptedFields, FoundationProtocolError> {
    validate_server_acceptance_ingress(MessageType::ServerAccepted, payload)?;
    let mut cursor = 0usize;
    let (mut session, mut world, mut channel) = (None, None, None);
    let (mut generation, mut sequence, mut next_command_id) = (None, None, None);
    let mut schema_revision = None;
    let mut capabilities = Vec::new();
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            1 => {
                session = Some(GameSessionId::decode(bounded_length_delimited(
                    payload,
                    &mut cursor,
                    16,
                    FoundationProtocolError::InvalidWireIdentifier,
                )?)?);
            }
            2 => {
                world = Some(WorldId::decode(bounded_length_delimited(
                    payload,
                    &mut cursor,
                    16,
                    FoundationProtocolError::InvalidWireIdentifier,
                )?)?);
            }
            3 => {
                channel = Some(ChannelId::decode(bounded_length_delimited(
                    payload,
                    &mut cursor,
                    16,
                    FoundationProtocolError::InvalidWireIdentifier,
                )?)?);
            }
            4 => generation = Some(read_varint(payload, &mut cursor)?),
            5 => sequence = Some(read_varint(payload, &mut cursor)?),
            6 => next_command_id = Some(read_varint(payload, &mut cursor)?),
            9 => schema_revision = Some(read_varint(payload, &mut cursor)?),
            10 if wire == 0 => {
                capabilities.push(
                    u32::try_from(read_varint(payload, &mut cursor)?)
                        .map_err(|_| FoundationProtocolError::InvalidCapabilitySet)?,
                );
            }
            10 => {
                let packed = bounded_length_delimited(
                    payload,
                    &mut cursor,
                    payload.len(),
                    FoundationProtocolError::MalformedEnvelope,
                )?;
                let mut packed_cursor = 0usize;
                while packed_cursor < packed.len() {
                    capabilities.push(
                        u32::try_from(read_varint(packed, &mut packed_cursor)?)
                            .map_err(|_| FoundationProtocolError::InvalidCapabilitySet)?,
                    );
                }
            }
            _ => skip_field(payload, &mut cursor, wire)?,
        }
    }
    Ok(ServerAcceptedFields {
        game_session_id: session.ok_or(FoundationProtocolError::InvalidWireIdentifier)?,
        world_id: world.ok_or(FoundationProtocolError::InvalidWireIdentifier)?,
        channel_id: channel.ok_or(FoundationProtocolError::InvalidWireIdentifier)?,
        connection_generation: generation.ok_or(FoundationProtocolError::MalformedEnvelope)?,
        current_server_sequence: sequence.unwrap_or(0),
        next_command_id: next_command_id.ok_or(FoundationProtocolError::MalformedEnvelope)?,
        schema_revision: u32::try_from(
            schema_revision.ok_or(FoundationProtocolError::MalformedEnvelope)?,
        )
        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
        selected_capabilities: capabilities,
    })
}

/// Decodes a `SnapshotBegin` or `SnapshotCommit` message payload's `snapshot_id` (field 1 of
/// both — FND-02 §16, `encode_single_chunk_snapshot`'s `begin`/`commit`). A client correlates
/// every frame of one snapshot transfer (`SnapshotBegin`, `SnapshotChunk` via
/// `decode_snapshot_chunk`, `SnapshotCommit`) by this identity before trusting any of them.
pub fn decode_snapshot_id(payload: &[u8]) -> Result<u64, FoundationProtocolError> {
    let mut cursor = 0usize;
    let mut snapshot_id = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        if field == 1 && wire == 0 && snapshot_id.is_none() {
            snapshot_id = Some(read_varint(payload, &mut cursor)?);
        } else {
            skip_field(payload, &mut cursor, wire)?;
        }
    }
    // Zero is invalid (`foundation.proto`: "Zero is invalid"); a standard proto3 encoder omits
    // a zero value, so an absent field and an explicit zero are indistinguishable on the wire
    // and both are refused the same way (FND-02 §7 semantic-presence validation).
    snapshot_id
        .filter(|id| *id != 0)
        .ok_or(FoundationProtocolError::MalformedEnvelope)
}

/// Decoded `SnapshotBegin` (FND-02 §16, `foundation.proto` `SnapshotBegin`): a transfer's full
/// declaration, which a client checks the rest of the transfer against — exactly `chunk_count`
/// `SnapshotChunk` frames with strictly increasing `chunk_index`, and their summed `data` bytes
/// equal to `total_encoded_bytes` — before trusting `SnapshotCommit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotBeginFields {
    pub snapshot_id: u64,
    pub chunk_count: u32,
    pub total_encoded_bytes: u64,
    /// After the atomic snapshot application this becomes the client's
    /// `last_applied_server_sequence` (`foundation.proto` `SnapshotBegin.target_server_sequence`;
    /// an omitted field is the proto3 default 0, the fresh-admission value).
    pub target_server_sequence: u64,
}

/// Decodes one `SnapshotBegin` message payload. Reuses `validate_snapshot_begin_ingress` (the
/// same `chunk_count <= MAX_SNAPSHOT_CHUNKS` / `total_encoded_bytes <= MAX_SNAPSHOT_ASSEMBLED_BYTES`
/// bounds `decode_wire_envelope` already applies), then extracts the fields.
pub fn decode_snapshot_begin(
    payload: &[u8],
) -> Result<SnapshotBeginFields, FoundationProtocolError> {
    validate_snapshot_begin_ingress(payload)?;
    let mut cursor = 0usize;
    let (mut snapshot_id, mut chunk_count, mut total_encoded_bytes, mut target_server_sequence) =
        (None, None, None, None);
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            1 if wire == 0 && snapshot_id.is_none() => {
                snapshot_id = Some(read_varint(payload, &mut cursor)?);
            }
            2 if wire == 0 && chunk_count.is_none() => {
                chunk_count = Some(
                    u32::try_from(read_varint(payload, &mut cursor)?)
                        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
                );
            }
            3 if wire == 0 && total_encoded_bytes.is_none() => {
                total_encoded_bytes = Some(read_varint(payload, &mut cursor)?);
            }
            4 if wire == 0 && target_server_sequence.is_none() => {
                target_server_sequence = Some(read_varint(payload, &mut cursor)?);
            }
            _ => skip_field(payload, &mut cursor, wire)?,
        }
    }
    Ok(SnapshotBeginFields {
        // Zero is invalid for snapshot_id (foundation.proto); an omitted field defaults to 0 on
        // the wire either way, so absence is refused exactly like an explicit zero would be.
        snapshot_id: snapshot_id
            .filter(|id| *id != 0)
            .ok_or(FoundationProtocolError::MalformedEnvelope)?,
        // chunk_count and total_encoded_bytes are ordinary proto3 scalars: a standard encoder
        // omits a zero value, so both default on omission (FND-02 §7); the caller judges whether
        // a declared zero is semantically usable.
        chunk_count: chunk_count.unwrap_or(0),
        total_encoded_bytes: total_encoded_bytes.unwrap_or(0),
        target_server_sequence: target_server_sequence.unwrap_or(0),
    })
}

/// A `SnapshotChunk` message payload's `chunk_index` (field 2, zero-based, proto3-omitted-as-0 —
/// `encode_single_chunk_snapshot` never emits it since its one chunk is always index 0) and its
/// raw `data` field (field 3): the exact byte slice `SnapshotBegin`'s `total_encoded_bytes`
/// counts and every chunk's `data` concatenates (in `chunk_index` order) into the assembled
/// `SnapshotBody` — a multi-chunk transfer may split a `SnapshotBody` field at any byte offset,
/// not necessarily a field boundary, so a client must concatenate every chunk's `data` before
/// decoding any of it (`decode_snapshot_body`), never decode one chunk's `data` on its own.
pub fn decode_snapshot_chunk_framing(
    payload: &[u8],
) -> Result<(u32, &[u8]), FoundationProtocolError> {
    let mut cursor = 0usize;
    let (mut chunk_index, mut data) = (None, None);
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            2 if wire == 0 && chunk_index.is_none() => {
                chunk_index = Some(
                    u32::try_from(read_varint(payload, &mut cursor)?)
                        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
                );
            }
            3 if wire == 2 && data.is_none() => {
                data = Some(bounded_length_delimited(
                    payload,
                    &mut cursor,
                    MAX_SNAPSHOT_CHUNK_BYTES,
                    FoundationProtocolError::SnapshotLimitExceeded,
                )?);
            }
            _ => skip_field(payload, &mut cursor, wire)?,
        }
    }
    Ok((chunk_index.unwrap_or(0), data.unwrap_or(&[])))
}

/// Decodes an assembled `SnapshotBody` (FND-02 §16: the concatenation, in `chunk_index` order, of
/// every `SnapshotChunk.data` in one transfer — for a single-chunk transfer, that one chunk's
/// `data` unchanged) into its `DomainSnapshot` entries. `FND02-STATE-DOMAINS-PER-SYNC`
/// (`RESOURCE_LIMITS_REGISTRY.json`) is enforced here, against the assembled body: at most
/// `MAX_STATE_DOMAINS_PER_SYNC` entries and no repeated `domain_id`, the same two checks
/// `encode_single_chunk_snapshot` enforces on the encode side.
pub fn decode_snapshot_body(
    body: &[u8],
) -> Result<Vec<DomainSnapshot<'_>>, FoundationProtocolError> {
    let mut domains = Vec::new();
    let mut body_cursor = 0usize;
    while body_cursor < body.len() {
        let key = read_varint(body, &mut body_cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        if field != 1 {
            skip_field(body, &mut body_cursor, wire)?;
            continue;
        }
        if wire != 2 {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        let entry = bounded_length_delimited(
            body,
            &mut body_cursor,
            body.len(),
            FoundationProtocolError::MalformedEnvelope,
        )?;
        let decoded = decode_domain_snapshot_entry(entry)?;
        if domains.len() >= MAX_STATE_DOMAINS_PER_SYNC {
            return Err(FoundationProtocolError::PayloadLimitExceeded);
        }
        if domains
            .iter()
            .any(|existing: &DomainSnapshot<'_>| existing.domain_id == decoded.domain_id)
        {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        domains.push(decoded);
    }
    Ok(domains)
}

/// Decodes one `SnapshotChunk` message payload (FND-02 §16) into its `snapshot_id` and the
/// `DomainSnapshot` entries `encode_single_chunk_snapshot` packed into the chunk body — the
/// client-direction counterpart of that encoder, for a single-chunk transfer (`chunk_count: 1`,
/// where the one chunk's `data` already is the complete `SnapshotBody`, so decoding it alone is
/// exact). A multi-chunk transfer must not decode any one chunk's `data` on its own: read every
/// chunk's `data` via `decode_snapshot_chunk_framing`, concatenate them in `chunk_index` order,
/// then decode the assembled result with `decode_snapshot_body`.
pub fn decode_snapshot_chunk(
    payload: &[u8],
) -> Result<(u64, Vec<DomainSnapshot<'_>>), FoundationProtocolError> {
    let (_chunk_index, body) = decode_snapshot_chunk_framing(payload)?;
    Ok((decode_snapshot_id(payload)?, decode_snapshot_body(body)?))
}

fn decode_domain_snapshot_entry(
    input: &[u8],
) -> Result<DomainSnapshot<'_>, FoundationProtocolError> {
    let mut cursor = 0usize;
    let (mut domain_id, mut revision, mut snapshot_type, mut entry_payload) =
        (None, None, None, None);
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            1 => {
                domain_id = Some(
                    u32::try_from(read_varint(input, &mut cursor)?)
                        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
                );
            }
            2 => revision = Some(read_varint(input, &mut cursor)?),
            3 => {
                snapshot_type = Some(
                    u32::try_from(read_varint(input, &mut cursor)?)
                        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
                );
            }
            4 => {
                entry_payload = Some(bounded_length_delimited(
                    input,
                    &mut cursor,
                    input.len(),
                    FoundationProtocolError::MalformedEnvelope,
                )?);
            }
            _ => skip_field(input, &mut cursor, wire)?,
        }
    }
    let domain_id = domain_id.filter(|id| *id != 0);
    let snapshot_type = snapshot_type.filter(|value| *value != 0);
    match (domain_id, snapshot_type) {
        (Some(domain_id), Some(snapshot_type)) => Ok(DomainSnapshot {
            domain_id,
            revision: revision.unwrap_or(0),
            snapshot_type,
            payload: entry_payload.unwrap_or(&[]),
        }),
        _ => Err(FoundationProtocolError::MalformedEnvelope),
    }
}

impl<'a> WireEnvelopeView<'a> {
    /// A post-admission `ClientCommand` of the current connection generation.
    pub fn client_command(
        &self,
        current_connection_generation: u64,
    ) -> Result<ClientCommandView<'a>, FoundationProtocolError> {
        self.validate(Direction::ClientToServer, true)?;
        if self.message_type != MessageType::ClientCommand {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        if current_connection_generation == 0
            || self.connection_generation != current_connection_generation
        {
            return Err(FoundationProtocolError::StaleConnectionGeneration);
        }
        let input = self.payload;
        let mut cursor = 0usize;
        let (mut command_id, mut command_type, mut payload) = (None, None, None);
        while cursor < input.len() {
            let key = read_varint(input, &mut cursor)?;
            let field = decode_field_number(key)?;
            let wire = (key & 7) as u8;
            match field {
                1 => read_singular_varint(input, &mut cursor, wire, &mut command_id)?,
                2 => read_singular_varint(input, &mut cursor, wire, &mut command_type)?,
                4 => read_singular_bytes(
                    input,
                    &mut cursor,
                    wire,
                    &mut payload,
                    MAX_COMMAND_PAYLOAD_BYTES,
                    FoundationProtocolError::PayloadLimitExceeded,
                )?,
                _ => skip_field(input, &mut cursor, wire)?,
            }
        }
        let command_id = command_id.filter(|id| *id != 0);
        let command_type = command_type
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value != 0);
        match (command_id, command_type) {
            (Some(command_id), Some(command_type)) => Ok(ClientCommandView {
                command_id,
                command_type,
                payload: payload.unwrap_or(&[]),
            }),
            _ => Err(FoundationProtocolError::MalformedEnvelope),
        }
    }

    pub fn client_bootstrap(&self) -> Result<ClientBootstrapView<'_>, FoundationProtocolError> {
        self.validate(Direction::ClientToServer, false)?;
        if self.message_type != MessageType::ClientBootstrap {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        let view = self
            .bootstrap
            .as_ref()
            .ok_or(FoundationProtocolError::MalformedEnvelope)?;
        Ok(ClientBootstrapView {
            protocol_major: PROTOCOL_MAJOR_V1,
            transport_profile: TRANSPORT_PROFILE_TCP_TLS13_V1,
            schema_revision: view.schema_revision,
            character_id: CharacterId(view.identity),
            admission_material: view.material,
            client_build_id: view.build_id,
            supported_capabilities: &view.capabilities[..view.capability_count],
        })
    }

    pub fn client_resume(&self) -> Result<ClientResumeView<'_>, FoundationProtocolError> {
        self.validate(Direction::ClientToServer, false)?;
        if self.message_type != MessageType::ClientResume {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        let view = self
            .bootstrap
            .as_ref()
            .ok_or(FoundationProtocolError::MalformedEnvelope)?;
        Ok(ClientResumeView {
            protocol_major: PROTOCOL_MAJOR_V1,
            transport_profile: TRANSPORT_PROFILE_TCP_TLS13_V1,
            schema_revision: view.schema_revision,
            game_session_id: GameSessionId(view.identity),
            reconnect_material: view.material,
            client_build_id: view.build_id,
            supported_capabilities: &view.capabilities[..view.capability_count],
            last_applied_server_sequence: view.last_applied_sequence,
        })
    }
    #[allow(dead_code)]
    pub fn liveness_ack(
        &self,
        current_connection_generation: u64,
    ) -> Result<LivenessAckView, FoundationProtocolError> {
        self.validate(Direction::ClientToServer, true)?;
        if self.message_type != MessageType::LivenessAck {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        if current_connection_generation == 0
            || self.connection_generation != current_connection_generation
        {
            return Err(FoundationProtocolError::StaleConnectionGeneration);
        }
        self.liveness_ack
            .ok_or(FoundationProtocolError::MalformedEnvelope)
    }
    pub const fn message_type(&self) -> MessageType {
        self.message_type
    }
    pub const fn connection_generation(&self) -> u64 {
        self.connection_generation
    }
    pub const fn server_sequence(&self) -> u64 {
        self.server_sequence
    }
    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }
    pub fn validate(
        &self,
        direction: Direction,
        admitted: bool,
    ) -> Result<(), FoundationProtocolError> {
        if self.message_type.direction() != direction {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        if direction == Direction::ClientToServer && self.server_sequence != 0 {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        match (admitted, self.message_type.phase()) {
            (true, Phase::Bootstrap) | (false, Phase::PostAdmission) => {
                return Err(FoundationProtocolError::MalformedEnvelope);
            }
            (true, Phase::PostAdmission | Phase::Any) if self.connection_generation == 0 => {
                return Err(FoundationProtocolError::StaleConnectionGeneration);
            }
            (false, Phase::Bootstrap | Phase::Any) if self.connection_generation != 0 => {
                return Err(FoundationProtocolError::MalformedEnvelope);
            }
            _ => {}
        }
        match self.message_type.sequencing() {
            Sequencing::ServerSequenced if self.server_sequence == 0 => {
                Err(FoundationProtocolError::MalformedEnvelope)
            }
            Sequencing::None | Sequencing::CommandId if self.server_sequence != 0 => {
                Err(FoundationProtocolError::MalformedEnvelope)
            }
            _ => Ok(()),
        }
    }
}
pub fn skip_field(
    input: &[u8],
    cursor: &mut usize,
    wire: u8,
) -> Result<(), FoundationProtocolError> {
    let width = match wire {
        0 => {
            read_varint(input, cursor)?;
            return Ok(());
        }
        1 => 8,
        2 => usize::try_from(read_varint(input, cursor)?)
            .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
        5 => 4,
        _ => return Err(FoundationProtocolError::MalformedEnvelope),
    };
    *cursor = cursor
        .checked_add(width)
        .filter(|end| *end <= input.len())
        .ok_or(FoundationProtocolError::MalformedEnvelope)?;
    Ok(())
}

const MAX_PROTOBUF_FIELD_NUMBER: u64 = (1u64 << 29) - 1;

fn decode_field_number(key: u64) -> Result<u32, FoundationProtocolError> {
    let raw = key >> 3;
    if raw == 0 || raw > MAX_PROTOBUF_FIELD_NUMBER {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    u32::try_from(raw).map_err(|_| FoundationProtocolError::MalformedEnvelope)
}

pub fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, FoundationProtocolError> {
    let mut value = 0u64;
    for shift in (0..70).step_by(7) {
        let byte = *input
            .get(*cursor)
            .ok_or(FoundationProtocolError::MalformedEnvelope)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(FoundationProtocolError::MalformedEnvelope)
}
pub fn bounded_length_delimited<'a>(
    input: &'a [u8],
    cursor: &mut usize,
    maximum: usize,
    limit_error: FoundationProtocolError,
) -> Result<&'a [u8], FoundationProtocolError> {
    let len = usize::try_from(read_varint(input, cursor)?)
        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?;
    if len > maximum {
        return Err(limit_error);
    }
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= input.len())
        .ok_or(FoundationProtocolError::MalformedEnvelope)?;
    let value = input
        .get(*cursor..end)
        .ok_or(FoundationProtocolError::MalformedEnvelope)?;
    *cursor = end;
    Ok(value)
}

fn unbounded_length_delimited<'a>(
    input: &'a [u8],
    cursor: &mut usize,
) -> Result<&'a [u8], FoundationProtocolError> {
    bounded_length_delimited(
        input,
        cursor,
        input.len(),
        FoundationProtocolError::MalformedEnvelope,
    )
}

fn validate_capability(
    raw: u64,
    count: &mut usize,
    previous: &mut Option<u32>,
    must_be_registered: bool,
) -> Result<(), FoundationProtocolError> {
    *count = count
        .checked_add(1)
        .ok_or(FoundationProtocolError::BootstrapLimitExceeded)?;
    if *count > MAX_CAPABILITY_COUNT {
        return Err(FoundationProtocolError::BootstrapLimitExceeded);
    }
    let capability =
        u32::try_from(raw).map_err(|_| FoundationProtocolError::InvalidCapabilitySet)?;
    if capability == 0 || previous.is_some_and(|prior| capability <= prior) {
        return Err(FoundationProtocolError::InvalidCapabilitySet);
    }
    if must_be_registered
        && REGISTERED_CAPABILITY_IDS_V1
            .binary_search(&capability)
            .is_err()
    {
        return Err(FoundationProtocolError::CapabilityMismatch);
    }
    *previous = Some(capability);
    Ok(())
}

fn validate_capability_field(
    payload: &[u8],
    cursor: &mut usize,
    wire: u8,
    count: &mut usize,
    previous: &mut Option<u32>,
    must_be_registered: bool,
) -> Result<(), FoundationProtocolError> {
    match wire {
        0 => validate_capability(
            read_varint(payload, cursor)?,
            count,
            previous,
            must_be_registered,
        ),
        2 => {
            let packed = unbounded_length_delimited(payload, cursor)?;
            let mut packed_cursor = 0usize;
            while packed_cursor < packed.len() {
                validate_capability(
                    read_varint(packed, &mut packed_cursor)?,
                    count,
                    previous,
                    must_be_registered,
                )?;
            }
            Ok(())
        }
        _ => Err(FoundationProtocolError::MalformedEnvelope),
    }
}

fn parse_state_revision_domain_id(input: &[u8]) -> Result<u32, FoundationProtocolError> {
    let mut cursor = 0usize;
    let mut domain_id = None;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (1, 0) if domain_id.is_none() => {
                let raw = read_varint(input, &mut cursor)?;
                let domain = u32::try_from(raw)
                    .map_err(|_| FoundationProtocolError::StateRevisionMismatch)?;
                if domain == 0 {
                    return Err(FoundationProtocolError::StateRevisionMismatch);
                }
                domain_id = Some(domain);
            }
            (2, 0) => {
                read_varint(input, &mut cursor)?;
            }
            (1 | 2, _) | (0, _) => return Err(FoundationProtocolError::MalformedEnvelope),
            (_, unknown_wire) => skip_field(input, &mut cursor, unknown_wire)?,
        }
    }
    domain_id.ok_or(FoundationProtocolError::StateRevisionMismatch)
}

fn validate_revision_list(
    payload: &[u8],
    cursor: &mut usize,
    count: &mut usize,
    maximum: usize,
    seen_domains: &mut BTreeSet<u32>,
) -> Result<(), FoundationProtocolError> {
    *count = count
        .checked_add(1)
        .ok_or(FoundationProtocolError::PayloadLimitExceeded)?;
    if *count > maximum {
        return Err(FoundationProtocolError::PayloadLimitExceeded);
    }
    let revision = unbounded_length_delimited(payload, cursor)?;
    let domain_id = parse_state_revision_domain_id(revision)?;
    if !seen_domains.insert(domain_id) {
        return Err(FoundationProtocolError::StateRevisionMismatch);
    }
    Ok(())
}

fn read_singular_varint(
    payload: &[u8],
    cursor: &mut usize,
    wire: u8,
    value: &mut Option<u64>,
) -> Result<(), FoundationProtocolError> {
    if wire != 0 || value.is_some() {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    *value = Some(read_varint(payload, cursor)?);
    Ok(())
}

fn read_singular_bytes<'a>(
    payload: &'a [u8],
    cursor: &mut usize,
    wire: u8,
    value: &mut Option<&'a [u8]>,
    maximum: usize,
    limit_error: FoundationProtocolError,
) -> Result<(), FoundationProtocolError> {
    if wire != 2 || value.is_some() {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    *value = Some(bounded_length_delimited(
        payload,
        cursor,
        maximum,
        limit_error,
    )?);
    Ok(())
}

fn validate_bootstrap_ingress(
    message_type: MessageType,
    payload: &[u8],
) -> Result<BootstrapIngressView<'_>, FoundationProtocolError> {
    let (
        protocol_field,
        transport_field,
        schema_field,
        capabilities_field,
        material_field,
        identity_field,
        build_field,
        sequence_field,
        material_maximum,
    ) = match message_type {
        MessageType::ClientBootstrap => (
            1u32,
            2u32,
            3u32,
            4u32,
            5u32,
            6u32,
            7u32,
            None,
            MAX_ADMISSION_MATERIAL_BYTES,
        ),
        MessageType::ClientResume => (
            4u32,
            5u32,
            6u32,
            7u32,
            2u32,
            1u32,
            8u32,
            Some(3u32),
            MAX_RECONNECT_MATERIAL_BYTES,
        ),
        _ => return Err(FoundationProtocolError::MalformedEnvelope),
    };
    let mut cursor = 0usize;
    let mut protocol_major = None;
    let mut transport_profile = None;
    let mut schema_revision = None;
    let mut last_applied_sequence = None;
    let mut material = None;
    let mut identity = None;
    let mut build_id = None;
    let mut capability_count = 0usize;
    let mut previous_capability = None;
    let mut capabilities = [0u32; MAX_CAPABILITY_COUNT];
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        if field == protocol_field {
            read_singular_varint(payload, &mut cursor, wire, &mut protocol_major)?;
        } else if field == transport_field {
            read_singular_varint(payload, &mut cursor, wire, &mut transport_profile)?;
        } else if field == schema_field {
            read_singular_varint(payload, &mut cursor, wire, &mut schema_revision)?;
        } else if field == capabilities_field {
            let mut push = |raw| -> Result<(), FoundationProtocolError> {
                validate_capability(raw, &mut capability_count, &mut previous_capability, false)?;
                capabilities[capability_count - 1] =
                    previous_capability.ok_or(FoundationProtocolError::InvalidCapabilitySet)?;
                Ok(())
            };
            match wire {
                0 => push(read_varint(payload, &mut cursor)?)?,
                2 => {
                    let packed = unbounded_length_delimited(payload, &mut cursor)?;
                    let mut packed_cursor = 0;
                    while packed_cursor < packed.len() {
                        push(read_varint(packed, &mut packed_cursor)?)?;
                    }
                }
                _ => return Err(FoundationProtocolError::MalformedEnvelope),
            }
        } else if field == material_field {
            read_singular_bytes(
                payload,
                &mut cursor,
                wire,
                &mut material,
                material_maximum,
                FoundationProtocolError::BootstrapLimitExceeded,
            )?;
            if material.is_some_and(|value| value.is_empty()) {
                return Err(FoundationProtocolError::MalformedEnvelope);
            }
        } else if field == identity_field {
            read_singular_bytes(
                payload,
                &mut cursor,
                wire,
                &mut identity,
                16,
                FoundationProtocolError::InvalidWireIdentifier,
            )?;
            decode_uuid_v7(identity.ok_or(FoundationProtocolError::InvalidWireIdentifier)?)?;
        } else if field == build_field {
            read_singular_bytes(
                payload,
                &mut cursor,
                wire,
                &mut build_id,
                MAX_CLIENT_BUILD_ID_BYTES,
                FoundationProtocolError::MalformedEnvelope,
            )?;
            let value = build_id.ok_or(FoundationProtocolError::MalformedEnvelope)?;
            if value.is_empty() || std::str::from_utf8(value).is_err() {
                return Err(FoundationProtocolError::MalformedEnvelope);
            }
        } else if sequence_field == Some(field) {
            read_singular_varint(payload, &mut cursor, wire, &mut last_applied_sequence)?;
        } else {
            skip_field(payload, &mut cursor, wire)?;
        }
    }

    if protocol_major != Some(u64::from(PROTOCOL_MAJOR_V1)) {
        return Err(FoundationProtocolError::ProtocolMajorMismatch);
    }
    if transport_profile != Some(u64::from(TRANSPORT_PROFILE_TCP_TLS13_V1)) {
        return Err(FoundationProtocolError::TransportProfileMismatch);
    }
    if !schema_revision.is_some_and(|revision| (1..=u64::from(u32::MAX)).contains(&revision)) {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if material.is_none() {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if identity.is_none() {
        return Err(FoundationProtocolError::InvalidWireIdentifier);
    }
    if build_id.is_none() {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    Ok(BootstrapIngressView {
        schema_revision: u32::try_from(
            schema_revision.ok_or(FoundationProtocolError::MalformedEnvelope)?,
        )
        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
        identity: decode_uuid_v7(identity.ok_or(FoundationProtocolError::InvalidWireIdentifier)?)?,
        material: material.ok_or(FoundationProtocolError::MalformedEnvelope)?,
        build_id: std::str::from_utf8(build_id.ok_or(FoundationProtocolError::MalformedEnvelope)?)
            .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
        capabilities,
        capability_count,
        last_applied_sequence: last_applied_sequence.unwrap_or(0),
    })
}

fn validate_client_command_ingress(payload: &[u8]) -> Result<(), FoundationProtocolError> {
    let mut cursor = 0usize;
    let mut revision_count = 0usize;
    let mut seen_domains = BTreeSet::new();
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (3, 2) => validate_revision_list(
                payload,
                &mut cursor,
                &mut revision_count,
                MAX_COMMAND_EXPECTED_REVISIONS,
                &mut seen_domains,
            )?,
            (4, 2) => {
                bounded_length_delimited(
                    payload,
                    &mut cursor,
                    MAX_COMMAND_PAYLOAD_BYTES,
                    FoundationProtocolError::PayloadLimitExceeded,
                )?;
            }
            (3 | 4, _) | (0, _) => return Err(FoundationProtocolError::MalformedEnvelope),
            (_, unknown_wire) => skip_field(payload, &mut cursor, unknown_wire)?,
        }
    }
    Ok(())
}

fn validate_liveness_ack_ingress(
    payload: &[u8],
) -> Result<LivenessAckView, FoundationProtocolError> {
    let mut cursor = 0usize;
    let mut probe_id = None;
    let mut last_applied_server_sequence = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match field {
            1 => read_singular_varint(payload, &mut cursor, wire, &mut probe_id)?,
            2 => read_singular_varint(
                payload,
                &mut cursor,
                wire,
                &mut last_applied_server_sequence,
            )?,
            _ => skip_field(payload, &mut cursor, wire)?,
        }
    }
    let probe_id = probe_id
        .filter(|id| *id != 0)
        .ok_or(FoundationProtocolError::MalformedEnvelope)?;
    Ok(LivenessAckView {
        probe_id,
        last_applied_server_sequence,
    })
}

fn validate_resync_request_ingress(payload: &[u8]) -> Result<(), FoundationProtocolError> {
    let mut cursor = 0usize;
    let mut revision_count = 0usize;
    let mut seen_domains = BTreeSet::new();
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (2, 2) => validate_revision_list(
                payload,
                &mut cursor,
                &mut revision_count,
                MAX_STATE_DOMAINS_PER_SYNC,
                &mut seen_domains,
            )?,
            (2, _) | (0, _) => return Err(FoundationProtocolError::MalformedEnvelope),
            (_, unknown_wire) => skip_field(payload, &mut cursor, unknown_wire)?,
        }
    }
    Ok(())
}

const SERVER_ACCEPTED_IDENTITY_FIELDS: &[u32] = &[1, 2, 3];
const SERVER_RESUME_ACCEPTED_IDENTITY_FIELDS: &[u32] = &[1];

fn validate_server_acceptance_ingress(
    message_type: MessageType,
    payload: &[u8],
) -> Result<(), FoundationProtocolError> {
    let (
        identity_fields,
        generation_field,
        sequence_field,
        next_command_field,
        protocol_field,
        transport_field,
        schema_field,
        capabilities_field,
    ) = match message_type {
        MessageType::ServerAccepted => (
            SERVER_ACCEPTED_IDENTITY_FIELDS,
            4u32,
            5u32,
            6u32,
            Some(7u32),
            Some(8u32),
            9u32,
            10u32,
        ),
        MessageType::ServerResumeAccepted => (
            SERVER_RESUME_ACCEPTED_IDENTITY_FIELDS,
            2u32,
            3u32,
            4u32,
            None,
            None,
            5u32,
            6u32,
        ),
        _ => return Ok(()),
    };
    let mut cursor = 0usize;
    let mut identity_seen = [false; 3];
    let mut connection_generation = None;
    let mut current_server_sequence = None;
    let mut next_command_id = None;
    let mut protocol_major = None;
    let mut transport_profile = None;
    let mut schema_revision = None;
    let mut capability_count = 0usize;
    let mut previous_capability = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        if let Some(index) = identity_fields
            .iter()
            .position(|identity_field| *identity_field == field)
        {
            if wire != 2 || identity_seen[index] {
                return Err(FoundationProtocolError::MalformedEnvelope);
            }
            let identity = bounded_length_delimited(
                payload,
                &mut cursor,
                16,
                FoundationProtocolError::InvalidWireIdentifier,
            )?;
            decode_uuid_v7(identity)?;
            identity_seen[index] = true;
        } else if field == generation_field {
            read_singular_varint(payload, &mut cursor, wire, &mut connection_generation)?;
        } else if field == sequence_field {
            read_singular_varint(payload, &mut cursor, wire, &mut current_server_sequence)?;
        } else if field == next_command_field {
            read_singular_varint(payload, &mut cursor, wire, &mut next_command_id)?;
        } else if protocol_field == Some(field) {
            read_singular_varint(payload, &mut cursor, wire, &mut protocol_major)?;
        } else if transport_field == Some(field) {
            read_singular_varint(payload, &mut cursor, wire, &mut transport_profile)?;
        } else if field == schema_field {
            read_singular_varint(payload, &mut cursor, wire, &mut schema_revision)?;
        } else if field == capabilities_field {
            validate_capability_field(
                payload,
                &mut cursor,
                wire,
                &mut capability_count,
                &mut previous_capability,
                true,
            )?;
        } else {
            skip_field(payload, &mut cursor, wire)?;
        }
    }

    if identity_seen[..identity_fields.len()]
        .iter()
        .any(|seen| !seen)
    {
        return Err(FoundationProtocolError::InvalidWireIdentifier);
    }
    if connection_generation.is_none_or(|generation| generation == 0)
        || next_command_id.is_none_or(|command_id| command_id == 0)
        || !schema_revision.is_some_and(|revision| (1..=u64::from(u32::MAX)).contains(&revision))
    {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    if protocol_field.is_some() && protocol_major != Some(u64::from(PROTOCOL_MAJOR_V1)) {
        return Err(FoundationProtocolError::ProtocolMajorMismatch);
    }
    if transport_field.is_some()
        && transport_profile != Some(u64::from(TRANSPORT_PROFILE_TCP_TLS13_V1))
    {
        return Err(FoundationProtocolError::TransportProfileMismatch);
    }
    Ok(())
}

fn validate_command_result_ingress(payload: &[u8]) -> Result<(), FoundationProtocolError> {
    let mut cursor = 0usize;
    let mut revision_count = 0usize;
    let mut seen_domains = BTreeSet::new();
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (4, 2) => validate_revision_list(
                payload,
                &mut cursor,
                &mut revision_count,
                MAX_ORDINARY_REPEATED_ENTRIES,
                &mut seen_domains,
            )?,
            (5, 2) => {
                bounded_length_delimited(
                    payload,
                    &mut cursor,
                    MAX_COMMAND_RESULT_PAYLOAD_BYTES,
                    FoundationProtocolError::PayloadLimitExceeded,
                )?;
            }
            (4 | 5, _) | (0, _) => return Err(FoundationProtocolError::MalformedEnvelope),
            (_, unknown_wire) => skip_field(payload, &mut cursor, unknown_wire)?,
        }
    }
    Ok(())
}

fn validate_state_delta_ingress(payload: &[u8]) -> Result<(), FoundationProtocolError> {
    let mut cursor = 0usize;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (5, 2) => {
                bounded_length_delimited(
                    payload,
                    &mut cursor,
                    MAX_STATE_DELTA_PAYLOAD_BYTES,
                    FoundationProtocolError::PayloadLimitExceeded,
                )?;
            }
            (5, _) | (0, _) => return Err(FoundationProtocolError::MalformedEnvelope),
            (_, unknown_wire) => skip_field(payload, &mut cursor, unknown_wire)?,
        }
    }
    Ok(())
}

fn validate_snapshot_begin_ingress(payload: &[u8]) -> Result<(), FoundationProtocolError> {
    let mut cursor = 0usize;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (2, 0) => {
                if read_varint(payload, &mut cursor)? > u64::from(MAX_SNAPSHOT_CHUNKS) {
                    return Err(FoundationProtocolError::SnapshotLimitExceeded);
                }
            }
            (3, 0) => {
                if read_varint(payload, &mut cursor)? > MAX_SNAPSHOT_ASSEMBLED_BYTES {
                    return Err(FoundationProtocolError::SnapshotLimitExceeded);
                }
            }
            (2 | 3, _) | (0, _) => return Err(FoundationProtocolError::MalformedEnvelope),
            (_, unknown_wire) => skip_field(payload, &mut cursor, unknown_wire)?,
        }
    }
    Ok(())
}

fn validate_snapshot_chunk_ingress(payload: &[u8]) -> Result<(), FoundationProtocolError> {
    let mut cursor = 0usize;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (3, 2) => {
                bounded_length_delimited(
                    payload,
                    &mut cursor,
                    MAX_SNAPSHOT_CHUNK_BYTES,
                    FoundationProtocolError::SnapshotLimitExceeded,
                )?;
            }
            (3, _) | (0, _) => return Err(FoundationProtocolError::MalformedEnvelope),
            (_, unknown_wire) => skip_field(payload, &mut cursor, unknown_wire)?,
        }
    }
    Ok(())
}

fn validate_client_ingress_payload(
    message_type: MessageType,
    payload: &[u8],
) -> Result<(), FoundationProtocolError> {
    match message_type {
        MessageType::ClientBootstrap | MessageType::ClientResume => {
            validate_bootstrap_ingress(message_type, payload).map(|_| ())
        }
        MessageType::ClientCommand => validate_client_command_ingress(payload),
        MessageType::ResyncRequest => validate_resync_request_ingress(payload),
        _ => Ok(()),
    }
}

fn validate_server_ingress_payload(
    message_type: MessageType,
    payload: &[u8],
) -> Result<(), FoundationProtocolError> {
    match message_type {
        MessageType::ServerAccepted | MessageType::ServerResumeAccepted => {
            validate_server_acceptance_ingress(message_type, payload)
        }
        MessageType::CommandResult => validate_command_result_ingress(payload),
        MessageType::StateDelta => validate_state_delta_ingress(payload),
        MessageType::SnapshotBegin => validate_snapshot_begin_ingress(payload),
        MessageType::SnapshotChunk => validate_snapshot_chunk_ingress(payload),
        _ => Ok(()),
    }
}

pub fn decode_wire_envelope(input: &[u8]) -> Result<WireEnvelopeView<'_>, FoundationProtocolError> {
    if input.is_empty() || input.len() > MAX_WIRE_FRAME_BYTES as usize {
        return Err(FoundationProtocolError::MalformedEnvelope);
    }
    let (mut cursor, mut mt, mut generation, mut sequence, mut payload) =
        (0usize, None, None, None, None);
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        match (field, wire) {
            (1, 0) if mt.is_none() => {
                mt = Some(
                    u32::try_from(read_varint(input, &mut cursor)?)
                        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?,
                )
            }
            (2, 0) if generation.is_none() => generation = Some(read_varint(input, &mut cursor)?),
            (3, 0) if sequence.is_none() => sequence = Some(read_varint(input, &mut cursor)?),
            (4, 2) if payload.is_none() => {
                let len = usize::try_from(read_varint(input, &mut cursor)?)
                    .map_err(|_| FoundationProtocolError::MalformedEnvelope)?;
                let end = cursor
                    .checked_add(len)
                    .ok_or(FoundationProtocolError::MalformedEnvelope)?;
                payload = Some(
                    input
                        .get(cursor..end)
                        .ok_or(FoundationProtocolError::MalformedEnvelope)?,
                );
                cursor = end;
            }
            _ if (1..=4).contains(&field) || field == 0 => {
                return Err(FoundationProtocolError::MalformedEnvelope);
            }
            (_, unknown_wire) => skip_field(input, &mut cursor, unknown_wire)?,
        }
    }
    let message_type =
        MessageType::try_from(mt.ok_or(FoundationProtocolError::MalformedEnvelope)?)?;
    let payload = payload.ok_or(FoundationProtocolError::MalformedEnvelope)?;
    if matches!(
        message_type,
        MessageType::ClientBootstrap | MessageType::ClientResume
    ) && payload.len() > MAX_BOOTSTRAP_PAYLOAD_BYTES
    {
        return Err(FoundationProtocolError::BootstrapLimitExceeded);
    }
    let mut liveness_ack = None;
    let bootstrap = if matches!(
        message_type,
        MessageType::ClientBootstrap | MessageType::ClientResume
    ) {
        Some(validate_bootstrap_ingress(message_type, payload)?)
    } else {
        match message_type.direction() {
            Direction::ClientToServer => {
                if message_type == MessageType::LivenessAck {
                    liveness_ack = Some(validate_liveness_ack_ingress(payload)?);
                } else {
                    validate_client_ingress_payload(message_type, payload)?;
                }
            }
            Direction::ServerToClient => validate_server_ingress_payload(message_type, payload)?,
        }
        None
    };
    Ok(WireEnvelopeView {
        message_type,
        connection_generation: generation.unwrap_or(0),
        server_sequence: sequence.unwrap_or(0),
        payload,
        bootstrap,
        liveness_ack,
    })
}

pub fn decode_framed_envelope(
    frame: &[u8],
) -> Result<WireEnvelopeView<'_>, FoundationProtocolError> {
    let prefix = frame
        .get(..4)
        .ok_or(FoundationProtocolError::MalformedFrame)?;
    let length = FrameLength::from_prefix(prefix)?;
    let body = frame
        .get(4..)
        .ok_or(FoundationProtocolError::MalformedFrame)?;
    if body.len() != length.get() as usize {
        return Err(FoundationProtocolError::MalformedFrame);
    }
    decode_wire_envelope(body)
}

pub mod account_achievements;
pub mod actor_spell;
pub mod analyser;
pub mod bestiary;
pub mod charm;
mod charm_wire;
pub mod chat;
pub mod world_object;
pub mod world_spatial;
pub mod world_spatial_entities;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seam_server_encoders_match_independent_proto_vectors() -> Result<(), FoundationProtocolError>
    {
        // Field tags and lengths transcribed from foundation.proto, not produced
        // by the encoder or the existing test serialization helpers.
        const ACCEPTED: &[u8] = &[
            8, 2, 34, 64, 10, 16, 0, 0, 0, 0, 0, 0, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1, 18, 16, 0, 0,
            0, 0, 0, 0, 112, 0, 128, 0, 0, 0, 0, 0, 0, 2, 26, 16, 0, 0, 0, 0, 0, 0, 112, 0, 128, 0,
            0, 0, 0, 0, 0, 3, 32, 1, 48, 1, 56, 1, 64, 1, 72, 1,
        ];
        const RESUMED: &[u8] = &[
            8, 4, 34, 26, 10, 16, 0, 0, 0, 0, 0, 0, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1, 16, 2, 24,
            42, 32, 3, 40, 1,
        ];
        const ERROR: &[u8] = &[8, 14, 34, 5, 8, 235, 7, 16, 4];
        let accepted = ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
            world_id: WorldId::decode(&test_uuid_v7(2))?,
            channel_id: ChannelId::decode(&test_uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: &[],
        };
        let resumed = ServerResumeAcceptedValue {
            game_session_id: accepted.game_session_id,
            connection_generation: 2,
            current_server_sequence: 42,
            next_command_id: 3,
            schema_revision: 1,
            selected_capabilities: &[],
        };
        assert_eq!(encode_server_accepted(&accepted)?, ACCEPTED);
        assert_eq!(encode_server_resume_accepted(&resumed)?, RESUMED);
        assert_eq!(
            encode_protocol_error(FoundationProtocolError::MalformedEnvelope, 0)?,
            ERROR
        );
        for (wire, kind) in [
            (ACCEPTED, MessageType::ServerAccepted),
            (RESUMED, MessageType::ServerResumeAccepted),
            (ERROR, MessageType::ProtocolError),
        ] {
            let envelope = decode_wire_envelope(wire)?;
            assert_eq!(envelope.message_type(), kind);
            assert_eq!(envelope.server_sequence(), 0);
            envelope.validate(Direction::ServerToClient, false)?;
        }
        assert!(
            encode_server_accepted(&ServerAcceptedValue {
                connection_generation: 0,
                ..accepted
            })
            .is_err()
        );
        assert!(
            encode_server_resume_accepted(&ServerResumeAcceptedValue {
                connection_generation: 0,
                ..resumed
            })
            .is_err()
        );
        assert!(
            encode_server_accepted(&ServerAcceptedValue {
                next_command_id: 0,
                ..accepted
            })
            .is_err()
        );
        assert!(
            encode_server_resume_accepted(&ServerResumeAcceptedValue {
                schema_revision: 0,
                ..resumed
            })
            .is_err()
        );
        assert!(
            encode_server_accepted(&ServerAcceptedValue {
                // 2 is reserved for PROF-WIRE-1 but not registered.
                selected_capabilities: &[2],
                ..accepted
            })
            .is_err()
        );
        assert!(
            encode_server_resume_accepted(&ServerResumeAcceptedValue {
                selected_capabilities: &[1; 129],
                ..resumed
            })
            .is_err()
        );
        let post_admission_error =
            encode_protocol_error(FoundationProtocolError::MalformedEnvelope, 7)?;
        decode_wire_envelope(&post_admission_error)?.validate(Direction::ServerToClient, true)?;
        Ok(())
    }

    /// `encode_client_bootstrap` round-trips through the same server-side ingress decode
    /// (`WireEnvelopeView::client_bootstrap`, driven by `validate_bootstrap_ingress`) the shipped
    /// server uses for every real `ClientBootstrap`: the wire format is unchanged, only a new
    /// client-direction encoder was added.
    #[test]
    fn client_bootstrap_encoder_round_trips_through_server_ingress_decode()
    -> Result<(), FoundationProtocolError> {
        let character = CharacterId::decode(&test_uuid_v7(4))?;
        let value = ClientBootstrapValue {
            schema_revision: 3,
            character_id: character,
            admission_material: b"fixture-grant",
            client_build_id: "dev-client/0.1.0",
            supported_capabilities: &[],
        };
        let wire = encode_client_bootstrap(&value)?;
        let envelope = decode_wire_envelope(&wire)?;
        assert_eq!(envelope.message_type(), MessageType::ClientBootstrap);
        let view = envelope.client_bootstrap()?;
        assert_eq!(view.protocol_major, PROTOCOL_MAJOR_V1);
        assert_eq!(view.transport_profile, TRANSPORT_PROFILE_TCP_TLS13_V1);
        assert_eq!(view.schema_revision, value.schema_revision);
        assert_eq!(view.character_id, character);
        assert_eq!(view.admission_material, value.admission_material);
        assert_eq!(view.client_build_id, value.client_build_id);
        assert!(view.supported_capabilities.is_empty());

        // Zero schema revision and empty admission material are refused before any bytes are
        // emitted, exactly like every other required field in this crate.
        assert!(
            encode_client_bootstrap(&ClientBootstrapValue {
                schema_revision: 0,
                ..value
            })
            .is_err()
        );
        assert!(
            encode_client_bootstrap(&ClientBootstrapValue {
                admission_material: &[],
                ..value
            })
            .is_err()
        );
        Ok(())
    }

    /// `decode_server_accepted` round-trips through the existing server-side
    /// `encode_server_accepted`: same fixture, same fields, both directions of the same wire
    /// format.
    #[test]
    fn server_accepted_decoder_round_trips_through_existing_server_encoder()
    -> Result<(), FoundationProtocolError> {
        let accepted = ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
            world_id: WorldId::decode(&test_uuid_v7(2))?,
            channel_id: ChannelId::decode(&test_uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 5,
            selected_capabilities: &[],
        };
        let wire = encode_server_accepted(&accepted)?;
        let envelope = decode_wire_envelope(&wire)?;
        assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
        let fields = decode_server_accepted(envelope.payload())?;
        assert_eq!(fields.game_session_id, accepted.game_session_id);
        assert_eq!(fields.world_id, accepted.world_id);
        assert_eq!(fields.channel_id, accepted.channel_id);
        assert_eq!(fields.connection_generation, accepted.connection_generation);
        assert_eq!(
            fields.current_server_sequence,
            accepted.current_server_sequence
        );
        assert_eq!(fields.next_command_id, accepted.next_command_id);
        assert_eq!(fields.schema_revision, accepted.schema_revision);
        assert!(fields.selected_capabilities.is_empty());

        // Malformed ServerAccepted payloads (the same negatives `validate_server_acceptance_ingress`
        // already covers) still fail closed through the new decoder.
        assert!(decode_server_accepted(&[]).is_err());
        Ok(())
    }

    /// `decode_snapshot_chunk` round-trips through the existing server-side
    /// `encode_single_chunk_snapshot`, recovering the exact domain entries it packed — the join
    /// snapshot a client decodes right after `ServerAccepted`.
    #[test]
    fn snapshot_chunk_decoder_round_trips_through_existing_server_encoder()
    -> Result<(), FoundationProtocolError> {
        let world_spatial = [1_u8, 2, 3];
        let world_object_overlay = [4_u8, 5];
        let frames = encode_single_chunk_snapshot(
            1,
            7,
            0,
            &[
                DomainSnapshot {
                    domain_id: 1,
                    revision: 1,
                    snapshot_type: 1,
                    payload: &world_spatial,
                },
                DomainSnapshot {
                    domain_id: 2,
                    revision: 0,
                    snapshot_type: 1,
                    payload: &world_object_overlay,
                },
            ],
        )?;
        let chunk_envelope = decode_wire_envelope(&frames[1])?;
        assert_eq!(chunk_envelope.message_type(), MessageType::SnapshotChunk);
        let (snapshot_id, domains) = decode_snapshot_chunk(chunk_envelope.payload())?;
        assert_eq!(snapshot_id, 7);
        assert_eq!(domains.len(), 2);
        assert_eq!(domains[0].domain_id, 1);
        assert_eq!(domains[0].revision, 1);
        assert_eq!(domains[0].snapshot_type, 1);
        assert_eq!(domains[0].payload, &world_spatial);
        assert_eq!(domains[1].domain_id, 2);
        assert_eq!(domains[1].revision, 0);
        assert_eq!(domains[1].snapshot_type, 1);
        assert_eq!(domains[1].payload, &world_object_overlay);
        Ok(())
    }

    /// One `SnapshotChunk` payload of `domain_ids.len()` minimal entries (each `snapshot_type`
    /// 1, empty payload), for exercising `decode_snapshot_chunk`'s own bounds directly — a
    /// malformed/adversarial peer can send more or duplicate domain IDs even though this crate's
    /// own `encode_single_chunk_snapshot` never would (it already refuses to construct either).
    fn snapshot_chunk_payload_with_domain_ids(domain_ids: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        for &domain_id in domain_ids {
            let mut entry = Vec::new();
            push_scalar(&mut entry, 1, u64::from(domain_id));
            push_scalar(&mut entry, 3, 1);
            push_bytes(&mut body, 1, &entry);
        }
        let mut chunk = Vec::new();
        push_scalar(&mut chunk, 1, 1);
        push_bytes(&mut chunk, 3, &body);
        chunk
    }

    /// FND02-STATE-DOMAINS-PER-SYNC (`RESOURCE_LIMITS_REGISTRY.json`): 256 unique domains
    /// accepted, 257 rejected.
    #[test]
    fn snapshot_chunk_domain_count_accepts_256_and_rejects_257()
    -> Result<(), FoundationProtocolError> {
        let ids: Vec<u32> = (1..=256).collect();
        let payload_256 = snapshot_chunk_payload_with_domain_ids(&ids);
        let (snapshot_id, domains) = decode_snapshot_chunk(&payload_256)?;
        assert_eq!(snapshot_id, 1);
        assert_eq!(domains.len(), 256);

        let ids: Vec<u32> = (1..=257).collect();
        let payload_257 = snapshot_chunk_payload_with_domain_ids(&ids);
        assert_eq!(
            decode_snapshot_chunk(&payload_257),
            Err(FoundationProtocolError::PayloadLimitExceeded)
        );
        Ok(())
    }

    /// FND02-STATE-DOMAINS-PER-SYNC: duplicate domain IDs rejected.
    /// `SnapshotBody.domain` (field 1) must be length-delimited: a varint-typed field 1 followed by
    /// bytes that happen to form a valid `StateDomainSnapshot` is refused, not decoded as a domain.
    #[test]
    fn snapshot_body_rejects_a_domain_field_with_a_non_length_delimited_wire_type() {
        let payload = [0_u8];
        let mut entry = Vec::new();
        entry.extend_from_slice(&[0x08, 0x01, 0x10, 0x01, 0x18, 0x01, 0x22, 0x01]);
        entry.extend_from_slice(&payload);
        let mut body = vec![0x08, u8::try_from(entry.len()).unwrap_or(u8::MAX)];
        body.extend_from_slice(&entry);
        assert_eq!(
            decode_snapshot_body(&body),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }

    #[test]
    fn snapshot_chunk_rejects_a_duplicate_domain_id() {
        assert_eq!(
            decode_snapshot_chunk(&snapshot_chunk_payload_with_domain_ids(&[1, 2, 1])),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }

    /// `decode_snapshot_id` round-trips through the existing server-side
    /// `encode_single_chunk_snapshot`'s `SnapshotBegin` and `SnapshotCommit` frames: a client
    /// correlates all three frames of one transfer by this identity.
    #[test]
    fn snapshot_id_decoder_round_trips_through_begin_and_commit()
    -> Result<(), FoundationProtocolError> {
        let payload = [0_u8];
        let frames = encode_single_chunk_snapshot(
            1,
            9,
            0,
            &[DomainSnapshot {
                domain_id: 1,
                revision: 1,
                snapshot_type: 1,
                payload: &payload,
            }],
        )?;
        let begin = decode_wire_envelope(&frames[0])?;
        assert_eq!(begin.message_type(), MessageType::SnapshotBegin);
        assert_eq!(decode_snapshot_id(begin.payload())?, 9);
        let commit = decode_wire_envelope(&frames[2])?;
        assert_eq!(commit.message_type(), MessageType::SnapshotCommit);
        assert_eq!(decode_snapshot_id(commit.payload())?, 9);
        Ok(())
    }

    /// `decode_snapshot_begin` and `decode_snapshot_chunk_framing` round-trip through the
    /// existing server-side `encode_single_chunk_snapshot`: its one chunk is always declared
    /// `chunk_count: 1`, `chunk_index: 0` (omitted on the wire, defaulted on decode), and
    /// `total_encoded_bytes` equal to that chunk's `data` length.
    #[test]
    fn snapshot_begin_and_chunk_framing_round_trip_through_existing_server_encoder()
    -> Result<(), FoundationProtocolError> {
        let payload = [0_u8; 3];
        let frames = encode_single_chunk_snapshot(
            1,
            9,
            0,
            &[DomainSnapshot {
                domain_id: 1,
                revision: 1,
                snapshot_type: 1,
                payload: &payload,
            }],
        )?;
        let begin_envelope = decode_wire_envelope(&frames[0])?;
        let begin = decode_snapshot_begin(begin_envelope.payload())?;
        assert_eq!(begin.snapshot_id, 9);
        assert_eq!(begin.chunk_count, 1);
        assert!(begin.total_encoded_bytes > 0);

        let chunk_envelope = decode_wire_envelope(&frames[1])?;
        let (chunk_index, data) = decode_snapshot_chunk_framing(chunk_envelope.payload())?;
        assert_eq!(chunk_index, 0);
        assert_eq!(data.len() as u64, begin.total_encoded_bytes);
        assert_eq!(decode_snapshot_body(data)?.len(), 1);
        Ok(())
    }

    /// FND-02 (`foundation.proto`: "Zero is invalid" for `snapshot_id`): an explicit `snapshot_id`
    /// of 0 is refused exactly like an absent one, everywhere it is decoded.
    #[test]
    fn snapshot_id_zero_is_rejected_everywhere_it_is_decoded() {
        // Field 1, wire type 0 (varint), explicit value 0 — a standard proto3 encoder never
        // emits this (this crate's own `push_scalar` skips a zero value), but an adversarial
        // peer could.
        let explicit_zero: &[u8] = &[0x08, 0x00];
        assert_eq!(
            decode_snapshot_id(explicit_zero),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        assert_eq!(
            decode_snapshot_begin(explicit_zero),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        // decode_snapshot_chunk reuses decode_snapshot_id for its own snapshot_id.
        assert_eq!(
            decode_snapshot_chunk(explicit_zero),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }

    #[test]
    fn seam_typed_bootstrap_borrows_validated_material() -> Result<(), FoundationProtocolError> {
        let id = test_uuid_v7(1);
        let wire = test_envelope(1, &test_client_bootstrap_payload(1, 1, 2, &id, &[7, 9]));
        let envelope = decode_wire_envelope(&wire)?;
        let view = envelope.client_bootstrap()?;
        assert_eq!(view.protocol_major, 1);
        assert_eq!(view.transport_profile, 1);
        assert_eq!(view.schema_revision, 2);
        assert_eq!(view.character_id.as_bytes(), &id);
        assert_eq!(view.admission_material, &[0xaa]);
        assert_eq!(view.client_build_id, "test-client");
        assert_eq!(view.supported_capabilities, &[7, 9]);
        let start = wire.as_ptr() as usize;
        assert!((start..start + wire.len()).contains(&(view.admission_material.as_ptr() as usize)));
        assert!(envelope.client_resume().is_err());
        Ok(())
    }

    #[test]
    fn seam_typed_resume_preserves_sequence_and_identity() -> Result<(), FoundationProtocolError> {
        let id = test_uuid_v7(2);
        let mut payload = test_client_resume_payload(1, 1, 3, &id, &[8]);
        push_test_varint_field(&mut payload, 3, 42);
        let wire = test_envelope(3, &payload);
        let envelope = decode_wire_envelope(&wire)?;
        let view = envelope.client_resume()?;
        assert_eq!(view.protocol_major, 1);
        assert_eq!(view.transport_profile, 1);
        assert_eq!(view.schema_revision, 3);
        assert_eq!(view.game_session_id.as_bytes(), &id);
        assert_eq!(view.reconnect_material, &[0xbb]);
        assert_eq!(view.client_build_id, "test-client");
        assert_eq!(view.supported_capabilities, &[8]);
        assert_eq!(view.last_applied_server_sequence, 42);
        assert!(envelope.client_bootstrap().is_err());
        Ok(())
    }

    #[test]
    fn seam_typed_extraction_rejects_bootstrap_generation_and_sequence()
    -> Result<(), FoundationProtocolError> {
        for message in [1, 3] {
            let id = test_uuid_v7(1);
            let payload = if message == 1 {
                test_client_bootstrap_payload(1, 1, 1, &id, &[])
            } else {
                test_client_resume_payload(1, 1, 1, &id, &[])
            };
            for field in [2, 3] {
                let mut wire = test_envelope(message, &payload);
                push_test_varint_field(&mut wire, field, 1);
                let envelope = decode_wire_envelope(&wire)?;
                if message == 1 {
                    assert!(envelope.client_bootstrap().is_err());
                } else {
                    assert!(envelope.client_resume().is_err());
                }
            }
        }
        Ok(())
    }

    #[test]
    fn frame_length_boundary_property_style() {
        for raw in [1u32, 2, 255, 65_535, 1_048_575, 1_048_576] {
            assert_eq!(FrameLength::new(raw).map(FrameLength::get), Ok(raw));
        }
        assert_eq!(
            FrameLength::new(0),
            Err(FoundationProtocolError::MalformedFrame)
        );
        assert_eq!(
            FrameLength::new(1_048_577),
            Err(FoundationProtocolError::FrameTooLarge)
        );
    }

    fn test_varint(mut value: usize) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if value == 0 {
                return out;
            }
        }
    }

    fn test_envelope(message_type: u8, payload: &[u8]) -> Vec<u8> {
        let mut envelope = vec![0x08, message_type, 0x22];
        envelope.extend(test_varint(payload.len()));
        envelope.extend_from_slice(payload);
        envelope
    }

    fn test_state_revision(domain_id: usize) -> Vec<u8> {
        let mut nested = vec![0x08];
        nested.extend(test_varint(domain_id));
        nested.extend([0x10, 0x01]);
        nested
    }

    fn push_test_varint_field(payload: &mut Vec<u8>, field: usize, value: usize) {
        payload.extend(test_varint(field << 3));
        payload.extend(test_varint(value));
    }

    fn push_test_bytes_field(payload: &mut Vec<u8>, field: usize, value: &[u8]) {
        payload.extend(test_varint((field << 3) | 2));
        payload.extend(test_varint(value.len()));
        payload.extend_from_slice(value);
    }

    fn test_uuid_v7(marker: u8) -> [u8; 16] {
        let mut id = [0u8; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = marker;
        id
    }

    fn test_client_bootstrap_payload(
        protocol_major: usize,
        transport_profile: usize,
        schema_revision: usize,
        character_id: &[u8],
        supported_capabilities: &[usize],
    ) -> Vec<u8> {
        test_client_bootstrap_payload_with_material_and_build(
            protocol_major,
            transport_profile,
            schema_revision,
            character_id,
            supported_capabilities,
            &[0xaa],
            b"test-client",
        )
    }

    fn test_client_bootstrap_payload_with_material_and_build(
        protocol_major: usize,
        transport_profile: usize,
        schema_revision: usize,
        character_id: &[u8],
        supported_capabilities: &[usize],
        admission_material: &[u8],
        client_build_id: &[u8],
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        push_test_varint_field(&mut payload, 1, protocol_major);
        push_test_varint_field(&mut payload, 2, transport_profile);
        push_test_varint_field(&mut payload, 3, schema_revision);
        for capability in supported_capabilities {
            push_test_varint_field(&mut payload, 4, *capability);
        }
        push_test_bytes_field(&mut payload, 5, admission_material);
        push_test_bytes_field(&mut payload, 6, character_id);
        push_test_bytes_field(&mut payload, 7, client_build_id);
        payload
    }

    fn test_client_resume_payload(
        protocol_major: usize,
        transport_profile: usize,
        schema_revision: usize,
        game_session_id: &[u8],
        supported_capabilities: &[usize],
    ) -> Vec<u8> {
        test_client_resume_payload_with_material(
            protocol_major,
            transport_profile,
            schema_revision,
            game_session_id,
            supported_capabilities,
            &[0xbb],
        )
    }

    fn test_client_resume_payload_with_material(
        protocol_major: usize,
        transport_profile: usize,
        schema_revision: usize,
        game_session_id: &[u8],
        supported_capabilities: &[usize],
        reconnect_material: &[u8],
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        push_test_bytes_field(&mut payload, 1, game_session_id);
        push_test_bytes_field(&mut payload, 2, reconnect_material);
        push_test_varint_field(&mut payload, 4, protocol_major);
        push_test_varint_field(&mut payload, 5, transport_profile);
        push_test_varint_field(&mut payload, 6, schema_revision);
        for capability in supported_capabilities {
            push_test_varint_field(&mut payload, 7, *capability);
        }
        push_test_bytes_field(&mut payload, 8, b"test-client");
        payload
    }

    fn test_server_accepted_payload(
        connection_generation: usize,
        next_command_id: usize,
        schema_revision: usize,
        ids: [&[u8]; 3],
        selected_capabilities: &[usize],
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        push_test_bytes_field(&mut payload, 1, ids[0]);
        push_test_bytes_field(&mut payload, 2, ids[1]);
        push_test_bytes_field(&mut payload, 3, ids[2]);
        push_test_varint_field(&mut payload, 4, connection_generation);
        push_test_varint_field(&mut payload, 6, next_command_id);
        push_test_varint_field(&mut payload, 7, PROTOCOL_MAJOR_V1 as usize);
        push_test_varint_field(&mut payload, 8, TRANSPORT_PROFILE_TCP_TLS13_V1 as usize);
        push_test_varint_field(&mut payload, 9, schema_revision);
        for capability in selected_capabilities {
            push_test_varint_field(&mut payload, 10, *capability);
        }
        payload
    }

    fn test_server_resume_accepted_payload(
        connection_generation: usize,
        next_command_id: usize,
        schema_revision: usize,
        game_session_id: &[u8],
        selected_capabilities: &[usize],
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        push_test_bytes_field(&mut payload, 1, game_session_id);
        push_test_varint_field(&mut payload, 2, connection_generation);
        push_test_varint_field(&mut payload, 4, next_command_id);
        push_test_varint_field(&mut payload, 5, schema_revision);
        for capability in selected_capabilities {
            push_test_varint_field(&mut payload, 6, *capability);
        }
        payload
    }

    #[test]
    fn client_handshakes_require_v1_protocol_profile_and_schema_semantics() {
        let id = test_uuid_v7(1);
        for (message_type, payload, expected) in [
            (
                1,
                test_client_bootstrap_payload(2, 1, 1, &id, &[]),
                FoundationProtocolError::ProtocolMajorMismatch,
            ),
            (
                1,
                test_client_bootstrap_payload(1, 2, 1, &id, &[]),
                FoundationProtocolError::TransportProfileMismatch,
            ),
            (
                1,
                test_client_bootstrap_payload(1, 1, 0, &id, &[]),
                FoundationProtocolError::MalformedEnvelope,
            ),
            (
                3,
                test_client_resume_payload(2, 1, 1, &id, &[]),
                FoundationProtocolError::ProtocolMajorMismatch,
            ),
            (
                3,
                test_client_resume_payload(1, 2, 1, &id, &[]),
                FoundationProtocolError::TransportProfileMismatch,
            ),
            (
                3,
                test_client_resume_payload(1, 1, 0, &id, &[]),
                FoundationProtocolError::MalformedEnvelope,
            ),
        ] {
            assert_eq!(
                decode_wire_envelope(&test_envelope(message_type, &payload)),
                Err(expected)
            );
        }
    }

    #[test]
    fn handshake_ingress_requires_exact_non_nil_uuidv7_identities() {
        let valid = test_uuid_v7(1);
        let nil = [0u8; 16];
        let short = [0u8; 15];
        let world = test_uuid_v7(2);
        let channel = test_uuid_v7(3);
        for (message_type, payload) in [
            (1, test_client_bootstrap_payload(1, 1, 1, &short, &[])),
            (3, test_client_resume_payload(1, 1, 1, &nil, &[])),
            (
                2,
                test_server_accepted_payload(1, 1, 1, [&nil, &world, &channel], &[]),
            ),
            (4, test_server_resume_accepted_payload(2, 1, 1, &short, &[])),
        ] {
            assert_eq!(
                decode_wire_envelope(&test_envelope(message_type, &payload)),
                Err(FoundationProtocolError::InvalidWireIdentifier)
            );
        }
        assert!(
            decode_wire_envelope(&test_envelope(
                2,
                &test_server_accepted_payload(1, 1, 1, [&valid, &world, &channel], &[])
            ))
            .is_ok()
        );
    }

    #[test]
    fn accepted_messages_require_nonzero_authority_counters_and_schema() {
        let session = test_uuid_v7(1);
        let world = test_uuid_v7(2);
        let channel = test_uuid_v7(3);
        for (message_type, payload) in [
            (
                2,
                test_server_accepted_payload(0, 1, 1, [&session, &world, &channel], &[]),
            ),
            (
                2,
                test_server_accepted_payload(1, 0, 1, [&session, &world, &channel], &[]),
            ),
            (
                2,
                test_server_accepted_payload(1, 1, 0, [&session, &world, &channel], &[]),
            ),
            (
                4,
                test_server_resume_accepted_payload(0, 1, 1, &session, &[]),
            ),
            (
                4,
                test_server_resume_accepted_payload(2, 0, 1, &session, &[]),
            ),
            (
                4,
                test_server_resume_accepted_payload(2, 1, 0, &session, &[]),
            ),
        ] {
            assert_eq!(
                decode_wire_envelope(&test_envelope(message_type, &payload)),
                Err(FoundationProtocolError::MalformedEnvelope)
            );
        }
    }

    #[test]
    fn selected_capabilities_are_registry_closed_but_supported_unknowns_are_additive() {
        let session = test_uuid_v7(1);
        let world = test_uuid_v7(2);
        let channel = test_uuid_v7(3);
        assert!(
            decode_wire_envelope(&test_envelope(
                1,
                &test_client_bootstrap_payload(1, 1, 1, &session, &[777])
            ))
            .is_ok()
        );
        assert!(
            decode_wire_envelope(&test_envelope(
                3,
                &test_client_resume_payload(1, 1, 1, &session, &[777])
            ))
            .is_ok()
        );
        // 1 BESTIARY_CHARMS_V1, 6 WORLD_SPATIAL_ENTITIES, 7 CHAT_V1 and 10 ANALYSER_V1 are
        // registered; 2 is reserved for PROF-WIRE-1 and 8 is unallocated: a selected capability
        // this build does not know fails.
        for selected in [
            &[6_usize][..],
            &[1, 6][..],
            &[7][..],
            &[10][..],
            &[1, 6, 7, 10][..],
        ] {
            assert!(
                decode_wire_envelope(&test_envelope(
                    2,
                    &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], selected)
                ))
                .is_ok()
            );
        }
        assert_eq!(
            decode_wire_envelope(&test_envelope(
                2,
                &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[8])
            )),
            Err(FoundationProtocolError::CapabilityMismatch)
        );
        assert!(
            decode_wire_envelope(&test_envelope(
                2,
                &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[1])
            ))
            .is_ok()
        );
        assert_eq!(
            decode_wire_envelope(&test_envelope(
                2,
                &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[2])
            )),
            Err(FoundationProtocolError::CapabilityMismatch)
        );
        assert_eq!(
            decode_wire_envelope(&test_envelope(
                4,
                &test_server_resume_accepted_payload(2, 1, 1, &session, &[2])
            )),
            Err(FoundationProtocolError::CapabilityMismatch)
        );
    }

    #[test]
    fn semantic_handshakes_preserve_same_major_and_additive_unknown_field_compatibility()
    -> Result<(), FoundationProtocolError> {
        let session = test_uuid_v7(1);
        let world = test_uuid_v7(2);
        let channel = test_uuid_v7(3);
        let mut payloads = [
            test_client_bootstrap_payload(1, 1, 2, &session, &[]),
            test_client_resume_payload(1, 1, 2, &session, &[]),
            test_server_accepted_payload(1, 1, 2, [&session, &world, &channel], &[]),
            test_server_resume_accepted_payload(2, 1, 2, &session, &[]),
        ];
        for (index, payload) in payloads.iter_mut().enumerate() {
            push_test_bytes_field(payload, 16, b"future-addition");
            let message_type = [1u8, 3, 2, 4][index];
            assert_eq!(
                decode_wire_envelope(&test_envelope(message_type, payload))?.payload(),
                payload
            );
        }
        Ok(())
    }

    #[test]
    fn duplicate_singular_handshake_fields_fail_closed() {
        let session = test_uuid_v7(1);
        let world = test_uuid_v7(2);
        let channel = test_uuid_v7(3);
        let mut bootstrap = test_client_bootstrap_payload(1, 1, 1, &session, &[]);
        push_test_varint_field(&mut bootstrap, 1, 1);
        assert_eq!(
            decode_wire_envelope(&test_envelope(1, &bootstrap)),
            Err(FoundationProtocolError::MalformedEnvelope)
        );

        let mut accepted = test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[]);
        push_test_varint_field(&mut accepted, 4, 1);
        assert_eq!(
            decode_wire_envelope(&test_envelope(2, &accepted)),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }

    #[test]
    fn protobuf_field_numbers_cannot_wrap_into_known_fields() -> Result<(), FoundationProtocolError>
    {
        let oversized_message_field = (1u64 << 32) + 1;
        let oversized_payload_field = (1u64 << 32) + 4;
        let mut raw = test_varint((oversized_message_field << 3) as usize);
        raw.push(0x01);
        raw.extend(test_varint(((oversized_payload_field << 3) | 2) as usize));
        raw.push(0x00);
        assert_eq!(
            decode_wire_envelope(&raw),
            Err(FoundationProtocolError::MalformedEnvelope)
        );

        let oversized_nested_field = (1u64 << 32) + 5;
        let mut payload = test_varint(((oversized_nested_field << 3) | 2) as usize);
        payload.push(0x00);
        assert_eq!(
            decode_wire_envelope(&test_envelope(1, &payload)),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        Ok(())
    }

    #[test]
    fn bootstrap_metadata_limits_cover_build_id_and_capabilities()
    -> Result<(), FoundationProtocolError> {
        let build_128 = vec![b'a'; 128];
        let id = test_uuid_v7(1);
        let payload = test_client_bootstrap_payload_with_material_and_build(
            1,
            1,
            1,
            &id,
            &[],
            &[0xaa],
            &build_128,
        );
        assert_eq!(
            decode_wire_envelope(&test_envelope(1, &payload))?.payload(),
            payload
        );

        let build_129 = vec![b'a'; 129];
        let mut payload = vec![0x3a];
        payload.extend(test_varint(build_129.len()));
        payload.extend_from_slice(&build_129);
        assert_eq!(
            decode_wire_envelope(&test_envelope(1, &payload)),
            Err(FoundationProtocolError::MalformedEnvelope)
        );

        let invalid_utf8 = [0x3a, 0x01, 0xff];
        assert_eq!(
            decode_wire_envelope(&test_envelope(1, &invalid_utf8)),
            Err(FoundationProtocolError::MalformedEnvelope)
        );

        let resume_build_129 = vec![b'a'; 129];
        let mut resume_payload = vec![0x42];
        resume_payload.extend(test_varint(resume_build_129.len()));
        resume_payload.extend_from_slice(&resume_build_129);
        assert_eq!(
            decode_wire_envelope(&test_envelope(3, &resume_payload)),
            Err(FoundationProtocolError::MalformedEnvelope)
        );

        let mut packed = Vec::new();
        for capability in 1..=128usize {
            packed.extend(test_varint(capability));
        }
        let mut payload = test_client_bootstrap_payload(1, 1, 1, &id, &[]);
        payload.push(0x22);
        payload.extend(test_varint(packed.len()));
        payload.extend_from_slice(&packed);
        assert!(decode_wire_envelope(&test_envelope(1, &payload)).is_ok());

        packed.extend(test_varint(129));
        let mut payload = vec![0x22];
        payload.extend(test_varint(packed.len()));
        payload.extend_from_slice(&packed);
        assert_eq!(
            decode_wire_envelope(&test_envelope(1, &payload)),
            Err(FoundationProtocolError::BootstrapLimitExceeded)
        );
        let mut resume_payload = vec![0x3a];
        resume_payload.extend(test_varint(packed.len()));
        resume_payload.extend_from_slice(&packed);
        assert_eq!(
            decode_wire_envelope(&test_envelope(3, &resume_payload)),
            Err(FoundationProtocolError::BootstrapLimitExceeded)
        );

        for invalid_caps in [[1usize, 1usize], [2usize, 1usize]] {
            let mut packed = Vec::new();
            for capability in invalid_caps {
                packed.extend(test_varint(capability));
            }
            let mut payload = vec![0x22];
            payload.extend(test_varint(packed.len()));
            payload.extend_from_slice(&packed);
            assert_eq!(
                decode_wire_envelope(&test_envelope(1, &payload)),
                Err(FoundationProtocolError::InvalidCapabilitySet)
            );
        }
        Ok(())
    }

    #[test]
    fn client_command_limits_cover_payload_and_expected_revisions()
    -> Result<(), FoundationProtocolError> {
        let mut payload = vec![0x22];
        payload.extend(test_varint(65_536));
        payload.resize(payload.len() + 65_536, 0);
        assert!(decode_wire_envelope(&test_envelope(7, &payload)).is_ok());

        let mut payload = vec![0x22];
        payload.extend(test_varint(65_537));
        payload.resize(payload.len() + 65_537, 0);
        assert_eq!(
            decode_wire_envelope(&test_envelope(7, &payload)),
            Err(FoundationProtocolError::PayloadLimitExceeded)
        );

        let mut payload = Vec::new();
        for domain in 1..=64usize {
            let revision = test_state_revision(domain);
            payload.push(0x1a);
            payload.extend(test_varint(revision.len()));
            payload.extend(revision);
        }
        assert!(decode_wire_envelope(&test_envelope(7, &payload)).is_ok());

        let revision = test_state_revision(65);
        payload.push(0x1a);
        payload.extend(test_varint(revision.len()));
        payload.extend(revision);
        assert_eq!(
            decode_wire_envelope(&test_envelope(7, &payload)),
            Err(FoundationProtocolError::PayloadLimitExceeded)
        );

        let revision = test_state_revision(1);
        let mut duplicate = Vec::new();
        for _ in 0..2 {
            duplicate.push(0x1a);
            duplicate.extend(test_varint(revision.len()));
            duplicate.extend_from_slice(&revision);
        }
        assert_eq!(
            decode_wire_envelope(&test_envelope(7, &duplicate)),
            Err(FoundationProtocolError::StateRevisionMismatch)
        );
        Ok(())
    }

    #[test]
    fn liveness_ack_requires_one_nonzero_probe_and_validates_optional_diagnostic() {
        for payload in [
            vec![0x08, 0x01],
            vec![0x08, 0x01, 0x10, 0x00],
            vec![0x10, 0x2a, 0x08, 0x01],
            vec![0x08, 0x01, 0x18, 0x07], // additive unknown varint
            vec![0x08, 0x01, 0x22, 0x01, 0xff], // additive unknown bytes
            vec![
                0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01,
            ],
        ] {
            assert!(decode_wire_envelope(&test_envelope(6, &payload)).is_ok());
        }

        for payload in [
            vec![],
            vec![0x10, 0x01],
            vec![0x08, 0x00],
            vec![0x08, 0x01, 0x08, 0x02],
            vec![0x08, 0x01, 0x10, 0x01, 0x10, 0x02],
            vec![0x0a, 0x01, 0x01],
            vec![0x08, 0x01, 0x12, 0x01, 0x01],
            vec![0x08],
            vec![0x08, 0x80],
            vec![
                0x08, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02,
            ],
            vec![
                0x08, 0x01, 0x10, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02,
            ],
            vec![0x08, 0x01, 0x1a, 0x02, 0x01], // truncated unknown bytes
        ] {
            assert_eq!(
                decode_wire_envelope(&test_envelope(6, &payload)),
                Err(FoundationProtocolError::MalformedEnvelope),
                "payload: {payload:?}"
            );
        }
    }

    #[test]
    fn liveness_probe_and_ack_exact_wire_codec() -> Result<(), FoundationProtocolError> {
        let probe = [0x08, 0x05, 0x10, 0x07, 0x22, 0x03, 0x08, 0x96, 0x01];
        assert_eq!(encode_liveness_probe(7, 150)?, probe);
        let outgoing = decode_wire_envelope(&probe)?;
        assert_eq!(outgoing.message_type(), MessageType::LivenessProbe);
        assert_eq!(outgoing.validate(Direction::ServerToClient, true), Ok(()));
        assert_eq!(outgoing.server_sequence(), 0);
        assert_eq!(outgoing.payload(), &[0x08, 0x96, 0x01]);

        // Explicit diagnostic zero remains present, distinct from omission.
        let ack = [
            0x08, 0x06, 0x10, 0x07, 0x22, 0x05, 0x08, 0x96, 0x01, 0x10, 0x00,
        ];
        assert_eq!(
            decode_wire_envelope(&ack)?.liveness_ack(7)?,
            LivenessAckView {
                probe_id: 150,
                last_applied_server_sequence: Some(0),
            }
        );
        let omitted = [0x08, 0x06, 0x10, 0x07, 0x22, 0x02, 0x08, 0x01];
        assert_eq!(
            decode_wire_envelope(&omitted)?.liveness_ack(7)?,
            LivenessAckView {
                probe_id: 1,
                last_applied_server_sequence: None,
            }
        );
        assert_eq!(
            encode_liveness_probe(0, 1),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        assert_eq!(
            encode_liveness_probe(1, 0),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        Ok(())
    }

    #[test]
    fn liveness_ack_extraction_rejects_wrong_direction_and_generation()
    -> Result<(), FoundationProtocolError> {
        let ack = [0x08, 0x06, 0x10, 0x07, 0x22, 0x02, 0x08, 0x01];
        let decoded = decode_wire_envelope(&ack)?;
        assert_eq!(
            decoded.liveness_ack(8),
            Err(FoundationProtocolError::StaleConnectionGeneration)
        );
        assert_eq!(
            decoded.liveness_ack(0),
            Err(FoundationProtocolError::StaleConnectionGeneration)
        );
        assert_eq!(
            decode_wire_envelope(&[0x08, 0x06, 0x22, 0x02, 0x08, 0x01])?.liveness_ack(7),
            Err(FoundationProtocolError::StaleConnectionGeneration)
        );
        assert_eq!(
            decode_wire_envelope(&[0x08, 0x06, 0x10, 0x07, 0x18, 0x01, 0x22, 0x02, 0x08, 0x01])?
                .liveness_ack(7),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        assert_eq!(
            decode_wire_envelope(&encode_liveness_probe(7, 1)?)?.liveness_ack(7),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        for payload in [
            &[0x08, 0x01, 0x08, 0x02][..],
            &[0x08, 0x01, 0x10, 0x00, 0x10, 0x01],
            &[0x0a, 0x01, 0x01],
            &[0x08, 0x01, 0x12, 0x01, 0x01],
            &[0x08, 0x80],
            &[0x08, 0x01, 0x10, 0x80],
            &[
                0x08, 0x01, 0x10, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02,
            ],
        ] {
            assert_eq!(
                decode_wire_envelope(&test_envelope(6, payload)),
                Err(FoundationProtocolError::MalformedEnvelope),
                "payload: {payload:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn resync_domain_count_is_bounded_and_unique() -> Result<(), FoundationProtocolError> {
        let mut payload = Vec::new();
        for domain in 1..=256usize {
            let revision = test_state_revision(domain);
            payload.push(0x12);
            payload.extend(test_varint(revision.len()));
            payload.extend(revision);
        }
        assert!(decode_wire_envelope(&test_envelope(10, &payload)).is_ok());

        let revision = test_state_revision(257);
        payload.push(0x12);
        payload.extend(test_varint(revision.len()));
        payload.extend(revision);
        assert_eq!(
            decode_wire_envelope(&test_envelope(10, &payload)),
            Err(FoundationProtocolError::PayloadLimitExceeded)
        );

        let revision = test_state_revision(1);
        let mut duplicate = Vec::new();
        for _ in 0..2 {
            duplicate.push(0x12);
            duplicate.extend(test_varint(revision.len()));
            duplicate.extend_from_slice(&revision);
        }
        assert_eq!(
            decode_wire_envelope(&test_envelope(10, &duplicate)),
            Err(FoundationProtocolError::StateRevisionMismatch)
        );
        Ok(())
    }

    #[test]
    fn nested_admission_material_limits_accept_16384_and_reject_16385() {
        let id = test_uuid_v7(1);
        let accepted_material = vec![0u8; 16_384];
        let rejected_material = vec![0u8; 16_385];

        let accepted_bootstrap = test_client_bootstrap_payload_with_material_and_build(
            1,
            1,
            1,
            &id,
            &[],
            &accepted_material,
            b"test-client",
        );
        assert!(decode_wire_envelope(&test_envelope(1, &accepted_bootstrap)).is_ok());
        let rejected_bootstrap = test_client_bootstrap_payload_with_material_and_build(
            1,
            1,
            1,
            &id,
            &[],
            &rejected_material,
            b"test-client",
        );
        assert_eq!(
            decode_wire_envelope(&test_envelope(1, &rejected_bootstrap)),
            Err(FoundationProtocolError::BootstrapLimitExceeded)
        );

        let accepted_resume =
            test_client_resume_payload_with_material(1, 1, 1, &id, &[], &accepted_material);
        assert!(decode_wire_envelope(&test_envelope(3, &accepted_resume)).is_ok());
        let rejected_resume =
            test_client_resume_payload_with_material(1, 1, 1, &id, &[], &rejected_material);
        assert_eq!(
            decode_wire_envelope(&test_envelope(3, &rejected_resume)),
            Err(FoundationProtocolError::BootstrapLimitExceeded)
        );
    }

    #[test]
    fn bootstrap_payload_limit_accepts_65536_and_rejects_65537()
    -> Result<(), FoundationProtocolError> {
        // The exact-boundary payload remains semantically valid and uses field 16
        // as an additive unknown bytes field to reach exactly 65,536 bytes.
        let id = test_uuid_v7(1);
        let mut accepted_payload = test_client_bootstrap_payload(1, 1, 1, &id, &[]);
        let unknown_bytes = 65_536 - accepted_payload.len() - 5;
        accepted_payload.extend([0x82, 0x01]);
        accepted_payload.extend(test_varint(unknown_bytes));
        accepted_payload.resize(65_536, 0);
        let accepted = test_envelope(1, &accepted_payload);
        assert_eq!(decode_wire_envelope(&accepted)?.payload().len(), 65_536);

        let mut rejected = vec![0x08, 0x01, 0x22, 0x81, 0x80, 0x04];
        rejected.resize(6 + 65_537, 0);
        assert_eq!(
            decode_wire_envelope(&rejected),
            Err(FoundationProtocolError::BootstrapLimitExceeded)
        );

        let mut resume = vec![0x08, 0x03, 0x22, 0x81, 0x80, 0x04];
        resume.resize(6 + 65_537, 0);
        assert_eq!(
            decode_wire_envelope(&resume),
            Err(FoundationProtocolError::BootstrapLimitExceeded)
        );
        Ok(())
    }

    #[test]
    fn envelope_oracle_bytes_decode_and_validate_direction() -> Result<(), FoundationProtocolError>
    {
        let bytes = [0x08, 0x07, 0x10, 0x01, 0x22, 0x04, 0x22, 0x02, 0xaa, 0xbb];
        let envelope = decode_wire_envelope(&bytes)?;
        assert_eq!(envelope.message_type(), MessageType::ClientCommand);
        assert_eq!(envelope.connection_generation(), 1);
        assert_eq!(envelope.server_sequence(), 0);
        assert_eq!(envelope.payload(), &[0x22, 0x02, 0xaa, 0xbb]);
        assert_eq!(envelope.validate(Direction::ClientToServer, true), Ok(()));
        assert_eq!(
            envelope.validate(Direction::ServerToClient, true),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        Ok(())
    }

    #[test]
    fn additive_unknown_envelope_field_is_safely_ignored() -> Result<(), FoundationProtocolError> {
        let payload = test_client_bootstrap_payload(1, 1, 1, &test_uuid_v7(1), &[]);
        let mut bytes = test_envelope(1, &payload);
        bytes.extend([0x80, 0x01, 0x01]);
        let envelope = decode_wire_envelope(&bytes)?;
        assert_eq!(envelope.message_type(), MessageType::ClientBootstrap);
        assert_eq!(envelope.payload(), payload);
        Ok(())
    }

    #[test]
    fn bootstrap_phase_requires_pre_admission_zero_generation()
    -> Result<(), FoundationProtocolError> {
        let payload = test_client_bootstrap_payload(1, 1, 1, &test_uuid_v7(1), &[]);
        let mut with_generation = vec![0x08, 0x01, 0x10, 0x01, 0x22];
        with_generation.extend(test_varint(payload.len()));
        with_generation.extend_from_slice(&payload);
        let envelope = decode_wire_envelope(&with_generation)?;
        assert_eq!(
            envelope.validate(Direction::ClientToServer, false),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        let zero_generation = test_envelope(1, &payload);
        let envelope = decode_wire_envelope(&zero_generation)?;
        assert_eq!(
            envelope.validate(Direction::ClientToServer, true),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        Ok(())
    }

    #[test]
    fn unknown_and_truncated_envelopes_fail_closed() {
        assert_eq!(
            decode_wire_envelope(&[0x08, 0x63]),
            Err(FoundationProtocolError::UnknownMessageType)
        );
        assert_eq!(
            decode_wire_envelope(&[0x08]),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
    }

    #[test]
    fn framed_envelope_rejects_truncation_and_oversized_prefix_before_body_access()
    -> Result<(), FoundationProtocolError> {
        let valid = [0, 0, 0, 4, 0x08, 0x0e, 0x22, 0x00];
        assert_eq!(
            decode_framed_envelope(&valid)?.message_type(),
            MessageType::ProtocolError
        );
        assert_eq!(
            decode_framed_envelope(&[0, 0, 0, 5, 0x08, 0x0e, 0x22, 0x00]),
            Err(FoundationProtocolError::MalformedFrame)
        );
        assert_eq!(
            decode_framed_envelope(&[0, 0x10, 0, 1]),
            Err(FoundationProtocolError::FrameTooLarge)
        );
        Ok(())
    }

    #[test]
    fn wire_identifier_requires_exact_nonzero_sixteen_bytes() -> Result<(), FoundationProtocolError>
    {
        assert_eq!(
            GameSessionId::decode(&[0; 15]),
            Err(FoundationProtocolError::InvalidWireIdentifier)
        );
        assert_eq!(
            GameSessionId::decode(&[0; 16]),
            Err(FoundationProtocolError::InvalidWireIdentifier)
        );
        let mut raw = [0u8; 16];
        raw[6] = 0x70;
        raw[8] = 0x80;
        raw[15] = 1;
        assert_eq!(GameSessionId::decode(&raw)?.as_bytes(), &raw);
        Ok(())
    }

    #[test]
    fn registry_error_dispositions_are_stable_and_safe() {
        assert_eq!(
            FoundationProtocolError::MalformedFrame.disposition(),
            ProtocolDisposition::TransportFatal
        );
        assert_eq!(
            FoundationProtocolError::CommandSequenceGap.disposition(),
            ProtocolDisposition::ResyncRequired
        );
        assert_eq!(
            FoundationProtocolError::SnapshotLimitExceeded.disposition(),
            ProtocolDisposition::SessionFatal
        );
        assert_eq!(
            FoundationProtocolError::AccountDataIntegrity.disposition(),
            ProtocolDisposition::OperationTerminal
        );
    }

    /// FND-02 §18: every code of this crate is the registry's, with the same name and default
    /// disposition, and the registry holds no code the crate lacks.
    #[test]
    #[allow(clippy::expect_used)]
    fn error_codes_match_the_registry() {
        use FoundationProtocolError as E;
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json"
        ))
        .expect("protocol registry");
        let registered: Vec<(u64, &str, &str)> = registry["error_codes"]
            .as_array()
            .expect("error_codes")
            .iter()
            .map(|entry| {
                (
                    entry["code"].as_u64().expect("code"),
                    entry["name"].as_str().expect("name"),
                    entry["default_disposition"].as_str().expect("disposition"),
                )
            })
            .collect();
        let crate_codes: Vec<(u64, &str, &str)> = [
            (E::MalformedFrame, "MALFORMED_FRAME"),
            (E::FrameTooLarge, "FRAME_TOO_LARGE"),
            (E::MalformedEnvelope, "MALFORMED_ENVELOPE"),
            (E::UnknownMessageType, "UNKNOWN_MESSAGE_TYPE"),
            (E::ProtocolMajorMismatch, "PROTOCOL_MAJOR_MISMATCH"),
            (E::TransportProfileMismatch, "TRANSPORT_PROFILE_MISMATCH"),
            (E::CapabilityMismatch, "CAPABILITY_MISMATCH"),
            (E::InvalidWireIdentifier, "INVALID_WIRE_IDENTIFIER"),
            (E::PayloadLimitExceeded, "PAYLOAD_LIMIT_EXCEEDED"),
            (E::StaleConnectionGeneration, "STALE_CONNECTION_GENERATION"),
            (E::CommandOutcomeExpired, "COMMAND_OUTCOME_EXPIRED"),
            (E::CommandSequenceGap, "COMMAND_SEQUENCE_GAP"),
            (
                E::TooManyOutstandingCommands,
                "TOO_MANY_OUTSTANDING_COMMANDS",
            ),
            (E::ServerSequenceGap, "SERVER_SEQUENCE_GAP"),
            (E::StateRevisionMismatch, "STATE_REVISION_MISMATCH"),
            (E::SnapshotAssemblyInvalid, "SNAPSHOT_ASSEMBLY_INVALID"),
            (E::SnapshotLimitExceeded, "SNAPSHOT_LIMIT_EXCEEDED"),
            (E::BootstrapLimitExceeded, "BOOTSTRAP_LIMIT_EXCEEDED"),
            (E::InvalidCapabilitySet, "INVALID_CAPABILITY_SET"),
            (E::AccountDataIntegrity, "ACCOUNT_DATA_INTEGRITY"),
        ]
        .into_iter()
        .map(|(error, name)| {
            let disposition = match error.disposition() {
                ProtocolDisposition::OperationTerminal => "OPERATION_TERMINAL",
                ProtocolDisposition::ResyncRequired => "RESYNC_REQUIRED",
                ProtocolDisposition::SessionFatal => "SESSION_FATAL",
                ProtocolDisposition::TransportFatal => "TRANSPORT_FATAL",
            };
            (u64::from(error.code()), name, disposition)
        })
        .collect();
        assert_eq!(crate_codes, registered);
    }

    #[test]
    fn semantic_wire_ids_require_uuidv7_and_rfc_variant() -> Result<(), FoundationProtocolError> {
        let mut valid = [0u8; 16];
        valid[6] = 0x70;
        valid[8] = 0x80;
        valid[15] = 1;
        assert!(GameSessionId::decode(&valid).is_ok());
        assert!(CharacterId::decode(&valid).is_ok());
        assert!(WorldId::decode(&valid).is_ok());
        assert!(ChannelId::decode(&valid).is_ok());
        assert!(NodeId::decode(&valid).is_ok());
        let mut wrong_version = valid;
        wrong_version[6] = 0x40;
        assert_eq!(
            NodeId::decode(&wrong_version),
            Err(FoundationProtocolError::InvalidWireIdentifier)
        );
        let mut wrong_variant = valid;
        wrong_variant[8] = 0x00;
        assert_eq!(
            NodeId::decode(&wrong_variant),
            Err(FoundationProtocolError::InvalidWireIdentifier)
        );
        Ok(())
    }

    fn client_command_frame(generation: u64, command_id: u64, command_type: u64) -> Vec<u8> {
        let mut body = Vec::new();
        push_scalar(&mut body, 1, command_id);
        push_scalar(&mut body, 2, command_type);
        push_bytes(&mut body, 4, &[0x08, 0x02]);
        let mut frame = Vec::new();
        push_scalar(&mut frame, 1, MessageType::ClientCommand as u64);
        push_scalar(&mut frame, 2, generation);
        push_bytes(&mut frame, 4, &body);
        frame
    }

    #[test]
    fn client_command_decodes_identity_type_and_payload() -> Result<(), FoundationProtocolError> {
        let frame = client_command_frame(3, 1, 1);
        let command = decode_wire_envelope(&frame)?.client_command(3)?;
        assert_eq!(
            command,
            ClientCommandView {
                command_id: 1,
                command_type: 1,
                payload: &[0x08, 0x02],
            }
        );
        assert_eq!(
            decode_wire_envelope(&frame)?.client_command(4),
            Err(FoundationProtocolError::StaleConnectionGeneration)
        );
        for (id, command_type) in [(0, 1), (1, 0), (1, u64::from(u32::MAX) + 1)] {
            let frame = client_command_frame(3, id, command_type);
            assert_eq!(
                decode_wire_envelope(&frame).and_then(|view| view.client_command(3)),
                Err(FoundationProtocolError::MalformedEnvelope),
                "{id} {command_type}"
            );
        }
        Ok(())
    }

    /// `encode_client_command` round-trips through the same server-side ingress
    /// (`decode_wire_envelope` + `WireEnvelopeView::client_command`) the shipped server uses for
    /// every real `ClientCommand`, and matches bytes transcribed independently from
    /// `foundation.proto` (envelope 1/2/4, command 1/2/4).
    #[test]
    fn client_command_encoder_round_trips_through_server_ingress_decode()
    -> Result<(), FoundationProtocolError> {
        const STEP_EAST: &[u8] = &[8, 7, 16, 3, 34, 8, 8, 5, 16, 1, 34, 2, 8, 2];
        let wire = encode_client_command(
            3,
            &ClientCommandValue {
                command_id: 5,
                command_type: 1,
                payload: &[0x08, 0x02],
            },
        )?;
        assert_eq!(wire, STEP_EAST);
        let envelope = decode_wire_envelope(&wire)?;
        assert_eq!(envelope.message_type(), MessageType::ClientCommand);
        assert_eq!(envelope.validate(Direction::ClientToServer, true), Ok(()));
        assert_eq!(envelope.server_sequence(), 0);
        assert_eq!(
            envelope.client_command(3)?,
            ClientCommandView {
                command_id: 5,
                command_type: 1,
                payload: &[0x08, 0x02],
            }
        );
        // A multi-byte command id and type survive the round trip.
        let wire = encode_client_command(
            u64::MAX,
            &ClientCommandValue {
                command_id: u64::MAX,
                command_type: u32::MAX,
                payload: &[],
            },
        )?;
        let view = decode_wire_envelope(&wire)?.client_command(u64::MAX)?;
        assert_eq!(
            (view.command_id, view.command_type, view.payload),
            (u64::MAX, u32::MAX, &[][..])
        );
        Ok(())
    }

    #[test]
    fn client_command_encoder_refuses_what_ingress_would_refuse() {
        let value = ClientCommandValue {
            command_id: 1,
            command_type: 1,
            payload: &[],
        };
        assert_eq!(
            encode_client_command(0, &value),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        assert_eq!(
            encode_client_command(
                1,
                &ClientCommandValue {
                    command_id: 0,
                    ..value
                }
            ),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        assert_eq!(
            encode_client_command(
                1,
                &ClientCommandValue {
                    command_type: 0,
                    ..value
                }
            ),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        let at_limit = vec![0_u8; MAX_COMMAND_PAYLOAD_BYTES];
        let over_limit = vec![0_u8; MAX_COMMAND_PAYLOAD_BYTES + 1];
        assert!(
            encode_client_command(
                1,
                &ClientCommandValue {
                    payload: &at_limit,
                    ..value
                }
            )
            .is_ok()
        );
        assert_eq!(
            encode_client_command(
                1,
                &ClientCommandValue {
                    payload: &over_limit,
                    ..value
                }
            ),
            Err(FoundationProtocolError::PayloadLimitExceeded)
        );
    }

    /// `encode_liveness_ack` matches the explicit-zero-diagnostic vector the ingress test above
    /// transcribes, and `decode_liveness_probe` reads back what the existing server-side
    /// `encode_liveness_probe` emits.
    #[test]
    fn liveness_ack_encoder_and_probe_decoder_round_trip_through_existing_codecs()
    -> Result<(), FoundationProtocolError> {
        let ack = encode_liveness_ack(7, 150, 0)?;
        assert_eq!(
            ack,
            [
                0x08, 0x06, 0x10, 0x07, 0x22, 0x05, 0x08, 0x96, 0x01, 0x10, 0x00
            ]
        );
        assert_eq!(
            decode_wire_envelope(&ack)?.liveness_ack(7)?,
            LivenessAckView {
                probe_id: 150,
                last_applied_server_sequence: Some(0),
            }
        );
        let ack = encode_liveness_ack(7, 1, 300)?;
        assert_eq!(
            decode_wire_envelope(&ack)?.liveness_ack(7)?,
            LivenessAckView {
                probe_id: 1,
                last_applied_server_sequence: Some(300),
            }
        );

        let probe = encode_liveness_probe(7, 150)?;
        let envelope = decode_wire_envelope(&probe)?;
        assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
        assert_eq!(decode_liveness_probe(envelope.payload())?, 150);

        assert_eq!(
            encode_liveness_ack(0, 1, 0),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        assert_eq!(
            encode_liveness_ack(1, 0, 0),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        // Absent, zero, wrong-wire-type and repeated probe ids all fail closed.
        for payload in [
            &[][..],
            &[0x08, 0x00],
            &[0x0a, 0x01, 0x01],
            &[0x08, 0x01, 0x08, 0x02],
        ] {
            assert_eq!(
                decode_liveness_probe(payload),
                Err(FoundationProtocolError::MalformedEnvelope),
                "{payload:?}"
            );
        }
        Ok(())
    }

    /// `decode_command_result` round-trips through the existing server-side
    /// `encode_command_result`, including the sequenced envelope the client validates first.
    #[test]
    fn command_result_decoder_round_trips_through_existing_server_encoder()
    -> Result<(), FoundationProtocolError> {
        for (status, payload) in [
            (CommandStatus::Accepted, &[0x08, 0x01][..]),
            (CommandStatus::Rejected, &[][..]),
        ] {
            let wire = encode_command_result(4, 9, 300, status, payload)?;
            let envelope = decode_wire_envelope(&wire)?;
            assert_eq!(envelope.message_type(), MessageType::CommandResult);
            assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
            assert_eq!(envelope.connection_generation(), 4);
            assert_eq!(envelope.server_sequence(), 9);
            assert_eq!(
                decode_command_result(envelope.payload())?,
                CommandResultView {
                    command_id: 300,
                    status,
                    error_code: 0,
                    payload,
                }
            );
        }
        Ok(())
    }

    /// An operation-terminal command error: `REJECTED`, the registered code in `error_code` and no
    /// payload, in hand-computed canonical bytes. Other dispositions are refused.
    #[test]
    fn command_error_result_carries_only_an_operation_terminal_code()
    -> Result<(), FoundationProtocolError> {
        let wire =
            encode_command_error_result(4, 9, 300, FoundationProtocolError::PayloadLimitExceeded)?;
        // CommandResult (8), generation 4, sequence 9, body: command 300 = ac 02, status 2,
        // error_code 1009 = f1 07.
        assert_eq!(
            wire,
            [
                0x08, 0x08, 0x10, 0x04, 0x18, 0x09, 0x22, 0x08, 0x08, 0xac, 0x02, 0x10, 0x02, 0x18,
                0xf1, 0x07
            ]
        );
        let envelope = decode_wire_envelope(&wire)?;
        assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
        assert_eq!(
            decode_command_result(envelope.payload())?,
            CommandResultView {
                command_id: 300,
                status: CommandStatus::Rejected,
                error_code: 1009,
                payload: &[],
            }
        );
        // ACCOUNT_DATA_INTEGRITY 1050 = 9a 08: the code and nothing else (no payload, no text).
        let wire =
            encode_command_error_result(4, 9, 300, FoundationProtocolError::AccountDataIntegrity)?;
        assert_eq!(
            wire,
            [
                0x08, 0x08, 0x10, 0x04, 0x18, 0x09, 0x22, 0x08, 0x08, 0xac, 0x02, 0x10, 0x02, 0x18,
                0x9a, 0x08
            ]
        );
        assert_eq!(
            decode_command_result(decode_wire_envelope(&wire)?.payload())?,
            CommandResultView {
                command_id: 300,
                status: CommandStatus::Rejected,
                error_code: 1050,
                payload: &[],
            }
        );
        for error in [
            FoundationProtocolError::MalformedFrame,
            FoundationProtocolError::UnknownMessageType,
            FoundationProtocolError::CommandSequenceGap,
        ] {
            assert_eq!(
                encode_command_error_result(4, 9, 300, error),
                Err(FoundationProtocolError::MalformedEnvelope)
            );
        }
        assert_eq!(
            encode_command_error_result(0, 9, 300, FoundationProtocolError::PayloadLimitExceeded),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        Ok(())
    }

    /// The registered duplicate statuses (FND-02 §13.2) round-trip through the existing server
    /// encoder, carrying the replayed payload (or none) and the sequenced envelope.
    #[test]
    fn command_result_decoder_represents_the_duplicate_statuses()
    -> Result<(), FoundationProtocolError> {
        for (status, wire_status, payload) in [
            (CommandStatus::DuplicateReplay, 3, &[0x08, 0x02][..]),
            (CommandStatus::DuplicateOutcomeExpired, 4, &[][..]),
        ] {
            assert_eq!(status as u32, wire_status);
            let wire = encode_command_result(4, 12, 6, status, payload)?;
            let envelope = decode_wire_envelope(&wire)?;
            assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
            assert_eq!(envelope.server_sequence(), 12);
            assert_eq!(
                decode_command_result(envelope.payload())?,
                CommandResultView {
                    command_id: 6,
                    status,
                    error_code: 0,
                    payload,
                }
            );
        }
        Ok(())
    }

    #[test]
    fn command_result_decoder_fails_closed_on_unusable_results() {
        // An unregistered status (5), unspecified or absent status, an absent or zero command
        // id, a duplicated singular field, a wrong-wire-type payload and an error code above
        // u32 all fail closed.
        for payload in [
            &[0x08, 0x01, 0x10, 0x05][..],
            &[0x08, 0x01, 0x10, 0x00],
            &[0x08, 0x01],
            &[0x10, 0x01],
            &[0x08, 0x00, 0x10, 0x01],
            &[0x08, 0x01, 0x08, 0x02, 0x10, 0x01],
            &[0x08, 0x01, 0x10, 0x01, 0x28, 0x01],
            &[0x08, 0x01, 0x10, 0x01, 0x18, 0xff, 0xff, 0xff, 0xff, 0x1f],
        ] {
            assert_eq!(
                decode_command_result(payload),
                Err(FoundationProtocolError::MalformedEnvelope),
                "{payload:?}"
            );
        }
        // Optional error code and unknown fields decode; a bad authoritative_revision list is
        // refused by the shared ingress validation.
        assert_eq!(
            decode_command_result(&[0x08, 0x01, 0x10, 0x02, 0x18, 0x07, 0x78, 0x01]),
            Ok(CommandResultView {
                command_id: 1,
                status: CommandStatus::Rejected,
                error_code: 7,
                payload: &[],
            })
        );
        assert_eq!(
            decode_command_result(&[0x08, 0x01, 0x10, 0x01, 0x22, 0x02, 0x10, 0x01]),
            Err(FoundationProtocolError::StateRevisionMismatch)
        );
    }

    /// `decode_state_delta` round-trips through the existing server-side `encode_state_delta`.
    #[test]
    fn state_delta_decoder_round_trips_through_existing_server_encoder()
    -> Result<(), FoundationProtocolError> {
        let wire = encode_state_delta(2, 11, 2, 40, 41, 1, &[0xaa, 0xbb])?;
        let envelope = decode_wire_envelope(&wire)?;
        assert_eq!(envelope.message_type(), MessageType::StateDelta);
        assert_eq!(envelope.validate(Direction::ServerToClient, true), Ok(()));
        assert_eq!(envelope.server_sequence(), 11);
        assert_eq!(
            decode_state_delta(envelope.payload())?,
            StateDeltaView {
                domain_id: 2,
                base_revision: 40,
                new_revision: 41,
                delta_type: 1,
                payload: &[0xaa, 0xbb],
            }
        );
        Ok(())
    }

    #[test]
    fn state_delta_decoder_applies_the_encoders_semantic_bounds() {
        // Zero domain, zero delta type, a non-advancing revision, an absent revision pair and a
        // duplicated singular field are all refused.
        for payload in [
            &[0x10, 0x01, 0x18, 0x02, 0x20, 0x01][..],
            &[0x08, 0x01, 0x10, 0x01, 0x18, 0x02],
            &[0x08, 0x01, 0x10, 0x02, 0x18, 0x02, 0x20, 0x01],
            &[0x08, 0x01, 0x10, 0x03, 0x18, 0x02, 0x20, 0x01],
            &[0x08, 0x01, 0x20, 0x01],
            &[0x08, 0x01, 0x08, 0x02, 0x10, 0x01, 0x18, 0x02, 0x20, 0x01],
        ] {
            assert_eq!(
                decode_state_delta(payload),
                Err(FoundationProtocolError::MalformedEnvelope),
                "{payload:?}"
            );
        }
        assert_eq!(
            decode_state_delta(&[0x08, 0x01, 0x18, 0x01, 0x20, 0x01]),
            Ok(StateDeltaView {
                domain_id: 1,
                base_revision: 0,
                new_revision: 1,
                delta_type: 1,
                payload: &[],
            })
        );
    }

    #[test]
    fn snapshot_begin_decoder_reads_target_server_sequence_from_existing_server_encoder()
    -> Result<(), FoundationProtocolError> {
        let payload = [0_u8; 3];
        for target in [0_u64, 42, u64::from(u32::MAX) + 1] {
            let frames = encode_single_chunk_snapshot(
                1,
                9,
                target,
                &[DomainSnapshot {
                    domain_id: 1,
                    revision: 1,
                    snapshot_type: 1,
                    payload: &payload,
                }],
            )?;
            let begin = decode_snapshot_begin(decode_wire_envelope(&frames[0])?.payload())?;
            assert_eq!(begin.snapshot_id, 9);
            assert_eq!(begin.target_server_sequence, target);
        }
        Ok(())
    }
}
