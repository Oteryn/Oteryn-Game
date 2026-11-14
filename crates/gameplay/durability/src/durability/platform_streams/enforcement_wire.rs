//! ACCEPTED revision 2, Platform c5780e2c: integer-based, closed enforcement wire.
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const COMMAND_BYTES: usize = 1024;
pub const PAGE_BYTES: usize = 262_144;
pub const CAPABILITY: &str = "enforcement.1";
macro_rules! enumeration {
    ($name:ident {$($variant:ident => $wire:literal),+ $(,)?}) => {
        #[derive(Debug,Clone,Copy,PartialEq,Eq,Serialize,Deserialize)]
        pub enum $name {$(#[serde(rename=$wire)] $variant),+}
        impl $name {pub const fn wire(self)->&'static str {match self {$(Self::$variant=>$wire),+}}}
    }
}
enumeration!(Intent {Apply=>"APPLY",Replace=>"REPLACE",Revoke=>"REVOKE",Expire=>"EXPIRE"});
enumeration!(Profile {Mute=>"MUTE",Jail=>"JAIL"});
enumeration!(State {AcceptedPending=>"ACCEPTED_PENDING",Applied=>"APPLIED",Replaced=>"REPLACED",Revoked=>"REVOKED",Expired=>"EXPIRED",Rejected=>"REJECTED"});
enumeration!(StreamState {None=>"NONE",Pending=>"PENDING",Active=>"ACTIVE",Revoked=>"REVOKED",Expired=>"EXPIRED"});
enumeration!(RuntimeEffect {OnlineApplied=>"ONLINE_APPLIED",OnNextAdmission=>"ON_NEXT_ADMISSION",None=>"NONE"});
enumeration!(Code {Ok=>"OK",PendingRuntime=>"PENDING_RUNTIME",PendingExpiry=>"PENDING_EXPIRY",Unsupported=>"UNSUPPORTED",TargetNotApplicable=>"TARGET_NOT_APPLICABLE",TargetMismatch=>"TARGET_MISMATCH",StaleRevision=>"STALE_REVISION",LifecycleConflict=>"LIFECYCLE_CONFLICT",OperationConflict=>"OPERATION_CONFLICT",CapabilityConflict=>"CAPABILITY_CONFLICT",InternalNonRetryable=>"INTERNAL_NON_RETRYABLE"});
fn required<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}
pub fn uuid7(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            14 => b == b'7',
            19 => b"89ab".contains(&b),
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        })
}
fn positive(n: i64) -> bool {
    n > 0
}
fn hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn digest(raw: &[u8]) -> String {
    Sha256::digest(raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn cursor(s: &str) -> bool {
    s.strip_prefix("v1:").is_some_and(|n| {
        !n.starts_with('0')
            && n.bytes().all(|b| b.is_ascii_digit())
            && n.parse::<i64>().is_ok_and(positive)
    })
}
fn parse<T: DeserializeOwned>(raw: &[u8], bound: usize) -> Option<T> {
    if raw.is_empty() || raw.len() > bound {
        return None;
    }
    let (mut depth, mut quoted, mut escaped) = (0u8, false, false);
    for &b in raw {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' && quoted {
            escaped = true;
            continue;
        }
        if b == b'"' {
            quoted = !quoted;
        } else if !quoted {
            match b {
                b'{' | b'[' => {
                    depth = depth.checked_add(1)?;
                    if depth > 4 {
                        return None;
                    }
                }
                b'}' | b']' => depth = depth.checked_sub(1)?,
                _ => {}
            }
        }
    }
    if depth != 0 || quoted {
        return None;
    }
    serde_json::from_slice(raw).ok()
}
fn encode<T: Serialize>(value: &T, bound: usize) -> Option<Vec<u8>> {
    let raw = serde_json::to_vec(value).ok()?;
    (raw.len() <= bound && !raw.contains(&b'\\')).then_some(raw)
}
/// Safe attribution for a terminal refusal of malformed bytes. Duplicate identity
/// members are rejected by serde; other command fields cannot establish authority.
#[derive(Deserialize)]
pub(super) struct CommandIdentity {
    pub operation_id: String,
    pub sanction_stream_id: String,
}
impl CommandIdentity {
    pub fn parse(raw: &[u8]) -> Option<Self> {
        let id: Self = parse(raw, COMMAND_BYTES)?;
        (uuid7(&id.operation_id) && uuid7(&id.sanction_stream_id)).then_some(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub operation_id: String,
    pub sanction_stream_id: String,
    pub command_version: i64,
    pub intent: Intent,
    pub profile: Profile,
    pub decision_revision: i64,
    #[serde(deserialize_with = "required")]
    pub prior_decision_revision: Option<i64>,
    pub policy_revision: i64,
    pub account_id: String,
    pub character_id: String,
    #[serde(deserialize_with = "required")]
    pub effective_at_ms: Option<i64>,
    #[serde(deserialize_with = "required")]
    pub expires_at_ms: Option<i64>,
    #[serde(deserialize_with = "required")]
    pub reason_code: Option<u16>,
    pub decided_at_ms: i64,
    pub audit_ref: String,
    pub correlation_id: String,
}
impl Command {
    fn valid(&self) -> bool {
        [
            &self.operation_id,
            &self.sanction_stream_id,
            &self.account_id,
            &self.character_id,
            &self.audit_ref,
            &self.correlation_id,
        ]
        .into_iter()
        .all(|s| uuid7(s))
            && self.command_version > 0
            && positive(self.decision_revision)
            && positive(self.policy_revision)
            && positive(self.decided_at_ms)
            && match self.intent {
                Intent::Apply | Intent::Replace => {
                    self.effective_at_ms.is_some_and(positive)
                        && self
                            .expires_at_ms
                            .zip(self.effective_at_ms)
                            .is_some_and(|(end, start)| end > start)
                        && self.reason_code.is_some_and(|n| n > 0)
                        && if self.intent == Intent::Apply {
                            self.prior_decision_revision.is_none()
                        } else {
                            self.prior_decision_revision
                                .is_some_and(|r| r > 0 && self.decision_revision > r)
                        }
                }
                Intent::Revoke => {
                    self.effective_at_ms.is_none()
                        && self.expires_at_ms.is_none()
                        && self.reason_code.is_some_and(|n| n > 0)
                        && self
                            .prior_decision_revision
                            .is_some_and(|r| r > 0 && self.decision_revision > r)
                }
                Intent::Expire => {
                    self.effective_at_ms.is_none()
                        && self.expires_at_ms.is_none()
                        && self.reason_code.is_none()
                        && self.prior_decision_revision.is_none()
                }
            }
    }
    pub fn parse(raw: &[u8]) -> Option<Self> {
        let c: Self = parse(raw, COMMAND_BYTES)?;
        (c.command_version == 1 && c.valid() && c.encode()?.as_slice() == raw).then_some(c)
    }
    pub(super) fn parse_for_decision(raw: &[u8]) -> Option<Self> {
        let c: Self = parse(raw, COMMAND_BYTES)?;
        (c.valid() && encode(&c, COMMAND_BYTES)?.as_slice() == raw).then_some(c)
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        (self.command_version == 1 && self.valid())
            .then(|| encode(self, COMMAND_BYTES))
            .flatten()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnforcementResult {
    pub schema: String,
    pub operation_id: String,
    pub sanction_stream_id: String,
    pub command_digest: String,
    pub receipt_revision: u16,
    pub state: State,
    pub result_code: Code,
    pub accepted_decision_revision: i64,
    pub stream_state: StreamState,
    #[serde(deserialize_with = "required")]
    pub effective_until_ms: Option<i64>,
    pub runtime_effect: RuntimeEffect,
    pub capability_revision: String,
    pub recovery_generation: i64,
    pub decided_at_ms: i64,
}
impl EnforcementResult {
    fn valid(&self) -> bool {
        self.schema == "oteryn.enforcement_result.v1"
            && uuid7(&self.operation_id)
            && uuid7(&self.sanction_stream_id)
            && hex(&self.command_digest)
            && self.receipt_revision > 0
            && self.accepted_decision_revision >= 0
            && self.recovery_generation > 0
            && self.decided_at_ms > 0
            && (self.accepted_decision_revision == 0) == (self.stream_state == StreamState::None)
            && if matches!(
                self.stream_state,
                StreamState::Pending | StreamState::Active
            ) {
                self.effective_until_ms.is_some_and(positive)
            } else {
                self.effective_until_ms.is_none()
            }
            && !self.capability_revision.is_empty()
            && self.capability_revision.len() <= 64
            && self
                .capability_revision
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._:-".contains(&b))
            && match self.state {
                State::AcceptedPending => {
                    matches!(self.result_code, Code::PendingExpiry | Code::PendingRuntime)
                }
                State::Rejected => {
                    !matches!(
                        self.result_code,
                        Code::Ok | Code::PendingExpiry | Code::PendingRuntime
                    ) && self.runtime_effect == RuntimeEffect::None
                }
                _ => self.result_code == Code::Ok,
            }
    }
    pub fn parse(raw: &[u8]) -> Option<Self> {
        let r: Self = parse(raw, 1024)?;
        (r.valid() && r.encode()?.as_slice() == raw).then_some(r)
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        self.valid().then(|| encode(self, 1024)).flatten()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamRequest {
    pub schema: String,
    #[serde(deserialize_with = "required")]
    pub after: Option<String>,
    pub limit: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadRequest {
    pub schema: String,
    pub operation_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandPage {
    pub schema: String,
    pub items: Vec<Command>,
    #[serde(deserialize_with = "required")]
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconcileItem {
    pub operation_id: String,
    pub sanction_stream_id: String,
    pub state: String,
    #[serde(deserialize_with = "required")]
    pub receipt_revision: Option<u16>,
    #[serde(deserialize_with = "required")]
    pub result_digest: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconcilePage {
    pub schema: String,
    pub items: Vec<ReconcileItem>,
    #[serde(deserialize_with = "required")]
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ack {
    pub schema: String,
    pub result: String,
}
#[derive(Debug, Clone)]
pub enum Message {
    Command(Command),
    Result(EnforcementResult),
    Request(StreamRequest),
    Read(ReadRequest),
    Page(CommandPage),
    Reconcile(ReconcilePage),
    Ack(Ack),
}
impl ReconcilePage {
    pub fn parse(raw: &[u8]) -> Option<Self> {
        match Message::parse(raw)? {
            Message::Reconcile(p) => Some(p),
            _ => None,
        }
    }
}
impl Message {
    pub fn parse(raw: &[u8]) -> Option<Self> {
        let value: serde_json::Value = parse(raw, PAGE_BYTES)?;
        let message = match value.get("schema").and_then(|v| v.as_str()) {
            None => Self::Command(Command::parse(raw)?),
            Some("oteryn.enforcement_result.v1") => Self::Result(EnforcementResult::parse(raw)?),
            Some("oteryn.enforcement_command_read_request.v1") => {
                let r: ReadRequest = parse(raw, 256)?;
                if !uuid7(&r.operation_id) {
                    return None;
                }
                Self::Read(r)
            }
            Some(
                "oteryn.enforcement_command_stream_request.v1"
                | "oteryn.enforcement_reconcile_request.v1",
            ) => {
                let r: StreamRequest = parse(raw, 256)?;
                if !(1..=500).contains(&r.limit) || r.after.as_deref().is_some_and(|s| !cursor(s)) {
                    return None;
                }
                Self::Request(r)
            }
            Some("oteryn.enforcement_command_stream_page.v1") => {
                let p: CommandPage = parse(raw, PAGE_BYTES)?;
                let mut ids = BTreeSet::new();
                let mut revisions = std::collections::BTreeMap::new();
                if p.items.len() > 500
                    || p.items.is_empty() != p.next_cursor.is_none()
                    || p.next_cursor.as_deref().is_some_and(|s| !cursor(s))
                {
                    return None;
                }
                for c in &p.items {
                    if c.command_version != 1
                        || !c.valid()
                        || !ids.insert(&c.operation_id)
                        || revisions
                            .insert(&c.sanction_stream_id, c.decision_revision)
                            .is_some_and(|r| r > c.decision_revision)
                    {
                        return None;
                    }
                }
                Self::Page(p)
            }
            Some("oteryn.enforcement_reconcile_page.v1") => {
                let p: ReconcilePage = parse(raw, PAGE_BYTES)?;
                let mut ids = BTreeSet::new();
                if p.items.len() > 500
                    || p.items.is_empty() != p.next_cursor.is_none()
                    || p.next_cursor.as_deref().is_some_and(|s| !cursor(s))
                {
                    return None;
                }
                for i in &p.items {
                    if !uuid7(&i.operation_id)
                        || !uuid7(&i.sanction_stream_id)
                        || !ids.insert(&i.operation_id)
                    {
                        return None;
                    }
                    if i.state == "PENDING" {
                        if i.receipt_revision.is_some() || i.result_digest.is_some() {
                            return None;
                        }
                    } else if serde_json::from_value::<State>(serde_json::Value::String(
                        i.state.clone(),
                    ))
                    .is_err()
                        || i.receipt_revision.is_none_or(|r| r == 0)
                        || i.result_digest.as_deref().is_none_or(|s| !hex(s))
                    {
                        return None;
                    }
                }
                Self::Reconcile(p)
            }
            Some("oteryn.enforcement_result_ack.v1") => {
                let a: Ack = parse(raw, 128)?;
                if a.result != "accepted" {
                    return None;
                }
                Self::Ack(a)
            }
            _ => return None,
        };
        if !matches!(message, Self::Request(_) | Self::Read(_))
            && message.serialize_raw()?.as_slice() != raw
        {
            return None;
        }
        Some(message)
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        let raw = self.serialize_raw()?;
        Self::parse(&raw)?;
        Some(raw)
    }
    fn serialize_raw(&self) -> Option<Vec<u8>> {
        match self {
            Self::Command(c) => c.encode(),
            Self::Result(r) => r.encode(),
            Self::Request(r) => {
                if !matches!(
                    r.schema.as_str(),
                    "oteryn.enforcement_command_stream_request.v1"
                        | "oteryn.enforcement_reconcile_request.v1"
                ) || !(1..=500).contains(&r.limit)
                    || r.after.as_deref().is_some_and(|s| !cursor(s))
                {
                    return None;
                }
                encode(r, 256)
            }
            Self::Read(r) => {
                if r.schema != "oteryn.enforcement_command_read_request.v1"
                    || !uuid7(&r.operation_id)
                {
                    return None;
                }
                encode(r, 256)
            }
            Self::Page(p) => encode(p, PAGE_BYTES),
            Self::Reconcile(p) => encode(p, PAGE_BYTES),
            Self::Ack(a) => {
                if a.schema != "oteryn.enforcement_result_ack.v1" || a.result != "accepted" {
                    return None;
                }
                encode(a, 128)
            }
        }
    }
}
