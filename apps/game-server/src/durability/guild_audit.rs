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
        } else if crate::durability::chat_text::ChatText::parse(&self.text)
            .ok()?
            .as_str()
            != self.text
        {
            return None;
        }
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= 4_096).then_some(bytes)
    }
}

/// Closed SetRank event DATA. Existing eight operation encodings remain unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuildRankAuditProposal {
    #[serde(flatten)]
    pub operation: GuildAuditProposal,
    pub expected_revision: u64,
}
impl GuildRankAuditProposal {
    pub(crate) fn checked_bytes(&self) -> Option<Vec<u8>> {
        let op = &self.operation;
        if op.version != 1
            || op.action != 9
            || op.previous_revision == 0
            || self.expected_revision != op.previous_revision
            || op.previous_revision.checked_add(1) != Some(op.revision)
            || op.occurred_at_unix_ms <= 0
            || op.removed_invitations != 0
            || op.actor == op.target
            || !op.rank_before.is_some_and(|r| (2..=20).contains(&r))
            || !op.rank_after.is_some_and(|r| (2..=20).contains(&r))
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
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= 4_096).then_some(bytes)
    }
}

/// Closed private Resign event: fixed actor1→2 and target2→1; never an admitted wire codec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuildResignAuditProposal {
    #[serde(flatten)]
    pub operation: GuildAuditProposal,
    pub expected_revision: u64,
}
impl GuildResignAuditProposal {
    pub(crate) fn checked_bytes(&self) -> Option<Vec<u8>> {
        let op = &self.operation;
        if op.version != 1
            || op.action != 10
            || op.previous_revision == 0
            || self.expected_revision != op.previous_revision
            || op.previous_revision.checked_add(1) != Some(op.revision)
            || op.actor == op.target
            || op.rank_before != Some(2)
            || op.rank_after != Some(1)
            || op.removed_invitations != 0
            || op.occurred_at_unix_ms <= 0
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
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= 4096).then_some(bytes)
    }
}

/// Exact ordered EditRanks DATA; not an admitted wire codec or naming-policy allocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuildEditRanksAuditProposal {
    #[serde(flatten)]
    pub operation: GuildAuditProposal,
    pub expected_revision: u64,
    pub names: Vec<String>,
}
impl GuildEditRanksAuditProposal {
    pub(crate) fn checked_bytes(&self) -> Option<Vec<u8>> {
        let op = &self.operation;
        if op.version != 1
            || op.action != 11
            || op.previous_revision == 0
            || self.expected_revision != op.previous_revision
            || op.previous_revision.checked_add(1) != Some(op.revision)
            || op.actor != op.target
            || op.rank_before.is_some()
            || op.rank_after.is_some()
            || op.removed_invitations != 0
            || op.occurred_at_unix_ms <= 0
            || crate::domain::guild::validate_rank_count(self.names.len()).is_err()
            || self
                .names
                .iter()
                .any(|name| name.is_empty() || name.contains('\0'))
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
        // Existing bounded event envelope; not a new per-rank lexical or length policy.
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= 4096).then_some(bytes)
    }
}

/// Decline event DATA only. Original codecs/encodings remain literal and closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuildDeclineAuditProposal {
    #[serde(flatten)]
    pub operation: GuildAuditProposal,
}
impl GuildDeclineAuditProposal {
    pub(crate) fn checked_bytes(&self) -> Option<Vec<u8>> {
        let op = &self.operation;
        if op.version != 1
            || op.action != 12
            || op.actor != op.target
            || op.previous_revision == 0
            || op.previous_revision.checked_add(1) != Some(op.revision)
            || op.rank_before.is_some()
            || op.rank_after.is_some()
            || op.removed_invitations != 1
            || op.occurred_at_unix_ms <= 0
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
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= 4_096).then_some(bytes)
    }
}

/// Private durable disband-start publication DATA, keyed by (Guild, step=1).
/// It does not allocate a public event ID, claim external closure, or authorize a job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuildJobAuditProposal {
    pub version: u8,
    pub guild_id: [u8; 16],
    pub world_id: [u8; 16],
    pub step: u8,
    pub reason: i16,
    pub previous_revision: u64,
    pub revision: u64,
    pub occurred_at_unix_ms: i64,
}
impl GuildJobAuditProposal {
    pub(crate) fn checked_bytes(&self) -> Option<Vec<u8>> {
        if self.version != 1
            || self.step != 1
            || !matches!(self.reason, 1 | 2)
            || self.previous_revision == 0
            || self.previous_revision.checked_add(1) != Some(self.revision)
            || self.occurred_at_unix_ms <= 0
        {
            return None;
        }
        check_uuid_v7(&self.guild_id).ok()?;
        check_uuid_v7(&self.world_id).ok()?;
        serde_json::to_vec(self).ok()
    }
}

/// Private Disband confirmation event DATA, with exact input bytes and no public wire ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GuildDisbandAuditProposal {
    #[serde(flatten)]
    pub operation: GuildAuditProposal,
    pub confirmation: String,
}
impl GuildDisbandAuditProposal {
    pub(crate) fn checked_bytes(&self) -> Option<Vec<u8>> {
        let op = &self.operation;
        if op.version != 1
            || op.action != 13
            || op.actor != op.target
            || op.previous_revision == 0
            || op.previous_revision.checked_add(1) != Some(op.revision)
            || op.rank_before.is_some()
            || op.rank_after.is_some()
            || op.removed_invitations != 0
            || op.occurred_at_unix_ms <= 0
            || self.confirmation.chars().count() > 29
            || self.confirmation.contains('\0')
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
        serde_json::to_vec(self).ok()
    }
}
