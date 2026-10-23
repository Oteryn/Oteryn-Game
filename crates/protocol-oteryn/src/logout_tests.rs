#![allow(clippy::expect_used)]

use super::*;
use crate::charm_wire::push_varint_field;
use serde_json::Value;

const PROTOCOL_REGISTRY: &str = crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON;
const RESOURCE_REGISTRY: &str = crate::RESOURCE_LIMITS_REGISTRY_JSON;
const LOGOUT_PROTO: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/logout_v1.proto");

fn raw(fields: &[(u64, u64)]) -> Vec<u8> {
    let mut output = Vec::new();
    for &(field, value) in fields {
        push_varint_field(&mut output, field, value);
    }
    output
}

#[test]
fn the_intent_is_empty_and_any_body_is_refused() {
    assert!(encode_logout_intent().is_empty());
    assert_eq!(decode_logout_intent(&[]), Ok(()));
    assert_eq!(
        decode_logout_intent(&raw(&[(1, 1)])),
        Err(LogoutWireError::LimitExceeded)
    );
    assert_eq!(
        decode_logout_intent(&[0]),
        Err(LogoutWireError::LimitExceeded)
    );
}

#[test]
fn every_result_code_round_trips() {
    for (result, wire) in [
        (LogoutResult::Accepted, raw(&[(1, 1)])),
        (
            LogoutResult::InFight { retry_after_ms: 1 },
            raw(&[(1, 2), (2, 1)]),
        ),
        (
            LogoutResult::InFight {
                retry_after_ms: MAX_LOGOUT_RETRY_AFTER_MS,
            },
            raw(&[(1, 2), (2, 60_000)]),
        ),
        (LogoutResult::Busy, raw(&[(1, 3)])),
    ] {
        let encoded = encode_logout_result(result).expect("encodes");
        assert_eq!(encoded, wire, "{result:?}");
        assert!(encoded.len() <= MAX_LOGOUT_RESULT_BYTES);
        assert_eq!(decode_logout_result(&encoded), Ok(result));
    }
    assert!(LogoutResult::Busy.is_rejected());
    assert!(LogoutResult::InFight { retry_after_ms: 5 }.is_rejected());
    assert!(!LogoutResult::Accepted.is_rejected());
}

#[test]
fn the_worst_result_fits_its_bound() {
    let worst = encode_logout_result(LogoutResult::InFight {
        retry_after_ms: MAX_LOGOUT_RETRY_AFTER_MS,
    })
    .expect("encodes");
    assert_eq!(worst.len(), MAX_LOGOUT_RESULT_BYTES);
}

#[test]
fn invalid_results_fail_closed() {
    for retry_after_ms in [0, MAX_LOGOUT_RETRY_AFTER_MS + 1] {
        assert_eq!(
            encode_logout_result(LogoutResult::InFight { retry_after_ms }),
            Err(LogoutWireError::Malformed)
        );
    }
    for payload in [
        Vec::new(),                  // code 0
        raw(&[(1, 0)]),              // explicit code 0
        raw(&[(1, 4)]),              // unknown code
        raw(&[(1, 2)]),              // IN_FIGHT without a retry
        raw(&[(1, 2), (2, 60_001)]), // retry over ATTACK0-RL-03
        raw(&[(1, 1), (2, 5)]),      // retry with ACCEPTED
        raw(&[(1, 3), (2, 5)]),      // retry with BUSY
        raw(&[(1, 1), (1, 1)]),      // repeated field
        raw(&[(1, 1), (3, 1)]),      // unknown field
        vec![0x0a, 0x00],            // wrong wire type
        raw(&[(1, 2), (2, u64::from(u32::MAX) + 1)]),
    ] {
        assert!(decode_logout_result(&payload).is_err(), "{payload:?}");
    }
    let mut oversized = raw(&[(1, 2), (2, 60_000)]);
    oversized.push(0);
    assert_eq!(
        decode_logout_result(&oversized),
        Err(LogoutWireError::LimitExceeded)
    );
}

#[test]
fn the_registries_bind_the_logout_constants() {
    let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let find = |list: &str, id: u32| {
        let matching: Vec<&Value> = protocol[list]
            .as_array()
            .expect(list)
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matching.len(), 1, "{list} {id}");
        matching[0].clone()
    };
    let capability = find("capabilities", CAPABILITY_LOGOUT_V1);
    assert_eq!(capability["name"], "LOGOUT_V1");
    assert_eq!(capability["offered"], true);
    assert_eq!(capability["requires"], serde_json::json!([]));
    assert_eq!(
        capability["command_types"],
        serde_json::json!([COMMAND_TYPE_LOGOUT_INTENT])
    );
    assert_eq!(capability["state_domains"], serde_json::json!([]));
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_LOGOUT_V1));

    let schema = "docs/contracts/protocol-oteryn/v1/logout_v1.proto#";
    let command = find("command_types", COMMAND_TYPE_LOGOUT_INTENT);
    assert_eq!(command["name"], "LOGOUT_INTENT");
    assert_eq!(command["capability"], CAPABILITY_LOGOUT_V1);
    assert_eq!(command["payload_schema"], format!("{schema}LogoutIntentV1"));
    assert_eq!(command["result_schema"], format!("{schema}LogoutResultV1"));
    assert_eq!(
        command["max_payload_bytes"].as_u64(),
        Some(MAX_LOGOUT_INTENT_BYTES as u64)
    );
    assert_eq!(
        command["max_result_payload_bytes"].as_u64(),
        Some(MAX_LOGOUT_RESULT_BYTES as u64)
    );
    assert!(LOGOUT_PROTO.contains("message LogoutIntentV1 {}"));
    assert!(LOGOUT_PROTO.contains("message LogoutResultV1 {"));
    assert!(LOGOUT_PROTO.contains(&format!("At most {MAX_LOGOUT_RESULT_BYTES} bytes")));

    let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
    let limit = |row: &str| {
        resources["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["id"] == row)
            .and_then(|entry| entry["hard_maximum"].as_u64())
    };
    assert_eq!(
        limit("LOGOUT-RL-01"),
        Some(u64::from(MAX_OUTSTANDING_LOGOUTS))
    );
    assert_eq!(
        limit("ATTACK0-RL-03"),
        Some(u64::from(MAX_LOGOUT_RETRY_AFTER_MS))
    );
    assert!(LOGOUT_PROTO.contains("LOGOUT-RL-01"));
    assert!(LOGOUT_PROTO.contains("ATTACK0-RL-03"));
}
