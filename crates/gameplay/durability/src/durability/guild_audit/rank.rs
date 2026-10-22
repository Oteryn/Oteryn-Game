use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuildRankAuditProposal {
    #[serde(flatten)]
    pub operation: GuildAuditProposal,
    pub expected_revision: u64,
}
impl GuildRankAuditProposal {
    pub fn checked_bytes(&self) -> Option<Vec<u8>> {
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
