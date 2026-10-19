#![allow(clippy::expect_used)]

use super::*;
use crate::charm_wire::{push_message_field, push_varint_field};
use serde_json::Value;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
const VIP_PROTO: &str = include_str!("../../../docs/contracts/protocol-oteryn/v1/vip_v1.proto");

const WIDE: char = '\u{1F600}';

fn raw(fields: &[(u64, u64)]) -> Vec<u8> {
    let mut output = Vec::new();
    for &(field, value) in fields {
        push_varint_field(&mut output, field, value);
    }
    output
}

fn wrap(field: u64, inner: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    push_message_field(&mut output, field, inner);
    output
}

fn wide(chars: usize) -> String {
    std::iter::repeat_n(WIDE, chars).collect()
}

fn name(byte: u8) -> String {
    std::iter::repeat_n(char::from(byte), MAX_VIP_NAME_BYTES).collect()
}

fn worst_entry(entry_no: u64) -> VipEntry {
    VipEntry {
        entry_no,
        name: name(b'A'),
        description: wide(MAX_VIP_DESCRIPTION_CHARS),
        icon: MAX_VIP_ICON,
        notify: true,
        groups: VipGroupSet::of(&[1, 2, 3, 4, 5, 6, 7, 8]).expect("groups"),
        status: VipStatus::Unconfirmed,
    }
}

fn worst_groups() -> Vec<VipGroup> {
    (FIRST_VIP_CUSTOM_GROUP..=LAST_VIP_GROUP)
        .map(|group_no| VipGroup {
            group_no,
            name: wide(MAX_VIP_GROUP_NAME_CHARS),
        })
        .collect()
}

fn worst_consent() -> VipConsentView {
    VipConsentView {
        invitations: (0..MAX_VIP_INVITATIONS as u64)
            .map(|index| VipInvitation {
                handle: u64::MAX - 1_000 + index,
                inviter_name: name(b'B'),
                expires_at_ms: u64::MAX,
            })
            .collect(),
        watchers: (0..MAX_VIP_WATCHERS as u64)
            .map(|index| VipWatcher {
                handle: u64::MAX - 1_000 + index,
                name: name(b'C'),
            })
            .collect(),
        visibility: VipVisibility::Hidden,
    }
}

fn worst_snapshot() -> VipSnapshot {
    VipSnapshot {
        groups: worst_groups(),
        entries: (0..MAX_VIP_ENTRIES as u64)
            .map(|index| worst_entry(u64::MAX - 1_000 + index))
            .collect(),
        consent: worst_consent(),
    }
}

#[test]
fn every_intent_round_trips() {
    let intents = [
        VipIntent::Add {
            name: "Ana Lis".to_owned(),
        },
        VipIntent::Remove { entry_no: 7 },
        VipIntent::Edit {
            entry_no: 1,
            description: String::new(),
            icon: 0,
            notify: false,
            groups: VipGroupSet::default(),
        },
        VipIntent::Edit {
            entry_no: 2,
            description: "friend".to_owned(),
            icon: 3,
            notify: true,
            groups: VipGroupSet::of(&[1, 4]).expect("groups"),
        },
        VipIntent::GroupAdd {
            name: "Guild".to_owned(),
        },
        VipIntent::GroupRename {
            group_no: 8,
            name: "Hunt".to_owned(),
        },
        VipIntent::GroupRemove { group_no: 4 },
        VipIntent::Accept { invitation: 1 },
        VipIntent::Reject { invitation: 2 },
        VipIntent::Revoke { watcher: 3 },
        VipIntent::Withdraw { entry_no: 4 },
        VipIntent::SetStatus(VipVisibility::Shown),
        VipIntent::SetStatus(VipVisibility::Hidden),
    ];
    for intent in intents {
        let encoded = encode_vip_intent(&intent).expect("encodes");
        assert!(encoded.len() <= MAX_VIP_INTENT_BYTES, "{intent:?}");
        assert_eq!(decode_vip_intent(&encoded), Ok(intent));
    }
    assert_eq!(
        encode_vip_intent(&VipIntent::Remove { entry_no: 5 }).expect("encodes"),
        wrap(2, &raw(&[(1, 5)]))
    );
    assert_eq!(
        VipGroupSet::of(&[1, 4, 8]).expect("groups").groups(),
        vec![1, 4, 8]
    );
    assert_eq!(VipGroupSet::of(&[0]), None);
    assert_eq!(VipGroupSet::of(&[9]), None);
}

