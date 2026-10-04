#![allow(clippy::expect_used)]

use super::*;
use serde_json::Value;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");

fn mission(index: u32, name: &str, text: &str, done: bool) -> QuestMission {
    QuestMission {
        mission_index: index,
        name: name.into(),
        text: text.into(),
        done,
    }
}

fn line(quest_index: u32, missions: Vec<QuestMission>) -> QuestLine {
    QuestLine {
        quest_index,
        completed: false,
        missions,
    }
}

/// The largest line `encode_line` accepts: missions with the largest indexes, longest names and
/// texts, filled to exactly the line bound by the texts of the last two.
fn worst_line(quest_index: u32) -> QuestLine {
    let full = |n: u32, text: usize| {
        mission(
            u32::MAX - 100 + n,
            &"n".repeat(MAX_MISSION_NAME_BYTES),
            &"t".repeat(text),
            true,
        )
    };
    let mut line = QuestLine {
        quest_index,
        completed: true,
        missions: (0..13).map(|n| full(n, MAX_JOURNAL_TEXT_BYTES)).collect(),
    };
    let used = encode_line(&line).expect("13 missions fit").len();
    // A mission with a text of t >= 128 bytes is an element of 3 + 6 + 131 + (3 + t) + 2.
    let texts = MAX_QUEST_LINE_BYTES - used - 2 * 145;
    line.missions.push(full(13, texts.div_ceil(2)));
    line.missions.push(full(14, texts / 2));
    line
}

fn round_trip(log: &QuestLog) -> Vec<u8> {
    let bytes = encode_quest_log(log).expect("encodes");
    assert_eq!(decode_quest_log(&bytes).expect("decodes"), *log);
    bytes
}

#[test]
fn an_empty_domain_is_no_view_and_no_tracked_quest() {
    assert!(round_trip(&QuestLog::default()).is_empty());
    // An empty list is a view: it is present and distinct from no view.
    let empty_list = QuestLog {
        view: Some(QuestLogView::List(Vec::new())),
        tracked: Vec::new(),
    };
    assert_eq!(round_trip(&empty_list), [0x0a, 0x00]);
}

#[test]
fn the_list_holds_1024_ascending_quests_and_refuses_more() {
    let entries = |count: u32| {
        (0..count)
            .map(|n| QuestListEntry {
                quest_index: u32::MAX - 2000 + n,
                completed: true,
            })
            .collect::<Vec<_>>()
    };
    let full = QuestLog {
        view: Some(QuestLogView::List(entries(MAX_LISTED_QUESTS as u32))),
        tracked: Vec::new(),
    };
    let bytes = round_trip(&full);
    // Each worst-case entry is its 10-byte element; the list is one field with a 2-byte length.
    assert_eq!(bytes.len(), 3 + MAX_QUEST_LIST_BYTES);
    assert_eq!(MAX_QUEST_LIST_BYTES, 10_240);
    const { assert!(MAX_QUEST_LIST_BYTES <= MAX_QUEST_LINE_BYTES) };
    let over = QuestLog {
        view: Some(QuestLogView::List(entries(MAX_LISTED_QUESTS as u32 + 1))),
        tracked: Vec::new(),
    };
    assert_eq!(
        encode_quest_log(&over),
        Err(QuestLogWireError::LimitExceeded)
    );
    // A 1,025th entry appended to valid bytes fails closed on decode too.
    let mut body = bytes[3..].to_vec();
    push_message_field(&mut body, 1, &[0x08, 0x01]);
    let mut appended = Vec::new();
    push_message_field(&mut appended, 1, &body);
    assert!(decode_quest_log(&appended).is_err());
    // Out of order or repeated quest indexes fail closed.
    for indexes in [[2, 1], [1, 1]] {
        let list = QuestLog {
            view: Some(QuestLogView::List(
                indexes
                    .iter()
                    .map(|&quest_index| QuestListEntry {
                        quest_index,
                        completed: false,
                    })
                    .collect(),
            )),
            tracked: Vec::new(),
        };
        assert_eq!(encode_quest_log(&list), Err(QuestLogWireError::Malformed));
    }
}

