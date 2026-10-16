//! UNINSTALLED Guild event-data proposal, not an admitted EventEnvelope or public codec.
//! CP must admit the normative guild event/retention and publication owner before activation.
use super::item_mint_audit::check_uuid_v7;
use serde::{Deserialize, Serialize};

/// One membership/invitation change, without Account links or CharacterRevision changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuildAuditProposal {
    pub version: u8,
    pub occurrence: [u8; 16],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub guild_id: [u8; 16],
    pub world_id: [u8; 16],
    pub actor: [u8; 16],
    pub target: [u8; 16],
    pub action: i16,
    pub previous_revision: u64,
    pub revision: u64,
    pub rank_before: Option<u8>,
    pub rank_after: Option<u8>,
    pub removed_invitations: u8,
    pub occurred_at_unix_ms: i64,
}

impl GuildAuditProposal {
    pub fn checked_bytes(&self) -> Option<Vec<u8>> {
        if self.version != 1
            || !(1..=6).contains(&self.action)
            || (self.action != 6 && self.previous_revision == 0)
            || (self.action == 6
                && (self.previous_revision != 0
                    || self.revision != 1
                    || self.removed_invitations != 0))
            || self.previous_revision.checked_add(1) != Some(self.revision)
            || self.occurred_at_unix_ms <= 0
            || self.removed_invitations > 50
            || (matches!(self.action, 3 | 4 | 6) && self.actor != self.target)
        {
            return None;
        }
        for id in [
            &self.occurrence,
            &self.transaction_id,
            &self.event_id,
            &self.guild_id,
            &self.world_id,
            &self.actor,
            &self.target,
        ] {
            check_uuid_v7(id).ok()?;
        }
        let ranks = match self.action {
            1 | 2 => self.rank_before.is_none() && self.rank_after.is_none(),
            3 => {
                self.rank_before.is_none() && self.rank_after.is_some_and(|r| (3..=20).contains(&r))
            }
            6 => self.rank_before.is_none() && self.rank_after == Some(1),
            4 | 5 => {
                self.rank_before.is_some_and(|r| (2..=20).contains(&r)) && self.rank_after.is_none()
            }
            _ => false,
        };
        if !ranks {
            return None;
        }
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= 4_096).then_some(bytes)
    }
}

/// Closed text-event DATA proposal. The old six-operation codec stays byte-identical;
/// this extension binds the expected revision and exact text, without new wire IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuildTextAuditProposal {
    #[serde(flatten)]
    pub operation: GuildAuditProposal,
    pub expected_revision: u64,
    pub text: String,
}
impl GuildTextAuditProposal {
    pub fn checked_bytes(&self) -> Option<Vec<u8>> {
        let op = &self.operation;
        if op.version != 1
            || !matches!(op.action, 7 | 8)
            || op.previous_revision == 0
            || self.expected_revision != op.previous_revision
            || op.previous_revision.checked_add(1) != Some(op.revision)
            || op.occurred_at_unix_ms <= 0
            || op.rank_before.is_some()
            || op.rank_after.is_some()
            || op.removed_invitations != 0
            || (op.action == 8 && op.actor != op.target)
        {
            return None;
        }
        for id in [
            &op.occurrence,
            &op.transaction_id,
            &op.event_id,
            &op.guild_id,
            &op.world_id,
            &op.actor,
            &op.target,
        ] {
            check_uuid_v7(id).ok()?;
        }
        if op.action == 7 {
            if self.text.chars().count() > 29 || self.text.contains('\0') {
                return None;
            }
        } else if crate::chat::ChatText::parse(&self.text).ok()?.as_str() != self.text {
            return None;
        }
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= 4_096).then_some(bytes)
    }
}
