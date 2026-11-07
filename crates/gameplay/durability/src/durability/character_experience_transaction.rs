//! The native XP receipt writer, shared by solo and atomic multi-Character settlement.
use super::DurabilityError;
use super::character_progression::*;
use super::character_progression::{
    assert_gameplay_fence, command_binding, decode_receipt, load_receipt, numeric_u64,
    policy_digest, state_matches_root, stored_context_matches, uuid_text, validate_request,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::domain::{
    CharacterRevision,
    progression::{CurrentProgressionSnapshot, ProgressionOperation, calculate_progression},
};
use oteryn_simulation_determinism::ExactI64;
use sha2::{Digest, Sha256};
use sqlx::Row;

pub(super) async fn commit_in_transaction<const N: usize>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &ExperienceAwardRequest<N>,
    quest_obligation: bool,
    hunting_binding: Option<[u8; 32]>,
) -> std::result::Result<
    std::result::Result<ExperienceCommitOutcome, CharacterProgressionError>,
    DurabilityError,
> {
    if let Err(error) = validate_request(&fence, request) {
        return Ok(Err(error));
    }
    let binding = match command_binding(&fence, request) {
        Ok(b) => b,
        Err(e) => return Ok(Err(e)),
    };
    let binding = if let Some(hunting) = hunting_binding {
        let mut bytes = binding;
        bytes.extend_from_slice(&hunting);
        let mut bound = vec![1];
        bound.extend_from_slice(&Sha256::digest(bytes));
        bound
    } else {
        binding
    };
    let policy_digest = match policy_digest(&request.policy) {
        Ok(d) => d,
        Err(e) => return Ok(Err(e)),
    };
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
     'oteryn:character-xp:' || encode($1, 'hex'), 0))",
    )
    .bind(request.occurrence.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;

    if let Some(row) = load_receipt(tx, request.occurrence).await? {
        let stored: Vec<u8> = row.try_get("command_binding")?;
        if stored != binding {
            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
        }
        let committed = decode_receipt(&row)?;
        return Ok(Ok(ExperienceCommitOutcome::AlreadyCommitted(committed)));
    }

    let root = match assert_gameplay_fence(tx, &fence, node).await? {
        Ok(root) => root,
        Err(error) => return Ok(Err(error)),
    };
    let root_revision = root.revision;

    let state = sqlx::query(
        "SELECT character_revision::text, level, total_experience, \
            profile_revision, ruleset_revision, content_revision, \
            simulation_revision, evidence_revision, declaration_revision, \
            policy_revision, reward_revision \
       FROM game_character_progression_state \
      WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(state) = state else {
        return Ok(Err(CharacterProgressionError::MissingProgressionState));
    };
    if numeric_u64(&state, "character_revision")? != root_revision
        || !stored_context_matches(
            &state,
            &request.context,
            &request.policy_revision,
            &request.reward_revision,
        )
        || !state_matches_root(&state, &root)
    {
        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
    }

    if quest_obligation {
        // The already-held character_root lock serializes obligation inserts
        // and consumption with the quest writer. FOR UPDATE on the obligation
        // would require UPDATE, which the runtime role intentionally lacks.
        let obligation = sqlx::query(
            "SELECT character_id::text, amount \
           FROM game_character_quest_xp_obligations \
          WHERE reward_occurrence_id = encode($1,'hex')::uuid",
        )
        .bind(request.occurrence.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await?;
        let Some(obligation) = obligation else {
            return Ok(Err(CharacterProgressionError::InvalidInput));
        };
        if uuid_text(obligation.try_get("character_id")?)? != *fence.character_id.as_bytes()
            || obligation.try_get::<i64, _>("amount")? != request.amount.get()
        {
            return Ok(Err(CharacterProgressionError::InvalidInput));
        }
    }

    let level_i64: i64 = state.try_get("level")?;
    let level = u32::try_from(level_i64).map_err(|_| DurabilityError::InvalidStoredState)?;
    let experience: i64 = state.try_get("total_experience")?;
    let snapshot = CurrentProgressionSnapshot {
        level,
        total_experience: ExactI64::new(experience),
        skill_progression: (),
        magic_progression: (),
    };
    let operation = ProgressionOperation::AwardExperience {
        source_occurrence: request.occurrence,
        reward_revision: request.reward_revision.clone(),
        amount: request.amount,
    };
    let staged = match calculate_progression(
        &snapshot,
        &request.context,
        &request.policy_revision,
        &operation,
        &request.policy,
    ) {
        Ok(staged) => staged,
        Err(error) => {
            return Ok(Err(CharacterProgressionError::Calculation(error)));
        }
    };
    let committed_revision = root_revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;

    let root_update = sqlx::query(
        "UPDATE game_character_roots SET character_revision = $2::text::numeric(20,0) \
      WHERE character_id = encode($1,'hex')::uuid \
        AND character_revision = $3::text::numeric(20,0)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(committed_revision.to_string())
    .bind(root_revision.to_string())
    .execute(&mut **tx)
    .await?;
    if root_update.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let progression_update = sqlx::query(
        "UPDATE game_character_progression_state \
        SET character_revision = $2::text::numeric(20,0), level = $3, \
            total_experience = $4 \
      WHERE character_id = encode($1,'hex')::uuid \
        AND character_revision = $5::text::numeric(20,0)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(committed_revision.to_string())
    .bind(i64::from(staged.level_after))
    .bind(staged.experience_after.get())
    .bind(root_revision.to_string())
    .execute(&mut **tx)
    .await?;
    if progression_update.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let committed_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    sqlx::query(
        "INSERT INTO game_character_xp_receipts(\
       reward_occurrence_id, command_binding, policy_digest, character_id, \
       original_character_revision, committed_character_revision, \
       level_before, level_after, experience_before, experience_after, \
       experience_awarded, profile_revision, ruleset_revision, \
       content_revision, simulation_revision, evidence_revision, \
       declaration_revision, policy_revision, reward_revision, committed_at) \
     VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, \
       $5::text::numeric(20,0), $6::text::numeric(20,0), $7, $8, $9, $10, \
       $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)",
    )
    .bind(request.occurrence.as_bytes().as_slice())
    .bind(&binding)
    .bind(policy_digest.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(root_revision.to_string())
    .bind(committed_revision.to_string())
    .bind(i64::from(staged.level_before))
    .bind(i64::from(staged.level_after))
    .bind(staged.experience_before.get())
    .bind(staged.experience_after.get())
    .bind(staged.experience_awarded.get())
    .bind(&request.context.profile)
    .bind(&request.context.ruleset)
    .bind(&request.context.content)
    .bind(&request.context.simulation)
    .bind(&request.context.evidence)
    .bind(&request.context.declaration)
    .bind(&request.policy_revision)
    .bind(&request.reward_revision)
    .bind(committed_at)
    .execute(&mut **tx)
    .await?;
    if quest_obligation {
        let consumed = sqlx::query(
            "DELETE FROM game_character_quest_xp_obligations \
          WHERE reward_occurrence_id = encode($1,'hex')::uuid",
        )
        .bind(request.occurrence.as_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
        if consumed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }

    let committed = CommittedExperienceAward {
        occurrence: request.occurrence,
        character_id: fence.character_id,
        original_character_revision: fence.expected_character_revision,
        committed_character_revision: CharacterRevision::new(committed_revision)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        level_before: staged.level_before,
        level_after: staged.level_after,
        experience_before: staged.experience_before,
        experience_after: staged.experience_after,
        experience_awarded: staged.experience_awarded,
    };
    Ok(Ok(ExperienceCommitOutcome::Committed(committed)))
}
