#![allow(clippy::expect_used)]

use super::*;

#[test]
fn bed_sleep_requires_one_live_handle_and_one_of_six_skills() {
    for skill in 1..=6 {
        let intent = decode_bed_sleep_intent(&[10, 2, 8, 1, 16, skill]).expect("intent");
        assert_eq!(intent.target.get(), 1);
        assert_eq!(intent.skill, skill);
    }
    for payload in [
        vec![],
        vec![10, 2, 8, 0, 16, 1],
        vec![10, 2, 8, 1],
        vec![16, 1],
        vec![10, 2, 8, 1, 16, 0],
        vec![10, 2, 8, 1, 16, 7],
        vec![10, 2, 8, 1, 16, 1, 16, 1],
        vec![10, 2, 8, 1, 10, 2, 8, 1, 16, 1],
        vec![10, 2, 8, 1, 16, 1, 24, 1],
        vec![0; MAX_BED_SLEEP_INTENT_BYTES + 1],
    ] {
        assert!(decode_bed_sleep_intent(&payload).is_err(), "{payload:?}");
    }
}

#[test]
fn bed_sleep_full_u64_handle_and_payload_bound() {
    let mut payload = vec![10, 11, 8];
    payload.extend_from_slice(&[255; 9]);
    payload.extend_from_slice(&[1, 16, 6]);
    let intent = decode_bed_sleep_intent(&payload).expect("full uint64 handle");
    assert_eq!(intent.target.get(), u64::MAX);
    // The largest canonical message is 15 bytes. Redundant varint bytes keep
    // the numeric value unchanged and exercise the 24-byte accepted ceiling.
    payload.pop();
    payload.extend_from_slice(&[134, 128, 128, 128, 128, 128, 128, 128, 128, 0]);
    assert_eq!(payload.len(), MAX_BED_SLEEP_INTENT_BYTES);
    assert_eq!(decode_bed_sleep_intent(&payload), Ok(intent));
    payload.push(0);
    assert_eq!(
        decode_bed_sleep_intent(&payload),
        Err(WorldInteractionWireError::LimitExceeded)
    );
}

#[test]
fn bed_sleep_dispositions_are_world_interaction_results_and_never_push_or_rune_results() {
    use crate::world_interaction::{decode_push_result, encode_push_result};
    use crate::world_object::{UseDisposition, decode_use_result, encode_use_result};
    for (disposition, code) in [
        (UseDisposition::BedChooseSkill, 20),
        (UseDisposition::BedOccupied, 21),
        (UseDisposition::BedWoken, 22),
        (UseDisposition::OfflineTraining, 23),
    ] {
        let payload = encode_use_result(disposition).expect("use result");
        assert_eq!(payload, [8, code]);
        assert_eq!(decode_use_result(&payload), Ok(disposition));
        assert!(disposition.needs_world_interaction());
        assert!(!disposition.needs_rune_use());
        assert!(encode_push_result(disposition, None).is_err());
        assert!(decode_push_result(&payload).is_err());
    }
}

#[test]
fn bed_sleep_registry_and_schema_agree() {
    let registry: serde_json::Value =
        serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON).expect("registry");
    let command = registry["command_types"]
        .as_array()
        .expect("commands")
        .iter()
        .find(|row| row["id"] == COMMAND_TYPE_BED_SLEEP_INTENT)
        .expect("command 31");
    assert_eq!(
        command["capability"],
        crate::world_interaction::CAPABILITY_WORLD_INTERACTION_V1
    );
    assert_eq!(command["max_payload_bytes"], MAX_BED_SLEEP_INTENT_BYTES);
    assert_eq!(command["max_result_payload_bytes"], 4);
    let schema =
        include_str!("../../../docs/contracts/protocol-oteryn/v1/world_interaction_v1.proto");
    assert!(schema.contains("message BedSleepIntentV1 {"));
    assert!(schema.contains("oteryn.protocol.v1.item_view.ItemTargetV1 target = 1;"));
    assert!(schema.contains("uint32 skill = 2;"));
}

// TEST-FUZZ-1: bounded randomized decoder and generated-value round trips.

#[test]
fn fuzz_decode_bed_sleep_intent() {
    use crate::item_view::encode_item_target;
    use oteryn_test_support::property::for_each_case;
    let mut seeds = Vec::new();
    for_each_case("bed_sleep_generated", |rng, _| {
        let target = crate::item_view::ItemHandle::new(rng.u64().max(1)).expect("handle");
        let skill = (rng.below(6) + 1) as u8;
        let inner = encode_item_target(target);
        let mut bytes = vec![10, inner.len() as u8];
        bytes.extend(inner);
        bytes.extend([16, skill]);
        assert_eq!(
            decode_bed_sleep_intent(&bytes),
            Ok(BedSleepIntent { target, skill })
        );
        if seeds.len() < 16 {
            seeds.push(bytes);
        }
        Ok(())
    });
    seeds.push(vec![10, 2, 8, 1, 16, 1]);
    crate::decoder_fuzz_support::decoder("decode_bed_sleep_intent", seeds, |b| {
        decode_bed_sleep_intent(b).map(|_| ())
    });
}
