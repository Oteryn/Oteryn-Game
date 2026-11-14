use super::*;

const ACCOUNT: &str = "01900000-0000-7000-8000-000000000001";
const CHARACTER: &str = "01900000-0000-7000-8000-000000000002";
const STREAM: &str = "01900000-0000-7000-8000-0000000000a1";

fn sanction_page(items: &str, next: &str) -> Vec<u8> {
    format!(
        r#"{{"schema":"oteryn.sanction_stream_page.v1","items":[{items}],"next_cursor":{next}}}"#
    )
    .into_bytes()
}

fn mute(revision: i64, ends: &str) -> String {
    format!(
        r#"{{"sanction_stream_id":"{STREAM}","decision_revision":{revision},"account_id":"{ACCOUNT}","character_id":"{CHARACTER}","kind":1,"reason_code":9,"reason_text":"Insulting","starts_at_ms":1000,"ends_at_ms":{ends},"revoked":false}}"#
    )
}

fn loyalty_page(items: &str, next: &str) -> Vec<u8> {
    format!(
        r#"{{"schema":"oteryn.loyalty_stream_page.v1","items":[{items}],"next_cursor":{next}}}"#
    )
    .into_bytes()
}

fn snapshot(points: i64, revision: i64) -> String {
    format!(
        r#"{{"account_id":"{ACCOUNT}","spent_premium_days":400,"loyalty_points":{points},"platform_revision":{revision}}}"#
    )
}

#[test]
fn requests_carry_schema_cursor_and_a_bounded_limit() -> Result<(), &'static str> {
    let json = |raw: Vec<u8>| serde_json::from_slice::<serde_json::Value>(&raw).ok();
    let first = request_body(PlatformStream::Sanctions, None, PAGE_LIMIT).ok_or("first")?;
    assert_eq!(
        json(first),
        Some(
            serde_json::json!({"schema":"oteryn.sanction_stream_request.v1","after":null,"limit":200})
        )
    );
    let next = request_body(PlatformStream::Loyalty, Some("c:42"), 500).ok_or("next")?;
    assert_eq!(
        json(next),
        Some(
            serde_json::json!({"schema":"oteryn.loyalty_stream_request.v1","after":"c:42","limit":500})
        )
    );
    assert!(request_body(PlatformStream::Loyalty, None, 0).is_none());
    assert!(request_body(PlatformStream::Loyalty, None, 501).is_none());
    assert!(request_body(PlatformStream::Loyalty, Some("a b"), 1).is_none());
    assert!(request_body(PlatformStream::Loyalty, Some(""), 1).is_none());
    Ok(())
}

#[test]
fn sanction_page_decodes_to_projection_records_and_its_cursor() -> Result<(), &'static str> {
    let raw = sanction_page(
        &format!("{},{}", mute(1, "2000"), mute(2, "2000")),
        r#""s:7""#,
    );
    let page = decode_page(PlatformStream::Sanctions, &raw, 2).ok_or("page")?;
    assert!(page.full);
    assert_eq!(page.next_cursor.as_deref(), Some("s:7"));
    let StreamPage::Sanctions(items) = &page.records else {
        return Err("stream");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].decision_revision, 1);
    assert_eq!(items[1].decision_revision, 2);
    assert_eq!(items[0].kind, 1);
    assert_eq!(items[0].reason_code, 9);
    assert_eq!(items[0].ends_at_ms, Some(2000));
    assert_eq!(items[0].character_id.map(|id| id[15]), Some(2));
    assert_eq!(items[0].sanction_stream_id[15], 0xa1);
    // The same payload decodes to the same records: a replayed page projects as replays.
    assert_eq!(
        decode_page(PlatformStream::Sanctions, &raw, 2),
        Some(page.clone())
    );
    let short = decode_page(PlatformStream::Sanctions, &raw, 3).ok_or("short")?;
    assert!(!short.full);
    let ban = format!(
        r#"{{"sanction_stream_id":"{STREAM}","decision_revision":1,"account_id":"{ACCOUNT}","character_id":null,"kind":4,"reason_code":3,"reason_text":"Account trading","starts_at_ms":5,"ends_at_ms":null,"revoked":true}}"#
    );
    let ban_page = decode_page(
        PlatformStream::Sanctions,
        &sanction_page(&ban, r#""s:8""#),
        10,
    )
    .ok_or("ban")?;
    let StreamPage::Sanctions(bans) = ban_page.records else {
        return Err("stream");
    };
    assert_eq!(bans[0].character_id, None);
    assert!(bans[0].revoked);
    // An empty page may keep the cursor.
    let idle =
        decode_page(PlatformStream::Sanctions, &sanction_page("", "null"), 10).ok_or("idle")?;
    assert_eq!(idle.next_cursor, None);
    assert_eq!(idle.records, StreamPage::Sanctions(Vec::new()));
    Ok(())
}

