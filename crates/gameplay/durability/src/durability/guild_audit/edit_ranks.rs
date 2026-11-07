use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuildEditRanksAuditProposal {
    #[serde(flatten)]
    pub operation: GuildAuditProposal,
    pub expected_revision: u64,
    pub names: Vec<String>,
}
impl GuildEditRanksAuditProposal {
    pub fn checked_bytes(&self) -> Option<Vec<u8>> {
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
            || oteryn_gameplay_social_rules::guild::validate_rank_count(self.names.len()).is_err()
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
