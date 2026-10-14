#![allow(clippy::expect_used)]

use serde_json::Value;

use super::*;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const SCHEMA_PATH: &str = "docs/contracts/protocol-oteryn/v1/world_clock_v1.proto";
const SCHEMA: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/world_clock_v1.proto");

fn clock(tibian_minute: u16, light_level: u8, light_color: u8) -> WorldClock {
    WorldClock {
        tibian_minute,
        light_level,
        light_color,
    }
}

#[test]
fn the_clock_round_trips_within_its_bound() {
    let day = clock(480, 250, 215);
    let bytes = encode_world_clock(day).expect("encode");
    assert_eq!(
        bytes,
        [0x08, 0xe0, 0x03, 0x10, 0xfa, 0x01, 0x18, 0xd7, 0x01]
    );
    assert_eq!(bytes.len(), MAX_WORLD_CLOCK_BYTES);
    assert_eq!(decode_world_clock(&bytes).expect("decode"), day);

    // The longest minute and zero values: midnight and a dark level are omitted.
    let last = clock(1_439, 255, 255);
    let bytes = encode_world_clock(last).expect("encode");
    assert_eq!(bytes.len(), 9);
    assert_eq!(decode_world_clock(&bytes).expect("decode"), last);
    let midnight = clock(0, 0, 0);
    assert!(encode_world_clock(midnight).expect("encode").is_empty());
    assert_eq!(decode_world_clock(&[]).expect("decode"), midnight);
}

#[test]
fn a_minute_outside_the_day_is_refused_in_both_directions() {
    assert_eq!(
        encode_world_clock(clock(1_440, 40, 215)),
        Err(WorldClockWireError::Malformed)
    );
    // 1,440 as a varint in field 1.
    assert_eq!(
        decode_world_clock(&[0x08, 0xa0, 0x0b]),
        Err(WorldClockWireError::Malformed)
    );
}

#[test]
fn malformed_clocks_fail_closed() {
    let malformed = Err(WorldClockWireError::Malformed);
    // Unknown field 4, a repeated field, a length-delimited field, a level and a color above 255,
    // a uint32 overflow and a truncated tail.
    for bytes in [
        &[0x20, 1][..],
        &[0x08, 1, 0x08, 2][..],
        &[0x0a, 1, 0][..],
        &[0x10, 0x80, 0x02][..],
        &[0x18, 0x80, 0x02][..],
        &[0x08, 0x80, 0x80, 0x80, 0x80, 0x10][..],
        &[0x08][..],
    ] {
        assert_eq!(decode_world_clock(bytes), malformed, "{bytes:?}");
    }
    assert_eq!(
        decode_world_clock(&[0; MAX_WORLD_CLOCK_BYTES + 1]),
        Err(WorldClockWireError::LimitExceeded)
    );
}

#[test]
fn registry_binds_the_clock_wire_ids_and_limits() {
    let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let only = |section: &str, id: u32| {
        let matches: Vec<&Value> = protocol[section]
            .as_array()
            .expect(section)
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matches.len(), 1, "{section} {id} registered once");
        matches[0].clone()
    };

    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_WORLD_LIGHT_V1));
    let capability = only("capabilities", CAPABILITY_WORLD_LIGHT_V1);
    assert_eq!(capability["name"], "WORLD_LIGHT_V1");
    assert_eq!(capability["offered"], false);
    assert_eq!(capability["requires"], serde_json::json!([6]));
    assert_eq!(capability["command_types"], serde_json::json!([]));
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_WORLD_CLOCK])
    );
    assert!(
        protocol["command_types"]
            .as_array()
            .expect("command types")
            .iter()
            .all(|command| command["capability"] != CAPABILITY_WORLD_LIGHT_V1)
    );

    let domain = only("state_domains", STATE_DOMAIN_WORLD_CLOCK);
    assert_eq!(domain["name"], "WORLD_CLOCK");
    assert_eq!(domain["capability"], CAPABILITY_WORLD_LIGHT_V1);
    let schema = format!("{SCHEMA_PATH}#WorldClockV1");
    for (section, id, name) in [
        (
            "delta_types",
            DELTA_TYPE_WORLD_CLOCK_V1,
            "WORLD_CLOCK_DELTA_V1",
        ),
        (
            "snapshot_types",
            SNAPSHOT_TYPE_WORLD_CLOCK_V1,
            "WORLD_CLOCK_SNAPSHOT_V1",
        ),
    ] {
        let entries = domain[section].as_array().expect(section);
        assert_eq!(entries.len(), 1, "{section}");
        assert_eq!(entries[0]["id"], id);
        assert_eq!(entries[0]["name"], name);
        assert_eq!(entries[0]["payload_schema"], schema.as_str());
        assert_eq!(entries[0]["max_payload_bytes"], MAX_WORLD_CLOCK_BYTES);
    }

    assert!(SCHEMA.contains("message WorldClockV1 {"));
    assert!(SCHEMA.contains(&format!("At most\n// {MAX_WORLD_CLOCK_BYTES} bytes")));
}