#[test]
fn sanction_pages_outside_the_export_shape_are_refused_whole() {
    let good = mute(1, "2000");
    let refused = [
        // A page with records must move the cursor.
        sanction_page(&good, "null"),
        // More records than asked for (limit 1 below).
        sanction_page(&format!("{good},{good}"), r#""s""#),
        // Cursor outside printable ASCII or too long.
        sanction_page(&good, r#""a b""#),
        sanction_page(&good, &format!(r#""{}""#, "c".repeat(257))),
        // A mute needs an end; the end follows the start.
        sanction_page(&mute(1, "null"), r#""s""#),
        sanction_page(&mute(1, "1000"), r#""s""#),
        // Positive revision.
        sanction_page(&mute(0, "2000"), r#""s""#),
        // The stream id is a UUIDv7.
        sanction_page(
            &good.replace(STREAM, "01900000-0000-4000-8000-0000000000a1"),
            r#""s""#,
        ),
        // Canonical lowercase UUIDs only.
        sanction_page(&good.replace(STREAM, &STREAM.to_uppercase()), r#""s""#),
        sanction_page(
            &good.replace(ACCOUNT, "0190000000007000800000000000000001"),
            r#""s""#,
        ),
        // Kinds 1..=6; an account ban names no Character.
        sanction_page(&good.replace(r#""kind":1"#, r#""kind":7"#), r#""s""#),
        sanction_page(&good.replace(r#""kind":1"#, r#""kind":4"#), r#""s""#),
        // Reason code 1..=65535, reason 1..=120 bytes without control characters.
        sanction_page(
            &good.replace(r#""reason_code":9"#, r#""reason_code":0"#),
            r#""s""#,
        ),
        sanction_page(
            &good.replace(r#""reason_code":9"#, r#""reason_code":65536"#),
            r#""s""#,
        ),
        sanction_page(&good.replace("Insulting", &"x".repeat(121)), r#""s""#),
        sanction_page(&good.replace("Insulting", "a\\nb"), r#""s""#),
        sanction_page(&good.replace("Insulting", ""), r#""s""#),
        // Unknown fields.
        sanction_page(
            &good.replace(r#""revoked":false"#, r#""revoked":false,"staff":"x""#),
            r#""s""#,
        ),
    ];
    for (index, raw) in refused.iter().enumerate() {
        assert_eq!(
            decode_page(PlatformStream::Sanctions, raw, 1),
            None,
            "case {index}"
        );
    }
    let other_schema = String::from_utf8_lossy(&sanction_page(&good, r#""s""#))
        .replace("sanction_stream_page", "loyalty_stream_page");
    assert!(decode_page(PlatformStream::Sanctions, other_schema.as_bytes(), 1).is_none());
    assert!(decode_page(PlatformStream::Loyalty, &sanction_page(&good, r#""s""#), 1).is_none());
    assert!(
        decode_page(
            PlatformStream::Sanctions,
            " ".repeat(MAX_PAGE_BYTES + 1).as_bytes(),
            1
        )
        .is_none()
    );
}

#[test]
fn loyalty_page_carries_platform_points_and_refuses_out_of_range_snapshots()
-> Result<(), &'static str> {
    let raw = loyalty_page(
        &format!("{},{}", snapshot(410, 3), snapshot(0, 4)),
        r#""l:9""#,
    );
    let page = decode_page(PlatformStream::Loyalty, &raw, PAGE_LIMIT).ok_or("page")?;
    assert!(!page.full);
    assert_eq!(page.next_cursor.as_deref(), Some("l:9"));
    let StreamPage::Loyalty(items) = page.records else {
        return Err("stream");
    };
    assert_eq!(items[0].loyalty_points, 410);
    assert_eq!(items[0].spent_premium_days, 400);
    assert_eq!(items[0].platform_revision, 3);
    assert_eq!(items[1].loyalty_points, 0);
    for bad in [
        snapshot(-1, 1),
        snapshot(2_147_483_648, 1),
        snapshot(1, 0),
        snapshot(1, 1).replace(r#""spent_premium_days":400"#, r#""spent_premium_days":-1"#),
        snapshot(1, 1).replace(ACCOUNT, "not-a-uuid"),
        snapshot(1, 1).replace(
            r#""platform_revision":1"#,
            r#""platform_revision":1,"extra":1"#,
        ),
    ] {
        assert!(
            decode_page(PlatformStream::Loyalty, &loyalty_page(&bad, r#""l""#), 5).is_none(),
            "{bad}"
        );
    }
    Ok(())
}

#[test]
fn streams_are_off_unless_explicitly_true() {
    assert_eq!(enabled(SANCTIONS_ENABLED_VAR, None), Ok(false));
    assert_eq!(
        enabled(SANCTIONS_ENABLED_VAR, Some("false".into())),
        Ok(false)
    );
    assert_eq!(
        enabled(SANCTIONS_ENABLED_VAR, Some("true".into())),
        Ok(true)
    );
    for other in ["1", "TRUE", "yes", ""] {
        assert!(enabled(LOYALTY_ENABLED_VAR, Some(other.into())).is_err());
    }
}

fn identity(leaf: &str) -> Vec<u8> {
    format!(
        "-----BEGIN CERTIFICATE-----\n{leaf}\n-----END CERTIFICATE-----\n-----BEGIN PRIVATE KEY-----\nS0VZ\n-----END PRIVATE KEY-----\n"
    )
    .into_bytes()
}

#[test]
fn each_stream_presents_its_own_identity_never_premium_or_the_other_streams() {
    let sanction = identity("U0FOQ1RJT04=");
    let loyalty = identity("TE9ZQUxUWQ==");
    let premium = identity("UFJFTUlVTQ==");
    assert_eq!(
        check_identities(
            &[
                (SANCTION_IDENTITY_VAR, sanction.as_slice()),
                (LOYALTY_IDENTITY_VAR, loyalty.as_slice())
            ],
            Some(premium.as_slice())
        ),
        Ok(())
    );
    assert_eq!(
        check_identities(&[(LOYALTY_IDENTITY_VAR, loyalty.as_slice())], None),
        Ok(())
    );
    // The Premium identity, also when only the PEM layout differs.
    let premium_rewrapped = identity("UFJF\n  TUlVTQ==");
    assert_eq!(
        check_identities(
            &[(LOYALTY_IDENTITY_VAR, premium_rewrapped.as_slice())],
            Some(premium.as_slice())
        ),
        Err(ClientConfigError::Invalid(LOYALTY_IDENTITY_VAR))
    );
    // The other stream's identity.
    assert_eq!(
        check_identities(
            &[
                (SANCTION_IDENTITY_VAR, sanction.as_slice()),
                (LOYALTY_IDENTITY_VAR, sanction.as_slice())
            ],
            Some(premium.as_slice())
        ),
        Err(ClientConfigError::Invalid(LOYALTY_IDENTITY_VAR))
    );
    // A file without a certificate.
    assert_eq!(
        check_identities(&[(SANCTION_IDENTITY_VAR, b"not a pem".as_slice())], None),
        Err(ClientConfigError::Invalid(SANCTION_IDENTITY_VAR))
    );
}

#[test]
fn security_revocation_fixtures_and_closed_decoder() -> Result<(), &'static str> {
    const PAGE: &[u8] = include_bytes!(
        "../../../../../docs/contracts/fixtures/security-revocation-stream-v1/page.json"
    );
    const EMPTY: &[u8] = include_bytes!(
        "../../../../../docs/contracts/fixtures/security-revocation-stream-v1/empty.json"
    );
    const REQUEST: &[u8] = include_bytes!(
        "../../../../../docs/contracts/fixtures/security-revocation-stream-v1/request.json"
    );
    let stream = PlatformStream::SecurityRevocations;
    assert_eq!(request_body(stream, None, 1).as_deref(), Some(REQUEST));
    assert!(decode_page(stream, EMPTY, 1).is_some());
    assert!(decode_page(stream, PAGE, 1).is_some());
    let text = std::str::from_utf8(PAGE).map_err(|_| "fixture")?;
    for bad in [
        text.replace(
            "\"schema\":\"oteryn.security_revocation_stream_page.v1\",\"items\":",
            "\"items\":",
        )
        .replace(
            ",\"next_cursor\":",
            ",\"schema\":\"oteryn.security_revocation_stream_page.v1\",\"next_cursor\":",
        ),
        text.replace("\"cause\":1", "\"cause\":0"),
        text.replace("\"cause\":1", "\"cause\":5"),
        text.replace(
            "019a0000-0000-7000-8000-000000000011",
            "019a0000-0000-4000-8000-000000000011",
        ),
        text.replace(
            "019a0000-0000-7000-8000-000000000002",
            "019a0000-0000-4000-8000-000000000002",
        ),
        text.replace(
            "\"native_security_generation\":2",
            "\"native_security_generation\":0",
        ),
        text.replace(
            "\"native_security_generation\":2",
            "\"native_security_generation\":-1",
        ),
        text.replace(
            "\"cause\":1,\"effective_at_ms\":1700000000000",
            "\"effective_at_ms\":1700000000000,\"cause\":1",
        ),
        text.replace("\"cause\":1", "\"cause\":1,\"extra\":1"),
        text.replace("\"cause\":1", "\"cause\":1,\"cause\":1"),
        text.replace("v1:1", "v1:01"),
        " ".repeat(MAX_PAGE_BYTES + 1),
    ] {
        assert!(
            decode_page(stream, bad.as_bytes(), 1).is_none(),
            "accepted invalid page"
        );
    }
    Ok(())
}

#[test]
fn security_revocation_default_off_real_config_and_distinct_identity() -> Result<(), &'static str> {
    assert!(
        StreamPuller::from_lookup(|_| None)
            .map_err(|_| "config")?
            .is_empty()
    );
    assert!(
        StreamPuller::from_lookup(
            |key| (key == SECURITY_REVOCATIONS_ENABLED_VAR).then(|| "false".into())
        )
        .map_err(|_| "config")?
        .is_empty()
    );
    assert!(
        StreamPuller::from_lookup(
            |key| (key == SECURITY_REVOCATIONS_ENABLED_VAR).then(|| "true".into())
        )
        .is_err()
    );
    let pem = b"-----BEGIN CERTIFICATE-----\nabc\n-----END CERTIFICATE-----";
    for other in [SANCTION_IDENTITY_VAR, LOYALTY_IDENTITY_VAR] {
        assert!(
            check_identities(
                &[(other, pem), (SECURITY_REVOCATION_IDENTITY_VAR, pem)],
                None
            )
            .is_err()
        );
    }
    Ok(())
}