#[test]
fn a_line_holds_128_missions_within_its_byte_bound_and_refuses_more() {
    let small = |count: u32| {
        line(
            7,
            (0..count)
                .map(|n| mission(n, "Mission", "", false))
                .collect(),
        )
    };
    let full = QuestLog {
        view: Some(QuestLogView::Line(small(MAX_QUEST_LINE_MISSIONS as u32))),
        tracked: Vec::new(),
    };
    round_trip(&full);
    let over = QuestLog {
        view: Some(QuestLogView::Line(small(
            MAX_QUEST_LINE_MISSIONS as u32 + 1,
        ))),
        tracked: Vec::new(),
    };
    assert_eq!(
        encode_quest_log(&over),
        Err(QuestLogWireError::LimitExceeded)
    );

    // The byte bound: exactly 16,384 encodes; one byte more is refused before any byte.
    let worst = worst_line(u32::MAX);
    assert_eq!(
        encode_line(&worst).expect("at the bound").len(),
        MAX_QUEST_LINE_BYTES
    );
    let mut over = worst.clone();
    over.missions.last_mut().expect("mission").text.push('t');
    assert_eq!(encode_line(&over), Err(QuestLogWireError::LimitExceeded));
    let log = QuestLog {
        view: Some(QuestLogView::Line(over)),
        tracked: Vec::new(),
    };
    assert_eq!(
        encode_quest_log(&log),
        Err(QuestLogWireError::LimitExceeded)
    );
}

#[test]
fn names_and_texts_are_bounded_utf8_and_names_are_required() {
    let at = |name: usize, text: usize| {
        line(
            1,
            vec![mission(
                1,
                &"é".repeat(name / 2),
                &"é".repeat(text / 2),
                false,
            )],
        )
    };
    let log = |line: QuestLine| QuestLog {
        view: Some(QuestLogView::Line(line)),
        tracked: Vec::new(),
    };
    round_trip(&log(at(MAX_MISSION_NAME_BYTES, MAX_JOURNAL_TEXT_BYTES)));
    for over in [
        at(MAX_MISSION_NAME_BYTES + 2, 2),
        at(2, MAX_JOURNAL_TEXT_BYTES + 2),
    ] {
        assert_eq!(
            encode_quest_log(&log(over)),
            Err(QuestLogWireError::LimitExceeded)
        );
    }
    assert_eq!(
        encode_quest_log(&log(line(1, vec![mission(1, "", "text", false)]))),
        Err(QuestLogWireError::Malformed)
    );
    // Missions ascend strictly.
    assert_eq!(
        encode_quest_log(&log(line(
            1,
            vec![mission(2, "a", "", false), mission(2, "b", "", false)]
        ))),
        Err(QuestLogWireError::Malformed)
    );
    // Invalid UTF-8 in a name fails closed.
    let mut element = Vec::new();
    push_message_field(&mut element, 2, &[0xff]);
    let mut body = Vec::new();
    push_message_field(&mut body, 3, &element);
    let mut payload = Vec::new();
    push_message_field(&mut payload, 2, &body);
    assert_eq!(
        decode_quest_log(&payload),
        Err(QuestLogWireError::Malformed)
    );
}

#[test]
fn the_domain_holds_a_view_and_10_tracked_lines_within_its_bound() {
    let full = QuestLog {
        view: Some(QuestLogView::Line(worst_line(u32::MAX))),
        tracked: (0..MAX_TRACKED_QUESTS as u32)
            .map(|n| worst_line(u32::MAX - 1 - n))
            .collect(),
    };
    let bytes = round_trip(&full);
    assert_eq!(bytes.len(), MAX_QUEST_LOG_BYTES);
    assert_eq!(MAX_QUEST_LOG_BYTES, 180_268);
    // Within the FND-02 delta and snapshot chunk bounds.
    const { assert!(MAX_QUEST_LOG_BYTES <= crate::MAX_STATE_DELTA_PAYLOAD_BYTES) };
    const { assert!(MAX_QUEST_LOG_BYTES <= crate::MAX_SNAPSHOT_CHUNK_BYTES) };
    // An eleventh tracked line, or a repeated tracked quest, fails closed both ways.
    let mut eleven = full.clone();
    eleven.view = None;
    eleven.tracked.push(line(1, Vec::new()));
    assert_eq!(
        encode_quest_log(&eleven),
        Err(QuestLogWireError::LimitExceeded)
    );
    let mut payload = encode_quest_log(&QuestLog {
        view: None,
        tracked: eleven.tracked[..10].to_vec(),
    })
    .expect("ten");
    push_message_field(&mut payload, 3, &[0x08, 0x01]);
    assert_eq!(
        decode_quest_log(&payload),
        Err(QuestLogWireError::LimitExceeded)
    );
    let repeated = QuestLog {
        view: None,
        tracked: vec![line(3, Vec::new()), line(3, Vec::new())],
    };
    assert_eq!(
        encode_quest_log(&repeated),
        Err(QuestLogWireError::Malformed)
    );
    // A payload one byte over the bound fails closed before parsing.
    assert_eq!(
        decode_quest_log(&vec![0; MAX_QUEST_LOG_BYTES + 1]),
        Err(QuestLogWireError::LimitExceeded)
    );
}

