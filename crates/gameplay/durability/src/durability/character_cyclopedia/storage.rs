use super::*;
use std::collections::BTreeMap;

pub(super) const DONATION_GOAL: u64 = 10_000_000;

pub(super) fn select_epoch(pool: u64) -> (bool, u64) {
    if pool >= DONATION_GOAL {
        (true, pool - DONATION_GOAL)
    } else {
        (false, pool)
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CanonicalProgress {
    pub(super) legacy: MapProgress,
    pub(super) subareas: BTreeMap<u32, MapProgress>,
}
impl CanonicalProgress {
    pub(super) fn decode(text: &str) -> Result<Self, DurabilityError> {
        serde_json::from_str(text)
            .or_else(|_| {
                serde_json::from_str(text).map(|legacy| Self {
                    legacy,
                    subareas: BTreeMap::new(),
                })
            })
            .map_err(|_| DurabilityError::InvalidStoredState)
    }
    fn convert(&mut self, request: &MapRequest) {
        for (_, id) in &request.subareas {
            self.subareas.entry(*id).or_insert_with(|| {
                if self.legacy.discovered {
                    MapProgress {
                        discovered: true,
                        exploration: None,
                    }
                } else {
                    // Retain the legacy source even when geometry makes its old POIs unusable.
                    let exploration = self
                        .legacy
                        .exploration
                        .as_ref()
                        .filter(|e| e.points.iter().all(|p| request.pool.contains(p)))
                        .cloned();
                    MapProgress {
                        discovered: false,
                        exploration: if Some(*id) == request.subarea {
                            exploration
                        } else {
                            None
                        },
                    }
                }
            });
        }
    }
}

async fn load_all(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &MapRequest,
) -> Result<CanonicalProgress, DurabilityError> {
    let row=sqlx::query("SELECT bundle_digest,catalogue_digest,progress::text FROM game_character_cyclopedia WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    match row {
        None => Ok(CanonicalProgress::default()),
        Some(row) => {
            if !BasicArea::reads_content_digest(&row.try_get::<Vec<u8>, _>("catalogue_digest")?) {
                return Err(DurabilityError::InvalidStoredState);
            }
            let text: String = row.try_get("progress")?;
            if text.len() > 262144 {
                return Err(DurabilityError::InvalidStoredState);
            }
            let mut progress = CanonicalProgress::decode(&text)?;
            progress.convert(request);
            if row.try_get::<Vec<u8>, _>("bundle_digest")? != request.bundle {
                // Crystal geometry revision invalidation keeps completed states.
                for state in progress
                    .subareas
                    .values_mut()
                    .filter(|state| !state.discovered)
                {
                    state.exploration = None;
                }
            }
            if let Some(state) = request
                .subarea
                .and_then(|id| progress.subareas.get_mut(&id))
                && !state.discovered
                && state
                    .exploration
                    .as_ref()
                    .is_some_and(|e| e.points.iter().any(|point| !request.pool.contains(point)))
            {
                state.exploration = None;
            }
            if progress.subareas.len() > 512 {
                return Err(DurabilityError::InvalidStoredState);
            }
            Ok(progress)
        }
    }
}
pub(super) async fn load(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &MapRequest,
) -> Result<MapProgress, DurabilityError> {
    let mut progress = load_all(tx, fence, request).await?;
    progress.convert(request);
    Ok(request
        .subarea
        .and_then(|id| progress.subareas.remove(&id))
        .unwrap_or_default())
}
pub(super) async fn percentage(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &MapRequest,
    current: &MapProgress,
) -> Result<u8, DurabilityError> {
    let mut progress = load_all(tx, fence, request).await?;
    progress.convert(request);
    if let Some(id) = request.subarea {
        progress.subareas.insert(id, current.clone());
    }
    let ids: Vec<_> = request
        .subareas
        .iter()
        .filter(|(area, _)| *area == request.area)
        .map(|(_, id)| *id)
        .collect();
    if ids.is_empty() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let done = ids
        .iter()
        .filter(|id| progress.subareas.get(*id).is_some_and(|p| p.discovered))
        .count();
    Ok((done * 100 / ids.len()) as u8)
}
// One row per canonical main area; the shared row serializes all Channels of this World.
pub(super) async fn world(
    tx: &mut Type2Transaction<'_>,
    world: crate::foundation::WorldId,
    bundle: [u8; 32],
    epoch: u64,
    area: u32,
    legacy_area: u32,
) -> Result<(u64, bool), DurabilityError> {
    if !(1..=1024).contains(&area) || !(1..=1024).contains(&legacy_area) {
        return Err(DurabilityError::InvalidStoredState);
    }
    // The reviewed entry parent's row imports the historical source-only pool once.
    sqlx::query("INSERT INTO game_world_cyclopedia(world_id,bundle_digest,catalogue_digest,area_id,pool,legacy_imported) SELECT encode($1,'hex')::uuid,$2,$3,$4,CASE WHEN claim.import THEN COALESCE((SELECT pool FROM game_world_cyclopedia WHERE world_id=encode($1,'hex')::uuid AND area_id=0),0) ELSE 0 END,claim.import FROM (SELECT $5 AND NOT EXISTS(SELECT 1 FROM game_world_cyclopedia WHERE world_id=encode($1,'hex')::uuid AND legacy_imported) AS import) claim ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(bundle.as_slice()).bind(BasicArea::content_digest().as_slice()).bind(i64::from(area)).bind(area==legacy_area).execute(&mut ***tx).await?;
    let row=sqlx::query("SELECT bundle_digest,catalogue_digest,pool,legacy_imported FROM game_world_cyclopedia WHERE world_id=encode($1,'hex')::uuid AND area_id=$2 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(i64::from(area)).fetch_one(&mut ***tx).await?;
    if row.try_get::<Vec<u8>, _>("bundle_digest")? != bundle
        || row.try_get::<Vec<u8>, _>("catalogue_digest")? != BasicArea::content_digest()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let pool = u64::try_from(row.try_get::<i64, _>("pool")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if let Some(improved)=sqlx::query_scalar("SELECT improved FROM game_world_cyclopedia_epochs WHERE world_id=encode($1,'hex')::uuid AND epoch=$2::text::numeric AND area_id=$3")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(i64::from(area)).fetch_optional(&mut ***tx).await? {
        return Ok((pool,improved));
    }
    if row.try_get::<bool,_>("legacy_imported")?
        && let Some(improved) = sqlx::query_scalar::<_,bool>("SELECT improved FROM game_world_cyclopedia_epochs WHERE world_id=encode($1,'hex')::uuid AND area_id=0 AND epoch=$2::text::numeric")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_optional(&mut ***tx).await? {
            // Preserve an already-selected legacy day without consuming its pool twice.
            sqlx::query("INSERT INTO game_world_cyclopedia_epochs(world_id,area_id,epoch,improved,donation_winner,pool_before) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4,false,$5)")
                .bind(world.as_bytes().as_slice()).bind(i64::from(area)).bind(epoch.to_string()).bind(improved).bind(pool as i64).execute(&mut ***tx).await?;
            return Ok((pool,improved));
    }
    // Crystal discovery_system.lua:269-280: every funded save consumes one goal,
    // carries excess, and can activate on consecutive days. There is no random draw.
    let (improved, remaining) = select_epoch(pool);
    sqlx::query("INSERT INTO game_world_cyclopedia_epochs(world_id,epoch,improved,donation_winner,pool_before,area_id) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$3,$4,$5)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(improved).bind(pool as i64).bind(i64::from(area)).execute(&mut ***tx).await?;
    if improved {
        sqlx::query("UPDATE game_world_cyclopedia SET pool=$3 WHERE world_id=encode($1,'hex')::uuid AND area_id=$2")
            .bind(world.as_bytes().as_slice()).bind(i64::from(area)).bind(remaining as i64).execute(&mut ***tx).await?;
    }
    Ok((remaining, improved))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn save(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &MapRequest,
    progress: &MapProgress,
    advanced: bool,
    donation: u64,
    pool: u64,
    pool_after: u64,
    result: &[u8],
) -> Result<(), DurabilityError> {
    let character = fence.character_id.as_bytes().as_slice();
    sqlx::query("INSERT INTO game_character_cyclopedia_receipts(occurrence,character_id,world_id,command_session_id,command_id,binding,result,fee_transaction_id,donation,pool_before,pool_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,area_id,reset_subarea) SELECT encode($1,'hex')::uuid,s.character_id,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,encode($7,'hex')::uuid,$8,$9,$10,s.character_revision,CASE WHEN $11 THEN s.character_revision+1 ELSE NULL END,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,$13,$14 FROM game_character_progression_state s WHERE s.character_id=encode($12,'hex')::uuid")
        .bind(request.occurrence.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(request.binding.as_slice()).bind(result).bind((donation>0).then_some(request.transaction.as_slice())).bind(donation as i64).bind(pool as i64).bind(pool_after as i64).bind(advanced).bind(character).bind(i64::from(request.area)).bind((advanced && request.action == MapAction::Shuffle).then_some(request.subarea.map(i64::from)).flatten()).execute(&mut ***tx).await?;
    if advanced {
        let mut all = load_all(tx, fence, request).await?;
        all.convert(request);
        let id = request.subarea.ok_or(DurabilityError::InvalidStoredState)?;
        all.subareas.insert(id, progress.clone());
        let progress =
            serde_json::to_string(&all).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_character_cyclopedia(character_id,bundle_digest,progress,occurrence,catalogue_digest) VALUES(encode($1,'hex')::uuid,$2,$3::text::jsonb,encode($4,'hex')::uuid,$5) ON CONFLICT(character_id) DO UPDATE SET bundle_digest=EXCLUDED.bundle_digest,progress=EXCLUDED.progress,occurrence=EXCLUDED.occurrence,catalogue_digest=EXCLUDED.catalogue_digest")
            .bind(character).bind(request.bundle.as_slice()).bind(progress).bind(request.occurrence.as_slice()).bind(BasicArea::content_digest().as_slice()).execute(&mut ***tx).await?;
        sqlx::query("UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid").bind(character).execute(&mut ***tx).await?;
        sqlx::query("UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid").bind(character).execute(&mut ***tx).await?;
    }
    if donation > 0 {
        sqlx::query(
            "UPDATE game_world_cyclopedia SET pool=$2 WHERE world_id=encode($1,'hex')::uuid AND area_id=$3",
        )
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(pool_after as i64)
        .bind(i64::from(request.area))
        .execute(&mut ***tx)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::select_epoch;

    #[test]
    fn w12_selection_carries_excess_over_goal() {
        assert_eq!(select_epoch(25_000_000), (true, 15_000_000));
        assert_eq!(select_epoch(10_000_000), (true, 0));
    }

    #[test]
    fn w12_selection_has_no_random_or_winner_exclusion() {
        assert_eq!(select_epoch(9_999_999), (false, 9_999_999));
        assert_eq!(select_epoch(0), (false, 0));
        let (first_improved, pool_after) = select_epoch(20_000_000);
        let (second_improved, _) = select_epoch(pool_after);
        assert!(first_improved);
        assert!(second_improved);
    }
}