#[test]
fn every_result_code_round_trips() {
    for code in VipResultCode::ALL {
        let result = if code == VipResultCode::Exhausted {
            VipIntentResult {
                code,
                wait_seconds: MAX_VIP_WAIT_SECONDS,
            }
        } else {
            VipIntentResult::of(code)
        };
        let encoded = encode_vip_intent_result(result).expect("encodes");
        assert!(encoded.len() <= MAX_VIP_RESULT_BYTES);
        assert_eq!(decode_vip_intent_result(&encoded), Ok(result));
        assert_eq!(result.is_rejected(), code != VipResultCode::Ok);
    }
    assert_eq!(
        encode_vip_intent_result(VipIntentResult {
            code: VipResultCode::Exhausted,
            wait_seconds: 3,
        })
        .expect("encodes")
        .len(),
        MAX_VIP_RESULT_BYTES
    );
}

#[test]
fn snapshots_and_deltas_round_trip() {
    let snapshot = VipSnapshot {
        groups: vec![VipGroup {
            group_no: 4,
            name: "Hunt".to_owned(),
        }],
        entries: vec![
            VipEntry {
                entry_no: 1,
                name: "Ana".to_owned(),
                description: String::new(),
                icon: 0,
                notify: false,
                groups: VipGroupSet::default(),
                status: VipStatus::Offline,
            },
            VipEntry {
                entry_no: 3,
                name: "Bo".to_owned(),
                description: "x".to_owned(),
                icon: 1,
                notify: true,
                groups: VipGroupSet::of(&[4]).expect("groups"),
                status: VipStatus::Online,
            },
        ],
        consent: VipConsentView {
            invitations: vec![VipInvitation {
                handle: 1,
                inviter_name: "Cy".to_owned(),
                expires_at_ms: 1_800_000_000_000,
            }],
            watchers: vec![VipWatcher {
                handle: 2,
                name: "Di".to_owned(),
            }],
            visibility: VipVisibility::Shown,
        },
    };
    let encoded = encode_vip_snapshot(&snapshot).expect("encodes");
    assert_eq!(decode_vip_snapshot(&encoded), Ok(snapshot.clone()));
    let empty = encode_vip_snapshot(&VipSnapshot::default()).expect("encodes");
    assert_eq!(decode_vip_snapshot(&empty), Ok(VipSnapshot::default()));

    for delta in [
        VipDelta::Entry(snapshot.entries[1].clone()),
        VipDelta::EntryRemoved { entry_no: 9 },
        VipDelta::Groups(Vec::new()),
        VipDelta::Groups(snapshot.groups.clone()),
        VipDelta::Status(VipStatusUpdate {
            entry_no: 1,
            status: VipStatus::Online,
            announce: true,
        }),
        VipDelta::Status(VipStatusUpdate {
            entry_no: 1,
            status: VipStatus::Unconfirmed,
            announce: false,
        }),
        VipDelta::Consent(snapshot.consent.clone()),
    ] {
        let encoded = encode_vip_delta(&delta).expect("encodes");
        assert_eq!(decode_vip_delta(delta.delta_type(), &encoded), Ok(delta));
    }
}

#[test]
fn worst_cases_fill_their_bounds() {
    let edit = VipIntent::Edit {
        entry_no: u64::MAX,
        description: wide(MAX_VIP_DESCRIPTION_CHARS),
        icon: MAX_VIP_ICON,
        notify: true,
        groups: VipGroupSet::of(&[1, 2, 3, 4, 5, 6, 7, 8]).expect("groups"),
    };
    let encoded = encode_vip_intent(&edit).expect("encodes");
    assert_eq!(encoded.len(), MAX_VIP_INTENT_BYTES);
    assert_eq!(decode_vip_intent(&encoded), Ok(edit));

    let cases = [
        (VipDelta::Entry(worst_entry(u64::MAX)), MAX_VIP_ENTRY_BYTES),
        (
            VipDelta::EntryRemoved { entry_no: u64::MAX },
            MAX_VIP_ENTRY_REMOVED_BYTES,
        ),
        (VipDelta::Groups(worst_groups()), MAX_VIP_GROUPS_BYTES),
        (
            VipDelta::Status(VipStatusUpdate {
                entry_no: u64::MAX,
                status: VipStatus::Unconfirmed,
                announce: true,
            }),
            MAX_VIP_STATUS_BYTES,
        ),
        (VipDelta::Consent(worst_consent()), MAX_VIP_CONSENT_BYTES),
    ];
    for (delta, bound) in cases {
        let encoded = encode_vip_delta(&delta).expect("encodes");
        assert_eq!(encoded.len(), bound, "{}", delta.delta_type());
        assert_eq!(decode_vip_delta(delta.delta_type(), &encoded), Ok(delta));
    }

    let snapshot = worst_snapshot();
    let encoded = encode_vip_snapshot(&snapshot).expect("encodes");
    assert_eq!(encoded.len(), MAX_VIP_SNAPSHOT_BYTES);
    assert_eq!(decode_vip_snapshot(&encoded), Ok(snapshot));
}

