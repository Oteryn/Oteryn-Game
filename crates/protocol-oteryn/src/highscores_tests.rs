#![allow(clippy::expect_used)]

use super::*;

// Hand-written protobuf reference bytes, independent of the encoder under test. The held IDL
// assigns query tags 1..4, row tags 1..5 and result tags 1..7. Epoch 1700 encodes a4 0d;
// snapshot/value 300 encode ac 02. No production helper generates these golden fixtures.
const FIRST_QUERY: &[u8] = &[0x08, 1, 0x18, 1];
const NEXT_QUERY: &[u8] = &[0x08, 10, 0x10, 5, 0x18, 2, 0x20, 0xac, 2];
const PAGE_HEADER: &[u8] = &[0x08, 1, 0x10, 0xac, 2, 0x18, 0xa4, 0x0d, 0x20, 1];
const ROW: &[u8] = &[
    0x08, 1, 0x12, 2, b'A', b'l', 0x1a, 6, b'k', b'n', b'i', b'g', b'h', b't', 0x20, 8, 0x28, 0xac,
    2,
];

fn row() -> HighscoreRow {
    HighscoreRow {
        rank: 1,
        name: "Al".into(),
        vocation: "knight".into(),
        level: 8,
        value: 300,
    }
}

fn page() -> HighscoresResult {
    HighscoresResult {
        category: 1,
        snapshot_id: 300,
        computed_at: 1700,
        page: 1,
        has_more: false,
        rows: vec![],
        own_row: None,
    }
}

fn result_with_row(raw: &[u8]) -> Vec<u8> {
    // All adversarial row fixtures below have one-byte lengths. This writer is test-local;
    // malformed field bodies never go through the production encoder.
    assert!(raw.len() < 128);
    [PAGE_HEADER, &[0x32, raw.len() as u8], raw].concat()
}

#[test]
fn query_golden_vectors_and_semantic_defaults() {
    for (bytes, query) in [
        (
            FIRST_QUERY,
            HighscoresQuery {
                category: 1,
                vocation: 0,
                page: 1,
                snapshot_id: 0,
            },
        ),
        (
            NEXT_QUERY,
            HighscoresQuery {
                category: 10,
                vocation: 5,
                page: 2,
                snapshot_id: 300,
            },
        ),
    ] {
        assert_eq!(decode_highscores_query(bytes), Ok(query));
        assert_eq!(encode_highscores_query(query), Ok(bytes.to_vec()));
    }
    // Reordered fields and explicit defaults remain the same query, not a new command identity.
    let explicit = [0x20, 0, 0x18, 1, 0x10, 0, 0x08, 1];
    assert_eq!(
        decode_highscores_query(&explicit),
        decode_highscores_query(FIRST_QUERY)
    );
    let nonminimal_varint = [0x08, 0x81, 0, 0x18, 1];
    assert_eq!(
        decode_highscores_query(&nonminimal_varint),
        decode_highscores_query(FIRST_QUERY)
    );
}

#[test]
fn result_golden_vector_keeps_page_and_own_row_separate() {
    let bytes = [PAGE_HEADER, &[0x32, 19], ROW, &[0x3a, 19], ROW].concat();
    let result = HighscoresResult {
        rows: vec![row()],
        own_row: Some(row()),
        ..page()
    };
    assert_eq!(decode_highscores_result(&bytes), Ok(result.clone()));
    assert_eq!(encode_highscores_result(&result), Ok(bytes));
    assert_eq!(decode_highscores_result(PAGE_HEADER), Ok(page()));
}

