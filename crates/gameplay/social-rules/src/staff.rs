//! GM-1 closed operator vocabulary. Signed strings are ASCII; control and duration
//! values are canonical decimal strings. NX-1 coordinates are bounded safe JSON
//! integers, so the closed subset has unambiguous JCS bytes.
use serde::{Deserialize, Serialize};

/// Donor groups.xml ids 4/5/6: gamemaster/community manager/god. Legacy wire
/// names remain canonical; the owner accepted this model for NX-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StaffRole {
    Gm,
    #[serde(alias = "CM")]
    SeniorGm,
    #[serde(alias = "GOD")]
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
    Kick {
        character_id: String,
    },
    Ban {
        character_id: String,
        duration_ms: String,
        reason: String,
    },
    Unban {
        character_id: String,
    },
    Teleport {
        character_id: String,
        x: i32,
        y: i32,
        floor: i16,
    },
    TeleportToPlayer {
        character_id: String,
        destination_character_id: String,
    },
    Bring {
        character_id: String,
    },
    Notify {
        character_id: String,
        text: String,
    },
    Broadcast {
        character_id: String,
        text: String,
    },
    Ghost {
        character_id: String,
        enabled: bool,
    },
    Invisible {
        character_id: String,
        enabled: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub action: Action,
    /// Required by capability330: the signed GM character, not the transport carrier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer_character_id: Option<String>,
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
/// NX-1 bounded native authorization, versus donor /ban's arbitrary positive days.
pub const MAX_BAN_MS: u64 = 365 * 86_400_000;
pub const MAX_STAFF_TEXT_BYTES: usize = 1024;
/// Existing server-message domain92 ceiling; notices do not truncate signed text.
pub const MAX_STAFF_NOTICE_BYTES: usize = 255;

impl Action {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Inspect { .. } => "INSPECT",
            Self::Mute { .. } => "MUTE",
            Self::Unmute { .. } => "UNMUTE",
            Self::Kick { .. } => "KICK",
            Self::Ban { .. } => "BAN",
            Self::Unban { .. } => "UNBAN",
            Self::Teleport { .. } => "TELEPORT",
            Self::TeleportToPlayer { .. } => "TELEPORT_TO_PLAYER",
            Self::Bring { .. } => "BRING",
            Self::Notify { .. } => "NOTIFY",
            Self::Broadcast { .. } => "BROADCAST",
            Self::Ghost { .. } => "GHOST",
            Self::Invisible { .. } => "INVISIBLE",
        }
    }
    pub fn character_id(&self) -> &str {
        match self {
            Self::Inspect { character_id }
            | Self::Mute { character_id, .. }
            | Self::Unmute { character_id, .. }
            | Self::Kick { character_id }
            | Self::Ban { character_id, .. }
            | Self::Unban { character_id }
            | Self::Teleport { character_id, .. }
            | Self::TeleportToPlayer { character_id, .. }
            | Self::Bring { character_id }
            | Self::Notify { character_id, .. }
            | Self::Broadcast { character_id, .. }
            | Self::Ghost { character_id, .. }
            | Self::Invisible { character_id, .. } => character_id,
        }
    }
    pub fn duration_ms(&self) -> u64 {
        match self {
            Self::Mute { duration_ms, .. } | Self::Ban { duration_ms, .. } => {
                decimal(duration_ms).unwrap_or(0)
            }
            _ => 0,
        }
    }
    /// Crystal main@201189f8 data/scripts/talkactions/god/mute.lua:32,61;
    /// other NX-1 actions follow gm/{ban,unban,position,push_creature,broadcast,ghost}.lua.
    /// A private Notify is the single-recipient subset of gm/broadcast.lua:3-10.
    pub const fn minimum_role(&self) -> StaffRole {
        match self {
            Self::Mute { .. } | Self::Unmute { .. } => StaffRole::Admin,
            _ => StaffRole::Gm,
        }
    }
    pub const fn is_extended(&self) -> bool {
        !matches!(
            self,
            Self::Inspect { .. } | Self::Mute { .. } | Self::Unmute { .. } | Self::Kick { .. }
        )
    }
}

fn target_id(value: &str) -> bool {
    hex(value, 16) && value.bytes().any(|b| b != b'0')
}
fn staff_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_STAFF_TEXT_BYTES
        && value.bytes().all(|b| b == b' ' || b.is_ascii_graphic())
}

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
    pub fn valid(&self, role: StaffRole) -> bool {
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
            Action::Kick { character_id } => {
                if !matches!(role, StaffRole::Gm | StaffRole::SeniorGm | StaffRole::Admin) {
                    return false;
                }
                character_id
            }
            Action::Ban {
                character_id,
                duration_ms,
                reason,
            } => {
                if !decimal(duration_ms).is_some_and(|n| (1..=MAX_BAN_MS).contains(&n))
                    || !staff_text(reason)
                {
                    return false;
                }
                character_id
            }
            Action::Teleport {
                character_id,
                x,
                y,
                floor,
            } => {
                if !(0..=65535).contains(x) || !(0..=65535).contains(y) || !(0..=15).contains(floor)
                {
                    return false;
                }
                character_id
            }
            Action::TeleportToPlayer {
                character_id,
                destination_character_id,
            } => {
                if !target_id(destination_character_id) {
                    return false;
                }
                character_id
            }
            Action::Notify { character_id, text } | Action::Broadcast { character_id, text } => {
                if !staff_text(text) || text.len() > MAX_STAFF_NOTICE_BYTES {
                    return false;
                }
                character_id
            }
            Action::Unban { character_id }
            | Action::Bring { character_id }
            | Action::Ghost { character_id, .. }
            | Action::Invisible { character_id, .. } => character_id,
        };
        target_id(target)
            && self.issuer_character_id.as_deref().is_none_or(target_id)
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
            && decimal(&self.issued_at).is_some_and(|n| {
                n > 0 && n <= i64::MAX as u64 - self.action.duration_ms().max(MAX_MUTE_MS)
            })
    }
    /// Active authorization for every new signed request uses the donor tiers.
    /// valid() remains the shape/history check so retained pre-NX-1 GM mute
    /// audit records can still be verified without granting new GM mute rights.
    pub fn valid_tools(&self, role: StaffRole) -> bool {
        self.valid(role) && role >= self.action.minimum_role()
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
