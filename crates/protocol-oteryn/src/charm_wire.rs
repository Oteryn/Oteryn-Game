//! Proto3 wire helpers shared by the Bestiary and Charm payload codecs
//! (`docs/contracts/protocol-oteryn/CHARM5_BESTIARY_CHARM_WIRE_PROPOSAL_V1.md`).
//!
//! The same strict subset as `actor_spell`: varint scalars, length-delimited submessages, no
//! packed repeated scalars, and every field at most once except the declared repeated message
//! field. Anything else fails closed.

/// Why a Bestiary or Charm payload was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CyclopediaWireError {
    /// Not a valid encoding of the schema, or a value outside its declared domain: a zero or
    /// unknown enum, an index of zero, entries out of order or repeated, a stage or kill count
    /// that contradicts the thresholds or the cost, an unknown or repeated field.
    Malformed,
    /// The payload is over its byte bound, a repeated field has more entries than its registered
    /// resource limit, or a value is above its registered bound.
    LimitExceeded,
}

pub(crate) type WireResult<T> = Result<T, CyclopediaWireError>;

pub(crate) fn push_varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

pub(crate) fn push_varint_field(output: &mut Vec<u8>, field: u64, value: u64) {
    push_varint(output, field << 3);
    push_varint(output, value);
}

/// Omits the field at its proto3 default (0), as a standard encoder does.
pub(crate) fn push_nonzero_varint_field(output: &mut Vec<u8>, field: u64, value: u64) {
    if value != 0 {
        push_varint_field(output, field, value);
    }
}

pub(crate) fn push_message_field(output: &mut Vec<u8>, field: u64, message: &[u8]) {
    push_varint(output, (field << 3) | 2);
    push_varint(output, message.len() as u64);
    output.extend_from_slice(message);
}

pub(crate) fn read_varint(input: &[u8], cursor: &mut usize) -> WireResult<u64> {
    let mut value = 0_u64;
    for shift in (0..70).step_by(7) {
        let byte = *input.get(*cursor).ok_or(CyclopediaWireError::Malformed)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err(CyclopediaWireError::Malformed);
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(CyclopediaWireError::Malformed)
}

pub(crate) fn read_uint32(input: &[u8], cursor: &mut usize) -> WireResult<u32> {
    u32::try_from(read_varint(input, cursor)?).map_err(|_| CyclopediaWireError::Malformed)
}

pub(crate) fn read_bytes<'a>(input: &'a [u8], cursor: &mut usize) -> WireResult<&'a [u8]> {
    let len =
        usize::try_from(read_varint(input, cursor)?).map_err(|_| CyclopediaWireError::Malformed)?;
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= input.len())
        .ok_or(CyclopediaWireError::Malformed)?;
    let value = &input[*cursor..end];
    *cursor = end;
    Ok(value)
}

/// Stores a singular field once; a repeated field fails closed.
pub(crate) fn set_once<T>(slot: &mut Option<T>, value: T) -> WireResult<()> {
    if slot.is_some() {
        return Err(CyclopediaWireError::Malformed);
    }
    *slot = Some(value);
    Ok(())
}

/// Reads a message whose fields `1..=N` are all singular uint32 varints, each at most once. An
/// omitted field reads as 0 (its proto3 default). Any other key fails closed.
pub(crate) fn read_uint32_fields<const N: usize>(input: &[u8]) -> WireResult<[u32; N]> {
    let mut cursor = 0;
    let mut fields = [None; N];
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        if key & 0x07 != 0 {
            return Err(CyclopediaWireError::Malformed);
        }
        let index = usize::try_from(key >> 3)
            .ok()
            .and_then(|field| field.checked_sub(1))
            .ok_or(CyclopediaWireError::Malformed)?;
        let slot = fields
            .get_mut(index)
            .ok_or(CyclopediaWireError::Malformed)?;
        let value = read_uint32(input, &mut cursor)?;
        set_once(slot, value)?;
    }
    Ok(fields.map(|field| field.unwrap_or(0)))
}

/// Reads a result payload: exactly one non-zero enum in field 1, within `maximum` bytes.
pub(crate) fn read_result_enum(input: &[u8], maximum: usize) -> WireResult<u32> {
    if input.len() > maximum {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let [value] = read_uint32_fields::<1>(input)?;
    if value == 0 {
        return Err(CyclopediaWireError::Malformed);
    }
    Ok(value)
}
