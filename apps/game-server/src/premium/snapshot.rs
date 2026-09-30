//! `oteryn.premium_snapshot.v1` validation (PREMIUM-DELIVERY-0 §3 and §4).
//!
//! [`validate`] turns one response body into [`PremiumEvidence`] or refuses it. A body that is
//! oversized, malformed, bound to another request (nonce) or another account is
//! [`SnapshotRejection::Malformed`] and changes nothing (consumer contract §8.1 step 1). A
//! well-formed, bound body outside the compatibility record of this consumer (schema, product,
//! product version, producer profile, or a lease above the bound product policy) is
//! [`SnapshotRejection::Unsupported`]: it fails closed for benefit (§4, §8.1 step 4).

use super::{
    MAX_AUTHORITY_LEASE_US, PRODUCER_PROFILE, PRODUCT_ID, PRODUCT_VERSION, SNAPSHOT_SCHEMA,
};
use crate::durability::premium_fence::{EntitlementState, PremiumEvidence};
use serde::Deserialize;

/// `PREMDEL0-RL-01`.
pub const MAX_SNAPSHOT_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotRejection {
    Malformed,
    Unsupported,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    schema: String,
    producer_revision: String,
    producer_profile: String,
    nonce: String,
    account_id: String,
    product_id: String,
    product_version: u64,
    entitlement_id: Option<String>,
    entitlement_state: String,
    lifecycle_revision: u64,
    authority_revision: u64,
    effective_from: String,
    effective_until: String,
    authority_issued_at: String,
    authority_valid_until: String,
    refresh_after: String,
}

/// Validate `body`, the response to a request for `account_id` with `nonce`.
pub fn validate(
    body: &[u8],
    account_id: [u8; 16],
    nonce: &str,
) -> Result<PremiumEvidence, SnapshotRejection> {
    use SnapshotRejection::{Malformed, Unsupported};
    if body.len() > MAX_SNAPSHOT_BYTES {
        return Err(Malformed);
    }
    // Duplicate and unknown members fail the typed parse; the untyped one proves the nullable
    // `entitlement_id` is present rather than defaulted.
    let members: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(body).map_err(|_| Malformed)?;
    let wire: Wire = serde_json::from_slice(body).map_err(|_| Malformed)?;
    if !members.contains_key("entitlement_id")
        || nonce.is_empty()
        || wire.nonce != nonce
        || wire.account_id != canonical_uuid(account_id)
    {
        return Err(Malformed);
    }
    if wire.schema != SNAPSHOT_SCHEMA
        || wire.product_id != PRODUCT_ID
        || wire.product_version != u64::from(PRODUCT_VERSION)
        || wire.producer_profile != PRODUCER_PROFILE
    {
        return Err(Unsupported);
    }
    let state = match wire.entitlement_state.as_str() {
        "ACTIVE" => EntitlementState::Active,
        "NOT_YET_EFFECTIVE" => EntitlementState::NotYetEffective,
        "EXPIRED" => EntitlementState::Expired,
        "REVOKED" => EntitlementState::Revoked,
        "NONE" => EntitlementState::None,
        _ => return Err(Malformed),
    };
    let evidence = PremiumEvidence {
        account_id,
        producer_revision: wire.producer_revision,
        producer_profile: wire.producer_profile,
        product_id: wire.product_id,
        product_version: PRODUCT_VERSION,
        entitlement_id: wire.entitlement_id,
        state,
        lifecycle_revision: wire.lifecycle_revision,
        authority_revision: wire.authority_revision,
        effective_from_us: rfc3339_utc_micros(&wire.effective_from).ok_or(Malformed)?,
        effective_until_us: rfc3339_utc_micros(&wire.effective_until).ok_or(Malformed)?,
        authority_issued_at_us: rfc3339_utc_micros(&wire.authority_issued_at).ok_or(Malformed)?,
        authority_valid_until_us: rfc3339_utc_micros(&wire.authority_valid_until)
            .ok_or(Malformed)?,
        refresh_after_us: rfc3339_utc_micros(&wire.refresh_after).ok_or(Malformed)?,
    };
    let lease = evidence.authority_valid_until_us - evidence.authority_issued_at_us;
    if !token(&evidence.producer_revision)
        || !evidence.entitlement_id.as_deref().is_none_or(token)
        || (state == EntitlementState::None) != evidence.entitlement_id.is_none()
        || (state != EntitlementState::None
            && evidence.effective_from_us > evidence.effective_until_us)
        || lease <= 0
    {
        return Err(Malformed);
    }
    if lease > MAX_AUTHORITY_LEASE_US {
        return Err(Unsupported);
    }
    Ok(evidence)
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