#[test]
fn queries_refuse_missing_semantics_bad_filters_and_unpinned_later_pages() {
    let bad: &[&[u8]] = &[
        &[],
        &[0x08, 1],
        &[0x18, 1],
        &[0x08, 1, 0x18, 0],
        &[0x08, 1, 0x10, 6, 0x18, 1],
        &[0x08, 1, 0x18, 2],
        &[0x08, 1, 0x18, 2, 0x20, 0],
    ];
    for bytes in bad {
        assert_eq!(
            decode_highscores_query(bytes),
            Err(HighscoresWireError::Malformed)
        );
    }
    let first = decode_highscores_query(FIRST_QUERY).expect("first query");
    for query in [
        HighscoresQuery {
            category: 0,
            ..first
        },
        HighscoresQuery {
            vocation: 6,
            ..first
        },
        HighscoresQuery { page: 0, ..first },
        HighscoresQuery { page: 2, ..first },
    ] {
        assert_eq!(
            encode_highscores_query(query),
            Err(HighscoresWireError::Malformed)
        );
    }
}

#[test]
fn query_never_accepts_world_identity_unknown_fields_or_duplicate_scalars() {
    for suffix in [
        vec![0x08, 1],
        vec![0x10, 0, 0x10, 0],
        vec![0x18, 1],
        vec![0x20, 0, 0x20, 0],
        vec![0x28, 1],
        vec![0x0a, 0],
        [vec![0x2a, 16], vec![1; 16]].concat(),
    ] {
        assert_eq!(
            decode_highscores_query(&[FIRST_QUERY, &suffix].concat()),
            Err(HighscoresWireError::Malformed)
        );
    }
    assert_eq!(
        decode_highscores_query(&[0; MAX_HIGHSCORES_QUERY_BYTES + 1]),
        Err(HighscoresWireError::LimitExceeded)
    );
}

#[test]
fn query_integer_ranges_do_not_narrow_opaque_tokens_or_unsupported_categories() {
    for category in [1, 10, 11, 12, 16, u32::MAX] {
        for vocation in 0..=5 {
            for (page, snapshot_id) in [(1, 0), (1, 1), (2, 300), (u32::MAX, u64::MAX)] {
                let query = HighscoresQuery {
                    category,
                    vocation,
                    page,
                    snapshot_id,
                };
                let bytes = encode_highscores_query(query).expect("bounded query");
                assert!(bytes.len() <= MAX_HIGHSCORES_QUERY_BYTES);
                assert_eq!(decode_highscores_query(&bytes), Ok(query));
            }
        }
    }
    // 2^32 in a uint32 category, and 2^64 in a snapshot token.
    assert_eq!(
        decode_highscores_query(&[0x08, 0x80, 0x80, 0x80, 0x80, 0x10, 0x18, 1]),
        Err(HighscoresWireError::Malformed)
    );
    let overflow = [FIRST_QUERY, &[0x20], &[0xff; 9], &[2]].concat();
    assert_eq!(
        decode_highscores_query(&overflow),
        Err(HighscoresWireError::Malformed)
    );
}

#[test]
fn truncation_and_false_lengths_fail_before_domain_work() {
    for end in 0..NEXT_QUERY.len() {
        assert!(decode_highscores_query(&NEXT_QUERY[..end]).is_err());
    }
    let bytes = result_with_row(ROW);
    // A complete header is a valid empty page; only cuts inside the row are malformed.
    for end in PAGE_HEADER.len() + 1..bytes.len() {
        assert!(decode_highscores_result(&bytes[..end]).is_err());
    }
    for suffix in [vec![0x32, 0x80], vec![0x32, 127], vec![0x10, 0x80]] {
        assert!(decode_highscores_result(&[PAGE_HEADER, &suffix].concat()).is_err());
    }
}

#[test]
fn result_refuses_unknown_repeated_and_wrong_wire_fields() {
    for suffix in [
        vec![0x08, 1],
        vec![0x10, 1],
        vec![0x18, 0],
        vec![0x20, 1],
        vec![0x28, 0, 0x28, 0],
        vec![0x40, 1],
        vec![0x30, 1],
        vec![0x38, 1],
        vec![0x28, 2],
    ] {
        assert_eq!(
            decode_highscores_result(&[PAGE_HEADER, &suffix].concat()),
            Err(HighscoresWireError::Malformed)
        );
    }
    let duplicate_own = [PAGE_HEADER, &[0x3a, 19], ROW, &[0x3a, 19], ROW].concat();
    assert_eq!(
        decode_highscores_result(&duplicate_own),
        Err(HighscoresWireError::Malformed)
    );
    for suffix in [
        vec![0x08, 1],
        vec![0x12, 2, b'A', b'l'],
        vec![0x1a, 0],
        vec![0x20, 1],
        vec![0x28, 0],
        vec![0x30, 1],
        vec![0x0a, 0],
    ] {
        assert_eq!(
            decode_highscores_result(&result_with_row(&[ROW, &suffix].concat())),
            Err(HighscoresWireError::Malformed)
        );
    }
}

