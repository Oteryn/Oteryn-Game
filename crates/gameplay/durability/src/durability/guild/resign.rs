use super::*;
impl DurabilityRoot {
    /// Both moves into a NEW level1/2 consume their actual Account's native Premium decision.
    pub async fn commit_guild_resign(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        resign: GuildResignRequest,
        premium: &impl PremiumGateOwner,
    ) -> Result<GuildOutcome> {
        let binding = resign_binding(&fence, &resign)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let accounts = match self
            .preflight_guild_target(
                recovery.clone(),
                node.clone(),
                fence,
                resign.common.clone(),
                binding.clone(),
            )
            .await?
        {
            GuildTargetPreflight::Replayed(value) => return Ok(value),
            GuildTargetPreflight::Fresh(actor, target) => target.map(|target| [actor, target]),
        };
        let write = ResignWrite {
            node: node.clone(),
            fence,
            resign,
            binding,
            accounts,
        };
        let Some(accounts) = accounts else {
            return self.run_guild_resign_write(recovery, write, None).await;
        };
        let root = self.clone();
        premium
            .with_premium_batch_gate(&accounts, move |statuses, tx| {
                Box::pin(async move {
                    match tx {
                        Some(tx) => {
                            if statuses.len() != 2
                                || !statuses
                                    .iter()
                                    .all(|s| matches!(s, PremiumStatus::NotActivated(_)))
                            {
                                return Err(invalid());
                            }
                            // The native gate commits even on callback error: roll back all local writes.
                            let mut savepoint = tx.begin().await?;
                            let result = write
                                .execute(&mut savepoint, &recovery, Some(&statuses))
                                .await;
                            match result {
                                Ok(value) => {
                                    savepoint.commit().await?;
                                    Ok(value)
                                }
                                Err(error) => {
                                    savepoint.rollback().await?;
                                    Err(error)
                                }
                            }
                        }
                        None => {
                            let mut owned = Vec::with_capacity(statuses.len());
                            for status in statuses {
                                owned.push(match status {
                                    PremiumStatus::Current => PremiumStatus::Current,
                                    PremiumStatus::NotCurrent => PremiumStatus::NotCurrent,
                                    PremiumStatus::NotActivated(_) => return Err(invalid()),
                                });
                            }
                            root.run_guild_resign_write(recovery, write, Some(owned))
                                .await
                        }
                    }
                })
            })
            .await?
    }
    pub(super) async fn run_guild_resign_write(
        &self,
        recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
        write: ResignWrite,
        premium: Option<Vec<PremiumStatus<'static>>>,
    ) -> Result<GuildOutcome> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = write.execute(&mut tx, &recovery, premium.as_deref()).await;
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

    pub(super) async fn run_guild_rank_write(
        &self,
        recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
        write: RankWrite,
        premium: Option<PremiumStatus<'static>>,
    ) -> Result<GuildOutcome> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = write.execute(&mut tx, &recovery, premium.as_ref()).await;
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
}
include!("resign_data.rs");
pub struct ResignEdit<'a, 'premium> {
    pub(super) request: &'a GuildResignRequest,
    pub(super) accounts: Option<[[u8; 16]; 2]>,
    pub(super) premium: Option<&'a [PremiumStatus<'premium>]>,
}
pub enum MemberEdit<'a, 'premium> {
    Rank(RankEdit<'a, 'premium>),
    Resign(ResignEdit<'a, 'premium>),
    EditRanks(&'a GuildEditRanksRequest),
    Disband(&'a GuildDisbandRequest),
}
impl ResignWrite {
    pub async fn execute(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        premium: Option<&[PremiumStatus<'_>]>,
    ) -> Result<GuildOutcome> {
        lock_fenced_guild_operation(tx, recovery, &self.resign.common).await?;
        let edit = MemberEdit::Resign(ResignEdit {
            request: &self.resign,
            accounts: self.accounts,
            premium,
        });
        execute(
            tx,
            &self.node,
            &self.fence,
            &self.resign.common,
            &self.binding,
            None,
            Some(&edit),
        )
        .await
    }
}
