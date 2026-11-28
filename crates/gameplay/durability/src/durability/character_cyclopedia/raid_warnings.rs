//! Read the scheduler's committed plans and the character's discovery in one snapshot.
use super::*;
use crate::content::ProductionKey;
use crate::durability::character_authority::observe_recovery_fence;
use crate::durability::character_progression::observe_item_only_fence;
use crate::durability::db::{begin_observation_transaction, commit_semantic_read};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use oteryn_gameplay_encounter_rules::raid::MAX_CANDIDATES;

pub struct CommittedRaidPlan {
    pub firing_id: [u8; 16],
    pub key: ProductionKey,
    pub fired_at_ms: i64,
}

pub struct CyclopediaRaidWarnings {
    progress: storage::CanonicalProgress,
    pub plans: Vec<CommittedRaidPlan>,
}

impl CyclopediaRaidWarnings {
    pub fn subarea_discovered(&self, subarea: u32) -> bool {
        self.progress
            .subareas
            .get(&subarea)
            .map_or(self.progress.legacy.discovered, |progress| {
                progress.discovered
            })
    }
}

impl DurabilityRoot {
    pub async fn read_cyclopedia_raid_warnings(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        bundle: [u8; 32],
        now_ms: i64,
    ) -> Result<Option<CyclopediaRaidWarnings>, DurabilityError> {
        let Some(channel) = fence.runtime_scope.channel_id() else {
            return Ok(None);
        };
        let world = fence.runtime_scope.world_id();
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_observation_transaction(holder, deadline).await?;
                    observe_recovery_fence(&mut tx, &recovery).await?;
                    if observe_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(None);
                    }
                    let row: Option<(Vec<u8>, String)> = sqlx::query_as(
                        "SELECT catalogue_digest,progress::text FROM game_character_cyclopedia
                 WHERE character_id=encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let progress = match row {
                        None => storage::CanonicalProgress::default(),
                        Some((digest, text)) => {
                            if !BasicArea::reads_content_digest(&digest) || text.len() > 262144 {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            storage::CanonicalProgress::decode(&text)?
                        }
                    };
                    // A wave delay is a u32 in the selected catalogue. Earlier firings
                    // cannot have their first wave in the future.
                    let rows: Vec<(Vec<u8>, String, i64)> = sqlx::query_as(
                        "SELECT uuid_send(f.firing_id),f.raid_key,f.fired_at_ms
                 FROM game_world_reset_active a
                 JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
                 JOIN game_world_raid_firings f USING(world_id,reset_epoch)
                 JOIN game_world_raid_targets t USING(firing_id)
                 WHERE a.world_id=encode($1,'hex')::uuid AND e.target_digest=$2
                   AND f.bundle_digest=$2 AND f.state=1 AND f.deadline_ms>$6
                   AND NOT EXISTS(SELECT 1 FROM game_world_reset_jobs j
                                  WHERE j.world_id=a.world_id AND j.phase='RETIRING')
                   AND t.channel_id=encode($3,'hex')::uuid
                   AND t.owner_generation=$4::text::numeric
                   AND f.fired_at_ms BETWEEN $5 AND $6
                 ORDER BY f.fired_at_ms,f.firing_id LIMIT $7",
                    )
                    .bind(world.as_bytes().as_slice())
                    .bind(bundle.as_slice())
                    .bind(channel.as_bytes().as_slice())
                    .bind(fence.scope_ownership_generation.get().to_string())
                    .bind(now_ms.saturating_sub(i64::from(u32::MAX)))
                    .bind(now_ms)
                    .bind(
                        i64::try_from(MAX_CANDIDATES + 1)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                    )
                    .fetch_all(&mut *tx)
                    .await?;
                    if rows.len() > MAX_CANDIDATES {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let plans = rows
                        .into_iter()
                        .map(|(id, key, fired_at_ms)| {
                            Ok(CommittedRaidPlan {
                                firing_id: id
                                    .try_into()
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                                key: ProductionKey::new(&key)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                                fired_at_ms,
                            })
                        })
                        .collect::<Result<Vec<_>, DurabilityError>>()?;
                    commit_semantic_read(tx, deadline).await?;
                    Ok(Some(CyclopediaRaidWarnings { progress, plans }))
                })
            })
            .await
    }
}
