#![allow(clippy::expect_used)]

use super::*;
use serde_json::Value;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
const ATTACK_PROTO: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/attack_v1.proto");

fn target(generation: u64) -> EntityRef {
    EntityRef {
        identity: [0xA5; ENTITY_IDENTITY_BYTES],
        generation,
    }
}

/// The wire form of an `EntityRefV1` with the given raw fields.
fn raw_target(identity: &[u8], generation: Option<u64>) -> Vec<u8> {
    let mut inner = Vec::new();
    push_message_field(&mut inner, 1, identity);
    if let Some(generation) = generation {
        push_varint_field(&mut inner, 2, generation);
    }
    inner
}

fn worst_state() -> ActorCombatState {
    ActorCombatState {
        target: Some(target(u64::MAX)),
        modes: FightModes {
            fight_mode: FightMode::Defensive,
            chase: ChaseMode::Chase,
            secure: true,
        },
        in_fight: true,
    }
}

#[test]
fn target_intents_round_trip_at_their_byte_bound() {
    let worst = encode_attack_target_intent(Some(&target(u64::MAX))).expect("encode");
    assert_eq!(worst.len(), MAX_ATTACK_TARGET_INTENT_BYTES);
    assert_eq!(
        decode_attack_target_intent(&worst),
        Ok(Some(target(u64::MAX)))
    );
    let small = encode_attack_target_intent(Some(&target(1))).expect("encode");
    assert_eq!(decode_attack_target_intent(&small), Ok(Some(target(1))));

    let stop = encode_attack_target_intent(None).expect("encode");
    assert!(stop.is_empty());
    assert_eq!(decode_attack_target_intent(&[]), Ok(None));
}

#[test]
fn target_intents_fail_closed() {
    assert_eq!(
        encode_attack_target_intent(Some(&target(0))),
        Err(AttackWireError::Malformed)
    );
    let malformed = |inner: Vec<u8>| {
        let mut payload = Vec::new();
        push_message_field(&mut payload, 1, &inner);
        assert_eq!(
            decode_attack_target_intent(&payload),
            Err(AttackWireError::Malformed),
            "{payload:?}"
        );
    };
    // An empty target, a missing identity or generation, a zero generation.
    malformed(Vec::new());
    malformed(raw_target(&[0xA5; 16], None));
    let mut no_identity = Vec::new();
    push_varint_field(&mut no_identity, 2, 7);
    malformed(no_identity);
    malformed(raw_target(&[0xA5; 16], Some(0)));
    // An identity that is not 16 bytes.
    malformed(raw_target(&[0xA5; 15], Some(7)));
    malformed(raw_target(&[0xA5; 17], Some(7)));
    // Repeated or unknown fields inside the target.
    let mut doubled = raw_target(&[0xA5; 16], Some(7));
    push_varint_field(&mut doubled, 2, 7);
    malformed(doubled);
    let mut unknown = raw_target(&[0xA5; 16], Some(7));
    push_varint_field(&mut unknown, 3, 1);
    malformed(unknown);

    // A doubled target, an unknown field and a wrong wire type at the top level.
    let one = encode_attack_target_intent(Some(&target(1))).expect("encode");
    // Two targets never fit the 31-byte bound.
    let doubled = [one.clone(), one.clone()].concat();
    assert_eq!(
        decode_attack_target_intent(&doubled),
        Err(AttackWireError::LimitExceeded)
    );
    let mut unknown = one.clone();
    push_varint_field(&mut unknown, 2, 1);
    assert_eq!(
        decode_attack_target_intent(&unknown),
        Err(AttackWireError::Malformed)
    );
    assert_eq!(
        decode_attack_target_intent(&[0x08, 0x01]),
        Err(AttackWireError::Malformed)
    );
    // Over the byte bound.
    let mut over = encode_attack_target_intent(Some(&target(u64::MAX))).expect("encode");
    over.push(0);
    assert_eq!(
        decode_attack_target_intent(&over),
        Err(AttackWireError::LimitExceeded)
    );
}

