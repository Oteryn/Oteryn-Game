//! `FIRST-CONTROL-WIRE-V1` typed payloads (#642 owner acceptance 5853424280; schema in
//! `docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto`). Command type 1
//! `WORLD_ACTOR_STEP_INTENT` and state domain 1 `WORLD_SPATIAL_VISIBILITY` with delta type 1 and
//! snapshot type 1. Decoding is strict: zero or unknown enum values, unknown or repeated fields and
//! over-bound payloads fail closed.

// The client-side codecs (intent encode, result and world-spatial decode) are exercised by the
// round-trip tests and the qualification client; the server composes only its own direction.
#![cfg_attr(not(test), allow(dead_code))]

pub const COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT: u32 = 1;
pub const STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY: u32 = 1;
pub const DELTA_TYPE_WORLD_SPATIAL_V1: u32 = 1;
pub const SNAPSHOT_TYPE_WORLD_SPATIAL_V1: u32 = 1;
/// Capability 13 `PACED_MOVEMENT_V1` (SPEED-1, CONDITIONS-0 §4.3): extends the command type 1
/// result with [`StepDisposition::TooEarly`] and the `step_wait_ms` pacing receipt
/// (STEP-PACE-WIRE-0), and owns no command type or domain.
pub const CAPABILITY_PACED_MOVEMENT_V1: u32 = 13;

/// First-child semantic bounds (#642 packet 5853255180).
pub const MAX_STEP_INTENT_BYTES: usize = 4;
/// The command type 1 result bound of a session without capability 13.
pub const MAX_STEP_RESULT_BYTES: usize = 4;
/// The command type 1 result bound under capability 13 (the registry `max_result_payload_bytes`):
/// the disposition (2 B) and `step_wait_ms` (tag 1 B, varint at most 3 B), with slack.
pub const MAX_PACED_STEP_RESULT_BYTES: usize = 8;
/// The largest `step_wait_ms`: the step-speed table maximum (speed 10, ground speed 1000).
pub const MAX_STEP_WAIT_MS: u32 = 111_150;
/// A `MOVED` step duration is a whole number of server beats.
pub const STEP_WAIT_BEAT_MS: u32 = 50;
pub const MAX_WORLD_SPATIAL_PAYLOAD_BYTES: usize = 64;
pub(crate) const CONTENT_GENERATION_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSpatialError {
    Malformed,
    LimitExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDirection {
    North = 1,
    East = 2,
    South = 3,
    West = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDisposition {
    Moved = 1,
    Blocked = 2,
    Rejected = 3,
    /// A second early step while one waits in the pacing buffer; only under capability 13.
    TooEarly = 4,
}

/// The command type 1 result with its capability 13 pacing receipt (STEP-PACE-WIRE-0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepResult {
    pub disposition: StepDisposition,
    /// Field 2; 0 when absent (no receipt). On `Moved` the committed step duration, a non-zero
    /// multiple of [`STEP_WAIT_BEAT_MS`]; on `TooEarly` the milliseconds left until the server
    /// deadline; never on `Blocked` or `Rejected`. At most [`MAX_STEP_WAIT_MS`].
    pub step_wait_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorPosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// The own-actor observation carried by both the delta and the snapshot of domain 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSpatialObservation {
    pub content_generation: [u8; 32],
    pub actor_position: ActorPosition,
}

pub(crate) fn push_varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

pub(crate) fn push_tag(output: &mut Vec<u8>, field: u64, wire: u64) {
    push_varint(output, (field << 3) | wire);
}

fn push_sint32(output: &mut Vec<u8>, field: u64, value: i32) {
    let zigzag = ((value << 1) ^ (value >> 31)) as u32;
    if zigzag != 0 {
        push_tag(output, field, 0);
        push_varint(output, u64::from(zigzag));
    }
}

pub(crate) fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, WorldSpatialError> {
    let mut value = 0_u64;
    for shift in (0..70).step_by(7) {
        let byte = *input.get(*cursor).ok_or(WorldSpatialError::Malformed)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err(WorldSpatialError::Malformed);
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(WorldSpatialError::Malformed)
}

pub(crate) fn read_bytes<'a>(
    input: &'a [u8],
    cursor: &mut usize,
) -> Result<&'a [u8], WorldSpatialError> {
    let len =
        usize::try_from(read_varint(input, cursor)?).map_err(|_| WorldSpatialError::Malformed)?;
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= input.len())
        .ok_or(WorldSpatialError::Malformed)?;
    let value = &input[*cursor..end];
    *cursor = end;
    Ok(value)
}

