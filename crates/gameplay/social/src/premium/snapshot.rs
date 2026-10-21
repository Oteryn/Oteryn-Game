//! `oteryn.premium_snapshot.v1` validation (PREMIUM-DELIVERY-0 §3, §3.1 and §4).
//!
//! [`validate`] turns one response body into [`PremiumEvidence`] or refuses it. A body that is
//! oversized, malformed, bound to another request (nonce) or another account is
//! [`SnapshotRejection::Malformed`] and changes nothing (consumer contract §8.1 step 1): a failed
//! pull. Only a complete, well-formed envelope (every §4 member present with its baseline type
//! and value form, the closed `NONE` variant and the structural interval checks) is compared with
//! the compatibility record of this consumer, at the very end (§3.1 item 2): outside it (schema,
//! product, product version, producer profile, or a lease above the bound product policy) it is
//! [`SnapshotRejection::Unsupported`], a durable semantic failure (§4, §8.1 step 4). A supported
//! envelope must then obey Platform's v1 rules (PREMIUM-DELIVERY-0 §12.3): breaking one is
//! `Malformed` again.

use super::{
    MAX_AUTHORITY_LEASE_US, PRODUCER_PROFILE, PRODUCT_ID, PRODUCT_VERSION, SNAPSHOT_SCHEMA,
};
use crate::durability::premium_fence::{
    EntitlementState, PremiumConflictKind, PremiumEvidence, PremiumSemanticFailure,
};
use serde::Deserialize;

/// `PREMDEL0-RL-01`.
pub const MAX_SNAPSHOT_BYTES: usize = 1024;

/// The §4 members. A member outside this list is malformed under this schema.
const MEMBERS: [&str; 16] = [
    "schema",
    "producer_revision",
    "producer_profile",
    "nonce",
    "account_id",
    "product_id",
    "product_version",
    "entitlement_id",
    "entitlement_state",
    "lifecycle_revision",
    "authority_revision",
    "effective_from",
    "effective_until",
    "authority_issued_at",
    "authority_valid_until",
    "refresh_after",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotRejection {
    Malformed,
    Unsupported,
}

/// Why [`validate`] refused a body. `Unsupported` carries the bounded facts the durable conflict
/// and audit rows record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotFailure {
    Malformed,
    Unsupported(Box<PremiumSemanticFailure>),
}

impl SnapshotFailure {
    pub fn rejection(&self) -> SnapshotRejection {
        match self {
            Self::Malformed => SnapshotRejection::Malformed,
            Self::Unsupported(_) => SnapshotRejection::Unsupported,
        }
    }
}

/// The typed envelope. Unknown members are tolerated here and refused after the compatibility
/// comparison, so a future version that adds members is `Unsupported`; a repeated member fails
/// this parse.
#[derive(Deserialize)]
struct Wire {
    schema: String,
    producer_revision: String,
    producer_profile: String,
    #[allow(dead_code)] // bound on the untyped map
    nonce: String,
    #[allow(dead_code)] // bound on the untyped map
    account_id: String,
    product_id: String,
    product_version: u32,
    entitlement_id: Option<String>,
    entitlement_state: String,
    lifecycle_revision: u64,
    authority_revision: u64,
    effective_from: Option<String>,
    effective_until: Option<String>,
    authority_issued_at: String,
    authority_valid_until: String,
    refresh_after: String,
}

