use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuildJobAuditProposal {
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
    pub fn checked_bytes(&self) -> Option<Vec<u8>> {
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