/// Reads exactly one singular enum field 1 (the only field of the step intent).
fn read_single_enum(input: &[u8], maximum: usize) -> Result<u64, WorldSpatialError> {
    if input.len() > maximum {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let mut cursor = 0;
    let mut value = None;
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        if key != (1 << 3) || value.is_some() {
            return Err(WorldSpatialError::Malformed);
        }
        value = Some(read_varint(input, &mut cursor)?);
    }
    value.ok_or(WorldSpatialError::Malformed)
}

pub fn decode_step_intent(payload: &[u8]) -> Result<StepDirection, WorldSpatialError> {
    match read_single_enum(payload, MAX_STEP_INTENT_BYTES)? {
        1 => Ok(StepDirection::North),
        2 => Ok(StepDirection::East),
        3 => Ok(StepDirection::South),
        4 => Ok(StepDirection::West),
        _ => Err(WorldSpatialError::Malformed),
    }
}

pub fn encode_step_intent(direction: StepDirection) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_tag(&mut output, 1, 0);
    push_varint(&mut output, direction as u64);
    output
}

/// The command type 1 result without a pacing receipt (field 2 absent).
pub fn encode_step_result(disposition: StepDisposition) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_tag(&mut output, 1, 0);
    push_varint(&mut output, disposition as u64);
    output
}

/// Whether `result` may be sent to a session; `paced` is whether it selected capability 13.
fn check_step_result(result: StepResult, paced: bool) -> Result<(), WorldSpatialError> {
    let wait = result.step_wait_ms;
    let valid = match result.disposition {
        StepDisposition::Blocked | StepDisposition::Rejected => wait == 0,
        StepDisposition::Moved => {
            wait == 0
                || (paced && wait <= MAX_STEP_WAIT_MS && wait.is_multiple_of(STEP_WAIT_BEAT_MS))
        }
        StepDisposition::TooEarly => paced && wait <= MAX_STEP_WAIT_MS,
    };
    if valid {
        Ok(())
    } else {
        Err(WorldSpatialError::Malformed)
    }
}

/// The command type 1 result with its pacing receipt (STEP-PACE-WIRE-0). `paced` is whether the
/// session selected capability 13 `PACED_MOVEMENT_V1`; a receipt or `TOO_EARLY` without it, a
/// receipt on `BLOCKED` or `REJECTED`, a `MOVED` duration off the 50 ms beat and any value above
/// [`MAX_STEP_WAIT_MS`] are refused. A zero `step_wait_ms` is not written.
pub fn encode_step_result_paced(
    result: StepResult,
    paced: bool,
) -> Result<Vec<u8>, WorldSpatialError> {
    check_step_result(result, paced)?;
    let mut output = encode_step_result(result.disposition);
    if result.step_wait_ms != 0 {
        push_tag(&mut output, 2, 0);
        push_varint(&mut output, u64::from(result.step_wait_ms));
    }
    Ok(output)
}

/// The command type 1 result of a session without capability 13: `TOO_EARLY` and field 2 fail
/// closed.
pub fn decode_step_result(payload: &[u8]) -> Result<StepDisposition, WorldSpatialError> {
    decode_step_result_paced(payload, false)
}

/// The command type 1 result's disposition; `paced` is whether the session selected capability
/// 13 `PACED_MOVEMENT_V1`, the only case in which `TOO_EARLY` (4) and `step_wait_ms` are
/// admitted. See [`decode_step_result_receipt`] for the receipt itself.
pub fn decode_step_result_paced(
    payload: &[u8],
    paced: bool,
) -> Result<StepDisposition, WorldSpatialError> {
    decode_step_result_receipt(payload, paced).map(|result| result.disposition)
}