#[test]
fn over_limit_lists_and_text_fail_closed() {
    assert_eq!(
        encode_vip_intent(&VipIntent::Add {
            name: "A".repeat(MAX_VIP_NAME_BYTES + 1),
        }),
        Err(VipWireError::LimitExceeded)
    );
    let mut edit = VipIntent::Edit {
        entry_no: 1,
        description: "d".repeat(MAX_VIP_DESCRIPTION_CHARS + 1),
        icon: 0,
        notify: false,
        groups: VipGroupSet::default(),
    };
    assert_eq!(encode_vip_intent(&edit), Err(VipWireError::LimitExceeded));
    if let VipIntent::Edit {
        description, icon, ..
    } = &mut edit
    {
        description.clear();
        *icon = MAX_VIP_ICON + 1;
    }
    assert_eq!(encode_vip_intent(&edit), Err(VipWireError::LimitExceeded));
    assert_eq!(
        decode_vip_intent(&wrap(3, &raw(&[(1, 1), (3, 11)]))),
        Err(VipWireError::LimitExceeded)
    );
    assert_eq!(
        encode_vip_intent(&VipIntent::GroupAdd {
            name: "g".repeat(MAX_VIP_GROUP_NAME_CHARS + 1),
        }),
        Err(VipWireError::LimitExceeded)
    );

    let snapshot = VipSnapshot {
        entries: (1..=MAX_VIP_ENTRIES as u64 + 1)
            .map(|entry_no| VipEntry {
                entry_no,
                name: "N".to_owned(),
                description: String::new(),
                icon: 0,
                notify: false,
                groups: VipGroupSet::default(),
                status: VipStatus::Offline,
            })
            .collect(),
        ..VipSnapshot::default()
    };
    assert_eq!(
        encode_vip_snapshot(&snapshot),
        Err(VipWireError::LimitExceeded)
    );
    let mut groups = worst_groups();
    groups.push(VipGroup {
        group_no: LAST_VIP_GROUP,
        name: "x".to_owned(),
    });
    assert_eq!(
        encode_vip_delta(&VipDelta::Groups(groups)),
        Err(VipWireError::LimitExceeded)
    );

    let mut consent = worst_consent();
    consent.invitations.push(VipInvitation {
        handle: u64::MAX,
        inviter_name: "X".to_owned(),
        expires_at_ms: 1,
    });
    assert_eq!(
        encode_vip_delta(&VipDelta::Consent(consent)),
        Err(VipWireError::LimitExceeded)
    );
    let mut consent = worst_consent();
    consent.watchers.push(VipWatcher {
        handle: u64::MAX,
        name: "X".to_owned(),
    });
    assert_eq!(
        encode_vip_delta(&VipDelta::Consent(consent)),
        Err(VipWireError::LimitExceeded)
    );

    // A payload above its bound is refused before any field is read.
    assert_eq!(
        decode_vip_intent(&vec![0; MAX_VIP_INTENT_BYTES + 1]),
        Err(VipWireError::LimitExceeded)
    );
    assert_eq!(
        decode_vip_snapshot(&vec![0; MAX_VIP_SNAPSHOT_BYTES + 1]),
        Err(VipWireError::LimitExceeded)
    );
    assert_eq!(
        decode_vip_delta(DELTA_TYPE_VIP_STATUS_V1, &[0; MAX_VIP_STATUS_BYTES + 1]),
        Err(VipWireError::LimitExceeded)
    );
    assert_eq!(
        decode_vip_intent_result(&[0; 5]),
        Err(VipWireError::LimitExceeded)
    );
}

