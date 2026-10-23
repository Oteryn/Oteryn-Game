use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuildDeclineAuditProposal {
    #[serde(flatten)]
    pub operation: GuildAuditProposal,
}
impl GuildDeclineAuditProposal {
    pub fn checked_bytes(&self) -> Option<Vec<u8>> {
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