#[test]
fn fight_modes_round_trip_at_their_byte_bound_and_fail_closed() {
    for fight_mode in [
        FightMode::Offensive,
        FightMode::Balanced,
        FightMode::Defensive,
    ] {
        for chase in [ChaseMode::Stand, ChaseMode::Chase] {
            for secure in [false, true] {
                let modes = FightModes {
                    fight_mode,
                    chase,
                    secure,
                };
                let payload = encode_fight_modes_intent(&modes);
                assert!(payload.len() <= MAX_FIGHT_MODES_INTENT_BYTES);
                assert_eq!(decode_fight_modes_intent(&payload), Ok(modes));
            }
        }
    }
    let worst = encode_fight_modes_intent(&worst_state().modes);
    assert_eq!(worst.len(), MAX_FIGHT_MODES_INTENT_BYTES);

    let refused = |payload: &[u8]| {
        assert_eq!(
            decode_fight_modes_intent(payload),
            Err(AttackWireError::Malformed),
            "{payload:?}"
        );
    };
    // Empty, a missing or zero or unknown enum, a bool above 1, doubled and unknown fields.
    refused(&[]);
    refused(&[0x10, 0x01]);
    refused(&[0x08, 0x01]);
    refused(&[0x08, 0x00, 0x10, 0x01]);
    refused(&[0x08, 0x02, 0x10, 0x00]);
    refused(&[0x08, 0x04, 0x10, 0x01]);
    refused(&[0x08, 0x02, 0x10, 0x03]);
    refused(&[0x08, 0x02, 0x10, 0x01, 0x18, 0x02]);
    refused(&[0x08, 0x02, 0x08, 0x02, 0x10, 0x01]);
    refused(&[0x08, 0x02, 0x10, 0x01, 0x20, 0x01]);
    refused(&[0x0A, 0x00, 0x10, 0x01]);
    // An explicitly encoded false is false.
    assert_eq!(
        decode_fight_modes_intent(&[0x08, 0x02, 0x10, 0x01, 0x18, 0x00]),
        Ok(FightModes {
            fight_mode: FightMode::Balanced,
            chase: ChaseMode::Stand,
            secure: false,
        })
    );
    assert_eq!(
        decode_fight_modes_intent(&[0x08, 0x02, 0x10, 0x01, 0x18, 0x01, 0x00]),
        Err(AttackWireError::LimitExceeded)
    );
    assert_eq!(
        FightModes::DEFAULT,
        FightModes {
            fight_mode: FightMode::Balanced,
            chase: ChaseMode::Stand,
            secure: true,
        }
    );
}

#[test]
fn results_carry_every_disposition_and_fail_closed() {
    for disposition in AttackIntentDisposition::ALL {
        let payload = encode_attack_intent_result(disposition);
        assert!(payload.len() <= MAX_ATTACK_INTENT_RESULT_BYTES);
        assert_eq!(decode_attack_intent_result(&payload), Ok(disposition));
    }
    for payload in [&[][..], &[0x08, 0x00], &[0x08, 0x07], &[0x10, 0x01]] {
        assert!(decode_attack_intent_result(payload).is_err(), "{payload:?}");
    }
    assert!(decode_attack_intent_result(&[0x08, 0x01, 0x08, 0x01]).is_err());
}

#[test]
fn the_combat_state_round_trips_at_its_byte_bound_and_fails_closed() {
    let worst = encode_actor_combat_state(&worst_state()).expect("encode");
    assert_eq!(worst.len(), MAX_ACTOR_COMBAT_STATE_BYTES);
    assert_eq!(decode_actor_combat_state(&worst), Ok(worst_state()));

    let idle = ActorCombatState {
        target: None,
        modes: FightModes::DEFAULT,
        in_fight: false,
    };
    let payload = encode_actor_combat_state(&idle).expect("encode");
    assert_eq!(decode_actor_combat_state(&payload), Ok(idle));

    let mut zero = worst_state();
    zero.target = Some(target(0));
    assert_eq!(
        encode_actor_combat_state(&zero),
        Err(AttackWireError::Malformed)
    );

    let refused = |payload: &[u8]| {
        assert_eq!(
            decode_actor_combat_state(payload),
            Err(AttackWireError::Malformed),
            "{payload:?}"
        );
    };
    let modes = [0x10, 0x02, 0x18, 0x01];
    refused(&[]);
    refused(&[0x0A, 0x00, 0x10, 0x02, 0x18, 0x01]);
    let mut zero_generation = Vec::new();
    push_message_field(&mut zero_generation, 1, &raw_target(&[0xA5; 16], Some(0)));
    zero_generation.extend_from_slice(&modes);
    refused(&zero_generation);
    refused(&[0x10, 0x00, 0x18, 0x01]);
    refused(&[0x10, 0x02, 0x18, 0x00]);
    refused(&[0x10, 0x02, 0x18, 0x01, 0x20, 0x02]);
    refused(&[0x10, 0x02, 0x18, 0x01, 0x28, 0x02]);
    refused(&[0x10, 0x02, 0x18, 0x01, 0x28, 0x00, 0x28, 0x00]);
    refused(&[0x10, 0x02, 0x18, 0x01, 0x28, 0x01, 0x28, 0x01]);
    refused(&[0x10, 0x02, 0x10, 0x02, 0x18, 0x01]);
    refused(&[0x10, 0x02, 0x18, 0x01, 0x30, 0x01]);
    refused(&[0x08, 0x01, 0x10, 0x02, 0x18, 0x01]);
    // Two targets never fit the 39-byte bound.
    let one = encode_attack_target_intent(Some(&target(1))).expect("encode");
    let doubled_target = [&one[..], &one[..]].concat();
    assert_eq!(
        decode_actor_combat_state(&doubled_target),
        Err(AttackWireError::LimitExceeded)
    );
    // Explicitly encoded falses are false.
    assert_eq!(
        decode_actor_combat_state(&[0x10, 0x02, 0x18, 0x01, 0x20, 0x00, 0x28, 0x00]),
        Ok(ActorCombatState {
            target: None,
            modes: FightModes {
                fight_mode: FightMode::Balanced,
                chase: ChaseMode::Stand,
                secure: false,
            },
            in_fight: false,
        })
    );
    let mut over = worst.clone();
    over.push(0);
    assert_eq!(
        decode_actor_combat_state(&over),
        Err(AttackWireError::LimitExceeded)
    );
}

