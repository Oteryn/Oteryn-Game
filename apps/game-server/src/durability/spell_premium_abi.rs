//! Pure bounded Premium evidence ABI. No transport, Gameplay, Content or actor dependency.
//! The sole production decoder producer is the authenticated mTLS PremiumSource adapter.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
pub(crate) const MAX_RESPONSE: usize = 1024;
pub(crate) const MAX_AUTHORITY_LEASE_SECONDS: i64 = 3600;
pub(crate) const MAX_CLOCK_UNCERTAINTY_SECONDS: i64 = 5;
#[derive(Debug)]
pub(crate) enum Error {
    Malformed,
    WrongBinding,
    UnavailableClock,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum State {
    Active,
    NotYetEffective,
    Expired,
    Revoked,
    None,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema: String,
    producer_revision: String,
    producer_profile: String,
    nonce: String,
    account_id: String,
    product_id: String,
    product_version: u32,
    #[serde(deserialize_with = "required_option")]
    entitlement_id: Option<String>,
    entitlement_state: State,
    lifecycle_revision: u64,
    authority_revision: u64,
    #[serde(deserialize_with = "required_option")]
    effective_from: Option<String>,
    #[serde(deserialize_with = "required_option")]
    effective_until: Option<String>,
    authority_issued_at: String,
    authority_valid_until: String,
    refresh_after: String,
}
fn required_option<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Classification {
    Current,
    Free,
    Revoked,
    Expired,
    NotYetEffective,
    Unavailable,
    Conflicting,
}
/// Data from the independently registered current NTP/clock owner, never from a client packet.
/// A missing or unsafe observation denies benefit. Consumers cannot use receipt time as lease origin.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrustedTime {
    lower: i64,
    upper: i64,
}
impl TrustedTime {
    pub(crate) fn from_current_clock_owner(lower: i64, upper: i64) -> Result<Self, Error> {
        if lower < 0 || upper < lower || upper - lower > 2 * MAX_CLOCK_UNCERTAINTY_SECONDS {
            return Err(Error::UnavailableClock);
        }
        Ok(Self { lower, upper })
    }
    pub(crate) fn lower(self) -> i64 {
        self.lower
    }
    pub(crate) fn upper(self) -> i64 {
        self.upper
    }
}
/// Only the real mTLS pull below constructs this producer value. It grants no session authority.
#[derive(Debug, Clone)]
pub(crate) struct AuthenticatedSnapshot {
    account: [u8; 16],
    source: String,
    snapshot: Snapshot,
    fingerprint: [u8; 32],
    supported: bool,
    issued: i64,
    until: i64,
    refresh: i64,
    from: Option<i64>,
    effective_until: Option<i64>,
}
impl AuthenticatedSnapshot {
    pub(crate) fn account(&self) -> [u8; 16] {
        self.account
    }
    pub(crate) fn source(&self) -> &str {
        &self.source
    }
    pub(crate) fn authority_revision(&self) -> u64 {
        self.snapshot.authority_revision
    }
    pub(crate) fn lifecycle_revision(&self) -> u64 {
        self.snapshot.lifecycle_revision
    }
    pub(crate) fn entitlement_id(&self) -> Option<&str> {
        self.snapshot.entitlement_id.as_deref()
    }
    pub(crate) fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub(crate) fn supported(&self) -> bool {
        self.supported
    }
    pub(crate) fn evidence_bytes(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(&self.snapshot).map_err(|_| Error::Malformed)
    }
    pub(crate) fn benefit_valid_until(&self) -> i64 {
        self.effective_until
            .map_or(self.until, |end| end.min(self.until))
    }
    pub(crate) fn valid_until(&self) -> i64 {
        self.until
    }
    pub(crate) fn refresh_after(&self) -> i64 {
        self.refresh
    }
    pub(crate) fn classification(
        &self,
        time: TrustedTime,
        conflict: bool,
        fresh_pull: bool,
    ) -> Classification {
        if conflict || !self.supported {
            return Classification::Conflicting;
        }
        if self.snapshot.entitlement_state == State::Revoked {
            return Classification::Revoked;
        }
        if self.snapshot.entitlement_state == State::Expired
            || time.upper >= self.until
            || self.effective_until.is_some_and(|end| time.upper >= end)
        {
            return Classification::Expired;
        }
        if self.snapshot.entitlement_state == State::NotYetEffective
            || self.from.is_some_and(|start| time.lower < start)
        {
            return Classification::NotYetEffective;
        }
        if !fresh_pull || time.lower < self.issued {
            return Classification::Unavailable;
        }
        if self.snapshot.entitlement_state == State::None {
            Classification::Free
        } else {
            Classification::Current
        }
    }
}
pub(crate) fn hash(s: &str, width: usize) -> bool {
    s.len() == width
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(crate) fn uuid(value: [u8; 16]) -> String {
    let text = hex(&value);
    format!(
        "{}-{}-{}-{}-{}",
        &text[..8],
        &text[8..12],
        &text[12..16],
        &text[16..20],
        &text[20..]
    )
}
fn canonical_uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}
pub(crate) fn decode_authenticated(
    bytes: &[u8],
    account: [u8; 16],
    nonce: &str,
    source: &str,
    builds: &BTreeSet<String>,
) -> Result<AuthenticatedSnapshot, Error> {
    if bytes.is_empty() || bytes.len() > MAX_RESPONSE {
        return Err(Error::Malformed);
    }
    let snapshot: Snapshot = serde_json::from_slice(bytes).map_err(|_| Error::Malformed)?;
    if snapshot.account_id != uuid(account)
        || snapshot.nonce != nonce
        || !hash(nonce, 32)
        || snapshot.authority_revision == 0
        || !hash(&snapshot.producer_revision, 40)
    {
        return Err(Error::WrongBinding);
    }
    let issued = utc_start(&snapshot.authority_issued_at)?;
    let until = utc(&snapshot.authority_valid_until)?;
    let refresh = utc(&snapshot.refresh_after)?;
    let from = snapshot
        .effective_from
        .as_deref()
        .map(utc_start)
        .transpose()?;
    let effective_until = snapshot.effective_until.as_deref().map(utc).transpose()?;
    if until <= issued
        || until - issued > MAX_AUTHORITY_LEASE_SECONDS
        || refresh < issued
        || refresh >= until
    {
        return Err(Error::Malformed);
    }
    if snapshot.entitlement_state == State::None {
        if snapshot.entitlement_id.is_some()
            || snapshot.lifecycle_revision != 0
            || from.is_some()
            || effective_until.is_some()
        {
            return Err(Error::Malformed);
        }
    } else if snapshot
        .entitlement_id
        .as_ref()
        .is_none_or(|id| !canonical_uuid(id))
        || snapshot.lifecycle_revision == 0
        || from.is_none()
        || effective_until.is_none()
        || from >= effective_until
    {
        return Err(Error::Malformed);
    }
    let supported = snapshot.schema == "oteryn.premium_snapshot.v1"
        && snapshot.producer_profile == "oteryn.entitlement.profile_b.v1"
        && snapshot.product_id == "oteryn.premium_time"
        && snapshot.product_version == 1
        && builds.contains(&snapshot.producer_revision);
    let mut semantic = serde_json::to_value(&snapshot).map_err(|_| Error::Malformed)?;
    let object = semantic.as_object_mut().ok_or(Error::Malformed)?;
    object.remove("nonce");
    object.remove("producer_revision");
    let fingerprint: [u8; 32] =
        Sha256::digest(serde_json::to_vec(&semantic).map_err(|_| Error::Malformed)?).into();
    Ok(AuthenticatedSnapshot {
        account,
        source: source.into(),
        snapshot,
        fingerprint,
        supported,
        issued,
        until,
        refresh,
        from,
        effective_until,
    })
}
// The transport contract requires UTC. Retain exact absolute seconds; never receipt-relative TTL.
fn utc(s: &str) -> Result<i64, Error> {
    if !s.is_ascii()
        || s.len() < 20
        || &s[4..5] != "-"
        || &s[7..8] != "-"
        || &s[10..11] != "T"
        || &s[13..14] != ":"
        || &s[16..17] != ":"
    {
        return Err(Error::Malformed);
    }
    let tail = &s[19..];
    let suffix = tail
        .strip_suffix('Z')
        .or_else(|| tail.strip_suffix("+00:00"))
        .ok_or(Error::Malformed)?;
    if !suffix.is_empty()
        && (!suffix.starts_with('.')
            || suffix.len() > 10
            || suffix.len() < 2
            || !suffix[1..].bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(Error::Malformed);
    }
    let number = |a, b| {
        let digits = &s[a..b];
        if !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::Malformed);
        }
        digits.parse::<i64>().map_err(|_| Error::Malformed)
    };
    let year = number(0, 4)?;
    let month = number(5, 7)?;
    let day = number(8, 10)?;
    let hour = number(11, 13)?;
    let minute = number(14, 16)?;
    let second = number(17, 19)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
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
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day < 1
        || day > days[(month - 1) as usize]
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(Error::Malformed);
    }
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = y / 400;
    let yoe = y - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok((era * 146097 + doe - 719468) * 86400 + hour * 3600 + minute * 60 + second)
}

