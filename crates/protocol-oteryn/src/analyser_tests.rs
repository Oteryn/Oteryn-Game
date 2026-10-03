#![allow(clippy::expect_used)]

use std::num::{NonZeroU16, NonZeroU32};

use serde_json::Value;

use super::*;
use crate::damage_element::DamageElement;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
const SCHEMA_PATH: &str = "docs/contracts/protocol-oteryn/v1/analyser_v1.proto";
const SCHEMA: &str = include_str!("../../../docs/contracts/protocol-oteryn/v1/analyser_v1.proto");

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap_or(NonZeroU32::MIN)
}

fn item(item_type: u32, count: u16) -> AnalyserItemCount {
    AnalyserItemCount {
        item_type: nz(item_type),
        count: NonZeroU16::new(count).unwrap_or(NonZeroU16::MIN),
    }
}

/// The largest fact of every kind: a full `kill_loot` with every field at its widest.
fn worst_kill_loot() -> AnalyserFact {
    AnalyserFact::KillLoot {
        race: Some(nz(MAX_BESTIARY_RACE)),
        corpse_items: vec![item(u32::MAX, u16::MAX); MAX_ANALYSER_LOOT_ITEMS],
        gold: u64::MAX,
    }
}

fn every_kind() -> Vec<AnalyserFact> {
    vec![
        AnalyserFact::Experience {
            raw: 1_200,
            gained: 1_800,
        },
        AnalyserFact::Experience { raw: 1, gained: 0 },
        worst_kill_loot(),
        AnalyserFact::KillLoot {
            race: None,
            corpse_items: Vec::new(),
            gold: 0,
        },
        AnalyserFact::SupplyUsed(item(268, 1)),
        AnalyserFact::Impact(AnalyserImpact::Damage {
            value: nz(250),
            element: DamageElement::Ice,
        }),
        AnalyserFact::Impact(AnalyserImpact::Healing(nz(90))),
        AnalyserFact::DamageInput {
            value: nz(40),
            element: DamageElement::Physical,
            source: AnalyserDamageSource::Creature(Some(nz(12))),
        },
        AnalyserFact::DamageInput {
            value: nz(40),
            element: DamageElement::Fire,
            source: AnalyserDamageSource::Creature(None),
        },
        AnalyserFact::DamageInput {
            value: nz(7),
            element: DamageElement::Untyped,
            source: AnalyserDamageSource::Player,
        },
        AnalyserFact::DamageInput {
            value: nz(7),
            element: DamageElement::Drowning,
            source: AnalyserDamageSource::None,
        },
        AnalyserFact::Dropped(nz(3)),
    ]
}

#[test]
fn every_fact_kind_round_trips_alone_and_in_a_batch() {
    let facts = every_kind();
    for fact in &facts {
        let encoded = encode_analyser_fact(fact).expect("encode");
        assert_eq!(decode_analyser_fact(&encoded), Ok(fact.clone()));
    }
    let batch = encode_analyser_facts(&facts).expect("batch");
    assert_eq!(decode_analyser_facts(&batch), Ok(facts));
    assert_eq!(decode_analyser_snapshot(&[]), Ok(()));
    assert_eq!(
        decode_analyser_snapshot(&[0x08, 0x01]),
        Err(AnalyserWireError::Malformed)
    );
}

/// RL-03 measurement: the worst fact is the full `kill_loot`, 64 of them exceed 4 KiB, and 64 of
/// any other kind fit.
// RL-03 is at most 4 KiB, and 64 worst-case facts would exceed it: the builder splits by bytes.
const _: () = assert!(MAX_ANALYSER_BATCH_BYTES <= 4 * 1024);
const _: () =
    assert!(MAX_ANALYSER_BATCH_FACTS * (MAX_ANALYSER_FACT_BYTES + 3) > MAX_ANALYSER_BATCH_BYTES);

