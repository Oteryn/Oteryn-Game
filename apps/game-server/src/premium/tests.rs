#![allow(clippy::unwrap_used)]

use super::refresh::{MIN_REFRESH_INTERVAL, REFRESH_LEAD, RETRY_CAP, after_failure, after_success};
use super::snapshot::{SnapshotRejection, canonical_uuid, rfc3339_utc_micros, validate};
use super::*;
use std::time::Duration;

const ACCOUNT: [u8; 16] = [
    0x01, 0x92, 0x3e, 0x4a, 0x5b, 0x6c, 0x7d, 0x8e, 0x9f, 0xa0, 0xb1, 0xc2, 0xd3, 0xe4, 0xf5, 0x06,
];
const NONCE: &str = "6b1f0c2d9e8a47f3a1b2c3d4e5f60718";
const ENT: &str = "01926c1e-8a40-7c3b-9f2e-5a1d3c4b6e70";
const HOUR: i64 = 3_600_000_000;
/// 2026-09-30T12:00:00Z.
const T0: i64 = 1_790_769_600_000_000;

fn body(overrides: &[(&str, serde_json::Value)]) -> Vec<u8> {
    let mut wire = serde_json::json!({
        "schema": SNAPSHOT_SCHEMA,
        "producer_revision": "0123456789abcdef0123456789abcdef01234567",
        "producer_profile": PRODUCER_PROFILE,
        "nonce": NONCE,
        "account_id": canonical_uuid(ACCOUNT),
        "product_id": PRODUCT_ID,
        "product_version": 1,
        "entitlement_id": ENT,
        "entitlement_state": "ACTIVE",
        "lifecycle_revision": 3,
        "authority_revision": 7,
        "effective_from": "2026-09-01T00:00:00Z",
        "effective_until": "2026-10-31T00:00:00Z",
        "authority_issued_at": "2026-09-30T12:00:00Z",
        "authority_valid_until": "2026-09-30T13:00:00Z",
        "refresh_after": "2026-09-30T12:40:00Z",
    });
    for (key, value) in overrides {
        if *value == absent() {
            wire.as_object_mut().unwrap().remove(*key);
        } else {
            wire[*key] = value.clone();
        }
    }
    serde_json::to_vec(&wire).unwrap()
}

/// An override that removes the member.
fn absent() -> serde_json::Value {
    serde_json::json!({"absent": true})
}

fn rejected(overrides: &[(&str, serde_json::Value)]) -> SnapshotRejection {
    validate(&body(overrides), ACCOUNT, NONCE)
        .unwrap_err()
        .rejection()
}

