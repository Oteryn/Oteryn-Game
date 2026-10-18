//! WEATHER-1 candidate visual payload codec (CP D1111, D1026–D1027).
//!
//! Registered (not offered) in `PROTOCOL_OTERYN_V1_REGISTRY.json`; protocol-owner D1027 sign-off is pending.
//! Candidate protobuf layout: required field 1 varint status (1 Unavailable, 2 Active);
//! Active additionally requires field 2 bytes binding token (exactly 32 public, non-secret
//! revision-identity bytes), field 3 uint64 absolute slot (present even at zero), and field 4
//! uint32 preset index (1..16). Unavailable has no Active fields. Snapshot/change use the same
//! full replacement value. The token is correlation data, never seed bytes or authority.
//!
//! The shape and 64-byte bound require D1027 approval before registration or production use.
//! D1137 allocates capability 26 and state domain 22; their registration remains A2 after
//! ITEM-LOOK-1 (#2182). No feature offer, current-owner check or binding derivation is
//! implemented here. The eventual consumer must also admit the preset against its
//! independently qualified policy. Unknown/repeated fields, mixed shapes and overflow refuse.

pub use crate::charm_wire::CyclopediaWireError as WorldWeatherWireError;
use crate::charm_wire::{
    WireResult, push_message_field, push_varint_field, read_bytes, read_varint, set_once,
};

/// D1137 allocated leaf ID; registry publication and D1027 frozen protocol review are pending.
pub const CAPABILITY_WORLD_WEATHER_V1: u32 = 26;
/// D1027: weather requires the World-spatial context.
pub const CAPABILITY_WORLD_WEATHER_V1_REQUIRES: [u32; 1] =
    [crate::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES];
/// D1137 allocated leaf ID; this constant does not register a state domain or enable an offer.
pub const STATE_DOMAIN_WORLD_WEATHER: u32 = 22;
/// Delta type 1 `WORLD_WEATHER_DELTA_V1`; snapshot and delta carry the same full replacement.
pub const DELTA_TYPE_WORLD_WEATHER_V1: u32 = 1;
/// Snapshot type 1 `WORLD_WEATHER_SNAPSHOT_V1`.
pub const SNAPSHOT_TYPE_WORLD_WEATHER_V1: u32 = 1;

pub const MAX_WORLD_WEATHER_BYTES: usize = 64;
const _: () = assert!(MAX_WORLD_WEATHER_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES);

/// A visual observation only; it carries no gameplay mutation or owner/session authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldWeather {
    Unavailable,
    Active {
        binding_token: [u8; 32],
        absolute_slot: u64,
        preset_index: u8,
    },
}

/// Candidate encoding is at most 49 bytes, including the longest uint64 slot. Invalid server
/// values are refused before returning any output. Slot zero is explicitly present.
pub fn encode_world_weather(weather: WorldWeather) -> WireResult<Vec<u8>> {
    let mut output = Vec::with_capacity(49);
    match weather {
        WorldWeather::Unavailable => push_varint_field(&mut output, 1, 1),
        WorldWeather::Active {
            binding_token,
            absolute_slot,
            preset_index,
        } => {
            if !(1..=16).contains(&preset_index) {
                return Err(WorldWeatherWireError::Malformed);
            }
            push_varint_field(&mut output, 1, 2);
            push_message_field(&mut output, 2, &binding_token);
            push_varint_field(&mut output, 3, absolute_slot);
            push_varint_field(&mut output, 4, u64::from(preset_index));
        }
    }
    Ok(output)
}