fn utc_start(s: &str) -> Result<i64, Error> {
    let floor = utc(s)?;
    let fractional = s
        .get(19..)
        .and_then(|tail| tail.strip_prefix('.'))
        .is_some_and(|tail| {
            tail.bytes()
                .take_while(|b| b.is_ascii_digit())
                .any(|b| b != b'0')
        });
    if fractional {
        floor.checked_add(1).ok_or(Error::Malformed)
    } else {
        Ok(floor)
    }
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    fn evidence(account: [u8; 16], nonce: &str) -> serde_json::Value {
        serde_json::json!({"schema":"oteryn.premium_snapshot.v1","producer_revision":"1".repeat(40),
            "producer_profile":"oteryn.entitlement.profile_b.v1","nonce":nonce,"account_id":uuid(account),
            "product_id":"oteryn.premium_time","product_version":1,"entitlement_id":null,
            "entitlement_state":"NONE","lifecycle_revision":0,"authority_revision":1,
            "effective_from":null,"effective_until":null,"authority_issued_at":"2026-10-01T00:00:00Z",
            "authority_valid_until":"2026-10-01T01:00:00Z","refresh_after":"2026-10-01T00:30:00Z"})
    }
    #[test]
    fn strict_account_nonce_null_shape_and_response_bounds_fail_closed() {
        let nonce = "2".repeat(32);
        let account = [1; 16];
        let builds = BTreeSet::from(["1".repeat(40)]);
        let value = evidence(account, &nonce);
        let raw = serde_json::to_vec(&value).unwrap();
        let good =
            decode_authenticated(&raw, account, &nonce, "configured-platform", &builds).unwrap();
        assert_eq!(
            good.classification(
                TrustedTime::from_current_clock_owner(
                    utc("2026-10-01T00:00:00Z").unwrap(),
                    utc("2026-10-01T00:00:01Z").unwrap()
                )
                .unwrap(),
                false,
                true
            ),
            Classification::Free
        );
        assert!(
            decode_authenticated(&raw, [3; 16], &nonce, "configured-platform", &builds).is_err()
        );
        assert!(
            decode_authenticated(
                &raw,
                account,
                &"3".repeat(32),
                "configured-platform",
                &builds
            )
            .is_err()
        );
        assert!(
            decode_authenticated(
                &vec![b' '; 1025],
                account,
                &nonce,
                "configured-platform",
                &builds
            )
            .is_err()
        );
        for field in ["entitlement_id", "effective_from", "effective_until"] {
            let mut changed = value.clone();
            changed.as_object_mut().unwrap().remove(field);
            assert!(
                decode_authenticated(
                    &serde_json::to_vec(&changed).unwrap(),
                    account,
                    &nonce,
                    "configured-platform",
                    &builds
                )
                .is_err()
            );
        }
    }
    #[test]
    fn timestamp_parser_never_panics_and_uses_conservative_fraction_bounds() {
        for bad in [
            "éééééééééé",
            "2026-10-01T-1:00:00Z",
            "2026-02-29T00:00:00Z",
            "2026-10-01T00:00:60Z",
            "2026-10-01T00:00:00+01:00",
            "2026-10-01T00:00:00.Z",
        ] {
            assert!(utc(bad).is_err(), "{bad}");
        }
        let seconds = utc("2026-10-01T00:00:00Z").unwrap();
        assert_eq!(utc("2026-10-01T00:00:00.999+00:00").unwrap(), seconds);
        assert_eq!(utc_start("2026-10-01T00:00:00.999Z").unwrap(), seconds + 1);
        assert_eq!(utc_start("2026-10-01T00:00:00.000Z").unwrap(), seconds);
    }
    #[test]
    fn semantic_fingerprint_ignores_nonce_build_and_preserves_every_commercial_field() {
        let account = [1; 16];
        let nonce = "2".repeat(32);
        let builds = BTreeSet::from(["1".repeat(40), "4".repeat(40)]);
        let mut value = evidence(account, &nonce);
        let original = decode_authenticated(
            &serde_json::to_vec(&value).unwrap(),
            account,
            &nonce,
            "source",
            &builds,
        )
        .unwrap();
        value["nonce"] = "3".repeat(32).into();
        value["producer_revision"] = "4".repeat(40).into();
        let refreshed = decode_authenticated(
            &serde_json::to_vec(&value).unwrap(),
            account,
            &"3".repeat(32),
            "source",
            &builds,
        )
        .unwrap();
        assert_eq!(original.fingerprint(), refreshed.fingerprint());
        value["refresh_after"] = "2026-10-01T00:31:00Z".into();
        let changed = decode_authenticated(
            &serde_json::to_vec(&value).unwrap(),
            account,
            &"3".repeat(32),
            "source",
            &builds,
        )
        .unwrap();
        assert_ne!(original.fingerprint(), changed.fingerprint());
        assert_eq!(
            changed.classification(
                TrustedTime::from_current_clock_owner(1, 1).unwrap(),
                true,
                true
            ),
            Classification::Conflicting
        );
        assert_eq!(
            changed.classification(
                TrustedTime::from_current_clock_owner(1, 1).unwrap(),
                false,
                false
            ),
            Classification::Unavailable
        );
    }
}
