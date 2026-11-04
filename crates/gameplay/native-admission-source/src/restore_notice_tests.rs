#![allow(clippy::unwrap_used)]
use super::*;

fn notice() -> RestoreNotice {
    RestoreNotice {
        source_authority: "oteryn-game-eu1".into(),
        restore_event_id: [
            0x01, 0x8f, 0x2a, 0x10, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ],
        restored_generation: 3,
        new_recovery_generation: 4,
        restore_point: 1_790_000_000,
        backup_label: "nas-2026-11-05.base".into(),
    }
}

#[test]
fn restore_notice_encodes_the_exact_platform_member_order() {
    assert_eq!(
        encode(&notice()).unwrap(),
        r#"{"contract_version":1,"operation":"PublishGameRestoreNoticeV1","source_authority":"oteryn-game-eu1","restore_event_id":"018f2a10-0000-7000-8000-000000000001","restored_generation":"3","new_recovery_generation":"4","restore_point":"1790000000","backup_label":"nas-2026-11-05.base"}"#
    );
}

#[test]
fn restore_notice_refuses_out_of_grammar_input() {
    let mut cases = Vec::new();
    let mut n = notice();
    n.restore_event_id[6] = 0x40;
    cases.push(n);
    let mut n = notice();
    n.restored_generation = 0;
    cases.push(n);
    let mut n = notice();
    n.new_recovery_generation = 3;
    cases.push(n);
    let mut n = notice();
    n.new_recovery_generation = 2;
    cases.push(n);
    let mut n = notice();
    n.restore_point = 0;
    cases.push(n);
    let mut n = notice();
    n.backup_label.clear();
    cases.push(n);
    let mut n = notice();
    n.backup_label = "a".repeat(129);
    cases.push(n);
    let mut n = notice();
    n.backup_label = "a b".into();
    cases.push(n);
    let mut n = notice();
    n.source_authority.clear();
    cases.push(n);
    let mut n = notice();
    n.source_authority = "a".repeat(129);
    cases.push(n);
    let mut n = notice();
    n.source_authority = "a\u{7f}".into();
    cases.push(n);
    for case in cases {
        assert!(encode(&case).is_err());
    }
}

#[test]
fn restore_notice_ack_decodes_only_the_exact_answer() {
    let id = notice().restore_event_id;
    let reconciling = br#"{"contract_version":1,"restore_event_id":"018f2a10-0000-7000-8000-000000000001","status":"RECONCILING","reopened":2,"outstanding":1}"#;
    let acknowledged = br#"{"contract_version":1,"restore_event_id":"018f2a10-0000-7000-8000-000000000001","status":"ACKNOWLEDGED","reopened":2,"outstanding":0}"#;
    assert_eq!(
        decode_response(reconciling, &id).unwrap(),
        Ack::Reconciling {
            reopened: 2,
            outstanding: 1
        }
    );
    assert_eq!(
        decode_response(acknowledged, &id).unwrap(),
        Ack::Acknowledged { reopened: 2 }
    );
    for bad in [
        br#"{"contract_version":1,"restore_event_id":"018f2a10-0000-7000-8000-000000000002","status":"ACKNOWLEDGED","reopened":2,"outstanding":0}"#.as_slice(),
        br#"{"restore_event_id":"018f2a10-0000-7000-8000-000000000001","contract_version":1,"status":"ACKNOWLEDGED","reopened":2,"outstanding":0}"#,
        br#"{"contract_version":1,"restore_event_id":"018f2a10-0000-7000-8000-000000000001","status":"ACKNOWLEDGED","reopened":2,"outstanding":0,"extra":1}"#,
        br#"{"contract_version":1,"restore_event_id":"018f2a10-0000-7000-8000-000000000001","status":"ACKNOWLEDGED","reopened":2,"outstanding":1}"#,
        br#"{"contract_version":1,"restore_event_id":"018f2a10-0000-7000-8000-000000000001","status":"RECONCILING","reopened":2,"outstanding":0}"#,
    ] { assert!(decode_response(bad, &id).is_err()); }
    assert!(decode_response(&[b'x'; RESPONSE_BYTES + 1], &id).is_err());
}

#[test]
fn restore_notice_status_classification_is_closed() {
    let id = notice().restore_event_id;
    for (status, expected) in [
        (400, NotDelivered::Malformed),
        (401, NotDelivered::Unauthenticated),
        (409, NotDelivered::Conflict),
        (413, NotDelivered::TooLarge),
        (429, NotDelivered::RateLimited),
        (503, NotDelivered::Unavailable),
        (404, NotDelivered::UnexpectedStatus),
    ] {
        assert_eq!(classify(status, b"", &id), Err(expected));
    }
    assert_eq!(classify(400, b"x", &id), Err(NotDelivered::InvalidResponse));
    assert_eq!(
        classify(200, b"{}", &id),
        Err(NotDelivered::InvalidResponse)
    );
    for class in [
        NotDelivered::InvalidNotice,
        NotDelivered::Malformed,
        NotDelivered::Unauthenticated,
        NotDelivered::Conflict,
        NotDelivered::TooLarge,
        NotDelivered::UnexpectedStatus,
    ] {
        assert!(class.definite());
    }
    for class in [
        NotDelivered::RateLimited,
        NotDelivered::InvalidResponse,
        NotDelivered::Unavailable,
    ] {
        assert!(!class.definite());
    }
}

#[test]
fn restore_notice_config_parses_and_refuses_invalid_documents() {
    let valid = br#"endpoint = "127.0.0.1:443"
peer_name = "platform.internal"
trust_roots_file = "/etc/roots.pem"
client_certificate_file = "/etc/client.pem"
client_key_file = "/etc/client.key"
source_authority = "oteryn-game-eu1"
"#;
    let config = RestoreNoticeConfig::parse(valid).unwrap();
    assert_eq!(config.endpoint.port(), 443);
    assert_eq!(config.source_authority, "oteryn-game-eu1");
    for bad in [
        String::from_utf8_lossy(valid).replace(":443", ":0"),
        String::from_utf8_lossy(valid).replace("platform.internal", ""),
        String::from_utf8_lossy(valid).replace("/etc/roots.pem", "roots.pem"),
        String::from_utf8_lossy(valid).replace("/etc/client.key", "/etc/client.pem"),
        format!("{}unknown = 1\n", String::from_utf8_lossy(valid)),
        String::from_utf8_lossy(valid).replace("oteryn-game-eu1", ""),
    ] {
        assert!(RestoreNoticeConfig::parse(bad.as_bytes()).is_err());
    }
}