/// The command type 1 result with its pacing receipt. Without capability 13 (`paced` false) the
/// bound is [`MAX_STEP_RESULT_BYTES`] and field 2 fails closed; with it the bound is
/// [`MAX_PACED_STEP_RESULT_BYTES`]. An explicit zero, a repeated field, a receipt on `BLOCKED` or
/// `REJECTED`, a `MOVED` duration off the 50 ms beat and a value above [`MAX_STEP_WAIT_MS`] fail
/// closed.
pub fn decode_step_result_receipt(
    payload: &[u8],
    paced: bool,
) -> Result<StepResult, WorldSpatialError> {
    let maximum = if paced {
        MAX_PACED_STEP_RESULT_BYTES
    } else {
        MAX_STEP_RESULT_BYTES
    };
    if payload.len() > maximum {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut disposition, mut wait) = (None, None);
    while cursor < payload.len() {
        let slot = match read_varint(payload, &mut cursor)? {
            0x08 => &mut disposition,
            0x10 if paced => &mut wait,
            _ => return Err(WorldSpatialError::Malformed),
        };
        if slot.is_some() {
            return Err(WorldSpatialError::Malformed);
        }
        *slot = Some(read_varint(payload, &mut cursor)?);
    }
    let disposition = match disposition.ok_or(WorldSpatialError::Malformed)? {
        1 => StepDisposition::Moved,
        2 => StepDisposition::Blocked,
        3 => StepDisposition::Rejected,
        4 if paced => StepDisposition::TooEarly,
        _ => return Err(WorldSpatialError::Malformed),
    };
    let step_wait_ms = match wait {
        None => 0,
        Some(0) => return Err(WorldSpatialError::Malformed),
        Some(value) => u32::try_from(value).map_err(|_| WorldSpatialError::Malformed)?,
    };
    let result = StepResult {
        disposition,
        step_wait_ms,
    };
    check_step_result(result, paced)?;
    Ok(result)
}

pub(crate) fn encode_position(position: &ActorPosition) -> Vec<u8> {
    let mut output = Vec::with_capacity(18);
    push_sint32(&mut output, 1, position.x);
    push_sint32(&mut output, 2, position.y);
    push_sint32(&mut output, 3, i32::from(position.floor));
    output
}

/// Encodes the domain-1 delta or snapshot payload (identical schemas, distinct registered types).
pub fn encode_world_spatial(observation: &WorldSpatialObservation) -> Vec<u8> {
    let position = encode_position(&observation.actor_position);
    let mut output = Vec::with_capacity(CONTENT_GENERATION_BYTES + position.len() + 6);
    push_tag(&mut output, 1, 2);
    push_varint(&mut output, CONTENT_GENERATION_BYTES as u64);
    output.extend_from_slice(&observation.content_generation);
    push_tag(&mut output, 2, 2);
    push_varint(&mut output, position.len() as u64);
    output.extend_from_slice(&position);
    output
}

fn decode_sint32(value: u64) -> Result<i32, WorldSpatialError> {
    let zigzag = u32::try_from(value).map_err(|_| WorldSpatialError::Malformed)?;
    Ok(((zigzag >> 1) as i32) ^ -((zigzag & 1) as i32))
}

pub(crate) fn decode_position(input: &[u8]) -> Result<ActorPosition, WorldSpatialError> {
    let mut cursor = 0;
    let (mut x, mut y, mut floor) = (None, None, None);
    while cursor < input.len() {
        let key = read_varint(input, &mut cursor)?;
        let slot = match key {
            0x08 => &mut x,
            0x10 => &mut y,
            0x18 => &mut floor,
            _ => return Err(WorldSpatialError::Malformed),
        };
        if slot.is_some() {
            return Err(WorldSpatialError::Malformed);
        }
        *slot = Some(decode_sint32(read_varint(input, &mut cursor)?)?);
    }
    let floor = i16::try_from(floor.unwrap_or(0)).map_err(|_| WorldSpatialError::Malformed)?;
    Ok(ActorPosition {
        x: x.unwrap_or(0),
        y: y.unwrap_or(0),
        floor,
    })
}

pub fn decode_world_spatial(payload: &[u8]) -> Result<WorldSpatialObservation, WorldSpatialError> {
    if payload.len() > MAX_WORLD_SPATIAL_PAYLOAD_BYTES {
        return Err(WorldSpatialError::LimitExceeded);
    }
    let mut cursor = 0;
    let (mut generation, mut position) = (None, None);
    while cursor < payload.len() {
        let key = read_varint(payload, &mut cursor)?;
        match key {
            0x0a if generation.is_none() => {
                let bytes: [u8; 32] = read_bytes(payload, &mut cursor)?
                    .try_into()
                    .map_err(|_| WorldSpatialError::Malformed)?;
                generation = Some(bytes);
            }
            0x12 if position.is_none() => {
                position = Some(decode_position(read_bytes(payload, &mut cursor)?)?);
            }
            _ => return Err(WorldSpatialError::Malformed),
        }
    }
    match (generation, position) {
        (Some(content_generation), Some(actor_position)) => Ok(WorldSpatialObservation {
            content_generation,
            actor_position,
        }),
        _ => Err(WorldSpatialError::Malformed),
    }
}