#[test]
fn domain_fields_fail_closed_when_repeated_or_unknown() {
    let list = [0x0a, 0x00];
    let quest = [0x12, 0x00];
    // Both views, or a view twice.
    for payload in [
        [&list[..], &quest[..]].concat(),
        [&list[..], &list[..]].concat(),
    ] {
        assert_eq!(
            decode_quest_log(&payload),
            Err(QuestLogWireError::Malformed)
        );
    }
    // An unknown field at each level, a bool above 1 and a repeated scalar.
    for payload in [
        vec![0x20, 0x01],
        vec![0x12, 0x02, 0x28, 0x01],
        vec![0x12, 0x04, 0x1a, 0x02, 0x28, 0x01],
        vec![0x12, 0x02, 0x10, 0x02],
        vec![0x12, 0x04, 0x08, 0x01, 0x08, 0x02],
        vec![0x0a, 0x04, 0x0a, 0x02, 0x18, 0x01],
    ] {
        assert_eq!(
            decode_quest_log(&payload),
            Err(QuestLogWireError::Malformed),
            "{payload:?}"
        );
    }
}

#[test]
fn queries_round_trip_within_62_bytes_and_fail_closed() {
    assert_eq!(MAX_QUEST_LOG_QUERY_BYTES, 62);
    let full = QuestLogQuery::Track {
        quest_indexes: (0..MAX_TRACKED_QUESTS as u32)
            .map(|n| u32::MAX - n)
            .collect(),
    };
    for query in [
        QuestLogQuery::List,
        QuestLogQuery::Quest { quest_index: 0 },
        QuestLogQuery::Quest {
            quest_index: u32::MAX,
        },
        QuestLogQuery::Track {
            quest_indexes: Vec::new(),
        },
        full.clone(),
    ] {
        let bytes = encode_quest_log_query(&query).expect("encodes");
        assert!(bytes.len() <= MAX_QUEST_LOG_QUERY_BYTES);
        assert_eq!(decode_quest_log_query(&bytes), Ok(query));
    }
    assert_eq!(
        encode_quest_log_query(&full).expect("full").len(),
        MAX_QUEST_LOG_QUERY_BYTES
    );
    // Eleven or repeated indexes.
    let eleven = QuestLogQuery::Track {
        quest_indexes: (0..11).collect(),
    };
    assert_eq!(
        encode_quest_log_query(&eleven),
        Err(QuestLogWireError::LimitExceeded)
    );
    let mut body = Vec::new();
    for index in 0..11 {
        push_varint_field(&mut body, 1, index);
    }
    let mut payload = Vec::new();
    push_message_field(&mut payload, 3, &body);
    assert_eq!(
        decode_quest_log_query(&payload),
        Err(QuestLogWireError::LimitExceeded)
    );
    let repeated = QuestLogQuery::Track {
        quest_indexes: vec![4, 4],
    };
    assert_eq!(
        encode_quest_log_query(&repeated),
        Err(QuestLogWireError::Malformed)
    );
    // An empty oneof, two actions, a packed index list, a list with fields, an unknown field.
    for payload in [
        vec![],
        vec![0x0a, 0x00, 0x0a, 0x00],
        vec![0x0a, 0x00, 0x12, 0x00],
        vec![0x1a, 0x03, 0x0a, 0x01, 0x05],
        vec![0x0a, 0x02, 0x08, 0x01],
        vec![0x12, 0x02, 0x10, 0x01],
        vec![0x22, 0x00],
    ] {
        assert_eq!(
            decode_quest_log_query(&payload),
            Err(QuestLogWireError::Malformed),
            "{payload:?}"
        );
    }
    assert_eq!(
        decode_quest_log_query(&[0; MAX_QUEST_LOG_QUERY_BYTES + 1]),
        Err(QuestLogWireError::LimitExceeded)
    );
}

