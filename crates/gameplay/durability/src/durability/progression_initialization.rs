//! The existing D88 initializer as a physical-transaction participant.
use super::*;
pub(crate) async fn initialize_in_transaction<const N: usize>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &ProgressionInitializationRequest<N>,
) -> std::result::Result<Result<ProgressionInitializationOutcome>, DurabilityError> {
    validate_initialization(&fence, request).map_err(|_| DurabilityError::InvalidStoredState)?;
    let root = match assert_gameplay_fence(tx, &fence, node).await? {
        Ok(root) => root,
        Err(error) => return Ok(Err(error)),
    };

    let existing = sqlx::query(
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
    if let Some(row) = existing {
        if numeric_u64(&row, "character_revision")? != root.revision {
            return Err(DurabilityError::InvalidStoredState);
        }
        if !stored_context_matches(
            &row,
            &request.context,
            &request.policy_revision,
            &request.reward_revision,
        ) || !state_matches_root(&row, &root)
        {
            return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
        }
        let state = decode_state(&row, fence.character_id)?;
        return Ok(Ok(ProgressionInitializationOutcome::AlreadyInitialized(
            state,
        )));
    }
    // Absence after revision one is corrupt state, never zero.
    if root.revision != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if request.context.profile != root.profile_revision
        || request.context.ruleset != root.ruleset_revision
        || request.context.content != root.content_revision
    {
        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
    }
    sqlx::query(
        "INSERT INTO game_character_progression_state(\
       character_id, character_revision, level, total_experience, \
       profile_revision, ruleset_revision, content_revision, \
       simulation_revision, evidence_revision, declaration_revision, \
       policy_revision, reward_revision) \
     VALUES (encode($1,'hex')::uuid, 1, $2, $3, $4, $5, $6, $7, $8, \
       $9, $10, $11)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(i64::from(INITIAL_CHARACTER_LEVEL))
    .bind(INITIAL_TOTAL_EXPERIENCE)
    .bind(&request.context.profile)
    .bind(&request.context.ruleset)
    .bind(&request.context.content)
    .bind(&request.context.simulation)
    .bind(&request.context.evidence)
    .bind(&request.context.declaration)
    .bind(&request.policy_revision)
    .bind(&request.reward_revision)
    .execute(&mut **tx)
    .await?;
    let state = CharacterProgressionState {
        character_id: fence.character_id,
        character_revision: fence.expected_character_revision,
        level: INITIAL_CHARACTER_LEVEL,
        total_experience: ExactI64::new(INITIAL_TOTAL_EXPERIENCE),
        context: request.context.clone(),
        policy_revision: request.policy_revision.clone(),
        reward_revision: request.reward_revision.clone(),
    };
    Ok(Ok(ProgressionInitializationOutcome::Initialized(state)))
}
