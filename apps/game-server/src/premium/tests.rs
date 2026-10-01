#![allow(clippy::unwrap_used)]

use super::snapshot::{SnapshotRejection, canonical_uuid, rfc3339_utc_micros, validate};
use super::*;

const ACCOUNT: [u8; 16] = [
    0x01, 0x92, 0x3e, 0x4a, 0x5b, 0x6c, 0x7d, 0x8e, 0x9f, 0xa0, 0xb1, 0xc2, 0xd3, 0xe4, 0xf5, 0x06,
];
const NONCE: &str = "6b1f0c2d9e8a47f3a1b2c3d4e5f60718";
const HOUR: i64 = 3_600_000_000;
/// 2026-09-30T12:00:00Z.
const T0: i64 = 1_790_769_600_000_000;

fn body(overrides: &[(&str, serde_json::Value)]) -> Vec<u8> {
    let mut wire = serde_json::json!({
        "schema": SNAPSHOT_SCHEMA,
        "producer_revision": "c914564",
        "producer_profile": PRODUCER_PROFILE,
        "nonce": NONCE,
        "account_id": canonical_uuid(ACCOUNT),
        "product_id": PRODUCT_ID,
        "product_version": 1,
        "entitlement_id": "ent-1",
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
        match value {
            serde_json::Value::Null if *key != "entitlement_id" => {
                wire.as_object_mut().unwrap().remove(*key);
            }
            _ => wire[*key] = value.clone(),
        }
    }
    serde_json::to_vec(&wire).unwrap()
}

fn rejected(overrides: &[(&str, serde_json::Value)]) -> SnapshotRejection {
    validate(&body(overrides), ACCOUNT, NONCE).unwrap_err()
}

#[test]
fn a_bound_snapshot_is_evidence_with_absolute_times() {
    let e = validate(&body(&[]), ACCOUNT, NONCE).unwrap();
    assert_eq!(e.state, EntitlementState::Active);
    assert_eq!(e.entitlement_id.as_deref(), Some("ent-1"));
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
    assert_eq!(validate(&body(&[]), ACCOUNT, "").unwrap_err(), Malformed);
    assert_eq!(rejected(&[("extra", 1.into())]), Malformed);
    assert_eq!(
        rejected(&[("refresh_after", serde_json::Value::Null)]),
        Malformed
    );
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
    let duplicate = String::from_utf8(body(&[]))
        .unwrap()
        .replacen('{', "{\"nonce\":\"x\",", 1);
    assert_eq!(
        validate(duplicate.as_bytes(), ACCOUNT, NONCE).unwrap_err(),
        Malformed
    );
    let mut large = body(&[]);
    large.truncate(large.len() - 1);
    large.extend(format!(",\"pad\":\"{}\"}}", " ".repeat(1024)).bytes());
    assert_eq!(validate(&large, ACCOUNT, NONCE).unwrap_err(), Malformed);
}

#[test]
fn a_none_snapshot_has_no_entitlement() {
    let none = [
        ("entitlement_id", serde_json::Value::Null),
        ("entitlement_state", "NONE".into()),
    ];
    let e = validate(&body(&none), ACCOUNT, NONCE).unwrap();
    assert_eq!((e.state, e.entitlement_id), (EntitlementState::None, None));
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
        rejected(&[("authority_valid_until", long_lease)]),
        Unsupported
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
    // A lapsed lease is EXPIRED (§8.3), the conservative bound included.
    assert_eq!(
        classify(Some(&active), at(T0 + HOUR - MAX_CLOCK_SKEW_US)),
        Expired
    );
    assert_eq!(
        classify(Some(&active), at(T0 + HOUR - MAX_CLOCK_SKEW_US - 1)),
        CurrentAuthority
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
    assert_eq!(
        classify(Some(&quarantined), at(T0 + 1)),
        InvalidOrConflicting
    );
    assert_eq!(classify(None, at(T0 + 1)), AuthorityUnavailable);
    let unsupported = AccountView {
        unsupported: true,
        ..active.clone()
    };
    assert_eq!(
        classify(Some(&unsupported), at(T0 + 1)),
        InvalidOrConflicting
    );
    // Overlapping ingests: proof from an ingest started before a newer failure does not clear
    // it, whichever finishes last; proof from a later ingest does.
    let fence = active.fence.clone().unwrap();
    for unsupported in [false, true] {
        let mut racing = active.clone();
        racing.fail(2, unsupported);
        racing.prove(1, fence.clone());
        assert_eq!(classify(Some(&racing), at(T0 + 1)), InvalidOrConflicting);
        racing.prove(3, fence.clone());
        assert_eq!(classify(Some(&racing), at(T0 + 1)), CurrentAuthority);
    }
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
    let mut conflicting = view(EntitlementState::Revoked, true);
    conflicting.fence.as_mut().unwrap().conflicting = true;
    put(conflicting);
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
