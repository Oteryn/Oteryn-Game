//! NX-1 donor staff tools. Negotiation carries signed bytes; it grants no staff rights.
use crate::FoundationProtocolError;
pub use crate::FoundationProtocolError as StaffToolsWireError;
use crate::charm_wire::{push_message_field, read_bytes, read_varint, set_once};
use crate::staff::{self, StaffCode, StaffResult};
pub use crate::staff::{SignedEnvelope, decode_envelope, encode_envelope};

pub const CAPABILITY_STAFF_TOOLS_V1: u32 = 330;
pub const COMMAND_TYPE_STAFF_TOOLS_REQUEST: u32 = 560;
pub const COMMAND_TYPE_STAFF_TOOLS_RECEIPT: u32 = 561;
pub const MAX_INFO_BYTES: usize = 4096;
pub const MAX_REPLY_BYTES: usize = staff::MAX_RESULT_BYTES + MAX_INFO_BYTES + 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffToolsReply {
    pub result: StaffResult,
    /// Bounded public staff inspection/progress text; never a signing key or player IP.
    pub info: String,
}
fn bad() -> FoundationProtocolError {
    FoundationProtocolError::MalformedEnvelope
}
pub fn encode_reply(reply: &StaffToolsReply) -> Result<Vec<u8>, StaffToolsWireError> {
    if reply.info.len() > MAX_INFO_BYTES
        || (!reply.info.is_empty()
            && (reply.result.disposition != 1 || reply.result.code != StaffCode::Ok))
    {
        return Err(bad());
    }
    let result = staff::encode_result(&reply.result)?;
    let mut bytes = Vec::new();
    push_message_field(&mut bytes, 1, &result);
    if !reply.info.is_empty() {
        push_message_field(&mut bytes, 2, reply.info.as_bytes());
    }
    Ok(bytes)
}
pub fn decode_reply(bytes: &[u8]) -> Result<StaffToolsReply, StaffToolsWireError> {
    if bytes.len() > MAX_REPLY_BYTES {
        return Err(bad());
    }
    let (mut cursor, mut result, mut info) = (0, None, None);
    while cursor < bytes.len() {
        match read_varint(bytes, &mut cursor).map_err(|_| bad())? {
            10 => {
                let value = read_bytes(bytes, &mut cursor).map_err(|_| bad())?;
                set_once(&mut result, staff::decode_result(value)?).map_err(|_| bad())?;
            }
            18 => {
                let value = read_bytes(bytes, &mut cursor).map_err(|_| bad())?;
                if value.len() > MAX_INFO_BYTES {
                    return Err(bad());
                }
                set_once(
                    &mut info,
                    std::str::from_utf8(value).map_err(|_| bad())?.to_owned(),
                )
                .map_err(|_| bad())?;
            }
            _ => return Err(bad()),
        }
    }
    let reply = StaffToolsReply {
        result: result.ok_or_else(bad)?,
        info: info.unwrap_or_default(),
    };
    encode_reply(&reply)?;
    Ok(reply)
}

#[cfg(test)]
#[path = "staff_tools_tests.rs"]
mod tests;