#[test]
fn row_domains_match_current_name_and_vocation_sources() {
    for name in [
        "", "A", " Al", "Al ", "Al  Dric", "Al1", "Al\nDric", "Aldríc",
    ] {
        let bad = HighscoresResult {
            rows: vec![HighscoreRow {
                name: name.into(),
                ..row()
            }],
            ..page()
        };
        assert_eq!(
            encode_highscores_result(&bad),
            Err(HighscoresWireError::Malformed)
        );
    }
    for vocation in ["", "_knight", "knight elite", "knight\n"] {
        let bad = HighscoresResult {
            own_row: Some(HighscoreRow {
                vocation: vocation.into(),
                ..row()
            }),
            ..page()
        };
        assert_eq!(
            encode_highscores_result(&bad),
            Err(HighscoresWireError::Malformed)
        );
    }
    for bad_row in [
        HighscoreRow { rank: 0, ..row() },
        HighscoreRow { level: 0, ..row() },
    ] {
        assert_eq!(
            encode_highscores_result(&HighscoresResult {
                rows: vec![bad_row],
                ..page()
            }),
            Err(HighscoresWireError::Malformed)
        );
    }
    let invalid_utf8 = [
        0x08, 1, 0x12, 2, 0xc0, 0xaf, 0x1a, 4, b'n', b'o', b'n', b'e', 0x20, 1,
    ];
    assert_eq!(
        decode_highscores_result(&result_with_row(&invalid_utf8)),
        Err(HighscoresWireError::Malformed)
    );
    let zero_rank = &ROW[2..];
    let zero_level = [&ROW[..14], &ROW[16..]].concat();
    for bad in [zero_rank, zero_level.as_slice(), &[]] {
        assert_eq!(
            decode_highscores_result(&result_with_row(bad)),
            Err(HighscoresWireError::Malformed)
        );
    }
}

#[test]
fn past_last_page_still_carries_own_row_and_global_rank_can_exceed_the_cap() {
    let own = HighscoreRow {
        rank: 2001,
        vocation: "none".into(),
        value: 0,
        ..row()
    };
    let result = HighscoresResult {
        category: 10,
        page: u32::MAX,
        own_row: Some(own),
        ..page()
    };
    let bytes = encode_highscores_result(&result).expect("own row on empty later page");
    assert_eq!(decode_highscores_result(&bytes), Ok(result));
    let filtered = HighscoresResult {
        rows: vec![HighscoreRow {
            rank: 5000,
            vocation: "elite_knight".into(),
            ..row()
        }],
        ..page()
    };
    assert_eq!(
        decode_highscores_result(&encode_highscores_result(&filtered).expect("global rank")),
        Ok(filtered)
    );
}