#[test]
fn a_bound_snapshot_is_evidence_with_absolute_times() {
    let e = validate(&body(&[]), ACCOUNT, NONCE).unwrap();
    assert_eq!(e.state, EntitlementState::Active);
    assert_eq!(e.entitlement_id.as_deref(), Some(ENT));
    assert_eq!((e.lifecycle_revision, e.authority_revision), (3, 7));
    assert_eq!(e.authority_issued_at_us, T0);
    assert_eq!(e.authority_valid_until_us, T0 + HOUR);
    assert_eq!(rfc3339_utc_micros("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(
        rfc3339_utc_micros("2000-02-29T00:00:01.5Z"),
        Some(951_782_401_500_000)
    );
}

#[test]
fn unbound_or_malformed_responses_are_dropped() {
    use SnapshotRejection::Malformed;
    let other = serde_json::json!("01923e4a-5b6c-7d8e-9fa0-b1c2d3e4f507");
    assert_eq!(rejected(&[("account_id", other)]), Malformed);
    assert_eq!(rejected(&[("nonce", "0".repeat(32).into())]), Malformed);
    assert_eq!(
        validate(&body(&[]), ACCOUNT, "").unwrap_err().rejection(),
        Malformed
    );
    assert_eq!(rejected(&[("extra", 1.into())]), Malformed);
    assert_eq!(
        rejected(&[("refresh_after", serde_json::Value::Null)]),
        Malformed
    );
    assert_eq!(rejected(&[("refresh_after", absent())]), Malformed);
    // `refresh_after` lies inside the snapshot's own authority interval (§4).
    for refresh in ["2026-09-30T11:59:59Z", "2026-09-30T13:00:01Z"] {
        assert_eq!(rejected(&[("refresh_after", refresh.into())]), Malformed);
    }
    assert_eq!(rejected(&[("authority_revision", (-1).into())]), Malformed);
    assert_eq!(
        rejected(&[("entitlement_state", "GRACE".into())]),
        Malformed
    );
    assert_eq!(
        rejected(&[("entitlement_id", serde_json::Value::Null)]),
        Malformed
    );
    assert_eq!(rejected(&[("entitlement_id", "a b".into())]), Malformed);
    assert_eq!(rejected(&[("entitlement_state", "NONE".into())]), Malformed);
    assert_eq!(
        rejected(&[("effective_until", "2026-08-01T00:00:00Z".into())]),
        Malformed
    );
    let issued_late = serde_json::json!("2026-09-30T13:00:00Z");
    assert_eq!(rejected(&[("authority_issued_at", issued_late)]), Malformed);
    for time in [
        "2026-09-30T12:00:00+00:00",
        "2026-09-30 12:00:00Z",
        "2026-02-29T00:00:00Z",
        "2026-09-30T24:00:00Z",
        "2026-09-30T12:00:60Z",
        "2026-09-30T12:00:00.1234567Z",
        "1969-12-31T23:59:59Z",
    ] {
        assert_eq!(
            rejected(&[("effective_from", time.into())]),
            Malformed,
            "{time}"
        );
    }
    let duplicate =
        String::from_utf8(body(&[]))
            .unwrap()
            .replacen('{', "{\"lifecycle_revision\":4,", 1);
    assert_eq!(
        validate(duplicate.as_bytes(), ACCOUNT, NONCE)
            .unwrap_err()
            .rejection(),
        Malformed
    );
    let mut large = body(&[]);
    large.truncate(large.len() - 1);
    large.extend(format!(",\"pad\":\"{}\"}}", " ".repeat(1024)).bytes());
    assert_eq!(
        validate(&large, ACCOUNT, NONCE).unwrap_err().rejection(),
        Malformed
    );
}

#[test]
fn the_producer_none_form_is_accepted_and_closed() {
    use serde_json::Value::Null;
    let none = [
        ("entitlement_id", Null),
        ("entitlement_state", "NONE".into()),
        ("lifecycle_revision", 0.into()),
        ("effective_from", Null),
        ("effective_until", Null),
    ];
    let e = validate(&body(&none), ACCOUNT, NONCE).unwrap();
    assert_eq!((e.state, e.entitlement_id), (EntitlementState::None, None));
    assert_eq!((e.effective_from_us, e.effective_until_us), (0, 0));
    // Any other combination fails closed: an interval, an entitlement or a lifecycle revision
    // in NONE, a missing nullable member, or a null interval in another state.
    for (key, value) in [
        ("effective_from", "2026-09-01T00:00:00Z".into()),
        ("effective_until", "2026-10-31T00:00:00Z".into()),
        ("entitlement_id", ENT.into()),
        ("lifecycle_revision", 3.into()),
        ("effective_from", absent()),
        ("entitlement_id", absent()),
    ] {
        let mut changed = none.to_vec();
        changed.push((key, value));
        assert_eq!(rejected(&changed), SnapshotRejection::Malformed, "{key}");
    }
    for key in ["effective_from", "effective_until"] {
        assert_eq!(rejected(&[(key, Null)]), SnapshotRejection::Malformed);
    }
    assert_eq!(
        rejected(&[("lifecycle_revision", 0.into())]),
        SnapshotRejection::Malformed
    );
}

#[test]
fn responses_outside_the_compatibility_record_fail_closed() {
    use SnapshotRejection::Unsupported;
    assert_eq!(
        rejected(&[("schema", "oteryn.premium_snapshot.v2".into())]),
        Unsupported
    );
    assert_eq!(
        rejected(&[("product_id", "oteryn.vip".into())]),
        Unsupported
    );
    assert_eq!(rejected(&[("product_version", 2.into())]), Unsupported);
    let profile = serde_json::json!("oteryn.entitlement.profile_b.v2");
    assert_eq!(rejected(&[("producer_profile", profile)]), Unsupported);
    // A future version that also adds members is still Unsupported, while it is bound.
    let v2 = serde_json::json!("oteryn.premium_snapshot.v2");
    assert_eq!(
        rejected(&[("schema", v2.clone()), ("tier", 2.into())]),
        Unsupported
    );
    assert_eq!(
        rejected(&[("product_version", 2.into()), ("tier", 2.into())]),
        Unsupported
    );
    assert_eq!(
        rejected(&[("schema", v2.clone()), ("nonce", "0".repeat(32).into())]),
        SnapshotRejection::Malformed
    );
    assert_eq!(
        rejected(&[
            ("schema", v2),
            ("account_id", canonical_uuid([0x62; 16]).into())
        ]),
        SnapshotRejection::Malformed
    );
    // A bound response with a missing or ill-typed compatibility member is malformed: it
    // changes nothing.
    for key in [
        "schema",
        "product_id",
        "product_version",
        "producer_profile",
    ] {
        assert_eq!(
            rejected(&[(key, serde_json::Value::Null)]),
            SnapshotRejection::Malformed
        );
        let ill_typed = if key == "product_version" {
            serde_json::json!("1")
        } else {
            serde_json::json!(1)
        };
        assert_eq!(rejected(&[(key, ill_typed)]), SnapshotRejection::Malformed);
    }
    assert_eq!(
        rejected(&[("product_version", (-1).into())]),
        SnapshotRejection::Malformed
    );
    let long_lease = serde_json::json!("2026-09-30T13:00:01Z");
    assert_eq!(
        rejected(&[("authority_valid_until", long_lease.clone())]),
        Unsupported
    );
    // The Unsupported failure carries the bounded facts of the conflict and audit rows.
    let failure = match validate(
        &body(&[("producer_profile", "oteryn.entitlement.profile_b.v2".into())]),
        ACCOUNT,
        NONCE,
    ) {
        Err(snapshot::SnapshotFailure::Unsupported(failure)) => failure,
        other => unreachable!("{other:?}"),
    };
    assert_eq!(
        (failure.account_id, failure.authority_revision),
        (ACCOUNT, 7)
    );
    assert_eq!(failure.producer_profile, "oteryn.entitlement.profile_b.v2");
}

#[test]
fn only_a_complete_well_formed_envelope_is_unsupported() {
    use SnapshotRejection::{Malformed, Unsupported};
    let v2 = || ("schema", serde_json::json!("oteryn.premium_snapshot.v2"));
    let profile = || ("producer_profile", serde_json::json!("oteryn.other.v9"));
    // An incompatible schema or profile with a missing or mistyped baseline member, or with a
    // malformed value form, is a failed pull: never the permanent marker (§3.1 item 2).
    for incompatible in [v2(), profile()] {
        for broken in [
            ("authority_issued_at", "2026-09-30 12:00:00Z".into()),
            ("effective_until", "2026-02-30T00:00:00Z".into()),
            ("refresh_after", absent()),
            ("authority_revision", "7".into()),
            ("entitlement_state", "GRACE".into()),
            ("entitlement_id", "a b".into()),
            ("producer_revision", "".into()),
            ("refresh_after", "2026-09-30T14:00:00Z".into()),
            ("entitlement_id", serde_json::Value::Null),
        ] {
            assert_eq!(
                rejected(&[incompatible.clone(), broken.clone()]),
                Malformed,
                "{incompatible:?} {broken:?}"
            );
        }
        // Complete and well formed: Unsupported, even with members a later version adds.
        assert_eq!(rejected(std::slice::from_ref(&incompatible)), Unsupported);
        assert_eq!(rejected(&[incompatible, ("tier", 2.into())]), Unsupported);
    }
    // Under this schema an unknown member is malformed, before the lease policy.
    assert_eq!(rejected(&[("tier", 2.into())]), Malformed);
    let long_lease = ("authority_valid_until", "2026-09-30T13:00:01Z".into());
    assert_eq!(
        rejected(&[long_lease.clone(), ("tier", 2.into())]),
        Malformed
    );
    // A long lease with a malformed member stays malformed.
    assert_eq!(
        rejected(&[long_lease, ("effective_from", "2026-09-01".into())]),
        Malformed
    );
}

fn view(state: EntitlementState, proven: bool) -> AccountView {
    let mut latest = validate(&body(&[]), ACCOUNT, NONCE).unwrap();
    latest.state = state;
    AccountView {
        fence: Some(PremiumFenceView {
            latest,
            conflicting: false,
        }),
        proven,
        ..AccountView::default()
    }
}

fn at(now_us: i64) -> Option<TrustedNow> {
    TrustedNow::new(now_us, MAX_CLOCK_SKEW_US)
}

#[test]
fn only_proven_active_evidence_inside_its_interval_is_current() {
    use PremiumClass::*;
    let active = view(EntitlementState::Active, true);
    assert_eq!(classify(Some(&active), at(T0 + 1)), CurrentAuthority);
    // From `refresh_after` the evidence is stale and denies (PREM-P §3, §8.2); a lapsed lease
    // is EXPIRED (§8.3); the conservative bound included in both.
    let refresh = T0 + HOUR * 2 / 3;
    assert_eq!(
        classify(Some(&active), at(refresh - MAX_CLOCK_SKEW_US - 1)),
        CurrentAuthority
    );
    assert_eq!(
        classify(Some(&active), at(refresh - MAX_CLOCK_SKEW_US)),
        StaleWithinBound
    );
    assert_eq!(
        classify(Some(&active), at(T0 + HOUR - MAX_CLOCK_SKEW_US - 1)),
        StaleWithinBound
    );
    assert_eq!(
        classify(Some(&active), at(T0 + HOUR - MAX_CLOCK_SKEW_US)),
        Expired
    );
    // Unsynchronized time or too much uncertainty: not current.
    assert_eq!(classify(Some(&active), None), AuthorityUnavailable);
    assert_eq!(TrustedNow::new(T0, MAX_CLOCK_SKEW_US + 1), None);
    // Durable evidence loaded after a restart authorizes nothing until re-proven (§6.3).
    let loaded = view(EntitlementState::Active, false);
    assert_eq!(classify(Some(&loaded), at(T0 + 1)), AuthorityUnavailable);
    // A fence write failure leaves the fence unsafe (consumer contract §6.4).
    let quarantined = AccountView {
        quarantined: true,
        ..active.clone()
    };
    // A failed pull denies at once, while the cached ACTIVE evidence is inside its interval.
    let unavailable = AccountView {
        unavailable: true,
        ..active.clone()
    };
    assert_eq!(
        classify(Some(&unavailable), at(T0 + 1)),
        AuthorityUnavailable
    );
    assert_eq!(
        classify(Some(&quarantined), at(T0 + 1)),
        InvalidOrConflicting
    );
    assert_eq!(classify(None, at(T0 + 1)), AuthorityUnavailable);
    let conflicting = AccountView {
        conflicting: true,
        ..active.clone()
    };
    assert_eq!(
        classify(Some(&conflicting), at(T0 + 1)),
        InvalidOrConflicting
    );
    // Overlapping pulls: proof from a pull started before a newer failure does not clear it,
    // whichever finishes last; proof from a later pull does. A failure from a pull started
    // before the latest proof changes nothing.
    let fence = active.fence.clone().unwrap();
    type Fail = fn(&mut AccountView, PullTicket);
    let failures: [(Fail, PremiumClass); 2] = [
        (AccountView::quarantine, InvalidOrConflicting),
        (AccountView::fail_pull, AuthorityUnavailable),
    ];
    for (fail, denied) in failures {
        let mut racing = active.clone();
        fail(&mut racing, PullTicket(2));
        racing.prove(PullTicket(1), fence.clone());
        assert_eq!(classify(Some(&racing), at(T0 + 1)), denied);
        racing.prove(PullTicket(3), fence.clone());
        assert_eq!(classify(Some(&racing), at(T0 + 1)), CurrentAuthority);
        fail(&mut racing, PullTicket(2));
        assert_eq!(classify(Some(&racing), at(T0 + 1)), CurrentAuthority);
        fail(&mut racing, PullTicket(4));
        assert_eq!(classify(Some(&racing), at(T0 + 1)), denied);
    }
    // Restrictive kept evidence still wins over a failed pull.
    let mut revoked = view(EntitlementState::Revoked, true);
    revoked.fail_pull(PullTicket(1));
    assert_eq!(classify(Some(&revoked), at(T0 + 1)), Revoked);
    let mut conflicting = active.clone();
    conflicting.fence.as_mut().unwrap().conflicting = true;
    assert_eq!(
        classify(Some(&conflicting), at(T0 + 1)),
        InvalidOrConflicting
    );
}

#[test]
fn restrictive_facts_win_and_start_is_conservative() {
    use PremiumClass::*;
    for (state, class) in [
        (EntitlementState::Revoked, Revoked),
        (EntitlementState::Expired, Expired),
        (EntitlementState::None, NoEntitlement),
        (EntitlementState::NotYetEffective, NotYetEffective),
    ] {
        assert_eq!(classify(Some(&view(state, true)), at(T0 + 1)), class);
        assert_eq!(classify(Some(&view(state, false)), None), class);
    }
    let mut early = view(EntitlementState::Active, true);
    let start = T0 + 10 * MAX_CLOCK_SKEW_US;
    early.fence.as_mut().unwrap().latest.effective_from_us = start;
    // Uncertainty straddling the start delays it.
    assert_eq!(classify(Some(&early), at(start)), NotYetEffective);
    assert_eq!(
        classify(Some(&early), at(start + MAX_CLOCK_SKEW_US)),
        CurrentAuthority
    );
    // The commercial end wins over a longer lease.
    let mut ending = view(EntitlementState::Active, true);
    ending.fence.as_mut().unwrap().latest.effective_until_us = T0 + HOUR / 2;
    assert_eq!(classify(Some(&ending), at(T0 + HOUR / 2)), Expired);
}

#[test]
fn surfaces_all_require_current_authority() {
    for surface in [
        Surface::FreshAdmission,
        Surface::ReconnectOrRecovery,
        Surface::RunningSession,
    ] {
        assert_eq!(surface_policy(surface), SurfacePolicy::RequireCurrent);
    }
}

#[test]
fn ended_means_the_entitlement_ended_never_a_lapsed_lease() {
    let consumer = PremiumConsumer::default();
    let put = |v: AccountView| {
        consumer.lock().insert(ACCOUNT, v);
    };
    let ended = |now| consumer.premium_entitlement_ended(ACCOUNT, now);
    assert!(!ended(at(T0)));
    put(view(EntitlementState::Active, true));
    assert!(!ended(at(T0 + 2 * HOUR)), "a lapsed lease never relocates");
    assert!(!consumer.premium_current(ACCOUNT, at(T0 + 2 * HOUR)));
    let until = view(EntitlementState::Active, false)
        .fence
        .unwrap()
        .latest
        .effective_until_us;
    assert!(ended(at(until + MAX_CLOCK_SKEW_US)));
    assert!(!ended(at(until)), "the end must surely have passed");
    assert!(!ended(None));
    for state in [
        EntitlementState::Expired,
        EntitlementState::Revoked,
        EntitlementState::None,
    ] {
        put(view(state, false));
        assert!(ended(None));
    }
    // A semantic failure or a failed pull neither makes it true nor clears it (§3.1, §6).
    let mut conflicting = view(EntitlementState::Revoked, true);
    conflicting.fence.as_mut().unwrap().conflicting = true;
    conflicting.conflicting = true;
    conflicting.fail_pull(PullTicket(1));
    put(conflicting);
    assert!(ended(at(T0)));
    let mut failed = view(EntitlementState::Active, true);
    failed.fail_pull(PullTicket(1));
    failed.conflicting = true;
    put(failed);
    assert!(!ended(at(T0)));
    consumer.release(ACCOUNT);
    assert!(!ended(None));
}

#[test]
fn a_merge_never_lowers_the_high_water_or_clears_a_conflict() {
    let mut current = view(EntitlementState::Revoked, true);
    let mut older = view(EntitlementState::Active, true).fence.unwrap();
    older.latest.authority_revision = 6;
    merge(&mut current, Some(older));
    assert_eq!(classify(Some(&current), at(T0 + 1)), PremiumClass::Revoked);
    current.fence.as_mut().unwrap().conflicting = true;
    let mut newer = view(EntitlementState::Active, true).fence.unwrap();
    newer.latest.authority_revision = 8;
    merge(&mut current, Some(newer));
    let fence = current.fence.unwrap();
    assert_eq!(
        (fence.latest.authority_revision, fence.conflicting),
        (8, true)
    );
}

#[test]
fn refresh_leads_refresh_after_and_waits_at_least_a_minute() {
    let minute = MIN_REFRESH_INTERVAL;
    // A valid past `refresh_after` does not schedule a pull within 60 seconds.
    assert_eq!(after_success(T0 - HOUR, T0), minute);
    assert_eq!(after_success(T0, T0), minute);
    assert_eq!(after_success(T0 + 30_000_000, T0), minute);
    // The pull starts `REFRESH_LEAD` before `refresh_after` (PREM-P §8.5).
    assert_eq!(REFRESH_LEAD, Duration::from_secs(5 * 60));
    assert_eq!(after_success(T0 + 6 * 60_000_000, T0), minute);
    assert_eq!(
        after_success(T0 + 40 * 60_000_000, T0),
        Duration::from_secs(35 * 60)
    );
    // A lease clipped to 30 minutes refreshes at 20 minutes; the pull starts at 15.
    assert_eq!(
        after_success(T0 + 20 * 60_000_000, T0),
        Duration::from_secs(15 * 60)
    );
}

#[test]
fn retries_back_off_exponentially_with_jitter_under_the_cap() {
    assert_eq!(after_failure(0, None, 0.0), Duration::from_millis(500));
    assert_eq!(after_failure(0, None, 1.0), Duration::from_secs(1));
    assert_eq!(after_failure(3, None, 1.0), Duration::from_secs(8));
    assert_eq!(after_failure(30, None, 1.0), RETRY_CAP);
    assert!(after_failure(30, None, 0.0) >= RETRY_CAP / 2);
    // A 429 or 503 `Retry-After` is honoured within the cap.
    let wait = Some(Duration::from_secs(7));
    assert_eq!(after_failure(0, wait, 0.5), Duration::from_secs(7));
    let long = Some(Duration::from_secs(3_600));
    assert_eq!(after_failure(0, long, 0.5), RETRY_CAP);
}

#[test]
fn the_request_is_the_exact_bounded_section_3_1_body() {
    let nonce = client::fresh_nonce().unwrap();
    assert_eq!(nonce.len(), 32);
    assert!(
        nonce
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    assert_ne!(client::fresh_nonce().unwrap(), nonce, "fresh per request");
    let body = client::request_body(ACCOUNT, &nonce).unwrap();
    assert!(body.len() <= client::MAX_REQUEST_BYTES);
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "schema": "oteryn.premium_snapshot_request.v1",
            "account_id": "01923e4a-5b6c-7d8e-9fa0-b1c2d3e4f506",
            "nonce": nonce,
        })
    );
    assert_eq!(client::request_body(ACCOUNT, &"x".repeat(200)), None);
}

