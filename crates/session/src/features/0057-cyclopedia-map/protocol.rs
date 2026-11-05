//! Adapter for the existing LOOP-MAP-1 fixed request layout.
pub use oteryn_protocol_oteryn::cyclopedia::*;

pub fn request(operation: u8, amount: u32) -> Result<(u32, Vec<u8>), MapWireError> {
    let (kind, payload) = if operation == 0 && amount == 0 {
        (COMMAND_TYPE_CYCLOPEDIA_QUERY, Vec::new())
    } else {
        let mut bytes = vec![operation];
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&amount.to_le_bytes());
        (COMMAND_TYPE_CYCLOPEDIA_INTENT, bytes)
    };
    decode_request(kind, &payload)?;
    Ok((kind, payload))
}