#[test]
fn snapshot_result_semantics_and_page_bounds_are_symmetric() {
    for bad in [
        HighscoresResult {
            category: 0,
            ..page()
        },
        HighscoresResult {
            snapshot_id: 0,
            ..page()
        },
        HighscoresResult {
            computed_at: -1,
            ..page()
        },
        HighscoresResult { page: 0, ..page() },
        HighscoresResult {
            has_more: true,
            ..page()
        },
        HighscoresResult {
            page: 21,
            rows: vec![row()],
            ..page()
        },
        HighscoresResult {
            page: 20,
            has_more: true,
            rows: vec![row(); 50],
            ..page()
        },
    ] {
        assert_eq!(
            encode_highscores_result(&bad),
            Err(HighscoresWireError::Malformed)
        );
    }
    for header in [
        vec![],
        vec![0x08, 1, 0x20, 1],
        vec![0x10, 1, 0x20, 1],
        vec![0x08, 1, 0x10, 1],
    ] {
        assert_eq!(
            decode_highscores_result(&header),
            Err(HighscoresWireError::Malformed)
        );
    }
    let negative_time = [&[0x08, 1, 0x10, 1, 0x18][..], &[0xff; 9], &[1, 0x20, 1]].concat();
    assert_eq!(
        decode_highscores_result(&negative_time),
        Err(HighscoresWireError::Malformed)
    );
    assert_eq!(
        decode_highscores_result(&[PAGE_HEADER, &[0x28, 1]].concat()),
        Err(HighscoresWireError::Malformed)
    );
    for count in [0, 1, 49, 50] {
        for has_more in [false, true] {
            if has_more && count != 50 {
                continue;
            }
            let result = HighscoresResult {
                rows: vec![row(); count],
                has_more,
                own_row: Some(row()),
                ..page()
            };
            assert_eq!(
                decode_highscores_result(&encode_highscores_result(&result).expect("valid page")),
                Ok(result)
            );
        }
    }
}

#[test]
fn row_count_and_text_limits_apply_in_both_directions() {
    let oversized = HighscoresResult {
        rows: vec![row(); 51],
        ..page()
    };
    assert_eq!(
        encode_highscores_result(&oversized),
        Err(HighscoresWireError::LimitExceeded)
    );
    let mut bytes = PAGE_HEADER.to_vec();
    for _ in 0..51 {
        bytes.extend_from_slice(&[0x32, 19]);
        bytes.extend_from_slice(ROW);
    }
    assert_eq!(
        decode_highscores_result(&bytes),
        Err(HighscoresWireError::LimitExceeded)
    );
    for bad_row in [
        HighscoreRow {
            name: "a".repeat(30),
            ..row()
        },
        HighscoreRow {
            vocation: "a".repeat(129),
            ..row()
        },
    ] {
        assert_eq!(
            encode_highscores_result(&HighscoresResult {
                own_row: Some(bad_row),
                ..page()
            }),
            Err(HighscoresWireError::LimitExceeded)
        );
    }
    let long_name = [&[0x08, 1, 0x12, 30][..], &[b'a'; 30], &ROW[6..]].concat();
    assert_eq!(
        decode_highscores_result(&result_with_row(&long_name)),
        Err(HighscoresWireError::LimitExceeded)
    );
    // Length 129 is 81 01; the row body fits its byte bound, but its vocation does not.
    let long_vocation = [&ROW[..6], &[0x1a, 0x81, 1], &[b'a'; 129], &ROW[14..]].concat();
    let raw = [
        PAGE_HEADER,
        &[0x32, long_vocation.len() as u8, 1],
        &long_vocation,
    ]
    .concat();
    assert_eq!(
        decode_highscores_result(&raw),
        Err(HighscoresWireError::LimitExceeded)
    );
    assert_eq!(
        decode_highscores_result(&vec![0; MAX_HIGHSCORES_RESULT_BYTES + 1]),
        Err(HighscoresWireError::LimitExceeded)
    );
}

#[test]
fn all_declared_field_bounds_fit_the_fnd02_parent() {
    let worst_row = HighscoreRow {
        rank: u64::MAX,
        name: "a".repeat(29),
        vocation: "a".repeat(128),
        level: u32::MAX,
        value: u64::MAX,
    };
    let result = HighscoresResult {
        category: u32::MAX,
        snapshot_id: u64::MAX,
        computed_at: i64::MAX,
        page: 19,
        has_more: true,
        rows: vec![worst_row.clone(); 50],
        own_row: Some(worst_row),
    };
    let bytes = encode_highscores_result(&result).expect("bounded page plus own row");
    assert_eq!(bytes.len(), 9874);
    assert!(bytes.len() <= MAX_HIGHSCORES_RESULT_BYTES);
    assert!(bytes.len() < crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);
    assert_eq!(decode_highscores_result(&bytes), Ok(result));
}

