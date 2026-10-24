use super::*;
impl DurabilityRoot {
    /// existing commands. Text/revision carry no role, session or membership authority.
    pub async fn commit_guild_text(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        text: GuildTextRequest,
    ) -> Result<GuildOutcome> {
        let mut digest = Sha256::new();
        digest.update(b"oteryn:guild-text-proposal:v1");
        digest.update(binding(&fence, &text.common)?);
        digest.update(text.expected_revision.to_be_bytes());
        digest.update(text.text.as_bytes());
        let binding = digest.finalize().to_vec();
        self.commit_guild_operation(
            authority,
            node,
            fence,
            text.common.clone(),
            binding,
            Some(text),
        )
        .await
    }
}
