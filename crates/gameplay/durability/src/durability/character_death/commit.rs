//! Fenced death commit, including promotion and held-blessing settlement.
use super::*;

impl DurabilityRoot {
    pub(in crate::durability) async fn commit_character_death_inner<const N: usize>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        mut formula: Option<&dyn BuildFormula>,
        premium: promotion::DeathPremiumState,
    ) -> Result<CharacterDeathOutcome> {
        validate_request(&fence, &request)?;
        let binding = command_binding(&fence, &request)?;
        let policy_digest = policy_digest(&request.policy)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run_with_context(&mut formula, move |holder, deadline, formula| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    if let Err(error) = promotion::hold_delivery(&mut tx, premium).await? { return Ok(Err(error)); }
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;

                    sqlx::query(
                        "SELECT pg_advisory_xact_lock(hashtextextended(\
                         'oteryn:character-death:' || encode($1, 'hex'), 0))",
                    )
                    .bind(request.occurrence.0.as_slice())
                    .execute(&mut *tx)
                    .await?;

                    if let Some(row) = load_receipt(&mut tx, request.occurrence).await? {
                        let stored: Vec<u8> = row.try_get("command_binding")?;
                        let committed = decode_receipt(&row)?;
                        let expected = match &committed.build_loss {
                            None => binding.clone(),
                            Some(build) => {
                                if formula.is_some_and(|formula| formula.digest() != build.formula_digest) {
                                    return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                                }
                                build_command_binding(&binding, build)
                            }
                        };
                        if stored != expected {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(CharacterDeathOutcome::AlreadyCommitted(committed)));
                    }

                    // The player died in the Channel that owns the session.
                    if fence.runtime_scope
                        != RuntimeScopeRefV1::channel(
                            request.death_cell.world_id,
                            request.death_cell.channel_id,
                        )
                    {
                        return Ok(Err(CharacterProgressionError::AuthorityRejected));
                    }
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let root_revision = root.revision;

                    // The character_root row lock taken by the fence serializes
                    // every Character writer, so these reads need no row lock
                    // (0016 grants the runtime no UPDATE on either relation).
                    let pending: Option<String> = sqlx::query_scalar(
                        "SELECT death_occurrence_id::text FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    if pending.is_some() {
                        return Ok(Err(CharacterProgressionError::RespawnPending));
                    }

                    let state = sqlx::query(
                        "SELECT character_revision::text, level, total_experience, \
                                profile_revision, ruleset_revision, content_revision, \
                                simulation_revision, evidence_revision, declaration_revision, \
                                policy_revision, reward_revision \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
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

                    let held: Vec<String> = sqlx::query_scalar(
                        "SELECT blessing_key COLLATE \"C\" FROM game_character_blessings \
                          WHERE character_id = encode($1,'hex')::uuid ORDER BY 1 LIMIT 33",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_all(&mut *tx)
                    .await?;
                    if held != request.held_blessings {
                        return Ok(Err(CharacterProgressionError::HeldBlessingsMismatch));
                    }

                    let level = u32::try_from(state.try_get::<i64, _>("level")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let snapshot = CurrentProgressionSnapshot {
                        level,
                        total_experience: ExactI64::new(state.try_get("total_experience")?),
                        skill_progression: (),
                        magic_progression: (),
                    };
                    // Every held blessing is a regular one until DEATH-4
                    // admits other kinds; more than D58's seven is rejected
                    // by the calculator.
                    let regular_blessings = u8::try_from(held.len())
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let operation = ProgressionOperation::ApplyDeathExperienceLoss {
                        death_occurrence: request.occurrence,
                        death_policy_revision: request.policy.death_policy_revision.clone(),
                        declared_difference_revision: request
                            .policy
                            .declared_difference_revision
                            .clone(),
                        regular_blessings,
                        promoted_with_current_premium: promotion::benefit(&mut tx, fence.character_id, premium).await?,
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
                    let build_before = load_death_build(&mut tx, fence.character_id).await?;
                    let build_loss = match prepare_death_build_loss(
                        *formula,
                        build_before,
                        &request.context.content,
                        staged.experience_before,
                        staged.death_experience_lost,
                    ) {
                        Ok(build) => build,
                        Err(error) => return Ok(Err(error)),
                    };
                    let binding = build_loss.as_ref().map_or_else(
                        || binding.clone(),
                        |build| build_command_binding(&binding, build),
                    );

                    let root_update = sqlx::query(
                        "UPDATE game_character_roots SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(root_revision.to_string())
                    .execute(&mut *tx)
                    .await?;
                    if root_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let progression_update = sqlx::query(
                        "UPDATE game_character_progression_state \
                            SET character_revision = $2::text::numeric(20,0), level = $3, \
                                total_experience = $4, harmony = 0, \
                                serene_forced_remaining_micros = 0 \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $5::text::numeric(20,0)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(committed_revision.to_string())
                    .bind(i64::from(staged.level_after))
                    .bind(staged.experience_after.get())
                    .bind(root_revision.to_string())
                    .execute(&mut *tx)
                    .await?;
                    if progression_update.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if let Some(build) = &build_loss {
                        update_death_build(
                            &mut tx,
                            fence.character_id,
                            request.occurrence,
                            committed_revision,
                            build,
                        ).await?;
                    }
                    let committed_at: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let blessings_after: Vec<String> = Vec::new();
                    let receipt = sqlx::query(
                        "INSERT INTO game_character_death_receipts(\
                           death_occurrence_id, command_binding, policy_digest, character_id, \
                           original_character_revision, committed_character_revision, \
                           level_before, level_after, experience_before, experience_after, \
                           experience_lost, blessings_before, blessings_after, \
                           amulet_of_loss_item_id, lost_item_ids, death_world_id, \
                           death_channel_id, death_spatial_position, death_map_revision, \
                           respawn_position, death_policy_revision, profile_revision, \
                           ruleset_revision, content_revision, simulation_revision, \
                           evidence_revision, declaration_revision, policy_revision, \
                           reward_revision, committed_at, \
                           vocation, magic_level_before, mana_spent_before, fist_level_before, fist_tries_before, club_level_before, club_tries_before, sword_level_before, sword_tries_before, axe_level_before, axe_tries_before, distance_level_before, distance_tries_before, shielding_level_before, shielding_tries_before, fishing_level_before, fishing_tries_before, magic_level_after, mana_spent_after, fist_level_after, fist_tries_after, club_level_after, club_tries_after, sword_level_after, sword_tries_after, axe_level_after, axe_tries_after, distance_level_after, distance_tries_after, shielding_level_after, shielding_tries_after, fishing_level_after, fishing_tries_after) \
                         VALUES (encode($1,'hex')::uuid, $2, $3, encode($4,'hex')::uuid, \
                           $5::text::numeric(20,0), $6::text::numeric(20,0), $7, $8, $9, $10, \
                           $11, $12, $13, NULL, ARRAY[]::uuid[], encode($14,'hex')::uuid, \
                           encode($15,'hex')::uuid, $16, $17, $18, $19, $20, $21, $22, $23, \
                           $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38, $39, $40, $41, $42, $43, $44, $45, $46, $47, $48, $49, $50, $51, $52, $53, $54, $55, $56, $57, $58, $59, $60, $61)",
                    )
                    .bind(request.occurrence.0.as_slice())
                    .bind(&binding)
                    .bind(policy_digest.as_slice())
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(root_revision.to_string())
                    .bind(committed_revision.to_string())
                    .bind(i64::from(staged.level_before))
                    .bind(i64::from(staged.level_after))
                    .bind(staged.experience_before.get())
                    .bind(staged.experience_after.get())
                    .bind(staged.death_experience_lost.get())
                    .bind(&held)
                    .bind(&blessings_after)
                    .bind(request.death_cell.world_id.as_bytes().as_slice())
                    .bind(request.death_cell.channel_id.as_bytes().as_slice())
                    .bind(&request.death_cell.spatial_position)
                    .bind(&request.death_cell.map_revision)
                    .bind(&request.respawn_position)
                    .bind(&request.policy.death_policy_revision)
                    .bind(&request.context.profile)
                    .bind(&request.context.ruleset)
                    .bind(&request.context.content)
                    .bind(&request.context.simulation)
                    .bind(&request.context.evidence)
                    .bind(&request.context.declaration)
                    .bind(&request.policy_revision)
                    .bind(&request.reward_revision)
                    .bind(committed_at);
                    bind_death_build(receipt, build_loss.as_ref())
                        .execute(&mut *tx)
                        .await?;
                    if let Err(error) =
                        super::super::pvp_death_consequences::apply_pvp_death_in_transaction(
                            &mut tx,
                            fence.character_id,
                            request.death_cell.world_id,
                            request.occurrence.as_bytes(),
                            &request.pvp_contributors,
                        )
                        .await
                    {
                        return Ok(Err(error));
                    }
                    let consumed = sqlx::query(
                        "DELETE FROM game_character_blessings \
                          WHERE character_id = encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .execute(&mut *tx)
                    .await?;
                    if usize::try_from(consumed.rows_affected()).ok() != Some(held.len()) {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    sqlx::query(
                        "INSERT INTO game_character_pending_respawns \
                         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3)",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(request.occurrence.0.as_slice())
                    .bind(&request.respawn_position)
                    .execute(&mut *tx)
                    .await?;

                    crate::durability::tutorial_death::reset_in_transaction(&mut tx, fence.character_id, &request.respawn_position).await?;
                    let committed = CommittedCharacterDeath {
                        occurrence: request.occurrence,
                        character_id: fence.character_id,
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        level_before: staged.level_before,
                        level_after: staged.level_after,
                        experience_before: staged.experience_before,
                        experience_after: staged.experience_after,
                        experience_lost: staged.death_experience_lost,
                        blessings_before: held,
                        blessings_after,
                        death_cell: request.death_cell.clone(),
                        respawn_position: request.respawn_position.clone(),
                        build_loss,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(CharacterDeathOutcome::Committed(committed)))
                })
            })
            .await?
    }
}