#[test]
fn the_largest_fact_is_measured_and_batches_stay_within_four_kib() {
    let worst = encode_analyser_fact(&worst_kill_loot()).expect("worst");
    assert_eq!(worst.len(), MAX_ANALYSER_FACT_BYTES);
    let widest_other = [
        AnalyserFact::Experience {
            raw: u64::MAX,
            gained: u64::MAX,
        },
        AnalyserFact::DamageInput {
            value: nz(u32::MAX),
            element: DamageElement::Untyped,
            source: AnalyserDamageSource::Creature(Some(nz(MAX_BESTIARY_RACE))),
        },
    ];
    for fact in widest_other {
        let full = vec![fact; MAX_ANALYSER_BATCH_FACTS];
        let batch = encode_analyser_facts(&full).expect("64 non-loot facts fit");
        assert!(batch.len() <= MAX_ANALYSER_BATCH_BYTES);
    }
    let too_many_bytes = vec![worst_kill_loot(); 11];
    assert_eq!(
        encode_analyser_facts(&too_many_bytes),
        Err(AnalyserWireError::LimitExceeded)
    );
    let mut oversized = [0x0a, 0x00].repeat(MAX_ANALYSER_BATCH_BYTES / 2);
    oversized.push(0x0a);
    assert_eq!(
        decode_analyser_facts(&oversized),
        Err(AnalyserWireError::LimitExceeded)
    );
}

#[test]
fn batch_and_item_bounds_fail_closed_both_ways() {
    let small = AnalyserFact::Dropped(nz(1));
    let max = vec![small.clone(); MAX_ANALYSER_BATCH_FACTS];
    let encoded = encode_analyser_facts(&max).expect("64 facts");
    assert_eq!(decode_analyser_facts(&encoded), Ok(max));
    let over = vec![small; MAX_ANALYSER_BATCH_FACTS + 1];
    assert_eq!(
        encode_analyser_facts(&over),
        Err(AnalyserWireError::LimitExceeded)
    );
    let mut raw = encoded.clone();
    raw.extend_from_slice(&encoded[..4]);
    assert_eq!(
        decode_analyser_facts(&raw),
        Err(AnalyserWireError::LimitExceeded)
    );
    assert_eq!(
        encode_analyser_facts(&[]),
        Err(AnalyserWireError::Malformed)
    );
    assert_eq!(
        decode_analyser_facts(&[]),
        Err(AnalyserWireError::Malformed)
    );

    let thirty_three = AnalyserFact::KillLoot {
        race: None,
        corpse_items: vec![item(1, 1); MAX_ANALYSER_LOOT_ITEMS + 1],
        gold: 0,
    };
    assert_eq!(
        encode_analyser_fact(&thirty_three),
        Err(AnalyserWireError::LimitExceeded)
    );
    // A 33rd item (field 2, 4 bytes) appended to a full but narrow kill_loot.
    let narrow = AnalyserFact::KillLoot {
        race: None,
        corpse_items: vec![item(1, 1); MAX_ANALYSER_LOOT_ITEMS],
        gold: 0,
    };
    // 32 items of 6 bytes: the oneof tag and a 2-byte length precede the 192-byte message.
    let mut inner = encode_analyser_fact(&narrow).expect("32 items")[3..].to_vec();
    inner.extend_from_slice(&[0x12, 0x04, 0x08, 0x01, 0x10, 0x01]);
    let len = inner.len();
    let mut fact = vec![0x12, (len as u8 & 0x7f) | 0x80, (len >> 7) as u8];
    fact.extend(inner);
    assert_eq!(
        decode_analyser_fact(&fact),
        Err(AnalyserWireError::LimitExceeded)
    );
    let race_over = AnalyserFact::KillLoot {
        race: Some(nz(MAX_BESTIARY_RACE + 1)),
        corpse_items: Vec::new(),
        gold: 0,
    };
    assert_eq!(
        encode_analyser_fact(&race_over),
        Err(AnalyserWireError::LimitExceeded)
    );
}

