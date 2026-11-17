//! The World row serializes jar breaks across Channels, while each Character
//! owns a separate failure timestamp for each of the first four jars.
use super::*;
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use crate::durability::quest_world_callback::{QuestWorldContext, context_facts_are_current};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_world_change::mummys_curse::{JarOutcome, use_jar};

impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the caller's authority and source context are independent proofs"
    )]
    pub async fn use_mummys_curse_jar(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        context: QuestWorldContext,
        action: u16,
        roll: u8,
    ) -> Result<(JarOutcome, u8), DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(DurabilityError::Unavailable);
        };
        permit.check_channel(world_id, channel_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut lane = permit;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut lane, move |holder, deadline, _| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err()
                        || !context_facts_are_current(&mut tx, fence, &context).await?
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    let world = world_id.as_bytes().as_slice();
                    let character = fence.character_id.as_bytes().as_slice();
                    sqlx::query("INSERT INTO game_mummys_curse_progress(world_id) VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
                        .bind(world).execute(&mut *tx).await?;
                    let progress: i16 = sqlx::query_scalar("SELECT broken_jars FROM game_mummys_curse_progress WHERE world_id=encode($1,'hex')::uuid FOR UPDATE")
                        .bind(world).fetch_one(&mut *tx).await?;
                    let mut progress = u8::try_from(progress).map_err(|_| DurabilityError::InvalidStoredState)?;
                    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                        .fetch_one(&mut *tx).await?;
                    let retry: Option<i64> = sqlx::query_scalar("SELECT retry_at FROM game_mummys_curse_retries WHERE world_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND action_id=$3")
                        .bind(world).bind(character).bind(i32::from(action)).fetch_optional(&mut *tx).await?;
                    let outcome = use_jar(action, progress, now.max(0) as u64, retry.unwrap_or_default() as u64, roll);
                    match &outcome {
                        JarOutcome::Failed { retry_at } => {
                            sqlx::query("INSERT INTO game_mummys_curse_retries(world_id,character_id,action_id,retry_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4) ON CONFLICT (world_id,character_id,action_id) DO UPDATE SET retry_at=excluded.retry_at")
                                .bind(world).bind(character).bind(i32::from(action)).bind(i64::try_from(*retry_at).map_err(|_| DurabilityError::InvalidStoredState)?)
                                .execute(&mut *tx).await?;
                        }
                        JarOutcome::Broken { .. } | JarOutcome::FinalJar => {
                            progress += 1;
                            sqlx::query("UPDATE game_mummys_curse_progress SET broken_jars=$2 WHERE world_id=encode($1,'hex')::uuid")
                                .bind(world).bind(i16::from(progress)).execute(&mut *tx).await?;
                        }
                        _ => {}
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok((outcome, progress))
                })
            })
            .await
    }
}