const PROTOCOL_REGISTRY: &str = crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON;
const RESOURCE_REGISTRY: &str = crate::RESOURCE_LIMITS_REGISTRY_JSON;
const HIGHSCORES_PROTO: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/highscores_v1.proto");

#[test]
fn the_registries_bind_the_highscores_constants() {
    use serde_json::{Value, json};
    let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let find = |list: &str, key: &str, id: u32| {
        let matching: Vec<&Value> = protocol[list]
            .as_array()
            .expect(list)
            .iter()
            .filter(|entry| entry[key] == id)
            .collect();
        assert_eq!(matching.len(), 1, "{list} {id}");
        matching[0].clone()
    };
    let capability = find("capabilities", "id", CAPABILITY_HIGHSCORES_V1);
    assert_eq!(capability["name"], "HIGHSCORES_V1");
    assert_eq!(capability["offered"], true);
    assert_eq!(capability["requires"], json!([]));
    assert_eq!(
        capability["command_types"],
        json!([COMMAND_TYPE_HIGHSCORES_QUERY_V1])
    );
    assert_eq!(capability["state_domains"], json!([]));
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_HIGHSCORES_V1));

    let schema = "docs/contracts/protocol-oteryn/v1/highscores_v1.proto#";
    let command = find("command_types", "id", COMMAND_TYPE_HIGHSCORES_QUERY_V1);
    assert_eq!(command["name"], "HIGHSCORES_QUERY_V1");
    assert_eq!(command["capability"], CAPABILITY_HIGHSCORES_V1);
    assert_eq!(
        command["payload_schema"],
        format!("{schema}HighscoresQueryV1")
    );
    assert_eq!(
        command["result_schema"],
        format!("{schema}HighscoresResultV1")
    );
    assert_eq!(
        command["max_payload_bytes"].as_u64(),
        Some(MAX_HIGHSCORES_QUERY_BYTES as u64)
    );
    assert_eq!(
        command["max_result_payload_bytes"].as_u64(),
        Some(MAX_HIGHSCORES_RESULT_BYTES as u64)
    );
    for message in [
        "message HighscoresQueryV1 {",
        "message HighscoreRowV1 {",
        "message HighscoresResultV1 {",
    ] {
        assert!(HIGHSCORES_PROTO.contains(message), "{message}");
    }
    assert!(HIGHSCORES_PROTO.contains(&format!(
        "At most {MAX_HIGHSCORES_QUERY_BYTES} encoded bytes"
    )));
    assert!(HIGHSCORES_PROTO.contains(&format!(
        "At most {MAX_HIGHSCORES_RESULT_BYTES} encoded bytes"
    )));
    assert!(HIGHSCORES_PROTO.contains(&format!("At most {MAX_HIGHSCORE_ROW_BYTES} encoded bytes")));
    // The two HIGHSCORES0 §6 results are FND-02 errors without a payload.
    for (error, name) in [
        (
            crate::FoundationProtocolError::NotSupported,
            "NOT_SUPPORTED",
        ),
        (
            crate::FoundationProtocolError::SnapshotExpired,
            "SNAPSHOT_EXPIRED",
        ),
    ] {
        let row = find("error_codes", "code", error.code());
        assert_eq!(row["name"], name);
        assert_eq!(row["default_disposition"], "OPERATION_TERMINAL");
        assert_eq!(
            error.disposition(),
            crate::ProtocolDisposition::OperationTerminal
        );
        assert!(HIGHSCORES_PROTO.contains(&format!("{name} ({}", error.code())));
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
    assert_eq!(
        limit("HIGHSCORES0-RL-02"),
        Some(u64::from(MAX_HIGHSCORES_LIST_ROWS))
    );
    assert_eq!(
        limit("HIGHSCORES0-RL-03"),
        Some(MAX_HIGHSCORES_PAGE_ROWS as u64)
    );
    assert!(HIGHSCORES_PROTO.contains("HIGHSCORES0-RL-02"));
    assert!(HIGHSCORES_PROTO.contains("HIGHSCORES0-RL-03"));
}

#[cfg(test)]
#[path = "highscores_fuzz_tests.rs"]
mod fuzz_tests;