pub fn decode_world_weather(payload: &[u8]) -> WireResult<WorldWeather> {
    if payload.len() > MAX_WORLD_WEATHER_BYTES {
        return Err(WorldWeatherWireError::LimitExceeded);
    }
    let (mut status, mut binding, mut slot, mut preset) = (None, None, None, None);
    let mut cursor = 0;
    while cursor < payload.len() {
        match read_varint(payload, &mut cursor)? {
            0x08 => set_once(&mut status, read_varint(payload, &mut cursor)?)?,
            0x12 => {
                let token: [u8; 32] = read_bytes(payload, &mut cursor)?
                    .try_into()
                    .map_err(|_| WorldWeatherWireError::Malformed)?;
                set_once(&mut binding, token)?;
            }
            0x18 => set_once(&mut slot, read_varint(payload, &mut cursor)?)?,
            0x20 => set_once(&mut preset, read_varint(payload, &mut cursor)?)?,
            _ => return Err(WorldWeatherWireError::Malformed),
        }
    }
    match (status, binding, slot, preset) {
        (Some(1), None, None, None) => Ok(WorldWeather::Unavailable),
        (Some(2), Some(binding_token), Some(absolute_slot), Some(index)) => {
            let preset_index = u8::try_from(index).map_err(|_| WorldWeatherWireError::Malformed)?;
            if !(1..=16).contains(&preset_index) {
                return Err(WorldWeatherWireError::Malformed);
            }
            Ok(WorldWeather::Active {
                binding_token,
                absolute_slot,
                preset_index,
            })
        }
        _ => Err(WorldWeatherWireError::Malformed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active(absolute_slot: u64, preset_index: u8) -> WorldWeather {
        WorldWeather::Active {
            binding_token: [0x31; 32],
            absolute_slot,
            preset_index,
        }
    }

    #[test]
    fn full_replacement_shapes_round_trip_at_slot_and_preset_bounds() -> WireResult<()> {
        assert_eq!(encode_world_weather(WorldWeather::Unavailable)?, [0x08, 1]);
        for observation in [
            WorldWeather::Unavailable,
            active(0, 1),
            active(u64::MAX, 16),
        ] {
            let bytes = encode_world_weather(observation)?;
            assert!(bytes.len() <= 49 && bytes.len() <= MAX_WORLD_WEATHER_BYTES);
            assert_eq!(decode_world_weather(&bytes)?, observation);
        }
        let zero = encode_world_weather(active(0, 1))?;
        assert_eq!(&zero[36..], [0x18, 0, 0x20, 1]);
        assert_eq!(encode_world_weather(active(u64::MAX, 16))?.len(), 49);
        Ok(())
    }

    #[test]
    fn over_64_bytes_is_refused_before_parsing() -> WireResult<()> {
        // Existing protobuf helpers accept non-minimal varints. Pad only the first tags/status
        // to reach the exact resource boundary without inventing an unknown padding field.
        let valid = encode_world_weather(active(0, 1))?;
        let at_limit = [
            vec![0x88],
            vec![0x80; 8],
            vec![0],
            vec![0x82],
            vec![0x80; 8],
            vec![0],
            vec![0x92],
            vec![0x80; 5],
            vec![0],
            valid[3..].to_vec(),
        ]
        .concat();
        assert_eq!(at_limit.len(), 64);
        assert_eq!(decode_world_weather(&at_limit)?, active(0, 1));
        let over_limit = [at_limit, vec![0]].concat();
        assert_eq!(
            decode_world_weather(&over_limit),
            Err(WorldWeatherWireError::LimitExceeded)
        );
        assert_eq!(
            decode_world_weather(&[0; 64]),
            Err(WorldWeatherWireError::Malformed)
        );
        Ok(())
    }

    #[test]
    fn unknown_repeated_and_wrong_wire_type_fields_are_refused() -> WireResult<()> {
        let valid = encode_world_weather(active(0, 1))?;
        for suffix in [
            vec![0x28, 1], // unknown field 5
            vec![0x08, 2], // repeated status, including the same value
            vec![0x12, 0], // repeated binding key; two full tokens exceed the byte cap
            vec![0x18, 0],
            vec![0x20, 1],
            vec![0x22, 1, 1], // field 4, wrong wire type
        ] {
            let bytes = [valid.as_slice(), suffix.as_slice()].concat();
            assert_eq!(
                decode_world_weather(&bytes),
                Err(WorldWeatherWireError::Malformed)
            );
        }
        Ok(())
    }

    #[test]
    fn missing_mixed_truncated_and_overflowed_fields_are_refused() -> WireResult<()> {
        let valid = encode_world_weather(active(0, 1))?;
        for range in [0..2, 2..36, 36..38, 38..40] {
            let bytes = [&valid[..range.start], &valid[range.end..]].concat();
            assert_eq!(
                decode_world_weather(&bytes),
                Err(WorldWeatherWireError::Malformed)
            );
        }
        for status in [0, 1, 3] {
            let mut mixed = valid.clone();
            mixed[1] = status;
            assert_eq!(
                decode_world_weather(&mixed),
                Err(WorldWeatherWireError::Malformed)
            );
        }
        for bytes in [
            vec![],
            vec![0x08],
            vec![0x08, 0],
            vec![0x08, 3],
            vec![0x08, 1, 0x18, 0],
            vec![0x08, 2, 0x12, 32, 1],
            [vec![0x18], vec![0xff; 9], vec![2]].concat(),
        ] {
            assert_eq!(
                decode_world_weather(&bytes),
                Err(WorldWeatherWireError::Malformed)
            );
        }
        for length in [31, 33] {
            let bytes = [
                vec![0x08, 2, 0x12, length],
                vec![0x31; usize::from(length)],
                vec![0x18, 0, 0x20, 1],
            ]
            .concat();
            assert_eq!(
                decode_world_weather(&bytes),
                Err(WorldWeatherWireError::Malformed)
            );
        }
        Ok(())
    }

    #[test]
    fn preset_zero_seventeen_and_integer_overflow_are_refused_in_both_directions() -> WireResult<()>
    {
        for index in [0, 17, u8::MAX] {
            assert_eq!(
                encode_world_weather(active(0, index)),
                Err(WorldWeatherWireError::Malformed)
            );
            let mut bytes = encode_world_weather(active(0, 1))?;
            bytes[39] = index;
            assert_eq!(
                decode_world_weather(&bytes),
                Err(WorldWeatherWireError::Malformed)
            );
        }
        let valid = encode_world_weather(active(0, 1))?;
        let overflow = [&valid[..38], &[0x20, 0x80, 0x02]].concat();
        assert_eq!(
            decode_world_weather(&overflow),
            Err(WorldWeatherWireError::Malformed)
        );
        Ok(())
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn registry_binds_the_weather_wire_ids_and_limits() {
        const REGISTRY: &str = include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
        const SCHEMA: &str =
            include_str!("../../../docs/contracts/protocol-oteryn/v1/world_weather_v1.proto");
        let protocol: serde_json::Value = serde_json::from_str(REGISTRY).expect("registry");
        let only = |section: &str, id: u32| {
            let matches: Vec<&serde_json::Value> = protocol[section]
                .as_array()
                .expect(section)
                .iter()
                .filter(|entry| entry["id"] == id)
                .collect();
            assert_eq!(matches.len(), 1, "{section} {id} registered once");
            matches[0].clone()
        };
        assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_WORLD_WEATHER_V1));
        let capability = only("capabilities", CAPABILITY_WORLD_WEATHER_V1);
        assert_eq!(capability["name"], "WORLD_WEATHER_V1");
        assert_eq!(capability["offered"], false);
        assert_eq!(capability["requires"], serde_json::json!([6]));
        assert_eq!(capability["command_types"], serde_json::json!([]));
        assert_eq!(
            capability["state_domains"],
            serde_json::json!([STATE_DOMAIN_WORLD_WEATHER])
        );
        let domain = only("state_domains", STATE_DOMAIN_WORLD_WEATHER);
        assert_eq!(domain["capability"], CAPABILITY_WORLD_WEATHER_V1);
        for (section, id) in [
            ("delta_types", DELTA_TYPE_WORLD_WEATHER_V1),
            ("snapshot_types", SNAPSHOT_TYPE_WORLD_WEATHER_V1),
        ] {
            let entries = domain[section].as_array().expect(section);
            assert_eq!(entries.len(), 1, "{section}");
            assert_eq!(entries[0]["id"], id);
            assert_eq!(entries[0]["max_payload_bytes"], MAX_WORLD_WEATHER_BYTES);
            assert_eq!(
                entries[0]["payload_schema"],
                "docs/contracts/protocol-oteryn/v1/world_weather_v1.proto#WorldWeatherV1"
            );
        }
        assert!(SCHEMA.contains("message WorldWeatherV1 {"));
    }
}
