use super::*;
impl DurabilityRoot {
    /// event commit together. A historical receipt is replayed first, never current authority.
    pub async fn commit_guild_membership(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildRequest,
    ) -> Result<GuildOutcome> {
        if matches!(
            request.operation,
            GuildCommand::Found
                | GuildCommand::SetTitle(_)
                | GuildCommand::SetMessage
                | GuildCommand::SetRank(_)
                | GuildCommand::Resign(_)
                | GuildCommand::EditRanks
                | GuildCommand::Disband
        ) {
            return Err(GuildError::InvalidInput);
        }
        let binding = binding(&fence, &request)?;
        self.commit_guild_operation(authority, node, fence, request, binding, None)
            .await
    }
}
