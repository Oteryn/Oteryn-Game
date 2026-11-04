use super::*;

pub(super) const DONATION_GOAL: u64 = 10_000_000;

pub(super) fn select_epoch(pool: u64) -> (bool, u64) {
    if pool >= DONATION_GOAL {
        (true, pool - DONATION_GOAL)
    } else {
        (false, pool)
    }
}

pub(super) async fn load(
    tx: &mut Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &MapRequest,
) -> Result<MapProgress, DurabilityError> {
    let row=sqlx::query("SELECT bundle_digest,catalogue_digest,progress::text FROM game_character_cyclopedia WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    match row {
        None => Ok(MapProgress::default()),
        Some(row) => {
            if row.try_get::<Vec<u8>, _>("bundle_digest")? != request.bundle
                || !BasicArea::reads_content_digest(&row.try_get::<Vec<u8>, _>("catalogue_digest")?)
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            let text: String = row.try_get("progress")?;
            if text.len() > 4096 {
                return Err(DurabilityError::InvalidStoredState);
            }
            serde_json::from_str(&text).map_err(|_| DurabilityError::InvalidStoredState)
        }
    }
}
// One area is a proven bound; the shared row serializes all Channels of this World.
pub(super) async fn world(
    tx: &mut Type2Transaction<'_>,
    world: crate::foundation::WorldId,
    bundle: [u8; 32],
    epoch: u64,
) -> Result<(u64, bool), DurabilityError> {
    sqlx::query("INSERT INTO game_world_cyclopedia(world_id,bundle_digest,catalogue_digest) VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(bundle.as_slice()).bind(BasicArea::content_digest().as_slice()).execute(&mut ***tx).await?;
    let row=sqlx::query("SELECT bundle_digest,catalogue_digest,pool FROM game_world_cyclopedia WHERE world_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(world.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    if row.try_get::<Vec<u8>, _>("bundle_digest")? != bundle
        || row.try_get::<Vec<u8>, _>("catalogue_digest")? != BasicArea::content_digest()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let pool = u64::try_from(row.try_get::<i64, _>("pool")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if let Some(improved)=sqlx::query_scalar("SELECT improved FROM game_world_cyclopedia_epochs WHERE world_id=encode($1,'hex')::uuid AND epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_optional(&mut ***tx).await? {
        return Ok((pool,improved));
    }
    let (improved, pool_after) = select_epoch(pool);
    sqlx::query("INSERT INTO game_world_cyclopedia_epochs(world_id,epoch,improved,donation_winner,pool_before) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$5)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(improved).bind(improved).bind(pool as i64).execute(&mut ***tx).await?;
    if improved {
        sqlx::query(
            "UPDATE game_world_cyclopedia SET pool=$2 WHERE world_id=encode($1,'hex')::uuid",
        )
        .bind(world.as_bytes().as_slice())
        .bind(pool_after as i64)
        .execute(&mut ***tx)
        .await?;
    }
    Ok((pool_after, improved))
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
    sqlx::query("INSERT INTO game_character_cyclopedia_receipts(occurrence,character_id,world_id,command_session_id,command_id,binding,result,fee_transaction_id,donation,pool_before,pool_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT encode($1,'hex')::uuid,s.character_id,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,encode($7,'hex')::uuid,$8,$9,$10,s.character_revision,CASE WHEN $11 THEN s.character_revision+1 ELSE NULL END,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision FROM game_character_progression_state s WHERE s.character_id=encode($12,'hex')::uuid")
        .bind(request.occurrence.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(request.binding.as_slice()).bind(result).bind((donation>0).then_some(request.transaction.as_slice())).bind(donation as i64).bind(pool as i64).bind(pool_after as i64).bind(advanced).bind(character).execute(&mut ***tx).await?;
    if advanced {
        let progress =
            serde_json::to_string(progress).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_character_cyclopedia(character_id,bundle_digest,progress,occurrence,catalogue_digest) VALUES(encode($1,'hex')::uuid,$2,$3::text::jsonb,encode($4,'hex')::uuid,$5) ON CONFLICT(character_id) DO UPDATE SET bundle_digest=EXCLUDED.bundle_digest,progress=EXCLUDED.progress,occurrence=EXCLUDED.occurrence,catalogue_digest=EXCLUDED.catalogue_digest")
            .bind(character).bind(request.bundle.as_slice()).bind(progress).bind(request.occurrence.as_slice()).bind(BasicArea::content_digest().as_slice()).execute(&mut ***tx).await?;
        sqlx::query("UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid").bind(character).execute(&mut ***tx).await?;
        sqlx::query("UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid").bind(character).execute(&mut ***tx).await?;
    }
    if donation > 0 {
        sqlx::query(
            "UPDATE game_world_cyclopedia SET pool=$2 WHERE world_id=encode($1,'hex')::uuid",
        )
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(pool_after as i64)
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