/// Validate `body`, the response to a request for `account_id` with `nonce`.
pub fn validate(
    body: &[u8],
    account_id: [u8; 16],
    nonce: &str,
) -> Result<PremiumEvidence, SnapshotFailure> {
    use SnapshotFailure::Malformed;
    if body.len() > MAX_SNAPSHOT_BYTES {
        return Err(Malformed);
    }
    let members: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(body).map_err(|_| Malformed)?;
    let text = |key: &str| members.get(key).and_then(serde_json::Value::as_str);
    if nonce.is_empty()
        || text("nonce") != Some(nonce)
        || text("account_id") != Some(canonical_uuid(account_id).as_str())
    {
        return Err(Malformed);
    }
    let wire: Wire = serde_json::from_slice(body).map_err(|_| Malformed)?;
    // The nullable members must be present, not defaulted.
    if ["entitlement_id", "effective_from", "effective_until"]
        .iter()
        .any(|key| !members.contains_key(*key))
    {
        return Err(Malformed);
    }
    let state = match wire.entitlement_state.as_str() {
        "ACTIVE" => EntitlementState::Active,
        "NOT_YET_EFFECTIVE" => EntitlementState::NotYetEffective,
        "EXPIRED" => EntitlementState::Expired,
        "REVOKED" => EntitlementState::Revoked,
        "NONE" => EntitlementState::None,
        _ => return Err(Malformed),
    };
    let time = |value: &str| rfc3339_utc_micros(value).ok_or(Malformed);
    // The closed `NONE` variant (§4): no entitlement, lifecycle revision 0, no interval. Every
    // other state names an entitlement, a lifecycle revision above 0 and its interval. `NONE`
    // stores a zero interval.
    let (effective_from_us, effective_until_us) = match (
        state,
        &wire.entitlement_id,
        &wire.effective_from,
        &wire.effective_until,
    ) {
        (EntitlementState::None, None, None, None) if wire.lifecycle_revision == 0 => (0, 0),
        (EntitlementState::None, ..) => return Err(Malformed),
        (_, Some(_), Some(from), Some(until)) if wire.lifecycle_revision > 0 => {
            (time(from)?, time(until)?)
        }
        _ => return Err(Malformed),
    };
    let evidence = PremiumEvidence {
        account_id,
        producer_revision: wire.producer_revision,
        producer_profile: wire.producer_profile,
        product_id: wire.product_id,
        product_version: wire.product_version,
        entitlement_id: wire.entitlement_id,
        state,
        lifecycle_revision: wire.lifecycle_revision,
        authority_revision: wire.authority_revision,
        effective_from_us,
        effective_until_us,
        authority_issued_at_us: time(&wire.authority_issued_at)?,
        authority_valid_until_us: time(&wire.authority_valid_until)?,
        refresh_after_us: time(&wire.refresh_after)?,
    };
    let lease = evidence.authority_valid_until_us - evidence.authority_issued_at_us;
    if ![
        wire.schema.as_str(),
        &evidence.producer_revision,
        &evidence.producer_profile,
        &evidence.product_id,
    ]
    .into_iter()
    .all(token)
        || !evidence.entitlement_id.as_deref().is_none_or(token)
        || evidence.effective_from_us > evidence.effective_until_us
        || lease <= 0
        || !(evidence.authority_issued_at_us..=evidence.authority_valid_until_us)
            .contains(&evidence.refresh_after_us)
    {
        return Err(Malformed);
    }
    // The envelope is complete and well-formed: only now is it compared with the compatibility
    // record and the bound product policy (§3.1 item 2).
    let unsupported = || {
        SnapshotFailure::Unsupported(Box::new(PremiumSemanticFailure {
            account_id,
            kind: PremiumConflictKind::Unsupported,
            authority_revision: evidence.authority_revision,
            schema: wire.schema.clone(),
            producer_profile: evidence.producer_profile.clone(),
            product_id: evidence.product_id.clone(),
            product_version: evidence.product_version,
        }))
    };
    if wire.schema != SNAPSHOT_SCHEMA
        || evidence.product_id != PRODUCT_ID
        || evidence.product_version != PRODUCT_VERSION
        || evidence.producer_profile != PRODUCER_PROFILE
    {
        return Err(unsupported());
    }
    if members.keys().any(|key| !MEMBERS.contains(&key.as_str())) {
        return Err(Malformed);
    }
    if lease > MAX_AUTHORITY_LEASE_US {
        return Err(unsupported());
    }
    let timestamps = [
        wire.effective_from.as_deref(),
        wire.effective_until.as_deref(),
        Some(wire.authority_issued_at.as_str()),
        Some(wire.authority_valid_until.as_str()),
        Some(wire.refresh_after.as_str()),
    ];
    if !timestamps
        .into_iter()
        .flatten()
        .all(|t| t.len() == WHOLE_SECOND_LEN)
        || !product_v1_rules(&evidence)
    {
        return Err(Malformed);
    }
    Ok(evidence)
}