/// Server side (SPEED-1, CONDITIONS-0 §4.3; STEP-PACE-WIRE-1): the command type 1 result one
/// session receives. `TOO_EARLY` and the `step_wait_ms` pacing receipt (field 2) reach only a
/// session that selected capability 13 `PACED_MOVEMENT_V1`; every other session gets `REJECTED`
/// for the same refusal and the exact receipt-less bytes it always got. `step_wait_ms` is 0 for
/// no receipt; a receipt outside the STEP-PACE-WIRE-0 contract (on `BLOCKED` or `REJECTED`, a
/// `MOVED` duration off the 50 ms beat, above [`MAX_STEP_WAIT_MS`]) is dropped, never sent.
pub fn encode_step_outcome(
    selected_capabilities: &[u32],
    disposition: StepDisposition,
    step_wait_ms: u32,
) -> Vec<u8> {
    if !selected_capabilities.contains(&CAPABILITY_PACED_MOVEMENT_V1) {
        return encode_step_result(match disposition {
            StepDisposition::TooEarly => StepDisposition::Rejected,
            disposition => disposition,
        });
    }
    let result = StepResult {
        disposition,
        step_wait_ms,
    };
    encode_step_result_paced(result, true).unwrap_or_else(|_| encode_step_result(disposition))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::Value;

    const PROTOCOL_REGISTRY: &str = crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON;
    const RESOURCE_REGISTRY: &str = crate::RESOURCE_LIMITS_REGISTRY_JSON;

    #[test]
    fn too_early_is_sent_only_to_a_session_with_capability_13() {
        let paced = [CAPABILITY_PACED_MOVEMENT_V1];
        assert_eq!(
            decode_step_result_paced(
                &encode_step_outcome(&paced, StepDisposition::TooEarly, 0),
                true
            ),
            Ok(StepDisposition::TooEarly)
        );
        assert_eq!(
            decode_step_result(&encode_step_outcome(&[], StepDisposition::TooEarly, 0)),
            Ok(StepDisposition::Rejected)
        );
        for disposition in [
            StepDisposition::Moved,
            StepDisposition::Blocked,
            StepDisposition::Rejected,
        ] {
            for selected in [&paced[..], &[]] {
                assert_eq!(
                    encode_step_outcome(selected, disposition, 0),
                    encode_step_result(disposition)
                );
            }
        }
    }

    /// A proto3 reader that knows only field 1 and skips every unknown varint field, as the
    /// protobuf rules require of an older client: what such a client makes of the result.
    fn field_1_only(payload: &[u8]) -> Option<u64> {
        fn varint(input: &[u8], cursor: &mut usize) -> Option<u64> {
            let mut value = 0_u64;
            for shift in (0..64).step_by(7) {
                let byte = *input.get(*cursor)?;
                *cursor += 1;
                value |= u64::from(byte & 0x7f) << shift;
                if byte & 0x80 == 0 {
                    return Some(value);
                }
            }
            None
        }
        let (mut cursor, mut disposition) = (0, None);
        while cursor < payload.len() {
            let key = varint(payload, &mut cursor)?;
            if key & 7 != 0 {
                return None;
            }
            let value = varint(payload, &mut cursor)?;
            if key >> 3 == 1 {
                disposition = Some(value);
            }
        }
        disposition
    }

    /// STEP-PACE-WIRE-1 field-2 compatibility gate: golden bytes with and without the receipt,
    /// field 2 only under capability 13, a round trip through the shared decoder, and a reader
    /// that ignores field 2 still decodes every result.
    #[test]
    fn step_wait_ms_is_sent_only_under_capability_13_and_old_readers_still_decode() {
        let paced = [CAPABILITY_PACED_MOVEMENT_V1];
        let cases: [(StepDisposition, u32, &[u8], &[u8]); 7] = [
            // (disposition, wait, capability 13 bytes, bytes without capability 13)
            (
                StepDisposition::Moved,
                50,
                &[0x08, 0x01, 0x10, 0x32],
                &[0x08, 0x01],
            ),
            (
                StepDisposition::Moved,
                100,
                &[0x08, 0x01, 0x10, 0x64],
                &[0x08, 0x01],
            ),
            (
                StepDisposition::Moved,
                MAX_STEP_WAIT_MS,
                &[0x08, 0x01, 0x10, 0xae, 0xe4, 0x06],
                &[0x08, 0x01],
            ),
            (StepDisposition::Moved, 0, &[0x08, 0x01], &[0x08, 0x01]),
            (
                StepDisposition::TooEarly,
                37,
                &[0x08, 0x04, 0x10, 0x25],
                &[0x08, 0x03],
            ),
            (StepDisposition::TooEarly, 0, &[0x08, 0x04], &[0x08, 0x03]),
            (StepDisposition::Blocked, 0, &[0x08, 0x02], &[0x08, 0x02]),
        ];
        for (disposition, wait, with, without) in cases {
            let sent = encode_step_outcome(&paced, disposition, wait);
            assert_eq!(sent, with, "{disposition:?} {wait}");
            assert!(sent.len() <= MAX_PACED_STEP_RESULT_BYTES);
            assert_eq!(
                decode_step_result_receipt(&sent, true),
                Ok(StepResult {
                    disposition,
                    step_wait_ms: wait
                })
            );
            assert_eq!(field_1_only(&sent), Some(disposition as u64));
            // Without capability 13: never field 2, the exact receipt-less bytes, which the
            // strict pre-receipt decoder accepts within its 4-byte bound.
            let legacy = encode_step_outcome(&[], disposition, wait);
            assert_eq!(legacy, without, "{disposition:?} {wait}");
            assert!(legacy.len() <= MAX_STEP_RESULT_BYTES);
            assert!(decode_step_result(&legacy).is_ok());
            assert_eq!(field_1_only(&legacy), field_1_only(without));
        }
        // The strict decoder of a session without capability 13 refuses field 2: that is why
        // the receipt is never sent to one.
        assert!(decode_step_result(&[0x08, 0x01, 0x10, 0x64]).is_err());
    }

    /// A receipt outside the STEP-PACE-WIRE-0 contract is dropped, never sent: the result keeps
    /// its disposition and goes out without field 2.
    #[test]
    fn an_out_of_contract_step_wait_ms_is_dropped() {
        let paced = [CAPABILITY_PACED_MOVEMENT_V1];
        for (disposition, wait) in [
            (StepDisposition::Moved, 80),
            (StepDisposition::Moved, 1),
            (StepDisposition::Moved, MAX_STEP_WAIT_MS + STEP_WAIT_BEAT_MS),
            (StepDisposition::TooEarly, MAX_STEP_WAIT_MS + 1),
            (StepDisposition::TooEarly, u32::MAX),
            (StepDisposition::Blocked, 50),
            (StepDisposition::Rejected, 50),
        ] {
            assert_eq!(
                encode_step_outcome(&paced, disposition, wait),
                encode_step_result(disposition),
                "{disposition:?} {wait}"
            );
        }
    }

    fn observation(x: i32, y: i32, floor: i16) -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: [0xab; 32],
            actor_position: ActorPosition { x, y, floor },
        }
    }

    #[test]
    fn step_intent_round_trips_and_refuses_everything_else() {
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let bytes = encode_step_intent(direction);
            assert!(bytes.len() <= MAX_STEP_INTENT_BYTES);
            assert_eq!(decode_step_intent(&bytes), Ok(direction));
        }
        for bad in [
            &[][..],                       // missing direction
            &[0x08, 0x00][..],             // zero enum
            &[0x08, 0x05][..],             // unknown enum
            &[0x08, 0x01, 0x08, 0x02][..], // repeated field
            &[0x10, 0x01][..],             // unknown field
            &[0x08][..],                   // truncated
        ] {
            assert_eq!(
                decode_step_intent(bad),
                Err(WorldSpatialError::Malformed),
                "{bad:?}"
            );
        }
        assert_eq!(
            decode_step_intent(&[0x08, 0x81, 0x80, 0x80, 0x00]),
            Err(WorldSpatialError::LimitExceeded)
        );
    }

    #[test]
    fn step_result_round_trips_and_refuses_unknown() {
        for disposition in [
            StepDisposition::Moved,
            StepDisposition::Blocked,
            StepDisposition::Rejected,
        ] {
            let bytes = encode_step_result(disposition);
            assert!(bytes.len() <= MAX_STEP_RESULT_BYTES);
            assert_eq!(decode_step_result(&bytes), Ok(disposition));
        }
        assert_eq!(
            decode_step_result(&[0x08, 0x04]),
            Err(WorldSpatialError::Malformed)
        );
        assert_eq!(
            decode_step_result(&[0x08, 0x00]),
            Err(WorldSpatialError::Malformed)
        );
    }

    #[test]
    fn too_early_decodes_only_under_capability_13() {
        let bytes = encode_step_result(StepDisposition::TooEarly);
        assert_eq!(bytes, [0x08, 0x04]);
        assert!(bytes.len() <= MAX_STEP_RESULT_BYTES);
        assert_eq!(
            decode_step_result_paced(&bytes, true),
            Ok(StepDisposition::TooEarly)
        );
        assert_eq!(
            decode_step_result_paced(&bytes, false),
            Err(WorldSpatialError::Malformed)
        );
        for disposition in [
            StepDisposition::Moved,
            StepDisposition::Blocked,
            StepDisposition::Rejected,
        ] {
            let bytes = encode_step_result(disposition);
            assert_eq!(decode_step_result_paced(&bytes, true), Ok(disposition));
        }
        for unknown in [[0x08, 0x00], [0x08, 0x05]] {
            assert_eq!(
                decode_step_result_paced(&unknown, true),
                Err(WorldSpatialError::Malformed)
            );
        }
    }

    fn paced(disposition: StepDisposition, step_wait_ms: u32) -> StepResult {
        StepResult {
            disposition,
            step_wait_ms,
        }
    }

    /// A raw result: disposition `disposition`, then field 2 holding `wait` as written.
    fn raw_result(disposition: u64, wait: u64) -> Vec<u8> {
        let mut bytes = vec![0x08];
        push_varint(&mut bytes, disposition);
        push_tag(&mut bytes, 2, 0);
        push_varint(&mut bytes, wait);
        bytes
    }

    #[test]
    fn step_wait_ms_round_trips_under_capability_13() {
        for result in [
            paced(StepDisposition::Moved, 0),
            paced(StepDisposition::Moved, STEP_WAIT_BEAT_MS),
            paced(StepDisposition::Moved, 550),
            paced(StepDisposition::Moved, MAX_STEP_WAIT_MS),
            paced(StepDisposition::TooEarly, 0),
            paced(StepDisposition::TooEarly, 1),
            paced(StepDisposition::TooEarly, 499),
            paced(StepDisposition::TooEarly, MAX_STEP_WAIT_MS),
            paced(StepDisposition::Blocked, 0),
            paced(StepDisposition::Rejected, 0),
        ] {
            let bytes = encode_step_result_paced(result, true).expect("valid receipt");
            assert!(bytes.len() <= MAX_PACED_STEP_RESULT_BYTES, "{result:?}");
            assert_eq!(decode_step_result_receipt(&bytes, true), Ok(result));
            // The disposition-only decoder a capability 13 session already uses admits it.
            assert_eq!(
                decode_step_result_paced(&bytes, true),
                Ok(result.disposition)
            );
        }
        // A zero receipt is not written: the result is byte-identical to the receipt-less one.
        assert_eq!(
            encode_step_result_paced(paced(StepDisposition::Moved, 0), true),
            Ok(encode_step_result(StepDisposition::Moved))
        );
        // The largest receipt is 6 bytes (tag, disposition, tag, 3-byte varint).
        assert_eq!(
            encode_step_result_paced(paced(StepDisposition::Moved, MAX_STEP_WAIT_MS), true),
            Ok(vec![0x08, 0x01, 0x10, 0xae, 0xe4, 0x06])
        );
        // Field order is free on the wire.
        assert_eq!(
            decode_step_result_receipt(&[0x10, 0x32, 0x08, 0x01], true),
            Ok(paced(StepDisposition::Moved, 50))
        );
        // Receipt-less results of a session without capability 13 decode unchanged.
        for disposition in [
            StepDisposition::Moved,
            StepDisposition::Blocked,
            StepDisposition::Rejected,
        ] {
            let bytes = encode_step_result_paced(paced(disposition, 0), false).expect("plain");
            assert!(bytes.len() <= MAX_STEP_RESULT_BYTES);
            assert_eq!(
                decode_step_result_receipt(&bytes, false),
                Ok(paced(disposition, 0))
            );
        }
    }

    #[test]
    fn step_wait_ms_refuses_adversarial_results() {
        let over = u64::from(MAX_STEP_WAIT_MS);
        for (bytes, paced_session) in [
            (raw_result(1, 50), false),           // receipt without capability 13
            (raw_result(4, 1), false),            // TOO_EARLY without capability 13
            (raw_result(2, 50), true),            // receipt on BLOCKED
            (raw_result(3, 50), true),            // receipt on REJECTED
            (raw_result(0, 50), true),            // receipt on UNSPECIFIED
            (raw_result(5, 50), true),            // unknown disposition
            (raw_result(1, 0), true),             // explicit zero on MOVED
            (raw_result(4, 0), true),             // explicit zero on TOO_EARLY
            (raw_result(1, 75), true),            // MOVED off the 50 ms beat
            (raw_result(1, over + 50), true),     // MOVED above the table maximum
            (raw_result(4, over + 1), true),      // TOO_EARLY above the table maximum
            (raw_result(1, 1 << 32), true),       // beyond uint32
            (vec![0x10, 0x32], true),             // missing disposition
            (vec![0x08, 0x01, 0x10], true),       // truncated tag only
            (vec![0x08, 0x01, 0x10, 0xb2], true), // truncated varint
            (vec![0x08, 0x01, 0x12, 0x01, 0x32], true), // field 2 as bytes
            (vec![0x08, 0x01, 0x18, 0x01], true), // unknown field 3
            (vec![0x08, 0x01, 0x10, 0x32, 0x10, 0x32], true), // repeated field 2
            (vec![0x08, 0x01, 0x08, 0x01, 0x10, 0x32], true), // repeated field 1
        ] {
            assert!(bytes.len() <= MAX_PACED_STEP_RESULT_BYTES, "{bytes:?}");
            assert_eq!(
                decode_step_result_receipt(&bytes, paced_session),
                Err(WorldSpatialError::Malformed),
                "{bytes:?}"
            );
            assert_eq!(
                decode_step_result_paced(&bytes, paced_session),
                Err(WorldSpatialError::Malformed),
                "{bytes:?}"
            );
        }
        // Over-bound: 5 bytes without capability 13, 9 bytes with it.
        assert_eq!(
            decode_step_result(&raw_result(1, 128)),
            Err(WorldSpatialError::LimitExceeded)
        );
        let mut long = raw_result(1, 50);
        long.extend_from_slice(&[0x80, 0x80, 0x80, 0x80, 0x00]);
        long[3] |= 0x80;
        assert_eq!(long.len(), MAX_PACED_STEP_RESULT_BYTES + 1);
        assert_eq!(
            decode_step_result_receipt(&long, true),
            Err(WorldSpatialError::LimitExceeded)
        );
        // The encoder refuses everything the decoder refuses.
        for (result, paced_session) in [
            (paced(StepDisposition::Moved, 50), false),
            (paced(StepDisposition::TooEarly, 0), false),
            (paced(StepDisposition::TooEarly, 1), false),
            (paced(StepDisposition::Blocked, 50), true),
            (paced(StepDisposition::Rejected, 1), true),
            (paced(StepDisposition::Moved, 75), true),
            (paced(StepDisposition::Moved, MAX_STEP_WAIT_MS + 50), true),
            (paced(StepDisposition::TooEarly, MAX_STEP_WAIT_MS + 1), true),
        ] {
            assert_eq!(
                encode_step_result_paced(result, paced_session),
                Err(WorldSpatialError::Malformed),
                "{result:?}"
            );
        }
    }

    #[test]
    fn world_spatial_round_trips_within_bound_and_refuses_malformed() {
        for value in [
            observation(0, 0, 0),
            observation(1, 0, 0),
            observation(0, -1, 0),
            observation(i32::MAX, i32::MIN, i16::MIN),
        ] {
            let bytes = encode_world_spatial(&value);
            assert!(
                bytes.len() <= MAX_WORLD_SPATIAL_PAYLOAD_BYTES,
                "{}",
                bytes.len()
            );
            assert_eq!(decode_world_spatial(&bytes), Ok(value));
        }
        let valid = encode_world_spatial(&observation(1, 0, 0));
        // Missing position, wrong generation width, duplicate or unknown fields.
        assert_eq!(
            decode_world_spatial(&valid[..34]),
            Err(WorldSpatialError::Malformed)
        );
        let mut short = vec![0x0a, 31];
        short.extend_from_slice(&[0; 31]);
        short.extend_from_slice(&valid[34..]);
        assert_eq!(
            decode_world_spatial(&short),
            Err(WorldSpatialError::Malformed)
        );
        let mut duplicate = valid.clone();
        duplicate.extend_from_slice(&valid[34..]);
        assert_eq!(
            decode_world_spatial(&duplicate),
            Err(WorldSpatialError::Malformed)
        );
        let mut unknown = valid.clone();
        unknown.extend_from_slice(&[0x18, 0x01]);
        assert_eq!(
            decode_world_spatial(&unknown),
            Err(WorldSpatialError::Malformed)
        );
        // A floor beyond the i16 range is refused.
        let mut wide = vec![0x0a, 32];
        wide.extend_from_slice(&[0; 32]);
        wide.extend_from_slice(&[0x12, 0x04, 0x18, 0x80, 0x80, 0x04]);
        assert_eq!(
            decode_world_spatial(&wide),
            Err(WorldSpatialError::Malformed)
        );
        assert_eq!(
            decode_world_spatial(&[0; MAX_WORLD_SPATIAL_PAYLOAD_BYTES + 1]),
            Err(WorldSpatialError::LimitExceeded)
        );
    }

    #[test]
    fn registries_bind_the_accepted_first_control_ids_and_limits() {
        let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
        let commands = protocol["command_types"].as_array().expect("command_types");
        // Looked up by id, not array position: the registry is shared and other command
        // types may be registered alongside this one (e.g. USE-WIRE-V1's USE_INTENT).
        let command = commands
            .iter()
            .find(|command| command["id"] == COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT)
            .expect("WORLD_ACTOR_STEP_INTENT registered");
        assert_eq!(command["name"], "WORLD_ACTOR_STEP_INTENT");
        assert_eq!(command["max_payload_bytes"], MAX_STEP_INTENT_BYTES as u64);
        // STEP-PACE-WIRE-0: the registered result bound is the one under capability 13; the
        // capability's offer gate states the 4-byte bound without it.
        assert_eq!(
            command["max_result_payload_bytes"],
            MAX_PACED_STEP_RESULT_BYTES as u64
        );
        let capability = protocol["capabilities"]
            .as_array()
            .expect("capabilities")
            .iter()
            .find(|capability| capability["id"] == CAPABILITY_PACED_MOVEMENT_V1)
            .expect("PACED_MOVEMENT_V1 registered");
        assert_eq!(capability["name"], "PACED_MOVEMENT_V1");
        assert_eq!(capability["command_types"], serde_json::json!([]));
        assert_eq!(capability["state_domains"], serde_json::json!([]));
        let gate = capability["offer_gate"].as_str().expect("offer_gate");
        assert!(gate.contains("step_wait_ms (field 2)"), "{gate}");
        assert!(
            gate.contains(&format!(
                "command type 1 result is at most {MAX_STEP_RESULT_BYTES} bytes"
            )),
            "{gate}"
        );
        let domains = protocol["state_domains"].as_array().expect("state_domains");
        let domain = domains
            .iter()
            .find(|domain| domain["id"] == STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY)
            .expect("WORLD_SPATIAL_VISIBILITY registered");
        assert_eq!(domain["name"], "WORLD_SPATIAL_VISIBILITY");
        assert_eq!(domain["delta_types"][0]["id"], DELTA_TYPE_WORLD_SPATIAL_V1);
        assert_eq!(
            domain["snapshot_types"][0]["id"],
            SNAPSHOT_TYPE_WORLD_SPATIAL_V1
        );

        let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
        let entries = resources["entries"].as_array().expect("entries");
        let limit = |id: &str| {
            entries
                .iter()
                .find(|entry| entry["id"] == id)
                .and_then(|entry| entry["hard_maximum"].as_u64())
        };
        assert_eq!(limit("MOVE-RL-02"), Some(1));
        assert_eq!(limit("MOVE-RL-03"), Some(1));
        // VIS-2 re-decided MOVE-RL-11 (D87): 256 entities per snapshot or delta.
        assert_eq!(limit("MOVE-RL-11"), Some(256));
    }
}

#[cfg(test)]
#[path = "world_spatial_fuzz_tests.rs"]
mod fuzz_tests;