#[test]
fn retry_after_is_honoured_in_either_form() {
    use client::parse_retry_after;
    // 1994-11-06T08:49:37Z.
    let at = 784_111_777_000_000;
    assert_eq!(parse_retry_after("7", at), Some(Duration::from_secs(7)));
    assert_eq!(
        parse_retry_after(" 120 ", at),
        Some(Duration::from_secs(120))
    );
    let date = "Sun, 06 Nov 1994 08:49:37 GMT";
    assert_eq!(
        parse_retry_after(date, at - 30_000_000),
        Some(Duration::from_secs(30))
    );
    assert_eq!(parse_retry_after(date, at + 1), Some(Duration::ZERO));
    for other in [
        "Sunday, 06-Nov-94 08:49:37 GMT",
        "Sun Nov  6 08:49:37 1994",
        "Sun, 06 Nov 1994 08:49:37 UTC",
        "Sun, 31 Feb 1994 08:49:37 GMT",
        "-1",
        "",
        "soon",
    ] {
        assert_eq!(parse_retry_after(other, at), None, "{other}");
    }
}

#[test]
fn a_partial_client_configuration_is_an_error() {
    use client::{IDENTITY_VAR, PLATFORM_CA_VAR, PremiumClientConfig, URL_VAR};
    use std::ffi::OsString;
    let some = |text: &str| Some(OsString::from(text));
    assert_eq!(
        PremiumClientConfig::from_vars(None, None, None)
            .unwrap()
            .map(|c| c.origin),
        None
    );
    let invalid =
        |url, identity, ca| PremiumClientConfig::from_vars(url, identity, ca).unwrap_err();
    assert_eq!(
        invalid(None, some("/id.pem"), some("/ca.pem")),
        client::ClientConfigError::Invalid(URL_VAR)
    );
    assert_eq!(
        invalid(some("https://p:1"), None, some("/ca.pem")),
        client::ClientConfigError::Invalid(IDENTITY_VAR)
    );
    assert_eq!(
        invalid(some("https://p:1"), some("/id.pem"), None),
        client::ClientConfigError::Invalid(PLATFORM_CA_VAR)
    );
    assert_eq!(
        invalid(None, None, some("/ca.pem")),
        client::ClientConfigError::Invalid(URL_VAR)
    );
    // An unreadable file is an error too.
    assert_eq!(
        invalid(
            some("https://p:1"),
            some("/nonexistent/oteryn-premium-id.pem"),
            some("/nonexistent/oteryn-premium-ca.pem")
        ),
        client::ClientConfigError::Invalid(IDENTITY_VAR)
    );
}

