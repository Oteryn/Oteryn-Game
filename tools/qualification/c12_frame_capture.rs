//! C12's #3521 envelopes describe the original frames fed to the native session decoder.

use oteryn_protocol_oteryn::{
    FrameLength, GameSessionId, MessageType, decode_command_result, decode_framed_envelope,
    decode_server_accepted, decode_state_delta,
};

pub const FRAME_LIMIT: usize = 1024 * 1024;

/// External admission input, carried beside the transcript without modifying any frame.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDictionaryPin {
    pub generation: [u8; 32],
    pub count: u32,
}

impl ItemDictionaryPin {
    pub fn sorted_refs(&self) -> Result<Vec<u32>, &'static str> {
        if !(1..=100_000).contains(&self.count) {
            return Err("C12 Item dictionary count outside bounds");
        }
        Ok((1..=self.count).collect())
    }
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

pub fn envelopes(
    mut wire: &[u8],
    session: GameSessionId,
) -> Result<Vec<Envelope>, Box<dyn std::error::Error + Send + Sync>> {
    if wire.is_empty() || wire.len() > FRAME_LIMIT {
        return Err("C12 wire capture size outside bounds".into());
    }
    let hex = format!("{:032x}", u128::from_be_bytes(*session.as_bytes()));
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..],
    );
    let mut result = Vec::new();
    while !wire.is_empty() {
        let prefix = wire.get(..4).ok_or("C12 truncated frame prefix")?;
        let size = FrameLength::from_prefix(prefix)?.get() as usize;
        let (frame, rest) = wire
            .split_at_checked(4 + size)
            .ok_or("C12 truncated original frame")?;
        let frame = decode_framed_envelope(frame)?;
        if result.is_empty() {
            if frame.message_type() != MessageType::ServerAccepted
                || decode_server_accepted(frame.payload())?.game_session_id != session
            {
                return Err("C12 original admission does not match capture session".into());
            }
        } else if frame.message_type() == MessageType::ServerAccepted {
            return Err("C12 wire contains a second admission".into());
        }
        // Snapshot and push frames carry no CommandResult ID or status on the wire.
        let (command_id, status, payload) = match frame.message_type() {
            MessageType::CommandResult => {
                let command = decode_command_result(frame.payload())?;
                (command.command_id, command.status as u32, command.payload)
            }
            MessageType::StateDelta => (0, 0, decode_state_delta(frame.payload())?.payload),
            _ => (0, 0, frame.payload()),
        };
        result.push(Envelope {
            command_id,
            game_session_id: uuid.clone(),
            payload: payload.to_vec(),
            server_sequence: frame.server_sequence(),
            status,
        });
        wire = rest;
    }
    Ok(result)
}