#[test]
fn the_registries_bind_the_attack_constants() {
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
    let capability = find("capabilities", CAPABILITY_ATTACK_V1);
    assert_eq!(capability["name"], "ATTACK_V1");
    assert_eq!(capability["offered"], false);
    assert!(
        capability["offer_gate"]
            .as_str()
            .expect("offer gate")
            .starts_with("ATTACK-1b")
    );
    assert_eq!(
        capability["requires"],
        serde_json::json!(CAPABILITY_ATTACK_V1_REQUIRES)
    );
    assert_eq!(
        capability["command_types"],
        serde_json::json!([
            COMMAND_TYPE_ATTACK_TARGET_INTENT,
            COMMAND_TYPE_FIGHT_MODES_INTENT
        ])
    );
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_ACTOR_COMBAT_STATE])
    );
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_ATTACK_V1));

    let schema = "docs/contracts/protocol-oteryn/v1/attack_v1.proto#";
    for (id, name, message, maximum) in [
        (
            COMMAND_TYPE_ATTACK_TARGET_INTENT,
            "ATTACK_TARGET_INTENT",
            "AttackTargetIntentV1",
            MAX_ATTACK_TARGET_INTENT_BYTES,
        ),
        (
            COMMAND_TYPE_FIGHT_MODES_INTENT,
            "FIGHT_MODES_INTENT",
            "FightModesIntentV1",
            MAX_FIGHT_MODES_INTENT_BYTES,
        ),
    ] {
        let command = find("command_types", id);
        assert_eq!(command["name"], name);
        assert_eq!(command["capability"], CAPABILITY_ATTACK_V1);
        assert_eq!(command["payload_schema"], format!("{schema}{message}"));
        assert_eq!(
            command["result_schema"],
            format!("{schema}AttackIntentResultV1")
        );
        assert_eq!(command["max_payload_bytes"].as_u64(), Some(maximum as u64));
        assert_eq!(
            command["max_result_payload_bytes"].as_u64(),
            Some(MAX_ATTACK_INTENT_RESULT_BYTES as u64)
        );
        assert!(ATTACK_PROTO.contains(&format!("message {message} {{")));
        assert!(ATTACK_PROTO.contains(&format!("At most {maximum} bytes")));
    }

    let domain = find("state_domains", STATE_DOMAIN_ACTOR_COMBAT_STATE);
    assert_eq!(domain["name"], "ACTOR_COMBAT_STATE");
    assert_eq!(domain["capability"], CAPABILITY_ATTACK_V1);
    for (key, type_id) in [
        ("delta_types", DELTA_TYPE_ACTOR_COMBAT_STATE_V1),
        ("snapshot_types", SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1),
    ] {
        let types = domain[key].as_array().expect(key);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0]["id"], type_id);
        assert_eq!(
            types[0]["max_payload_bytes"].as_u64(),
            Some(MAX_ACTOR_COMBAT_STATE_BYTES as u64)
        );
        assert_eq!(
            types[0]["payload_schema"],
            format!("{schema}ActorCombatStateV1")
        );
    }
    assert!(ATTACK_PROTO.contains("message AttackIntentResultV1 {"));
    assert!(ATTACK_PROTO.contains("message ActorCombatStateV1 {"));
    for maximum in [MAX_ATTACK_INTENT_RESULT_BYTES, MAX_ACTOR_COMBAT_STATE_BYTES] {
        assert!(ATTACK_PROTO.contains(&format!("At most {maximum} bytes")));
    }

    let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
    let limit = |row: &str| {
        resources["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["id"] == row)
            .and_then(|entry| entry["hard_maximum"].as_u64())
    };
    for (row, value) in [
        ("ATTACK0-RL-01", MAX_ATTACK_TARGET_CHANGES_PER_SECOND),
        ("ATTACK0-RL-02", MAX_FIGHT_MODE_CHANGES_PER_SECOND),
    ] {
        assert_eq!(limit(row), Some(u64::from(value)), "{row}");
        assert!(ATTACK_PROTO.contains(row));
    }
}
