use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture(name: &str) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(format!(
        "{}/../../../docs/contracts/fixtures/profile-b-authority-v1/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
}
#[test]
fn profile_b_fixtures_round_trip_byte_exact() -> TestResult {
    for name in [
        "authority-request",
        "authority",
        "changes-request",
        "changes-page",
        "changes-empty",
    ] {
        let raw = fixture(name)?;
        let value = Message::parse(&raw).ok_or("fixture rejected")?;
        assert_eq!(value.encode().ok_or("encode")?, raw);
    }
    Ok(())
}
fn answer() -> Result<Answer, Box<dyn std::error::Error>> {
    match Message::parse(&fixture("authority")?).ok_or("answer")? {
        Message::Answer(a) => Ok(a),
        _ => Err("answer type".into()),
    }
}
#[test]
fn profile_b_rejects_malformed_bounds_and_lease_over_3600() -> TestResult {
    let a = answer()?;
    let mut cases = Vec::new();
    let mut b = a.clone();
    b.authority_revision = 0;
    cases.push(b);
    let mut b = a.clone();
    b.authority_revision = INT53_MAX + 1;
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].product_version = 0;
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].lifecycle_revision = INT53_MAX + 1;
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].authority_valid_until = "2023-11-14T23:13:21Z".into();
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].refresh_after = b.entitlements[0].authority_valid_until.clone();
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].entitlement_id = None;
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[1].effective_from = Some(a.authority_issued_at.clone());
    cases.push(b);
    let mut b = a.clone();
    b.entitlements[0].lease_policy = "other.v1".into();
    cases.push(b);
    let mut b = a.clone();
    b.entitlements.push(b.entitlements[0].clone());
    cases.push(b);
    let mut b = a.clone();
    b.account_id = "019a0000-0000-4000-8000-000000000002".into();
    cases.push(b);
    for b in cases {
        assert!(Message::parse(&serde_json::to_vec(&b)?).is_none());
    }
    let raw = String::from_utf8(fixture("authority")?)?;
    for invalid in [
        raw.replacen("\"authority_revision\":7", "\"authority_revision\":7.0", 1),
        raw.replacen("\"authority_revision\":7", "\"authority_revision\":7e0", 1),
        raw.replacen("\"authority_revision\":7", "\"authority_revision\":true", 1),
        raw.replacen("\"authority_revision\":7,", "", 1),
        raw.replacen(
            "\"authority_revision\":7",
            "\"authority_revision\":7,\"authority_revision\":7",
            1,
        ),
        raw.replacen(
            "\"authority_revision\":7",
            "\"authority_revision\":7,\"unknown\":1",
            1,
        ),
        raw.replacen("\"effective_until\":null,", "", 1),
        format!("{raw}\n"),
        " ".repeat(AUTHORITY_BYTES + 1),
    ] {
        assert!(Message::parse(invalid.as_bytes()).is_none());
    }
    Ok(())
}
#[test]
fn profile_b_lease_failure_matrix_and_boundaries() -> TestResult {
    let a = answer()?;
    let e = &a.entitlements[0];
    let issued = time(&a.authority_issued_at).ok_or("time")?;
    let at = |seconds: i64, uncertainty: i64| {
        Some(GameNow {
            now_us: issued + seconds * 1_000_000,
            uncertainty_us: uncertainty,
        })
    };
    // Unavailability never refreshes a lease: outages of 30/40/60 minutes age these same facts.
    for (seconds, expected) in [
        (0, Evaluation::Active),
        (1800, Evaluation::Active),
        (2399, Evaluation::Active),
        (2400, Evaluation::Stale),
        (3599, Evaluation::Stale),
        (3600, Evaluation::Expired),
        (7200, Evaluation::Expired),
    ] {
        assert_eq!(evaluate(e, at(seconds, 0), 0), expected);
    }
    assert_eq!(evaluate(e, at(2399, 5_000_000), 0), Evaluation::Stale);
    assert_eq!(evaluate(e, at(1, 5_000_001), 0), Evaluation::None);
    assert_eq!(evaluate(e, None, 0), Evaluation::None);
    assert_eq!(
        evaluate(e, at(1, 0), e.lifecycle_revision + 1),
        Evaluation::Stale
    );
    assert_eq!(evaluate(&a.entitlements[1], at(1, 0), 4), Evaluation::None);
    let mut revoked = e.clone();
    revoked.entitlement_state = CommercialState::Revoked;
    assert_eq!(evaluate(&revoked, at(1, 0), 0), Evaluation::None);
    let mut future = e.clone();
    future.effective_from = Some("2023-11-14T22:14:20Z".into());
    future.entitlement_state = CommercialState::NotYetEffective;
    assert_eq!(evaluate(&future, at(64, 5_000_000), 0), Evaluation::None);
    assert_eq!(evaluate(&future, at(65, 5_000_000), 0), Evaluation::Active);
    let mut clipped = a.clone();
    let item = &mut clipped.entitlements[0];
    item.effective_until = Some("2023-11-14T22:23:20Z".into());
    item.authority_valid_until = "2023-11-14T22:23:20Z".into();
    item.refresh_after = "2023-11-14T22:20:00Z".into();
    assert!(clipped.valid());
    assert!(
        time(&clipped.entitlements[0].refresh_after)
            < time(&clipped.entitlements[0].authority_valid_until)
    );
    assert_eq!(
        evaluate(&clipped.entitlements[0], at(600, 0), 0),
        Evaluation::Expired
    );
    let mut ended = a.clone();
    ended.entitlements[0].effective_until = Some(a.authority_issued_at.clone());
    ended.entitlements[0].entitlement_state = CommercialState::Expired;
    assert!(ended.valid());
    assert_eq!(
        evaluate(&ended.entitlements[0], at(0, 0), 0),
        Evaluation::Expired
    );
    let mut open = a.clone();
    open.entitlements[0].effective_until = None;
    assert!(open.valid());
    assert_eq!(
        time(&open.entitlements[0].authority_valid_until).ok_or("time")? - issued,
        MAX_LEASE_US
    );
    Ok(())
}