#[test]
fn malformed_facts_fail_closed() {
    let malformed = [
        &[][..],                                                       // empty oneof
        &[0x0a, 0x00, 0x0a, 0x00], // two experience facts in one oneof
        &[0x3a, 0x00],             // unknown oneof field 7
        &[0x08, 0x01],             // a varint where a message belongs
        &[0x1a, 0x02, 0x10, 0x01], // supply_used without item_type
        &[0x1a, 0x04, 0x08, 0x01, 0x10, 0x00], // supply count 0
        &[0x22, 0x02, 0x10, 0x05], // impact without kind
        &[0x22, 0x04, 0x08, 0x01, 0x10, 0x05], // damage impact without element
        &[0x22, 0x06, 0x08, 0x02, 0x10, 0x05, 0x18, 0x02], // healing with an element
        &[0x22, 0x06, 0x08, 0x01, 0x10, 0x00, 0x18, 0x02], // value 0
        &[0x22, 0x06, 0x08, 0x01, 0x10, 0x05, 0x18, 0x0c], // unknown element 12
        &[0x22, 0x06, 0x08, 0x03, 0x10, 0x05, 0x18, 0x02], // unknown kind 3
        &[0x2a, 0x08, 0x08, 0x05, 0x10, 0x01, 0x18, 0x02, 0x20, 0x01], // race on a player source
        &[0x2a, 0x08, 0x08, 0x05, 0x10, 0x01, 0x18, 0x03, 0x20, 0x01], // race on no source
        &[0x2a, 0x06, 0x08, 0x05, 0x10, 0x01, 0x18, 0x04], // unknown source 4
        &[0x2a, 0x04, 0x08, 0x05, 0x10, 0x01], // no source
        &[0x0a, 0x02, 0x10, 0x05], // experience with raw 0
        &[0x32, 0x00],             // dropped count 0
        &[0x0a, 0x04, 0x08, 0x01, 0x08, 0x01], // experience raw twice
        &[0x0a, 0x02, 0x18, 0x01], // unknown experience field
        &[0x12, 0x04, 0x18, 0x01, 0x18, 0x01], // gold twice
    ];
    for payload in malformed {
        assert_eq!(
            decode_analyser_fact(payload),
            Err(AnalyserWireError::Malformed),
            "{payload:02x?}"
        );
    }
    // A race above the Bestiary bound and a count above u16 are over their bounds.
    for payload in [
        &[
            0x2a, 0x09, 0x08, 0x05, 0x10, 0x01, 0x18, 0x01, 0x20, 0x81, 0x08,
        ][..],
        &[0x1a, 0x06, 0x08, 0x01, 0x10, 0x80, 0x80, 0x04][..],
    ] {
        assert_eq!(
            decode_analyser_fact(payload),
            Err(AnalyserWireError::LimitExceeded),
            "{payload:02x?}"
        );
    }
}

#[test]
fn pending_queue_drops_the_oldest_and_reports_the_count() {
    let mut queue = AnalyserPendingQueue::default();
    assert!(queue.is_empty());
    assert_eq!(queue.take_batch(), None);
    let fact = |n: u64| AnalyserFact::Experience {
        raw: n + 1,
        gained: n,
    };
    for raw in 0..MAX_ANALYSER_PENDING_FACTS as u64 {
        queue.push(&fact(raw)).expect("push");
    }
    assert_eq!(queue.len(), MAX_ANALYSER_PENDING_FACTS);
    queue.push(&fact(5_000)).expect("push");
    queue.push(&fact(5_001)).expect("push");
    assert_eq!(queue.len(), MAX_ANALYSER_PENDING_FACTS);

    let first = decode_analyser_facts(&queue.take_batch().expect("batch")).expect("decode");
    assert_eq!(first.len(), MAX_ANALYSER_BATCH_FACTS);
    assert_eq!(first[0], AnalyserFact::Dropped(nz(2)));
    assert_eq!(first[1], fact(2), "facts 0 and 1 were dropped");
    let second = decode_analyser_facts(&queue.take_batch().expect("batch")).expect("decode");
    assert_eq!(second[0], fact(65), "dropped is reported once");
    assert!(queue.push(&worst_kill_loot()).is_ok());
    assert_eq!(
        encode_analyser_fact(&AnalyserFact::Experience { raw: 0, gained: 5 }),
        Err(AnalyserWireError::Malformed)
    );
    let invalid = AnalyserFact::KillLoot {
        race: None,
        corpse_items: vec![item(1, 1); MAX_ANALYSER_LOOT_ITEMS + 1],
        gold: 0,
    };
    let before = queue.len();
    assert_eq!(queue.push(&invalid), Err(AnalyserWireError::LimitExceeded));
    assert_eq!(queue.len(), before);
}

