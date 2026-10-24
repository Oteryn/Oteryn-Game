//! GM-1 carries off-host signatures; a negotiated capability grants no authority.
use crate::FoundationProtocolError;
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint, set_once};
pub const CAPABILITY_STAFF_V1: u32 = 51;
pub const COMMAND_TYPE_STAFF_REQUEST: u32 = 60;
pub const COMMAND_TYPE_STAFF_RECEIPT: u32 = 61;
pub const MAX_SIGNED_BYTES: usize = 8192;
pub const MAX_SIGNATURE_BYTES: usize = 4096;
pub const MAX_PRINCIPAL_BYTES: usize = 64;
pub const MAX_ENVELOPE_BYTES: usize = 12_364;
pub const MAX_RESULT_BYTES: usize = 512;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedEnvelope {
    pub principal: String,
    pub canonical: Vec<u8>,
    pub signature: Vec<u8>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum StaffCode {
    Ok = 1,
    RoleRevoked = 2,
    TierInsufficient = 3,
    TargetNotFound = 4,
    TargetNotInChannel = 5,
    Stale = 6,
    AlreadyReleased = 7,
    RateLimited = 8,
}
impl StaffCode {
    fn decode(value: u64) -> Result<Self> {
        Ok(match value {
            1 => Self::Ok,
            2 => Self::RoleRevoked,
            3 => Self::TierInsufficient,
            4 => Self::TargetNotFound,
            5 => Self::TargetNotInChannel,
            6 => Self::Stale,
            7 => Self::AlreadyReleased,
            8 => Self::RateLimited,
            _ => return Err(bad()),
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffResult {
    pub code: StaffCode,
    /// 1 audited read/mutation, 2 audited refusal, 3 stored signed receipt.
    pub disposition: u64,
    pub control_seq: u64,
    pub link_sha256: Vec<u8>,
    /// Inspect projects only the sanction, never private Character data.
    pub mute_until_ms: u64,
}
type Result<T> = std::result::Result<T, FoundationProtocolError>;
fn bad() -> FoundationProtocolError {
    FoundationProtocolError::MalformedEnvelope
}
fn valid(e: &SignedEnvelope) -> bool {
    !e.principal.is_empty()
        && e.principal.len() <= MAX_PRINCIPAL_BYTES
        && e.principal
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.@".contains(&c))
        && !e.canonical.is_empty()
        && e.canonical.len() <= MAX_SIGNED_BYTES
        && !e.signature.is_empty()
        && e.signature.len() <= MAX_SIGNATURE_BYTES
}
pub fn encode_envelope(e: &SignedEnvelope) -> Result<Vec<u8>> {
    if !valid(e) {
        return Err(bad());
    }
    let mut out = Vec::new();
    push_message_field(&mut out, 1, e.principal.as_bytes());
    push_message_field(&mut out, 2, &e.canonical);
    push_message_field(&mut out, 3, &e.signature);
    Ok(out)
}
pub fn decode_envelope(input: &[u8]) -> Result<SignedEnvelope> {
    if input.len() > MAX_ENVELOPE_BYTES {
        return Err(bad());
    }
    let (mut cursor, mut principal, mut canonical, mut signature) = (0, None, None, None);
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor).map_err(|_| bad())?;
        let bytes = read_bytes(input, &mut cursor).map_err(|_| bad())?;
        match key {
            10 if !bytes.is_empty() && bytes.len() <= MAX_PRINCIPAL_BYTES => set_once(
                &mut principal,
                std::str::from_utf8(bytes).map_err(|_| bad())?.to_owned(),
            ),
            18 if !bytes.is_empty() && bytes.len() <= MAX_SIGNED_BYTES => {
                set_once(&mut canonical, bytes.to_vec())
            }
            26 if !bytes.is_empty() && bytes.len() <= MAX_SIGNATURE_BYTES => {
                set_once(&mut signature, bytes.to_vec())
            }
            _ => return Err(bad()),
        }
        .map_err(|_| bad())?;
    }
    let result = SignedEnvelope {
        principal: principal.ok_or_else(bad)?,
        canonical: canonical.ok_or_else(bad)?,
        signature: signature.ok_or_else(bad)?,
    };
    if !valid(&result) || encode_envelope(&result)?.as_slice() != input {
        return Err(bad());
    }
    Ok(result)
}
pub fn encode_result(r: &StaffResult) -> Result<Vec<u8>> {
    if !(1..=3).contains(&r.disposition)
        || r.control_seq == 0
        || r.link_sha256.len() != 32
        || (r.disposition == 2) == (r.code == StaffCode::Ok)
        || (r.disposition != 1 && r.mute_until_ms != 0)
    {
        return Err(bad());
    }
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, r.disposition);
    push_varint_field(&mut out, 2, r.control_seq);
    push_message_field(&mut out, 3, &r.link_sha256);
    push_varint_field(&mut out, 4, r.mute_until_ms);
    push_varint_field(&mut out, 5, r.code as u64);
    Ok(out)
}
pub fn decode_result(input: &[u8]) -> Result<StaffResult> {
    if input.len() > MAX_RESULT_BYTES {
        return Err(bad());
    }
    let (mut at, mut disposition, mut seq, mut digest, mut until, mut code) =
        (0, None, None, None, None, None);
    while at < input.len() {
        match read_varint(input, &mut at).map_err(|_| bad())? {
            8 => set_once(
                &mut disposition,
                read_varint(input, &mut at).map_err(|_| bad())?,
            ),
            16 => set_once(&mut seq, read_varint(input, &mut at).map_err(|_| bad())?),
            26 => set_once(
                &mut digest,
                read_bytes(input, &mut at).map_err(|_| bad())?.to_vec(),
            ),
            32 => set_once(&mut until, read_varint(input, &mut at).map_err(|_| bad())?),
            40 => set_once(&mut code, read_varint(input, &mut at).map_err(|_| bad())?),
            _ => return Err(bad()),
        }
        .map_err(|_| bad())?;
    }
    let r = StaffResult {
        code: StaffCode::decode(code.ok_or_else(bad)?)?,
        disposition: disposition.ok_or_else(bad)?,
        control_seq: seq.ok_or_else(bad)?,
        link_sha256: digest.ok_or_else(bad)?,
        mute_until_ms: until.unwrap_or(0),
    };
    encode_result(&r)?;
    Ok(r)
}
#[cfg(test)]
#[path = "staff_tests.rs"]
mod tests;
