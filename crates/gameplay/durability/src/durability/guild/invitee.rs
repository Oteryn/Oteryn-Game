//! Resolve an untrusted name to native Character data, never to an authorization fact.
use super::*;
impl DurabilityRoot {
    /// Reuse the original target on replay so a later rename/deletion cannot redirect an
    /// ambiguous command. The writer still checks the original wire digest and all authority.
    pub async fn resolve_guild_invitee(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        name: GuildName,
    ) -> Result<Option<CharacterId>> {
        if command.game_session_id() != fence.game_session_id {
            return Err(GuildError::InvalidInput);
        }
        let recovery = authority.record_for(self).map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        read_current_guild_chat_data(&mut tx, &recovery, &node, &fence, None).await?;
                        let original: Option<String> = sqlx::query_scalar(
                            "SELECT target::text FROM game_guild_operations WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric",
                        )
                        .bind(command.game_session_id().as_bytes().as_slice())
                        .bind(command.command_id().get().to_string())
                        .fetch_optional(&mut *tx)
                        .await?;
                        let target = match original {
                            Some(target) => Some(target),
                            None => sqlx::query_scalar(
                                "SELECT character_id::text FROM game_character_roots WHERE world_id=encode($1,'hex')::uuid AND lifecycle=1 AND name_key=$2 LIMIT 1",
                            )
                            .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                            .bind(name.comparison_key())
                            .fetch_optional(&mut *tx)
                            .await?,
                        };
                        target.map(|target| {
                            CharacterId::from_bytes(uuid_text(&target)?).map_err(|_| invalid())
                        }).transpose()
                    }.await;
                    match result {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(GuildError::Unavailable(error)) => {
                            tx.rollback().await?;
                            Err(error)
                        }
                        Err(error) => {
                            tx.rollback().await?;
                            Ok(Err(error))
                        }
                    }
                })
            })
            .await?
    }
}