#[test]
fn pending_queue_splits_worst_case_batches_by_bytes_and_never_drops_them() {
    let mut queue = AnalyserPendingQueue::default();
    for _ in 0..MAX_ANALYSER_BATCH_FACTS {
        queue.push(&worst_kill_loot()).expect("push");
    }
    let mut seen = 0;
    while let Some(batch) = queue.take_batch() {
        assert!(batch.len() <= MAX_ANALYSER_BATCH_BYTES, "{}", batch.len());
        let facts = decode_analyser_facts(&batch).expect("decode");
        assert!(facts.iter().all(|fact| *fact == worst_kill_loot()));
        seen += facts.len();
    }
    assert_eq!(seen, MAX_ANALYSER_BATCH_FACTS);
    assert!(queue.is_empty());
}

#[test]
fn registries_bind_the_analyser_ids_and_limits() {
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
    let capability = only("capabilities", CAPABILITY_ANALYSER_V1);
    assert_eq!(capability["name"], "ANALYSER_V1");
    assert_eq!(
        capability["offered"], false,
        "not offered before ANALYSER-EMIT-1"
    );
    assert_eq!(capability["command_types"], serde_json::json!([]));
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_ACTOR_ANALYSER])
    );
    let domain = only("state_domains", STATE_DOMAIN_ACTOR_ANALYSER);
    assert_eq!(domain["name"], "ACTOR_ANALYSER");
    assert_eq!(domain["capability"], CAPABILITY_ANALYSER_V1);
    let entry = |section: &str| {
        let entries = domain[section].as_array().expect(section);
        assert_eq!(entries.len(), 1, "{section}");
        entries[0].clone()
    };
    let delta = entry("delta_types");
    assert_eq!(delta["id"], DELTA_TYPE_ANALYSER_FACTS_V1);
    assert_eq!(delta["name"], "ANALYSER_FACTS_DELTA_V1");
    assert_eq!(
        delta["payload_schema"],
        format!("{SCHEMA_PATH}#AnalyserFactsV1")
    );
    assert_eq!(delta["max_payload_bytes"], MAX_ANALYSER_BATCH_BYTES as u64);
    let snapshot = entry("snapshot_types");
    assert_eq!(snapshot["id"], SNAPSHOT_TYPE_ANALYSER_V1);
    assert_eq!(snapshot["name"], "ANALYSER_SNAPSHOT_V1");
    assert_eq!(
        snapshot["payload_schema"],
        format!("{SCHEMA_PATH}#AnalyserSnapshotV1")
    );
    assert_eq!(snapshot["max_payload_bytes"], 0);
    for message in [
        "AnalyserExperienceV1",
        "AnalyserItemCountV1",
        "AnalyserKillLootV1",
        "AnalyserImpactV1",
        "AnalyserDamageInputV1",
        "AnalyserDroppedV1",
        "AnalyserFactV1",
        "AnalyserFactsV1",
        "AnalyserSnapshotV1 {}",
    ] {
        assert!(SCHEMA.contains(&format!("message {message}")), "{message}");
    }
    assert!(SCHEMA.contains(&format!("At most {MAX_ANALYSER_FACT_BYTES} bytes")));
    assert!(SCHEMA.contains(&format!("At most {MAX_ANALYSER_BATCH_BYTES} bytes")));

    let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
    let entries = resources["entries"].as_array().expect("entries");
    let limit = |id: &str| {
        let matches: Vec<&Value> = entries.iter().filter(|entry| entry["id"] == id).collect();
        assert_eq!(matches.len(), 1, "{id} registered once");
        matches[0]["hard_maximum"].as_u64()
    };
    assert_eq!(
        limit("ANALYSERS0-RL-01"),
        Some(MAX_ANALYSER_BATCH_FACTS as u64)
    );
    assert_eq!(
        limit("ANALYSERS0-RL-02"),
        Some(MAX_ANALYSER_LOOT_ITEMS as u64)
    );
    assert_eq!(
        limit("ANALYSERS0-RL-03"),
        Some(MAX_ANALYSER_BATCH_BYTES as u64)
    );
    assert_eq!(
        limit("ANALYSERS0-RL-04"),
        Some(MAX_ANALYSER_PENDING_FACTS as u64)
    );
}
