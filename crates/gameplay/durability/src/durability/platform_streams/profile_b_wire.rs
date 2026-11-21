//! Frozen Profile-B v1 wire (Platform c5780e2c, accepted revision 2).
use super::enforcement_wire::uuid7;
use oteryn_gameplay_social_rules::premium_snapshot::{lower_hex, rfc3339_utc_micros};
use serde::{Deserialize, Serialize};
pub const INT53_MAX: u64 = (1 << 53) - 1;
pub const MAX_LEASE_US: i64 = 3_600_000_000;
pub const MAX_UNCERTAINTY_US: i64 = 5_000_000;
pub const AUTHORITY_BYTES: usize = 8192;
pub const PAGE_BYTES: usize = 262144;
pub const PROFILE: &str = "oteryn.entitlement.profile_b.v1";
pub const LEASE_POLICY: &str = "oteryn.profile_b.lease.v1";
pub fn time(s: &str) -> Option<i64> {
    if s.len() != 20 || !s.ends_with('Z') {
        return None;
    }
    rfc3339_utc_micros(s)
}
pub fn product(s: &str) -> bool {
    let parts: Vec<_> = s.split('.').collect();
    s.len() <= 64
        && (2..=4).contains(&parts.len())
        && parts.iter().all(|p| {
            (1..=32).contains(&p.len())
                && p.as_bytes()[0].is_ascii_lowercase()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        })
}
pub fn cursor(s: &str) -> bool {
    s.strip_prefix("v1:").is_some_and(|n| {
        !n.starts_with('0')
            && n.bytes().all(|b| b.is_ascii_digit())
            && n.parse::<i64>().is_ok_and(|n| n > 0)
    })
}
fn revision(n: u64) -> bool {
    (1..=INT53_MAX).contains(&n)
}
fn required<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub account_id: String,
    pub nonce: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommercialState {
    Active,
    NotYetEffective,
    Expired,
    Revoked,
    None,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub product_id: String,
    pub product_version: u64,
    #[serde(deserialize_with = "required")]
    pub entitlement_id: Option<String>,
    pub entitlement_state: CommercialState,
    pub lifecycle_revision: u64,
    #[serde(deserialize_with = "required")]
    pub effective_from: Option<String>,
    #[serde(deserialize_with = "required")]
    pub effective_until: Option<String>,
    pub authority_valid_until: String,
    pub refresh_after: String,
    pub lease_policy: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    pub schema: String,
    pub producer_revision: String,
    pub producer_profile: String,
    pub nonce: String,
    pub account_id: String,
    pub authority_revision: u64,
    pub authority_issued_at: String,
    pub entitlements: Vec<Item>,
}
impl Answer {
    pub fn valid(&self) -> bool {
        if self.schema != "oteryn.entitlement_authority.v1"
            || !lower_hex(&self.producer_revision, 40)
            || self.producer_profile != PROFILE
            || !lower_hex(&self.nonce, 32)
            || !uuid7(&self.account_id)
            || !revision(self.authority_revision)
            || self.entitlements.len() > 16
        {
            return false;
        }
        let Some(issued) = time(&self.authority_issued_at) else {
            return false;
        };
        let mut seen = std::collections::BTreeSet::new();
        self.entitlements
            .iter()
            .all(|e| seen.insert((&e.product_id, e.product_version)) && e.valid(issued))
    }
    pub fn bound_to(&self, request: &Request, products: &[(String, u64)]) -> bool {
        self.valid()
            && request.schema == "oteryn.entitlement_authority_request.v1"
            && self.account_id == request.account_id
            && self.nonce == request.nonce
            && self
                .entitlements
                .iter()
                .map(|e| (&e.product_id, e.product_version))
                .eq(products.iter().map(|(p, v)| (p, *v)))
    }
}
impl Item {
    fn valid(&self, issued: i64) -> bool {
        if !product(&self.product_id)
            || !revision(self.product_version)
            || self.product_id == "oteryn.premium_time" && self.product_version == 1
            || self.lease_policy != LEASE_POLICY
            || self.lifecycle_revision > INT53_MAX
        {
            return false;
        }
        let Some(cutoff) = time(&self.authority_valid_until) else {
            return false;
        };
        let Some(refresh) = time(&self.refresh_after) else {
            return false;
        };
        let Some(span) = cutoff.checked_sub(issued) else {
            return false;
        };
        if !(1..=MAX_LEASE_US).contains(&span)
            || span % 1_000_000 != 0
            || refresh != issued + (2 * (span / 1_000_000) / 3) * 1_000_000
        {
            return false;
        }
        if self.entitlement_state == CommercialState::None {
            return self.entitlement_id.is_none()
                && self.lifecycle_revision == 0
                && self.effective_from.is_none()
                && self.effective_until.is_none();
        }
        if !self.entitlement_id.as_deref().is_some_and(uuid7) || !revision(self.lifecycle_revision)
        {
            return false;
        }
        let Some(from) = self.effective_from.as_deref().and_then(time) else {
            return false;
        };
        let until = match &self.effective_until {
            Some(s) => match time(s) {
                Some(t) => Some(t),
                None => return false,
            },
            None => None,
        };
        if until.is_some_and(|t| t <= from) {
            return false;
        }
        match self.entitlement_state {
            CommercialState::Active | CommercialState::NotYetEffective => {
                !until.is_some_and(|t| t <= issued)
                    && !until.is_some_and(|t| cutoff > t)
                    && from <= issued.saturating_add(MAX_LEASE_US)
                    && (self.entitlement_state != CommercialState::NotYetEffective || from > issued)
            }
            CommercialState::Expired => until.is_some_and(|t| t <= issued),
            CommercialState::Revoked => true,
            CommercialState::None => false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangesRequest {
    pub schema: String,
    #[serde(deserialize_with = "required")]
    pub after: Option<String>,
    pub limit: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hint {
    pub account_id: String,
    pub product_id: String,
    pub product_version: u64,
    pub lifecycle_revision: u64,
}
impl Hint {
    pub fn valid(&self) -> bool {
        uuid7(&self.account_id)
            && product(&self.product_id)
            && revision(self.product_version)
            && revision(self.lifecycle_revision)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub schema: String,
    pub items: Vec<Hint>,
    #[serde(deserialize_with = "required")]
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Request(Request),
    Answer(Answer),
    ChangesRequest(ChangesRequest),
    Page(Page),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    Size,
    Malformed,
    NonCanonical,
}
impl Message {
    pub fn parse(raw: &[u8]) -> Option<Self> {
        Self::decode(raw).ok()
    }
    pub fn decode(raw: &[u8]) -> Result<Self, ParseError> {
        if raw.is_empty() || raw.len() > PAGE_BYTES {
            return Err(ParseError::Size);
        }
        let value: serde_json::Value =
            serde_json::from_slice(raw).map_err(|_| ParseError::Malformed)?;
        let schema = value
            .get("schema")
            .and_then(serde_json::Value::as_str)
            .ok_or(ParseError::Malformed)?;
        let (canonical, bound) = match schema {
            "oteryn.entitlement_authority_request.v1" | "oteryn.entitlement_change_request.v1" => {
                (false, 256)
            }
            "oteryn.entitlement_authority.v1" => (true, AUTHORITY_BYTES),
            "oteryn.entitlement_change_page.v1" => (true, PAGE_BYTES),
            _ => return Err(ParseError::Malformed),
        };
        if raw.len() > bound {
            return Err(ParseError::Size);
        }
        let message = match schema {
            "oteryn.entitlement_authority_request.v1" => {
                Self::Request(serde_json::from_slice(raw).map_err(|_| ParseError::Malformed)?)
            }
            "oteryn.entitlement_change_request.v1" => Self::ChangesRequest(
                serde_json::from_slice(raw).map_err(|_| ParseError::Malformed)?,
            ),
            "oteryn.entitlement_authority.v1" => {
                Self::Answer(serde_json::from_slice(raw).map_err(|_| ParseError::Malformed)?)
            }
            _ => Self::Page(serde_json::from_slice(raw).map_err(|_| ParseError::Malformed)?),
        };
        if !message.valid() {
            return Err(ParseError::Malformed);
        }
        if canonical
            && (raw.contains(&b'\\')
                || message.encode().ok_or(ParseError::Malformed)?.as_slice() != raw)
        {
            return Err(ParseError::NonCanonical);
        }
        Ok(message)
    }
    fn valid(&self) -> bool {
        match self {
            Self::Request(r) => {
                r.schema == "oteryn.entitlement_authority_request.v1"
                    && uuid7(&r.account_id)
                    && lower_hex(&r.nonce, 32)
            }
            Self::Answer(a) => a.valid(),
            Self::ChangesRequest(r) => {
                r.schema == "oteryn.entitlement_change_request.v1"
                    && (1..=500).contains(&r.limit)
                    && r.after.as_deref().is_none_or(cursor)
            }
            Self::Page(p) => {
                p.schema == "oteryn.entitlement_change_page.v1"
                    && p.items.len() <= 500
                    && p.items.iter().all(Hint::valid)
                    && p.items.is_empty() == p.next_cursor.is_none()
                    && p.next_cursor.as_deref().is_none_or(cursor)
            }
        }
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let (raw, bound) = match self {
            Self::Request(r) => (serde_json::to_vec(r).ok()?, 256),
            Self::Answer(a) => (serde_json::to_vec(a).ok()?, AUTHORITY_BYTES),
            Self::ChangesRequest(r) => (serde_json::to_vec(r).ok()?, 256),
            Self::Page(p) => (serde_json::to_vec(p).ok()?, PAGE_BYTES),
        };
        (raw.len() <= bound).then_some(raw)
    }
}
/// Trusted server window, never derived from an authority answer.
#[derive(Debug, Clone, Copy)]
pub struct GameNow {
    pub now_us: i64,
    pub uncertainty_us: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evaluation {
    Active,
    Stale,
    Expired,
    None,
}
/// One already validated item. A higher hint may only reduce its result to STALE.
pub fn evaluate(answer: &Item, game_now: Option<GameNow>, hinted_revision: u64) -> Evaluation {
    let Some(now) = game_now.filter(|n| (0..=MAX_UNCERTAINTY_US).contains(&n.uncertainty_us))
    else {
        return Evaluation::None;
    };
    let latest = now.now_us.saturating_add(now.uncertainty_us);
    let earliest = now.now_us.saturating_sub(now.uncertainty_us);
    if answer.entitlement_state == CommercialState::None
        || answer.entitlement_state == CommercialState::Revoked
    {
        return Evaluation::None;
    }
    if answer.entitlement_state == CommercialState::Expired
        || answer
            .effective_until
            .as_deref()
            .and_then(time)
            .is_some_and(|t| latest >= t)
        || time(&answer.authority_valid_until).is_none_or(|t| latest >= t)
    {
        return Evaluation::Expired;
    }
    if answer
        .effective_from
        .as_deref()
        .and_then(time)
        .is_none_or(|t| earliest < t)
    {
        return Evaluation::None;
    }
    if hinted_revision > answer.lifecycle_revision
        || time(&answer.refresh_after).is_none_or(|t| latest >= t)
    {
        return Evaluation::Stale;
    }
    Evaluation::Active
}
