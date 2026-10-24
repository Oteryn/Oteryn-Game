//! GM-1 closed operator vocabulary. Strings in signed objects are ASCII; all numbers
//! are canonical decimal strings, so the closed JSON subset has unambiguous JCS bytes.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StaffRole {
    Gm,
    SeniorGm,
    Admin,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Reason {
    Bot,
    Harassment,
    Name,
    Cheating,
    BugAbuse,
    Support,
    PlatformSecurity,
    Roster,
    Restore,
    Maintenance,
    LegalHold,
    PrivacyDsr,
    DetectorSignal,
    IntegrityEcode,
    SupportReport,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum Action {
    Inspect {
        character_id: String,
    },
    Mute {
        character_id: String,
        duration_ms: String,
    },
    Unmute {
        character_id: String,
        sanction_id: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub action: Action,
    pub activation_sha256: String,
    pub channel_id: String,
    pub control_seq: String,
    pub environment: String,
    pub issued_at: String,
    pub operation_id: String,
    pub previous_link_sha256: String,
    pub reason: Reason,
    pub roster_sha256: String,
    pub world_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub activation_sha256: String,
    pub control_seq: String,
    pub disposition: ReceiptDisposition,
    pub environment: String,
    pub operation_id: String,
    pub outcome_sha256: String,
    pub previous_link_sha256: String,
    pub request_sha256: String,
    pub roster_sha256: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReceiptDisposition {
    Committed,
    Refused,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    pub control_seq: String,
    pub environment: String,
    pub previous_link_sha256: String,
    pub roster_sha256: String,
}

pub const MAX_MUTE_MS: u64 = 86_400_000;

pub fn decimal(value: &str) -> Option<u64> {
    let number = value.parse::<u64>().ok()?;
    (number.to_string() == value).then_some(number)
}
pub fn hex(value: &str, bytes: usize) -> bool {
    value.len() == bytes * 2
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
pub fn atom(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.@".contains(&c))
}
impl Request {
    pub fn valid(&self, _role: StaffRole) -> bool {
        let target = match &self.action {
            Action::Inspect { character_id } => character_id,
            Action::Mute {
                character_id,
                duration_ms,
            } => {
                if !decimal(duration_ms).is_some_and(|n| (1..=MAX_MUTE_MS).contains(&n)) {
                    return false;
                }
                character_id
            }
            Action::Unmute {
                character_id,
                sanction_id,
            } => {
                if !hex(sanction_id, 16) {
                    return false;
                }
                character_id
            }
        };
        hex(target, 16)
            && target.bytes().any(|b| b != b'0')
            && hex(&self.operation_id, 16)
            && self.operation_id.bytes().any(|b| b != b'0')
            && hex(&self.world_id, 16)
            && hex(&self.channel_id, 16)
            && self.world_id != self.channel_id
            && hex(&self.activation_sha256, 32)
            && hex(&self.roster_sha256, 32)
            && hex(&self.previous_link_sha256, 32)
            && atom(&self.environment)
            && decimal(&self.control_seq).is_some_and(|n| n > 1 && n <= i64::MAX as u64)
            && decimal(&self.issued_at).is_some_and(|n| n > 0 && n <= i64::MAX as u64 - MAX_MUTE_MS)
    }
    pub fn fresh(&self, lower_ms: u64, upper_ms: u64) -> bool {
        decimal(&self.issued_at).is_some_and(|issued| {
            issued <= lower_ms.saturating_add(300_000) && upper_ms <= issued.saturating_add(600_000)
        })
    }
}

#[cfg(test)]
#[path = "staff_tests.rs"]
mod tests;