#[test]
fn malformed_payloads_fail_closed() {
    // No intent, two intents, an unknown intent and a scalar where a message belongs.
    assert_eq!(decode_vip_intent(&[]), Err(VipWireError::Malformed));
    let mut two = wrap(2, &raw(&[(1, 1)]));
    two.extend(wrap(10, &raw(&[(1, 1)])));
    assert_eq!(decode_vip_intent(&two), Err(VipWireError::Malformed));
    assert_eq!(
        decode_vip_intent(&wrap(12, &[])),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&raw(&[(2, 1)])),
        Err(VipWireError::Malformed)
    );
    // Zero numbers, a fixed group, an unknown enum, a repeated or unknown field, a bad bool.
    assert_eq!(
        decode_vip_intent(&wrap(2, &[])),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(7, &raw(&[(1, 0)]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(6, &raw(&[(1, 3)]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(6, &raw(&[(1, 9)]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(11, &raw(&[(1, 0)]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(11, &raw(&[(1, 3)]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(2, &raw(&[(1, 1), (1, 2)]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(2, &raw(&[(1, 1), (2, 2)]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(3, &raw(&[(1, 1), (4, 2)]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(3, &raw(&[(1, 1), (5, 256)]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(1, &[])),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_intent(&wrap(1, &wrap(1, &[0xff]))),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        encode_vip_intent(&VipIntent::GroupRemove { group_no: 3 }),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        encode_vip_intent(&VipIntent::Accept { invitation: 0 }),
        Err(VipWireError::Malformed)
    );

    // Results: the wait comes with EXHAUSTED only, within 1..=3 s; codes are 1..=10.
    for fields in [
        &[][..],
        &[(1, 0)],
        &[(1, 11)],
        &[(1, 8)],
        &[(1, 8), (2, 4)],
        &[(1, 1), (2, 1)],
        &[(1, 1), (1, 1)],
        &[(1, 1), (3, 1)],
    ] {
        assert_eq!(
            decode_vip_intent_result(&raw(fields)),
            Err(VipWireError::Malformed),
            "{fields:?}"
        );
    }
    assert_eq!(
        encode_vip_intent_result(VipIntentResult::of(VipResultCode::Exhausted)),
        Err(VipWireError::Malformed)
    );

    // Domain 21: TRAINING is never accepted; statuses and entries need their required fields.
    assert_eq!(
        decode_vip_delta(DELTA_TYPE_VIP_STATUS_V1, &raw(&[(1, 1), (2, 4)])),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_delta(DELTA_TYPE_VIP_STATUS_V1, &raw(&[(1, 1)])),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_delta(DELTA_TYPE_VIP_STATUS_V1, &raw(&[(1, 1), (2, 2), (3, 2)])),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_delta(DELTA_TYPE_VIP_ENTRY_REMOVED_V1, &[]),
        Err(VipWireError::Malformed)
    );
    assert_eq!(decode_vip_delta(6, &[]), Err(VipWireError::Malformed));
    assert_eq!(decode_vip_delta(0, &[]), Err(VipWireError::Malformed));
    assert_eq!(
        decode_vip_delta(DELTA_TYPE_VIP_ENTRY_V1, &raw(&[(1, 1), (7, 1)])),
        Err(VipWireError::Malformed)
    );
    assert_eq!(
        decode_vip_delta(DELTA_TYPE_VIP_CONSENT_V1, &[]),
        Err(VipWireError::Malformed)
    );

    // Lists are in strictly ascending number.
    let entry = VipEntry {
        entry_no: 2,
        name: "N".to_owned(),
        description: String::new(),
        icon: 0,
        notify: false,
        groups: VipGroupSet::default(),
        status: VipStatus::Offline,
    };
    let snapshot = VipSnapshot {
        entries: vec![entry.clone(), entry.clone()],
        ..VipSnapshot::default()
    };
    assert_eq!(encode_vip_snapshot(&snapshot), Err(VipWireError::Malformed));
    let mut snapshot_bytes = Vec::new();
    for _ in 0..2 {
        push_message_field(
            &mut snapshot_bytes,
            2,
            &encode_vip_delta(&VipDelta::Entry(entry.clone())).expect("encodes"),
        );
    }
    push_message_field(
        &mut snapshot_bytes,
        3,
        &encode_vip_delta(&VipDelta::Consent(VipConsentView::default())).expect("encodes"),
    );
    assert_eq!(
        decode_vip_snapshot(&snapshot_bytes),
        Err(VipWireError::Malformed)
    );
    let groups = vec![
        VipGroup {
            group_no: 5,
            name: "a".to_owned(),
        },
        VipGroup {
            group_no: 4,
            name: "b".to_owned(),
        },
    ];
    assert_eq!(
        encode_vip_delta(&VipDelta::Groups(groups)),
        Err(VipWireError::Malformed)
    );
    let consent = VipConsentView {
        watchers: vec![
            VipWatcher {
                handle: 2,
                name: "a".to_owned(),
            },
            VipWatcher {
                handle: 1,
                name: "b".to_owned(),
            },
        ],
        ..VipConsentView::default()
    };
    assert_eq!(
        encode_vip_delta(&VipDelta::Consent(consent)),
        Err(VipWireError::Malformed)
    );
    // A snapshot without its consent view.
    assert_eq!(decode_vip_snapshot(&[]), Err(VipWireError::Malformed));
}

#[test]
fn the_registries_bind_the_vip_constants() {
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
    let capability = find("capabilities", CAPABILITY_VIP_V1);
    assert_eq!(capability["name"], "VIP_V1");
    assert_eq!(capability["offered"], true);
    assert_eq!(capability["requires"], serde_json::json!([]));
    assert_eq!(
        capability["command_types"],
        serde_json::json!([COMMAND_TYPE_VIP_INTENT])
    );
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_VIP])
    );
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_VIP_V1));
    assert!(
        protocol["allocation_ranges"]["capability_id"]["next_unallocated"]
            .as_u64()
            .is_some_and(|next| next > u64::from(CAPABILITY_VIP_V1))
    );

    let schema = "docs/contracts/protocol-oteryn/v1/vip_v1.proto#";
    let command = find("command_types", COMMAND_TYPE_VIP_INTENT);
    assert_eq!(command["name"], "VIP_INTENT");
    assert_eq!(command["capability"], CAPABILITY_VIP_V1);
    assert_eq!(command["payload_schema"], format!("{schema}VipIntentV1"));
    assert_eq!(
        command["result_schema"],
        format!("{schema}VipIntentResultV1")
    );
    assert_eq!(
        command["max_payload_bytes"].as_u64(),
        Some(MAX_VIP_INTENT_BYTES as u64)
    );
    assert_eq!(
        command["max_result_payload_bytes"].as_u64(),
        Some(MAX_VIP_RESULT_BYTES as u64)
    );

    let domain = find("state_domains", STATE_DOMAIN_VIP);
    assert_eq!(domain["name"], "VIP");
    assert_eq!(domain["capability"], CAPABILITY_VIP_V1);
    let typed = |list: &str, id: u32| {
        let matching: Vec<&Value> = domain[list]
            .as_array()
            .expect(list)
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matching.len(), 1, "{list} {id}");
        (
            matching[0]["payload_schema"]
                .as_str()
                .expect("schema")
                .to_owned(),
            matching[0]["max_payload_bytes"].as_u64().expect("bytes"),
        )
    };
    for (id, message, bound) in [
        (DELTA_TYPE_VIP_ENTRY_V1, "VipEntryV1", MAX_VIP_ENTRY_BYTES),
        (
            DELTA_TYPE_VIP_ENTRY_REMOVED_V1,
            "VipEntryRemovedV1",
            MAX_VIP_ENTRY_REMOVED_BYTES,
        ),
        (
            DELTA_TYPE_VIP_GROUPS_V1,
            "VipGroupsV1",
            MAX_VIP_GROUPS_BYTES,
        ),
        (
            DELTA_TYPE_VIP_STATUS_V1,
            "VipStatusV1",
            MAX_VIP_STATUS_BYTES,
        ),
        (
            DELTA_TYPE_VIP_CONSENT_V1,
            "VipConsentViewV1",
            MAX_VIP_CONSENT_BYTES,
        ),
    ] {
        assert_eq!(
            typed("delta_types", id),
            (format!("{schema}{message}"), bound as u64)
        );
        assert!(VIP_PROTO.contains(&format!("message {message} {{")));
        assert!(VIP_PROTO.contains(&format!("At most {bound} bytes")));
    }
    assert_eq!(domain["delta_types"].as_array().map(Vec::len), Some(5));
    assert_eq!(
        typed("snapshot_types", SNAPSHOT_TYPE_VIP_V1),
        (
            format!("{schema}VipSnapshotV1"),
            MAX_VIP_SNAPSHOT_BYTES as u64
        )
    );
    assert!(VIP_PROTO.contains(&format!("At most {MAX_VIP_INTENT_BYTES} bytes")));
    assert!(VIP_PROTO.contains(&format!("At most {MAX_VIP_RESULT_BYTES} bytes")));
    assert!(VIP_PROTO.contains(&format!("{MAX_VIP_SNAPSHOT_BYTES} bytes")));

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
        ("VIP0-RL-02", MAX_VIP_ENTRIES),
        ("VIP0-RL-03", MAX_VIP_CUSTOM_GROUPS),
        ("VIP0-RL-04", MAX_VIP_DESCRIPTION_CHARS),
        ("VIP0-RL-05", usize::from(MAX_VIP_ICON)),
        ("VIP0-RL-06", MAX_VIP_GROUP_NAME_CHARS),
        ("VIP0-RL-08", MAX_VIP_SNAPSHOT_BYTES),
        ("VIP0-RL-13", MAX_VIP_INVITATIONS),
        ("VIP0-RL-15", MAX_VIP_WATCHERS),
    ] {
        assert_eq!(limit(row), Some(value as u64), "{row}");
        assert!(VIP_PROTO.contains(row), "{row}");
    }
    assert_eq!(limit("VIP0-RL-01"), Some(20));
    assert_eq!(limit("VIP0-RL-07"), Some(20));
    assert_eq!(
        MAX_VIP_DESCRIPTION_BYTES,
        4 * MAX_VIP_DESCRIPTION_CHARS,
        "VIP0-RL-04"
    );
    assert_eq!(MAX_VIP_GROUP_NAME_BYTES, 4 * MAX_VIP_GROUP_NAME_CHARS);
}

#[test]
fn vip_snapshot_apply_replaces_pushes_and_refuses_overflow_atomically() {
    let mut snapshot = VipSnapshot::default();
    let first = worst_entry(1);
    snapshot
        .apply(&VipDelta::Entry(first.clone()))
        .expect("push");
    assert_eq!(snapshot.entries, vec![first.clone()]);
    let mut replacement = first;
    replacement.status = VipStatus::Online;
    snapshot
        .apply(&VipDelta::Entry(replacement.clone()))
        .expect("replace");
    assert_eq!(snapshot.entries, vec![replacement]);
    for entry_no in 2..=MAX_VIP_ENTRIES as u64 {
        snapshot
            .apply(&VipDelta::Entry(worst_entry(entry_no)))
            .expect("within bound");
    }
    let before = snapshot.clone();
    assert_eq!(
        snapshot.apply(&VipDelta::Entry(worst_entry(MAX_VIP_ENTRIES as u64 + 1))),
        Err(VipWireError::LimitExceeded)
    );
    assert_eq!(snapshot, before);
    snapshot
        .apply(&VipDelta::Entry(worst_entry(1)))
        .expect("replace at capacity");
    assert_eq!(snapshot.entries.len(), MAX_VIP_ENTRIES);
    assert_eq!(snapshot.entries[0], worst_entry(1));
}

#[test]
fn vip_snapshot_apply_removes_updates_status_and_replaces_groups_and_consent() {
    let mut snapshot = VipSnapshot::default();
    let missing = VipDelta::Status(VipStatusUpdate {
        entry_no: 1,
        status: VipStatus::Online,
        announce: true,
    });
    snapshot.apply(&missing).expect("missing status");
    assert_eq!(snapshot, VipSnapshot::default());
    snapshot
        .apply(&VipDelta::Entry(worst_entry(1)))
        .expect("entry");
    snapshot.apply(&missing).expect("status");
    assert_eq!(snapshot.entries[0].status, VipStatus::Online);
    snapshot
        .apply(&VipDelta::EntryRemoved { entry_no: 1 })
        .expect("remove");
    assert!(snapshot.entries.is_empty());
    snapshot
        .apply(&VipDelta::Groups(worst_groups()))
        .expect("groups");
    snapshot
        .apply(&VipDelta::Consent(worst_consent()))
        .expect("consent");
    assert_eq!(snapshot.groups, worst_groups());
    assert_eq!(snapshot.consent, worst_consent());
    snapshot
        .apply(&VipDelta::Groups(Vec::new()))
        .expect("replace groups");
    snapshot
        .apply(&VipDelta::Consent(VipConsentView::default()))
        .expect("replace consent");
    assert_eq!(snapshot, VipSnapshot::default());
}
