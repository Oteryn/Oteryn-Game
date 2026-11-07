use super::*;
impl DurabilityRoot {
    /// No Premium read is needed: N>=3 preserves both leadership levels and all positions.
    pub async fn commit_guild_edit_ranks(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        edit_ranks: GuildEditRanksRequest,
    ) -> Result<GuildOutcome> {
        let binding = edit_ranks_binding(&fence, &edit_ranks)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let write = EditRanksWrite {
            node: node.clone(),
            fence,
            edit_ranks,
            binding,
        };
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = write.execute(&mut tx, &recovery).await;
                    match result {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    pub(super) async fn commit_guild_operation(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildRequest,
        binding: Vec<u8>,
        text: Option<GuildTextRequest>,
    ) -> Result<GuildOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = execute_fenced_operation(
                        &mut tx,
                        &recovery,
                        &node,
                        &fence,
                        &request,
                        &binding,
                        text.as_ref(),
                    )
                    .await;
                    match result {
                        Ok(result) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(result))
                        }
                        Err(GuildError::Unavailable(e)) => Err(e),
                        Err(error) => Ok(Err(error)), // Dropping the transaction rolls back every effect.
                    }
                })
            })
            .await?
    }
}
include!("edit_ranks_data.rs");
impl EditRanksWrite {
    pub async fn execute(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    ) -> Result<GuildOutcome> {
        lock_fenced_guild_operation(tx, recovery, &self.edit_ranks.common).await?;
        let edit = MemberEdit::EditRanks(&self.edit_ranks);
        execute(
            tx,
            &self.node,
            &self.fence,
            &self.edit_ranks.common,
            &self.binding,
            None,
            Some(&edit),
        )
        .await
    }
}