#[test]
fn a_supported_envelope_obeys_the_platform_v1_rules() {
    use SnapshotRejection::{Malformed, Unsupported};
    let rev = |text: &str| ("producer_revision", serde_json::json!(text));
    let ent = |text: &str| ("entitlement_id", serde_json::json!(text));
    let time = |key, text: &str| (key, serde_json::json!(text));
    // Identifier forms and revision bounds (PREM-P §5.1).
    for case in [
        vec![rev("c914564")],
        vec![rev("0123456789ABCDEF0123456789ABCDEF01234567")],
        vec![ent("ent-1")],
        vec![ent("01926C1E-8A40-7C3B-9F2E-5A1D3C4B6E70")],
        vec![ent("01926c1e-8a40-4c3b-9f2e-5a1d3c4b6e70")],
        vec![ent("01926c1e-8a40-7c3b-cf2e-5a1d3c4b6e70")],
        vec![ent("01926c1e8a407c3b9f2e5a1d3c4b6e70----")],
        vec![("authority_revision", 0.into())],
        vec![(
            "authority_revision",
            (snapshot::MAX_WIRE_REVISION + 1).into(),
        )],
        vec![(
            "lifecycle_revision",
            (snapshot::MAX_WIRE_REVISION + 1).into(),
        )],
        // Whole seconds only.
        vec![time("authority_issued_at", "2026-09-30T12:00:00.000Z")],
        vec![time("effective_until", "2026-10-31T00:00:00.5Z")],
        // The cutoff is min(issued + 3600 s, effective_until) for ACTIVE.
        vec![
            time("authority_valid_until", "2026-09-30T12:30:00Z"),
            time("refresh_after", "2026-09-30T12:20:00Z"),
        ],
        // Refresh at exactly two thirds of the lease, strictly before the cutoff.
        vec![time("refresh_after", "2026-09-30T12:30:00Z")],
        vec![time("refresh_after", "2026-09-30T13:00:00Z")],
        // The state matches the interval at issue.
        vec![time("effective_from", "2026-09-30T12:00:01Z")],
        vec![("entitlement_state", "EXPIRED".into())],
        vec![("entitlement_state", "NOT_YET_EFFECTIVE".into())],
        // EXPIRED, REVOKED and NONE always carry the full lease.
        vec![
            ("entitlement_state", "REVOKED".into()),
            time("authority_valid_until", "2026-09-30T12:30:00Z"),
            time("refresh_after", "2026-09-30T12:20:00Z"),
        ],
    ] {
        assert_eq!(rejected(&case), Malformed, "{case:?}");
    }
    // A clipped lease, an expiry at issue, a revocation and a future start are accepted.
    for case in [
        vec![
            time("effective_until", "2026-09-30T12:30:00Z"),
            time("authority_valid_until", "2026-09-30T12:30:00Z"),
            time("refresh_after", "2026-09-30T12:20:00Z"),
        ],
        vec![
            ("entitlement_state", "EXPIRED".into()),
            time("effective_until", "2026-09-30T12:00:00Z"),
        ],
        vec![
            ("entitlement_state", "REVOKED".into()),
            time("effective_until", "2026-09-20T00:00:00Z"),
        ],
        vec![
            ("entitlement_state", "NOT_YET_EFFECTIVE".into()),
            time("effective_from", "2026-10-01T00:00:00Z"),
        ],
        vec![
            time("effective_until", "2026-09-30T12:00:01Z"),
            time("authority_valid_until", "2026-09-30T12:00:01Z"),
            time("refresh_after", "2026-09-30T12:00:00Z"),
        ],
    ] {
        assert!(validate(&body(&case), ACCOUNT, NONCE).is_ok(), "{case:?}");
    }
    // The v1 rules belong to the product version: an unsupported envelope stays Unsupported.
    let v2 = ("schema", serde_json::json!("oteryn.premium_snapshot.v2"));
    assert_eq!(rejected(&[v2.clone(), rev("c914564")]), Unsupported);
    assert_eq!(
        rejected(&[v2, time("refresh_after", "2026-09-30T12:30:00Z")]),
        Unsupported
    );
}