/// `YYYY-MM-DDTHH:MM:SSZ`: the only timestamp form of the Platform wire (PREM-P §5.1).
const WHOLE_SECOND_LEN: usize = 20;
/// The largest revision on the wire, 2^53 - 1 (PREM-P §5.1).
pub const MAX_WIRE_REVISION: u64 = (1 << 53) - 1;

/// The rules Platform's contract for `oteryn.premium_time` v1 fixes on a supported envelope
/// (`OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md` §5, §7 `cross_field_rules`; PREMIUM-DELIVERY-0
/// §12): the identifier forms, the revision bounds, and the cutoff, refresh and state arithmetic.
/// They are product-version rules, so they run after the compatibility comparison; a body that
/// breaks one is malformed (a failed pull), never a durable semantic failure.
fn product_v1_rules(e: &PremiumEvidence) -> bool {
    const SECOND_US: i64 = 1_000_000;
    let issued = e.authority_issued_at_us;
    let valid = e.authority_valid_until_us;
    let cutoff = match e.state {
        EntitlementState::Active | EntitlementState::NotYetEffective => {
            (issued + MAX_AUTHORITY_LEASE_US).min(e.effective_until_us)
        }
        EntitlementState::Expired | EntitlementState::Revoked | EntitlementState::None => {
            issued + MAX_AUTHORITY_LEASE_US
        }
    };
    let lease_s = (valid - issued) / SECOND_US;
    let state_at_issue = match e.state {
        EntitlementState::None => true,
        EntitlementState::Revoked => e.effective_from_us < e.effective_until_us,
        EntitlementState::Active => e.effective_from_us <= issued && issued < e.effective_until_us,
        EntitlementState::NotYetEffective => {
            issued < e.effective_from_us && e.effective_from_us < e.effective_until_us
        }
        EntitlementState::Expired => {
            issued >= e.effective_until_us && e.effective_from_us < e.effective_until_us
        }
    };
    lower_hex(&e.producer_revision, 40)
        && e.entitlement_id.as_deref().is_none_or(uuid_v7)
        && (1..=MAX_WIRE_REVISION).contains(&e.authority_revision)
        && e.lifecycle_revision <= MAX_WIRE_REVISION
        && valid == cutoff
        && e.refresh_after_us == issued + 2 * lease_s / 3 * SECOND_US
        && e.refresh_after_us < valid
        && state_at_issue
}

fn lower_hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// A lowercase hyphenated RFC 9562 UUIDv7 (version 7, RFC variant).
pub fn uuid_v7(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 36
        && [8, 13, 18, 23].iter().all(|&i| b[i] == b'-')
        && value.split('-').map(str::len).eq([8, 4, 4, 4, 12])
        && lower_hex(&value.replace('-', ""), 32)
        && b[14] == b'7'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b')
}

/// An opaque producer token: the 0029 CHECK grammar.
fn token(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

/// Lowercase hyphenated UUID text of an AccountId.
pub fn canonical_uuid(id: [u8; 16]) -> String {
    let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// `YYYY-MM-DDTHH:MM:SS[.ffffff]Z` (at most six fraction digits, no leap second, year 1970 or
/// later) as Unix microseconds. Any other form, offset included, is refused.
pub fn rfc3339_utc_micros(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' {
        return None;
    }
    if b[16] != b':' || *b.last()? != b'Z' {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let digits = b.get(range)?;
        digits
            .iter()
            .all(u8::is_ascii_digit)
            .then(|| std::str::from_utf8(digits).ok()?.parse().ok())?
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let fraction = &b[19..b.len() - 1];
    let micros = match fraction {
        [] => 0,
        [b'.', digits @ ..] if (1..=6).contains(&digits.len()) => {
            let value = number(20..20 + digits.len())?;
            value * 10_i64.pow(6 - digits.len() as u32)
        }
        _ => return None,
    };
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if year < 1970
        || !(1..=12).contains(&month)
        || day < 1
        || day > month_days[usize::try_from(month - 1).ok()?]
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    // Days from civil (Howard Hinnant), valid for the proleptic Gregorian calendar.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y / 400;
    let yoe = y - era * 400;
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400 + hour * 3_600 + minute * 60 + second) * 1_000_000) + micros)
}
