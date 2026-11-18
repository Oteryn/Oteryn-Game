//! Party cooldowns for a prepared Channel encounter, under its exact session fences.
use super::cooldown_storage::{
    BossCooldownAdmissionData, BossCooldownChange, BossRewardKey, BossRewardStateError,
    read_boss_cooldown_data, set_boss_cooldowns_data,
};
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::assert_item_only_fence;
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct ChannelBossCooldown<'a> {
    pub boss: &'a str,
    pub seconds: u32,
    pub reset_epoch: u64,
    pub cause: CommandRef,
    pub lever_key: u64,
    pub lever_revision: u64,
    pub party: Vec<CurrentCharacterItemFence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelBossCooldownDecision {
    Admitted,
    OnCooldown,
    Rejected,
}

fn cooldown_error(error: BossRewardStateError) -> DurabilityError {
    match error {
        BossRewardStateError::Unavailable(error) => error,
        _ => DurabilityError::InvalidStoredState,
    }
}

impl DurabilityRoot {
    /// Write one deadline for every member after local preparations succeeded.
    /// The same exact command and roster can resolve a lost COMMIT response.
    pub async fn commit_channel_boss_cooldown(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        mut request: ChannelBossCooldown<'_>,
    ) -> Result<ChannelBossCooldownDecision, DurabilityError> {
        use ChannelBossCooldownDecision::{Admitted, OnCooldown, Rejected};

        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = permit.runtime_scope()
        else {
            return Ok(Rejected);
        };
        request
            .party
            .sort_unstable_by_key(|member| member.character_id);
        if permit.has_unresolved()
            || !(1..=15).contains(&request.party.len())
            || !(1..=30 * 24 * 60 * 60).contains(&request.seconds)
            || !request
                .party
                .iter()
                .any(|member| member.game_session_id == request.cause.game_session_id())
            || request
                .party
                .windows(2)
                .any(|pair| pair[0].character_id == pair[1].character_id)
            || request.party.iter().any(|member| {
                member.runtime_scope != permit.runtime_scope()
                    || member.scope_ownership_generation
                        != request.party[0].scope_ownership_generation
            })
        {
            return Ok(Rejected);
        }
        let Ok(keys) = request
            .party
            .iter()
            .map(|member| BossRewardKey::new(member.character_id, request.boss))
            .collect::<Result<Vec<_>, _>>()
        else {
            return Ok(Rejected);
        };
        let mut digest = Sha256::new();
        digest.update(b"oteryn:channel-boss-cooldown:v1\0");
        digest.update(world_id.as_bytes());
        digest.update(channel_id.as_bytes());
        digest.update(request.reset_epoch.to_be_bytes());
        digest.update(request.cause.game_session_id().as_bytes());
        digest.update(request.cause.command_id().get().to_be_bytes());
        digest.update(request.lever_key.to_be_bytes());
        digest.update(request.lever_revision.to_be_bytes());
        digest.update(request.seconds.to_be_bytes());
        for member in &request.party {
            digest.update(member.character_id.as_bytes());
            digest.update(member.game_session_id.as_bytes());
            digest.update(member.character_lease_generation.to_be_bytes());
            digest.update(member.scope_ownership_generation.get().to_be_bytes());
        }
        digest.update(request.boss.as_bytes());
        let mut admission = [0; 16];
        admission.copy_from_slice(&digest.finalize()[..16]);
        admission[6] = admission[6] & 0x0f | 0x70;
        admission[8] = admission[8] & 0x3f | 0x80;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        let mut party = request.party;
        let reset_epoch = request.reset_epoch;
        let cooldown_s = u64::from(request.seconds);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut party, move |holder, deadline, party| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    for member in party {
                        if assert_item_only_fence(&mut tx, member, &node)
                            .await?
                            .is_err()
                        {
                            return Ok(Rejected);
                        }
                    }
                    let clock: Option<(i64, String)> = sqlx::query_as(
                    "SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint,reset_epoch::text \
                     FROM game_world_reset_read_locked(encode($1,'hex')::uuid)",
                )
                .bind(world_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                    let Some((now, epoch)) = clock else {
                        return Ok(Rejected);
                    };
                    if epoch.parse::<u64>().ok() != Some(reset_epoch) {
                        return Ok(Rejected);
                    }
                    let mut replay_count = 0;
                    let mut replay_deadline = None;
                    for key in &keys {
                        let old = read_boss_cooldown_data(&mut tx, key)
                            .await
                            .map_err(cooldown_error)?;
                        if let Some(old) = old.filter(|old| old.admission == admission) {
                            if old.reset_epoch != reset_epoch
                                || old.available_at <= now
                                || replay_deadline.is_some_and(|until| old.available_at != until)
                            {
                                return Ok(Rejected);
                            }
                            replay_deadline = Some(old.available_at);
                            replay_count += 1;
                        }
                    }
                    if replay_count != 0 {
                        return Ok(if replay_count == keys.len() {
                            Admitted
                        } else {
                            Rejected
                        });
                    }
                    let disposition = set_boss_cooldowns_data(
                        &mut tx,
                        &keys,
                        &BossCooldownAdmissionData {
                            admission,
                            now,
                            reset_epoch,
                            cooldown_s,
                            boosted: false,
                        },
                    )
                    .await
                    .map_err(cooldown_error)?;
                    if disposition == BossCooldownChange::OnCooldown {
                        return Ok(OnCooldown);
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Admitted)
                })
            })
            .await
    }
}