/// Platform's shared fixtures (`tests/fixtures/premium-snapshot-v1/`, PROVENANCE.md).
const FIXTURE_ACCOUNT: [u8; 16] = [
    0x01, 0x89, 0x0f, 0x4e, 0x7c, 0x00, 0x70, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
];
const FIXTURE_NONCE: &str = "4f1c2a9be07d4c3a8b6e5d4c3b2a1908";
/// 2026-10-01T12:00:00Z, the fixtures' `authority_issued_at`.
const FIXTURE_ISSUED: i64 = T0 + 24 * HOUR;
const MINUTE: i64 = 60_000_000;

macro_rules! fixture {
    ($name:literal) => {
        include_bytes!(concat!(
            "../../../../tests/fixtures/premium-snapshot-v1/",
            $name
        ))
        .as_slice()
    };
}

#[test]
fn platform_fixtures_classify_as_the_contract_requires() {
    use PremiumClass::*;
    let proven = |bytes: &[u8]| {
        let latest = validate(bytes, FIXTURE_ACCOUNT, FIXTURE_NONCE).unwrap();
        AccountView {
            fence: Some(PremiumFenceView {
                latest,
                conflicting: false,
            }),
            proven: true,
            ..AccountView::default()
        }
    };
    let class = |view: &AccountView, minutes: i64| {
        classify(Some(view), at(FIXTURE_ISSUED + minutes * MINUTE))
    };
    let skew = MAX_CLOCK_SKEW_US;
    // ACTIVE, full lease: current within [effective_from, refresh_after) conservatively,
    // STALE_WITHIN_BOUND (deny) from refresh_after, EXPIRED from authority_valid_until.
    let active = proven(fixture!("snapshot.active.json"));
    assert_eq!(class(&active, 0), CurrentAuthority);
    let at_us = |us| classify(Some(&active), at(us));
    assert_eq!(
        at_us(FIXTURE_ISSUED + 40 * MINUTE - skew - 1),
        CurrentAuthority
    );
    assert_eq!(at_us(FIXTURE_ISSUED + 40 * MINUTE - skew), StaleWithinBound);
    assert_eq!(class(&active, 59), StaleWithinBound);
    assert_eq!(class(&active, 60), Expired);
    // ACTIVE, lease clipped to the commercial end: refresh at two thirds of 30 minutes.
    let clipped = proven(fixture!("snapshot.active-clipped.json"));
    assert_eq!(class(&clipped, 19), CurrentAuthority);
    assert_eq!(class(&clipped, 20), StaleWithinBound);
    assert_eq!(class(&clipped, 30), Expired);
    // EXPIRED, REVOKED and NONE deny, whatever the time.
    for (bytes, denied) in [
        (fixture!("snapshot.expired.json"), Expired),
        (fixture!("snapshot.revoked.json"), Revoked),
        (fixture!("snapshot.none.json"), NoEntitlement),
    ] {
        let view = proven(bytes);
        for minutes in [0, 20, 45, 61] {
            assert_eq!(class(&view, minutes), denied);
        }
    }
    // Only the active fixtures ever read as Premium.
    let consumer = PremiumConsumer::default();
    consumer.lock().insert(FIXTURE_ACCOUNT, active);
    assert!(consumer.premium_current(FIXTURE_ACCOUNT, at(FIXTURE_ISSUED)));
    assert!(!consumer.premium_entitlement_ended(FIXTURE_ACCOUNT, at(FIXTURE_ISSUED)));
    consumer
        .lock()
        .insert(FIXTURE_ACCOUNT, proven(fixture!("snapshot.revoked.json")));
    assert!(!consumer.premium_current(FIXTURE_ACCOUNT, at(FIXTURE_ISSUED)));
    assert!(consumer.premium_entitlement_ended(FIXTURE_ACCOUNT, at(FIXTURE_ISSUED)));
}
