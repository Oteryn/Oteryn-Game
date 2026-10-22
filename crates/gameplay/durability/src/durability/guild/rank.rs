use super::*;
impl DurabilityRoot {
    /// Only a new move into level2 consumes the actual TARGET Account's Premium decision.
    pub async fn commit_guild_rank(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        rank: GuildRankRequest,
        premium: &impl PremiumGateOwner,
    ) -> Result<GuildOutcome> {
        let binding = rank_binding(&fence, &rank)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let account = match self
            .preflight_guild_target(
                recovery.clone(),
                node.clone(),
                fence,
                rank.common.clone(),
                binding.clone(),
            )
            .await?
        {
            GuildTargetPreflight::Replayed(value) => return Ok(value),
            GuildTargetPreflight::Fresh(_, account) => account,
        };
        let write = RankWrite {
            node: node.clone(),
            fence,
            rank,
            binding,
            account,
        };
        let Some(account) = account else {
            // Unknown/inactive/other-World target cannot acquire a Premium decision.
            // The real writer rechecks all current facts and can only refuse without one.
            return self.run_guild_rank_write(recovery, write, None).await;
        };
        let root = self.clone();
        premium
            .with_premium_gate(account, move |status, tx| {
                Box::pin(async move {
                    match tx {
                        Some(tx) => {
                            if !matches!(&status, PremiumStatus::NotActivated(_)) {
                                return Err(invalid());
                            }
                            // The native gate commits its physical transaction even on callback error.
                            let mut savepoint = tx.begin().await?;
                            let result = write
                                .execute(&mut savepoint, &recovery, Some(&status))
                                .await;
                            match result {
                                Ok(outcome) => {
                                    savepoint.commit().await?;
                                    Ok(outcome)
                                }
                                Err(error) => {
                                    savepoint.rollback().await?;
                                    Err(error)
                                }
                            }
                        }
                        None => {
                            let status: PremiumStatus<'static> = match status {
                                PremiumStatus::Current => PremiumStatus::Current,
                                PremiumStatus::NotCurrent => PremiumStatus::NotCurrent,
                                PremiumStatus::NotActivated(_) => return Err(invalid()),
                            };
                            root.run_guild_rank_write(recovery, write, Some(status))
                                .await
                        }
                    }
                })
            })
            .await?
    }

    pub(super) async fn preflight_guild_target(
        &self,
        recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
        node: NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildRequest,
        binding: Vec<u8>,
    ) -> Result<GuildTargetPreflight> {
        let pre_recovery = recovery;
        let pre_node = node;
        let pre_request = request;
        let pre_binding = binding;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                let result = async {
                    lock_fenced_guild_operation(&mut tx, &pre_recovery, &pre_request).await?;
                    if let Some(outcome) = replay(&mut tx, &pre_request, &pre_binding).await? {
                        return Ok(GuildTargetPreflight::Replayed(outcome));
                    }
                    let actor = assert_session_fence(&mut tx, &fence, &pre_node).await?
                        .map_err(|_| GuildError::NotCurrent)?;
                    // A selector read, never caller Account data or target session authority.
                    // GUILD0-LO-01 explicitly takes no target root lock for this command.
                    let target = pre_request.operation.target(fence.character_id);
                    let row = sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                        .bind(target.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                    let account = match row {
                        Some(row) if row.try_get::<i16,_>("lifecycle")? == 1
                            && uuid_text(&row.try_get::<String,_>("world_id")?)? == *fence.runtime_scope.world_id().as_bytes() =>
                            Some(uuid_text(&row.try_get::<String,_>("account_id")?)?),
                        _ => None,
                    };
                    Ok(GuildTargetPreflight::Fresh(uuid_text(&actor)?,account))
                }.await;
                match result {
                    Ok(value) => { commit_semantic_transaction(tx, deadline).await?; Ok(Ok(value)) }
                    Err(GuildError::Unavailable(error)) => Err(error),
                    Err(error) => Ok(Err(error)),
                }
            })
        }).await?
    }
}
pub(super) enum GuildTargetPreflight {
    Replayed(GuildOutcome),
    Fresh([u8; 16], Option<[u8; 16]>),
}
include!("rank_data.rs");
pub struct RankEdit<'a, 'premium> {
    pub(super) request: &'a GuildRankRequest,
    pub(super) account: Option<[u8; 16]>,
    pub(super) premium: Option<&'a PremiumStatus<'premium>>,
}
impl RankWrite {
    pub async fn execute(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        premium: Option<&PremiumStatus<'_>>,
    ) -> Result<GuildOutcome> {
        lock_fenced_guild_operation(tx, recovery, &self.rank.common).await?;
        let edit = MemberEdit::Rank(RankEdit {
            request: &self.rank,
            account: self.account,
            premium,
        });
        execute(
            tx,
            &self.node,
            &self.fence,
            &self.rank.common,
            &self.binding,
            None,
            Some(&edit),
        )
        .await
    }
}
