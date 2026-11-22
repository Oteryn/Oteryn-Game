//! One pre-admission World lookup on a dedicated login probe connection.
use serde::{Deserialize, Serialize};

pub const COMMAND_TYPE: u32 = 6326;
pub const MAX_REPLY_BYTES: usize = 4096;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub world_id: [u8; 16],
    pub info: Option<WorldInfo>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldInfo {
    pub players_online: u32,
    pub boost: Option<Boost>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Boost {
    pub reset_epoch: u64,
    pub effective_from: i64,
    pub effective_until: i64,
    pub creature: Race,
    pub boss: Race,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Race {
    pub key: String,
    pub name: String,
    pub artwork: Artwork,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", deny_unknown_fields)]
pub enum Artwork {
    Outfit { look_type: u32 },
    Item { look_type: u32 },
}

fn frame(message: crate::MessageType, body: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    crate::push_scalar(&mut frame, 1, message as u64);
    crate::push_bytes(&mut frame, 4, body);
    frame
}

fn payload(bytes: &[u8], max: usize, message: crate::MessageType) -> Option<&[u8]> {
    let mut cursor = 0;
    if crate::read_varint(bytes, &mut cursor).ok()? != 8
        || crate::read_varint(bytes, &mut cursor).ok()? != message as u64
        || crate::read_varint(bytes, &mut cursor).ok()? != 34
    {
        return None;
    }
    let payload = crate::bounded_length_delimited(
        bytes,
        &mut cursor,
        max,
        crate::FoundationProtocolError::BootstrapLimitExceeded,
    )
    .ok()?;
    (cursor == bytes.len()).then_some(payload)
}

pub fn encode_request(world_id: [u8; 16]) -> Vec<u8> {
    let mut body = Vec::new();
    crate::push_scalar(&mut body, 1, 1);
    crate::push_scalar(&mut body, 2, u64::from(COMMAND_TYPE));
    crate::push_bytes(&mut body, 4, &world_id);
    frame(crate::MessageType::ClientCommand, &body)
}

pub fn decode_request(bytes: &[u8]) -> Option<[u8; 16]> {
    let body = payload(bytes, 24, crate::MessageType::ClientCommand)?;
    let mut cursor = 0;
    if crate::read_varint(body, &mut cursor).ok()? != 8
        || crate::read_varint(body, &mut cursor).ok()? != 1
        || crate::read_varint(body, &mut cursor).ok()? != 16
        || crate::read_varint(body, &mut cursor).ok()? != u64::from(COMMAND_TYPE)
        || crate::read_varint(body, &mut cursor).ok()? != 34
        || crate::read_varint(body, &mut cursor).ok()? != 16
    {
        return None;
    }
    let world = <[u8; 16]>::try_from(body.get(cursor..)?).ok()?;
    (world != [0; 16]).then_some(world)
}

pub fn encode_reply(reply: &Reply) -> Option<Vec<u8>> {
    let bytes = serde_json::to_vec(reply).ok()?;
    if bytes.len() > MAX_REPLY_BYTES {
        return None;
    }
    let mut body = Vec::new();
    crate::push_scalar(&mut body, 1, 1);
    crate::push_scalar(&mut body, 2, crate::CommandStatus::Accepted as u64);
    crate::push_bytes(&mut body, 5, &bytes);
    Some(frame(crate::MessageType::CommandResult, &body))
}

pub fn decode_reply(bytes: &[u8]) -> Option<Reply> {
    let body = payload(
        bytes,
        MAX_REPLY_BYTES + 7,
        crate::MessageType::CommandResult,
    )?;
    let result = crate::decode_command_result(body).ok()?;
    if result.command_id != 1
        || result.status != crate::CommandStatus::Accepted
        || result.error_code != 0
        || result.payload.len() > MAX_REPLY_BYTES
    {
        return None;
    }
    serde_json::from_slice(result.payload).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_probe_is_bounded_and_outside_the_player_envelope() {
        let mut request = encode_request([1; 16]);
        assert_eq!(decode_request(&request), Some([1; 16]));
        let envelope = crate::decode_wire_envelope(&request);
        assert!(envelope.is_ok_and(|envelope| envelope.client_command(1).is_err()));
        request.push(0);
        assert_eq!(decode_request(&request), None);
        assert_eq!(decode_request(&encode_request([0; 16])), None);
    }
}
