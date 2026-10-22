use super::*;
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
