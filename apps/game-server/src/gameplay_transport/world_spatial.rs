//! `FIRST-CONTROL-WIRE-V1` typed payloads (#642 owner acceptance 5853424280; schema in
//! `docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto`). Command type 1
//! `WORLD_ACTOR_STEP_INTENT` and state domain 1 `WORLD_SPATIAL_VISIBILITY` with delta type 1 and
//! snapshot type 1. Decoding is strict: zero or unknown enum values, unknown or repeated fields and
//! over-bound payloads fail closed.

// Composed by the first-control Server Seam child (M2, #822).
#![allow(dead_code)]

pub(crate) const COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT: u32 = 1;
pub(crate) const STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY: u32 = 1;
pub(crate) const DELTA_TYPE_WORLD_SPATIAL_V1: u32 = 1;
pub(crate) const SNAPSHOT_TYPE_WORLD_SPATIAL_V1: u32 = 1;

/// First-child semantic bounds (#642 packet 5853255180).
pub(crate) const MAX_STEP_INTENT_BYTES: usize = 4;
pub(crate) const MAX_STEP_RESULT_BYTES: usize = 4;
pub(crate) const MAX_WORLD_SPATIAL_PAYLOAD_BYTES: usize = 64;
const CONTENT_GENERATION_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorldSpatialError {
    Malformed,
    LimitExceeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepDirection {
    North = 1,
    East = 2,
    South = 3,
    West = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepDisposition {
    Moved = 1,
    Blocked = 2,
    Rejected = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActorPosition {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) floor: i16,
}

/// The own-actor observation carried by both the delta and the snapshot of domain 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorldSpatialObservation {
    pub(crate) content_generation: [u8; 32],
    pub(crate) actor_position: ActorPosition,
}

fn push_varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn push_tag(output: &mut Vec<u8>, field: u64, wire: u64) {
    push_varint(output, (field << 3) | wire);
}

fn push_sint32(output: &mut Vec<u8>, field: u64, value: i32) {
    let zigzag = ((value << 1) ^ (value >> 31)) as u32;
    if zigzag != 0 {
        push_tag(output, field, 0);
        push_varint(output, u64::from(zigzag));
    }
}

fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, WorldSpatialError> {
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

fn read_bytes<'a>(input: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], WorldSpatialError> {
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

/// Reads exactly one singular enum field 1 (the only field of the step intent and result).
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

pub(crate) fn decode_step_intent(payload: &[u8]) -> Result<StepDirection, WorldSpatialError> {
    match read_single_enum(payload, MAX_STEP_INTENT_BYTES)? {
        1 => Ok(StepDirection::North),
        2 => Ok(StepDirection::East),
        3 => Ok(StepDirection::South),
        4 => Ok(StepDirection::West),
        _ => Err(WorldSpatialError::Malformed),
    }
}

pub(crate) fn encode_step_intent(direction: StepDirection) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_tag(&mut output, 1, 0);
    push_varint(&mut output, direction as u64);
    output
}

pub(crate) fn encode_step_result(disposition: StepDisposition) -> Vec<u8> {
    let mut output = Vec::with_capacity(2);
    push_tag(&mut output, 1, 0);
    push_varint(&mut output, disposition as u64);
    output
}

pub(crate) fn decode_step_result(payload: &[u8]) -> Result<StepDisposition, WorldSpatialError> {
    match read_single_enum(payload, MAX_STEP_RESULT_BYTES)? {
        1 => Ok(StepDisposition::Moved),
        2 => Ok(StepDisposition::Blocked),
        3 => Ok(StepDisposition::Rejected),
        _ => Err(WorldSpatialError::Malformed),
    }
}

/// Encodes the domain-1 delta or snapshot payload (identical schemas, distinct registered types).
pub(crate) fn encode_world_spatial(observation: &WorldSpatialObservation) -> Vec<u8> {
    let mut position = Vec::with_capacity(18);
    push_sint32(&mut position, 1, observation.actor_position.x);
    push_sint32(&mut position, 2, observation.actor_position.y);
    push_sint32(
        &mut position,
        3,
        i32::from(observation.actor_position.floor),
    );
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

fn decode_position(input: &[u8]) -> Result<ActorPosition, WorldSpatialError> {
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

pub(crate) fn decode_world_spatial(
    payload: &[u8],
) -> Result<WorldSpatialObservation, WorldSpatialError> {
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::Value;

    const PROTOCOL_REGISTRY: &str =
        include_str!("../../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
    const RESOURCE_REGISTRY: &str =
        include_str!("../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");

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
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0]["id"], COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        assert_eq!(commands[0]["name"], "WORLD_ACTOR_STEP_INTENT");
        let domains = protocol["state_domains"].as_array().expect("state_domains");
        assert_eq!(domains.len(), 1);
        assert_eq!(domains[0]["id"], STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY);
        assert_eq!(domains[0]["name"], "WORLD_SPATIAL_VISIBILITY");
        assert_eq!(
            domains[0]["delta_types"][0]["id"],
            DELTA_TYPE_WORLD_SPATIAL_V1
        );
        assert_eq!(
            domains[0]["snapshot_types"][0]["id"],
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
        assert_eq!(limit("MOVE-RL-11"), Some(1));
    }
}
