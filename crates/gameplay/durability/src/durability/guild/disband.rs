use super::*;

#[derive(Clone, Debug)]
pub struct GuildDisbandJobRequest {
    pub world_id: crate::foundation::WorldId,
    pub guild_id: GuildId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildDisbandOutcome {
    NoLongerPending,
    WaitingForHouseDisposition,
    Advanced { step: i32, removed: usize },
    Disbanded { revision: u64 },
}

impl DurabilityRoot {
    /// One bounded committed cleanup step. A retained cursor never supplies current authority.
    pub async fn advance_guild_disband(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: GuildDisbandJobRequest,
    ) -> Result<GuildDisbandOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        let Some(context) =
                            lock_guild_disband_with_recovery(&mut tx, recovery, &node, &request)
                                .await?
                        else {
                            return Ok(GuildDisbandOutcome::NoLongerPending);
                        };
                        if core_participants_installed(&mut tx).await? {
                            use crate::durability::guild_bank::{
                                PayoutProgress, ensure_disband_payout,
                            };
                            match ensure_disband_payout(&mut tx, &context).await? {
                                PayoutProgress::Blocked => {
                                    return Ok(GuildDisbandOutcome::WaitingForHouseDisposition);
                                }
                                PayoutProgress::Paid => {
                                    return Ok(GuildDisbandOutcome::Advanced {
                                        step: 1,
                                        removed: 0,
                                    });
                                }
                                PayoutProgress::AlreadyPaid => {}
                            }
                        }
                        super::disband_cleanup::advance_disband_transaction(&mut tx, &context).await
                    }
                    .await;
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
async fn core_participants_installed(tx: &mut Transaction<'_, Postgres>) -> Result<bool> {
    Ok(
        sqlx::query_scalar("SELECT game_guild_core_participants_installed()")
            .fetch_one(&mut **tx)
            .await?,
    )
}