#[test]
fn the_worst_case_line_estimate_matches_the_encoder() {
    let worst = worst_line(u32::MAX);
    let estimate = worst_case_line_bytes(
        worst
            .missions
            .iter()
            .map(|mission| (mission.name.len(), mission.text.len())),
    );
    assert_eq!(estimate, encode_line(&worst).expect("line").len());
    // Missions without text and the empty line.
    let plain = line(0, vec![mission(0, "a", "", false)]);
    assert!(
        worst_case_line_bytes([(1, 0)].into_iter()) >= encode_line(&plain).expect("line").len()
    );
    assert_eq!(worst_case_line_bytes(std::iter::empty()), 8);
}

#[test]
fn registries_bind_capability_16_and_its_limits() {
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
    let capability = find("capabilities", CAPABILITY_QUEST_LOG_V1);
    assert_eq!(capability["name"], "QUEST_LOG_V1");
    assert_eq!(capability["offered"], false);
    assert!(capability.get("requires").is_none());
    assert_eq!(
        capability["command_types"],
        serde_json::json!([COMMAND_TYPE_QUEST_LOG_QUERY])
    );
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_QUEST_LOG])
    );
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_QUEST_LOG_V1));

    let command = find("command_types", COMMAND_TYPE_QUEST_LOG_QUERY);
    assert_eq!(command["name"], "QUEST_LOG_QUERY");
    assert_eq!(command["capability"], CAPABILITY_QUEST_LOG_V1);
    assert_eq!(
        command["max_payload_bytes"].as_u64(),
        Some(MAX_QUEST_LOG_QUERY_BYTES as u64)
    );
    assert_eq!(
        command["max_result_payload_bytes"].as_u64(),
        Some(MAX_QUEST_LOG_QUERY_RESULT_BYTES as u64)
    );

    let domain = find("state_domains", STATE_DOMAIN_QUEST_LOG);
    assert_eq!(domain["name"], "QUEST_LOG");
    assert_eq!(domain["capability"], CAPABILITY_QUEST_LOG_V1);
    for (key, type_id) in [
        ("delta_types", DELTA_TYPE_QUEST_LOG_V1),
        ("snapshot_types", SNAPSHOT_TYPE_QUEST_LOG_V1),
    ] {
        let types = domain[key].as_array().expect(key);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0]["id"], type_id);
        assert_eq!(
            types[0]["max_payload_bytes"].as_u64(),
            Some(MAX_QUEST_LOG_BYTES as u64)
        );
        assert_eq!(
            types[0]["payload_schema"],
            "docs/contracts/protocol-oteryn/v1/quest_log_v1.proto#QuestLogV1"
        );
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
        ("QUESTGATE0-RL-06", MAX_TRACKED_QUESTS),
        ("QUESTGATE0-RL-07", MAX_LISTED_QUESTS),
        ("QUESTGATE0-RL-08", MAX_QUEST_LINE_MISSIONS),
        ("QUESTGATE0-RL-08-NAME", MAX_MISSION_NAME_BYTES),
        ("QUESTGATE0-RL-08-TEXT", MAX_JOURNAL_TEXT_BYTES),
        ("QUESTGATE0-RL-08-BYTES", MAX_QUEST_LINE_BYTES),
        ("QUESTGATE0-RL-08-DOMAIN-BYTES", MAX_QUEST_LOG_BYTES),
        ("QUESTGATE0-RL-09", MAX_QUEST_LOG_QUERIES_PER_SECOND),
    ] {
        assert_eq!(limit(row), Some(value as u64), "{row}");
    }
}
