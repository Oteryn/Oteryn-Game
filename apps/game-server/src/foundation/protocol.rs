use super::FoundationProtocolError;
use std::collections::{BTreeMap, BTreeSet};

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
foundation_uuid_v7_id!(GameSessionId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandRef {
    game_session_id: GameSessionId,
    command_id: super::CommandId,
}
impl CommandRef {
    #[must_use]
    pub const fn new(game_session_id: GameSessionId, command_id: super::CommandId) -> Self {
        Self {
            game_session_id,
            command_id,
        }
    }
    #[must_use]
    pub const fn game_session_id(self) -> GameSessionId {
        self.game_session_id
    }
    #[must_use]
    pub const fn command_id(self) -> super::CommandId {
        self.command_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResyncPlan {
    UpToDate,
    ReplayFrom(u64),
    SnapshotRequired,
}
#[must_use]
pub fn plan_resync(last_applied: u64, current: u64, retained_from: u64) -> ResyncPlan {
    if last_applied == current {
        return ResyncPlan::UpToDate;
    }
    let Some(next) = last_applied.checked_add(1) else {
        return ResyncPlan::SnapshotRequired;
    };
    if last_applied < current && next >= retained_from {
        ResyncPlan::ReplayFrom(next)
    } else {
        ResyncPlan::SnapshotRequired
    }
}

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
}
impl<'a> WireEnvelopeView<'a> {
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
fn skip_field(input: &[u8], cursor: &mut usize, wire: u8) -> Result<(), FoundationProtocolError> {
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

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, FoundationProtocolError> {
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
fn bounded_length_delimited<'a>(
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
    *previous = Some(capability);
    Ok(())
}

fn validate_capability_field(
    payload: &[u8],
    cursor: &mut usize,
    wire: u8,
    count: &mut usize,
    previous: &mut Option<u32>,
) -> Result<(), FoundationProtocolError> {
    match wire {
        0 => validate_capability(read_varint(payload, cursor)?, count, previous),
        2 => {
            let packed = unbounded_length_delimited(payload, cursor)?;
            let mut packed_cursor = 0usize;
            while packed_cursor < packed.len() {
                validate_capability(read_varint(packed, &mut packed_cursor)?, count, previous)?;
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

fn validate_bootstrap_ingress(
    message_type: MessageType,
    payload: &[u8],
) -> Result<(), FoundationProtocolError> {
    let (material_field, capabilities_field, build_field, material_maximum) = match message_type {
        MessageType::ClientBootstrap => (5u32, 4u32, 7u32, MAX_ADMISSION_MATERIAL_BYTES),
        MessageType::ClientResume => (2u32, 7u32, 8u32, MAX_RECONNECT_MATERIAL_BYTES),
        _ => return Ok(()),
    };
    let mut cursor = 0usize;
    let mut capability_count = 0usize;
    let mut previous_capability = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        if field == 0 {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        if field == material_field {
            if wire != 2 {
                return Err(FoundationProtocolError::MalformedEnvelope);
            }
            bounded_length_delimited(
                payload,
                &mut cursor,
                material_maximum,
                FoundationProtocolError::BootstrapLimitExceeded,
            )?;
        } else if field == capabilities_field {
            validate_capability_field(
                payload,
                &mut cursor,
                wire,
                &mut capability_count,
                &mut previous_capability,
            )?;
        } else if field == build_field {
            if wire != 2 {
                return Err(FoundationProtocolError::MalformedEnvelope);
            }
            let build_id = bounded_length_delimited(
                payload,
                &mut cursor,
                MAX_CLIENT_BUILD_ID_BYTES,
                FoundationProtocolError::MalformedEnvelope,
            )?;
            std::str::from_utf8(build_id)
                .map_err(|_| FoundationProtocolError::MalformedEnvelope)?;
        } else {
            skip_field(payload, &mut cursor, wire)?;
        }
    }
    Ok(())
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

fn validate_server_capabilities_ingress(
    message_type: MessageType,
    payload: &[u8],
) -> Result<(), FoundationProtocolError> {
    let capabilities_field = match message_type {
        MessageType::ServerAccepted => 10u32,
        MessageType::ServerResumeAccepted => 6u32,
        _ => return Ok(()),
    };
    let mut cursor = 0usize;
    let mut capability_count = 0usize;
    let mut previous_capability = None;
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        let field = decode_field_number(key)?;
        let wire = (key & 7) as u8;
        if field == capabilities_field {
            validate_capability_field(
                payload,
                &mut cursor,
                wire,
                &mut capability_count,
                &mut previous_capability,
            )?;
        } else {
            skip_field(payload, &mut cursor, wire)?;
        }
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
            validate_bootstrap_ingress(message_type, payload)
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
            validate_server_capabilities_ingress(message_type, payload)
        }
        MessageType::CommandResult => validate_command_result_ingress(payload),
        MessageType::StateDelta => validate_state_delta_ingress(payload),
        MessageType::SnapshotBegin => validate_snapshot_begin_ingress(payload),
        MessageType::SnapshotChunk => validate_snapshot_chunk_ingress(payload),
        _ => Ok(()),
    }
}

pub fn decode_wire_envelope(input: &[u8]) -> Result<WireEnvelopeView<'_>, FoundationProtocolError> {
    if input.is_empty() || input.len() > super::MAX_WIRE_FRAME_BYTES as usize {
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
    match message_type.direction() {
        Direction::ClientToServer => validate_client_ingress_payload(message_type, payload)?,
        Direction::ServerToClient => validate_server_ingress_payload(message_type, payload)?,
    }
    Ok(WireEnvelopeView {
        message_type,
        connection_generation: generation.unwrap_or(0),
        server_sequence: sequence.unwrap_or(0),
        payload,
    })
}

pub fn decode_framed_envelope(
    frame: &[u8],
) -> Result<WireEnvelopeView<'_>, FoundationProtocolError> {
    let prefix = frame
        .get(..4)
        .ok_or(FoundationProtocolError::MalformedFrame)?;
    let length = super::FrameLength::from_prefix(prefix)?;
    let body = frame
        .get(4..)
        .ok_or(FoundationProtocolError::MalformedFrame)?;
    if body.len() != length.get() as usize {
        return Err(FoundationProtocolError::MalformedFrame);
    }
    decode_wire_envelope(body)
}

#[derive(Debug, Clone, Default)]
pub struct StateRevisionTracker {
    revisions: BTreeMap<u32, u64>,
}
impl StateRevisionTracker {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn revision(&self, domain_id: u32) -> Option<u64> {
        self.revisions.get(&domain_id).copied()
    }
    pub fn apply_delta(
        &mut self,
        domain_id: u32,
        base: u64,
        new: u64,
    ) -> Result<(), FoundationProtocolError> {
        if domain_id == 0 || new <= base {
            return Err(FoundationProtocolError::StateRevisionMismatch);
        }
        let current = self.revision(domain_id).unwrap_or(0);
        if current != base {
            return Err(FoundationProtocolError::StateRevisionMismatch);
        }
        self.revisions.insert(domain_id, new);
        Ok(())
    }
    pub fn apply_snapshot_revision(
        &mut self,
        domain_id: u32,
        revision: u64,
    ) -> Result<(), FoundationProtocolError> {
        if domain_id == 0
            || self
                .revision(domain_id)
                .is_some_and(|current| revision < current)
        {
            return Err(FoundationProtocolError::StateRevisionMismatch);
        }
        self.revisions.insert(domain_id, revision);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceDecision {
    Apply,
    Duplicate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerSequenceTracker {
    last_applied: u64,
}
impl Default for ServerSequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}
impl ServerSequenceTracker {
    pub const fn new() -> Self {
        Self { last_applied: 0 }
    }
    pub const fn last_applied(&self) -> u64 {
        self.last_applied
    }
    pub fn next_expected(&self) -> Option<u64> {
        self.last_applied.checked_add(1)
    }
    pub fn observe(&self, sequence: u64) -> Result<SequenceDecision, FoundationProtocolError> {
        if sequence == 0 {
            return Err(FoundationProtocolError::MalformedEnvelope);
        }
        if sequence <= self.last_applied {
            return Ok(SequenceDecision::Duplicate);
        }
        let expected = self
            .next_expected()
            .ok_or(FoundationProtocolError::ServerSequenceGap)?;
        if sequence != expected {
            return Err(FoundationProtocolError::ServerSequenceGap);
        }
        Ok(SequenceDecision::Apply)
    }
    pub fn commit_applied(&mut self, sequence: u64) -> Result<(), FoundationProtocolError> {
        let expected = self
            .next_expected()
            .ok_or(FoundationProtocolError::ServerSequenceGap)?;
        if sequence != expected {
            return Err(FoundationProtocolError::ServerSequenceGap);
        }
        self.last_applied = sequence;
        Ok(())
    }
    pub fn apply_snapshot_boundary(&mut self, target: u64) -> Result<(), FoundationProtocolError> {
        if target < self.last_applied {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        self.last_applied = target;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct SnapshotAssembly {
    id: u64,
    chunk_count: u32,
    total_bytes: u64,
    target_sequence: u64,
    generation: u64,
    chunks: BTreeMap<u32, Vec<u8>>,
    received: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotCommitResult {
    target_server_sequence: u64,
    body: Vec<u8>,
}
impl SnapshotCommitResult {
    #[must_use]
    pub const fn target_server_sequence(&self) -> u64 {
        self.target_server_sequence
    }
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

#[derive(Debug, Clone, Default)]
pub struct SnapshotBarrier {
    active: Option<SnapshotAssembly>,
    highest_snapshot_id: Option<u64>,
}
impl SnapshotBarrier {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }
    pub fn begin(
        &mut self,
        id: u64,
        chunk_count: u32,
        total_bytes: u64,
        target_sequence: u64,
        generation: u64,
    ) -> Result<(), FoundationProtocolError> {
        if id == 0
            || generation == 0
            || chunk_count > MAX_SNAPSHOT_CHUNKS
            || total_bytes > MAX_SNAPSHOT_ASSEMBLED_BYTES
        {
            return Err(FoundationProtocolError::SnapshotLimitExceeded);
        }
        if self.active.is_some()
            || self
                .highest_snapshot_id
                .is_some_and(|highest| id <= highest)
        {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        self.highest_snapshot_id = Some(id);
        self.active = Some(SnapshotAssembly {
            id,
            chunk_count,
            total_bytes,
            target_sequence,
            generation,
            chunks: BTreeMap::new(),
            received: 0,
        });
        Ok(())
    }
    pub fn chunk(
        &mut self,
        id: u64,
        index: u32,
        data: &[u8],
        generation: u64,
    ) -> Result<(), FoundationProtocolError> {
        let Some(active) = self.active.as_mut() else {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        };
        if generation != active.generation {
            self.active = None;
            return Err(FoundationProtocolError::StaleConnectionGeneration);
        }
        if data.len() > MAX_SNAPSHOT_CHUNK_BYTES {
            self.active = None;
            return Err(FoundationProtocolError::SnapshotLimitExceeded);
        }
        if id != active.id || index >= active.chunk_count {
            self.active = None;
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        if let Some(existing) = active.chunks.get(&index) {
            if existing.as_slice() == data {
                return Ok(());
            }
            self.active = None;
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        let Some(next) = active.received.checked_add(data.len() as u64) else {
            self.active = None;
            return Err(FoundationProtocolError::SnapshotLimitExceeded);
        };
        if next > active.total_bytes || next > MAX_SNAPSHOT_ASSEMBLED_BYTES {
            self.active = None;
            return Err(FoundationProtocolError::SnapshotLimitExceeded);
        }
        active.received = next;
        active.chunks.insert(index, data.to_vec());
        Ok(())
    }
    pub fn commit(
        &mut self,
        id: u64,
        generation: u64,
    ) -> Result<SnapshotCommitResult, FoundationProtocolError> {
        let Some(active) = self.active.take() else {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        };
        if generation != active.generation {
            return Err(FoundationProtocolError::StaleConnectionGeneration);
        }
        if id != active.id
            || active.received != active.total_bytes
            || active.chunks.len() != active.chunk_count as usize
            || (0..active.chunk_count).any(|i| !active.chunks.contains_key(&i))
        {
            return Err(FoundationProtocolError::SnapshotAssemblyInvalid);
        }
        let capacity = usize::try_from(active.total_bytes)
            .map_err(|_| FoundationProtocolError::SnapshotLimitExceeded)?;
        let mut body = Vec::with_capacity(capacity);
        for index in 0..active.chunk_count {
            let chunk = active
                .chunks
                .get(&index)
                .ok_or(FoundationProtocolError::SnapshotAssemblyInvalid)?;
            body.extend_from_slice(chunk);
        }
        Ok(SnapshotCommitResult {
            target_server_sequence: active.target_sequence,
            body,
        })
    }
    pub fn may_emit_sequenced(&self, sequence: u64, generation: u64) -> bool {
        self.active
            .as_ref()
            .is_none_or(|a| a.generation != generation || sequence <= a.target_sequence)
    }
    pub fn discard_for_generation_change(&mut self) {
        self.active = None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::ProtocolDisposition;

    #[test]
    fn command_ref_scopes_command_id_to_game_session() -> Result<(), FoundationProtocolError> {
        let mut one = [0u8; 16];
        one[6] = 0x70;
        one[8] = 0x80;
        one[15] = 1;
        let mut two = [0u8; 16];
        two[6] = 0x70;
        two[8] = 0x80;
        two[15] = 2;
        let command = super::super::CommandId::new(1)
            .map_err(|_| FoundationProtocolError::InvalidWireIdentifier)?;
        let a = CommandRef::new(GameSessionId::decode(&one)?, command);
        let b = CommandRef::new(GameSessionId::decode(&two)?, command);
        assert_ne!(a, b);
        assert_eq!(a.command_id(), command);
        assert_eq!(a.game_session_id().as_bytes(), &one);
        Ok(())
    }

    #[test]
    fn empty_snapshot_body_is_valid_when_encoded_length_is_zero()
    -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(41, 0, 0, 9, 1)?;
        assert_eq!(barrier.commit(41, 1)?.target_server_sequence(), 9);
        Ok(())
    }

    #[test]
    fn frame_length_boundary_property_style() {
        for raw in [1u32, 2, 255, 65_535, 1_048_575, 1_048_576] {
            assert_eq!(
                super::super::FrameLength::new(raw).map(super::super::FrameLength::get),
                Ok(raw)
            );
        }
        assert_eq!(
            super::super::FrameLength::new(0),
            Err(FoundationProtocolError::MalformedFrame)
        );
        assert_eq!(
            super::super::FrameLength::new(1_048_577),
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
        let mut payload = Vec::new();
        push_test_varint_field(&mut payload, 1, protocol_major);
        push_test_varint_field(&mut payload, 2, transport_profile);
        push_test_varint_field(&mut payload, 3, schema_revision);
        for capability in supported_capabilities {
            push_test_varint_field(&mut payload, 4, *capability);
        }
        push_test_bytes_field(&mut payload, 5, &[0xaa]);
        push_test_bytes_field(&mut payload, 6, character_id);
        push_test_bytes_field(&mut payload, 7, b"test-client");
        payload
    }

    fn test_client_resume_payload(
        protocol_major: usize,
        transport_profile: usize,
        schema_revision: usize,
        game_session_id: &[u8],
        supported_capabilities: &[usize],
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        push_test_bytes_field(&mut payload, 1, game_session_id);
        push_test_bytes_field(&mut payload, 2, &[0xbb]);
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
        push_test_varint_field(
            &mut payload,
            8,
            TRANSPORT_PROFILE_TCP_TLS13_V1 as usize,
        );
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
            (
                1,
                test_client_bootstrap_payload(1, 1, 1, &short, &[]),
            ),
            (3, test_client_resume_payload(1, 1, 1, &nil, &[])),
            (
                2,
                test_server_accepted_payload(1, 1, 1, [&nil, &world, &channel], &[]),
            ),
            (
                4,
                test_server_resume_accepted_payload(2, 1, 1, &short, &[]),
            ),
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
        assert!(decode_wire_envelope(&test_envelope(
            1,
            &test_client_bootstrap_payload(1, 1, 1, &session, &[777])
        ))
        .is_ok());
        assert!(decode_wire_envelope(&test_envelope(
            3,
            &test_client_resume_payload(1, 1, 1, &session, &[777])
        ))
        .is_ok());
        assert_eq!(
            decode_wire_envelope(&test_envelope(
                2,
                &test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[1])
            )),
            Err(FoundationProtocolError::CapabilityMismatch)
        );
        assert_eq!(
            decode_wire_envelope(&test_envelope(
                4,
                &test_server_resume_accepted_payload(2, 1, 1, &session, &[1])
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

        let mut accepted =
            test_server_accepted_payload(1, 1, 1, [&session, &world, &channel], &[]);
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
        let mut payload = vec![0x3a];
        payload.extend(test_varint(build_128.len()));
        payload.extend_from_slice(&build_128);
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
        let mut payload = vec![0x22];
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
    fn nested_admission_material_limits_accept_16384_and_reject_16385()
    -> Result<(), FoundationProtocolError> {
        fn varint(mut value: usize) -> Vec<u8> {
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

        fn envelope(message_type: u8, material_key: u8, material_len: usize) -> Vec<u8> {
            let mut payload = vec![material_key];
            payload.extend(varint(material_len));
            payload.resize(payload.len() + material_len, 0);

            let mut envelope = vec![0x08, message_type, 0x22];
            envelope.extend(varint(payload.len()));
            envelope.extend(payload);
            envelope
        }

        for (message_type, material_key) in [(1u8, 0x2a), (3u8, 0x12)] {
            assert_eq!(
                decode_wire_envelope(&envelope(message_type, material_key, 16_384))?
                    .payload()
                    .len(),
                16_388
            );
            assert_eq!(
                decode_wire_envelope(&envelope(message_type, material_key, 16_385)),
                Err(FoundationProtocolError::BootstrapLimitExceeded)
            );
        }
        Ok(())
    }

    #[test]
    fn bootstrap_payload_limit_accepts_65536_and_rejects_65537()
    -> Result<(), FoundationProtocolError> {
        // The exact-boundary payload remains syntactically valid protobuf: field 16
        // is an additive unknown bytes field whose key+length+body total 65,536 bytes.
        let mut accepted_payload = vec![0x82, 0x01, 0xfb, 0xff, 0x03];
        accepted_payload.resize(65_536, 0);
        let mut accepted = vec![0x08, 0x01, 0x22, 0x80, 0x80, 0x04];
        accepted.extend_from_slice(&accepted_payload);
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
        let bytes = [0x08, 0x01, 0x22, 0x00, 0x80, 0x01, 0x01];
        let envelope = decode_wire_envelope(&bytes)?;
        assert_eq!(envelope.message_type(), MessageType::ClientBootstrap);
        assert_eq!(envelope.payload(), &[]);
        Ok(())
    }

    #[test]
    fn bootstrap_phase_requires_pre_admission_zero_generation()
    -> Result<(), FoundationProtocolError> {
        let with_generation = [0x08, 0x01, 0x10, 0x01, 0x22, 0x00];
        let envelope = decode_wire_envelope(&with_generation)?;
        assert_eq!(
            envelope.validate(Direction::ClientToServer, false),
            Err(FoundationProtocolError::MalformedEnvelope)
        );
        let zero_generation = [0x08, 0x01, 0x22, 0x00];
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
    fn server_sequence_gap_requests_resync_without_advancing() -> Result<(), FoundationProtocolError>
    {
        let mut sequence = ServerSequenceTracker::new();
        assert_eq!(sequence.observe(1)?, SequenceDecision::Apply);
        sequence.commit_applied(1)?;
        assert_eq!(sequence.observe(1)?, SequenceDecision::Duplicate);
        assert_eq!(
            sequence.observe(3),
            Err(FoundationProtocolError::ServerSequenceGap)
        );
        assert_eq!(sequence.next_expected(), Some(2));
        Ok(())
    }

    #[test]
    fn server_sequence_advances_only_after_payload_commit() -> Result<(), FoundationProtocolError> {
        let mut sequence = ServerSequenceTracker::new();
        assert_eq!(sequence.observe(1)?, SequenceDecision::Apply);
        assert_eq!(sequence.last_applied(), 0);
        assert_eq!(sequence.observe(1)?, SequenceDecision::Apply);
        sequence.commit_applied(1)?;
        assert_eq!(sequence.last_applied(), 1);
        assert_eq!(sequence.observe(1)?, SequenceDecision::Duplicate);
        Ok(())
    }

    #[test]
    fn snapshot_barrier_blocks_post_target_sequence_until_commit()
    -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(9, 2, 4, 12, 1)?;
        assert!(!barrier.may_emit_sequenced(13, 1));
        barrier.chunk(9, 0, &[1, 2], 1)?;
        barrier.chunk(9, 1, &[3, 4], 1)?;
        let committed = barrier.commit(9, 1)?;
        assert_eq!(committed.target_server_sequence(), 12);
        assert_eq!(committed.body(), &[1, 2, 3, 4]);
        assert!(barrier.may_emit_sequenced(13, 1));
        Ok(())
    }

    #[test]
    fn framed_envelope_rejects_truncation_and_oversized_prefix_before_body_access()
    -> Result<(), FoundationProtocolError> {
        let valid = [0, 0, 0, 4, 0x08, 0x01, 0x22, 0x00];
        assert_eq!(
            decode_framed_envelope(&valid)?.message_type(),
            MessageType::ClientBootstrap
        );
        assert_eq!(
            decode_framed_envelope(&[0, 0, 0, 5, 0x08, 0x01, 0x22, 0x00]),
            Err(FoundationProtocolError::MalformedFrame)
        );
        assert_eq!(
            decode_framed_envelope(&[0, 0x10, 0, 1]),
            Err(FoundationProtocolError::FrameTooLarge)
        );
        Ok(())
    }

    #[test]
    fn snapshot_revisions_and_server_sequence_never_roll_back()
    -> Result<(), FoundationProtocolError> {
        let mut revisions = StateRevisionTracker::new();
        revisions.apply_snapshot_revision(7, 5)?;
        assert_eq!(
            revisions.apply_snapshot_revision(7, 4),
            Err(FoundationProtocolError::StateRevisionMismatch)
        );
        assert_eq!(revisions.revision(7), Some(5));

        let mut sequence = ServerSequenceTracker::new();
        assert_eq!(sequence.observe(1)?, SequenceDecision::Apply);
        sequence.commit_applied(1)?;
        assert_eq!(sequence.observe(2)?, SequenceDecision::Apply);
        sequence.commit_applied(2)?;
        assert_eq!(
            sequence.apply_snapshot_boundary(1),
            Err(FoundationProtocolError::SnapshotAssemblyInvalid)
        );
        assert_eq!(sequence.last_applied(), 2);
        Ok(())
    }

    #[test]
    fn state_revision_mismatch_never_guesses_forward() -> Result<(), FoundationProtocolError> {
        let mut revisions = StateRevisionTracker::new();
        revisions.apply_delta(7, 0, 1)?;
        assert_eq!(
            revisions.apply_delta(7, 0, 2),
            Err(FoundationProtocolError::StateRevisionMismatch)
        );
        assert_eq!(revisions.revision(7), Some(1));
        Ok(())
    }

    #[test]
    fn resync_replays_only_when_contiguous_history_is_retained() {
        assert_eq!(plan_resync(5, 8, 6), ResyncPlan::ReplayFrom(6));
        assert_eq!(plan_resync(2, 8, 6), ResyncPlan::SnapshotRequired);
        assert_eq!(plan_resync(8, 8, 6), ResyncPlan::UpToDate);
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
    }

    #[test]
    fn oversized_snapshot_chunk_is_a_limit_error_and_discards_assembly()
    -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(7, 1, 524_289, 4, 1)?;
        let oversized = vec![0u8; 524_289];
        assert_eq!(
            barrier.chunk(7, 0, &oversized, 1),
            Err(FoundationProtocolError::SnapshotLimitExceeded)
        );
        assert!(!barrier.is_active());
        Ok(())
    }

    #[test]
    fn snapshot_ids_are_monotonic_across_commit_and_generation_change()
    -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(2, 0, 0, 4, 1)?;
        barrier.commit(2, 1)?;
        assert_eq!(
            barrier.begin(2, 0, 0, 4, 1),
            Err(FoundationProtocolError::SnapshotAssemblyInvalid)
        );
        assert_eq!(
            barrier.begin(1, 0, 0, 4, 2),
            Err(FoundationProtocolError::SnapshotAssemblyInvalid)
        );
        barrier.begin(3, 1, 1, 5, 2)?;
        assert_eq!(
            barrier.chunk(3, 0, &[7], 3),
            Err(FoundationProtocolError::StaleConnectionGeneration)
        );
        assert_eq!(
            barrier.begin(2, 0, 0, 5, 3),
            Err(FoundationProtocolError::SnapshotAssemblyInvalid)
        );
        barrier.begin(4, 0, 0, 5, 3)?;
        Ok(())
    }

    #[test]
    fn generation_change_discards_partial_snapshot() -> Result<(), FoundationProtocolError> {
        let mut barrier = SnapshotBarrier::new();
        barrier.begin(1, 1, 1, 4, 1)?;
        assert_eq!(
            barrier.chunk(1, 0, &[7], 2),
            Err(FoundationProtocolError::StaleConnectionGeneration)
        );
        assert!(!barrier.is_active());
        Ok(())
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
        let mut wrong_version = valid;
        wrong_version[6] = 0x40;
        assert_eq!(
            GameSessionId::decode(&wrong_version),
            Err(FoundationProtocolError::InvalidWireIdentifier)
        );
        let mut wrong_variant = valid;
        wrong_variant[8] = 0x00;
        assert_eq!(
            GameSessionId::decode(&wrong_variant),
            Err(FoundationProtocolError::InvalidWireIdentifier)
        );
        Ok(())
    }
}
