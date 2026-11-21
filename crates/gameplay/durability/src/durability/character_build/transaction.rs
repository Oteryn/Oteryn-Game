/// Every level up to the current one is reachable, and progress is held within the current level
/// (SKILLS-0 §3.1): no family has paid for its next reachable level without advancing.
///
/// An uncommitted build successor. It grants no durable or gameplay authority.
/// Only a proof of COMMIT of this same physical transaction exposes its receipt.
pub struct PendingBuildChange {
    outcome: BuildCommitOutcome,
    physical_transaction: String,
    source_transaction: Option<String>,
}
impl PendingBuildChange {
    pub(super) fn historical_outcome(&self) -> Option<BuildCommitOutcome> {
        match &self.outcome {
            BuildCommitOutcome::AlreadyCommitted(_) => Some(self.outcome.clone()),
            BuildCommitOutcome::Committed(_) => None,
        }
    }
    pub fn after_commit(
        self,
        proof: &super::spell_owner_commit::CommittedSpellOwnerTransaction,
    ) -> Result<BuildCommitOutcome> {
        if self.source_transaction.as_deref() != Some(proof.physical_transaction())
            || (matches!(self.outcome, BuildCommitOutcome::Committed(_))
                && self.physical_transaction != proof.physical_transaction())
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(self.outcome)
    }
}

/// Join a genuine Character build write to the caster's existing transaction.
/// Current recovery, session, lease, root revision and node are verified here;
/// a preceding write in this transaction cannot substitute for current authority.
pub async fn prepare_character_build_in_transaction(
    root: &DurabilityRoot,
    tx: &mut sqlx::Transaction<'_, Postgres>,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: BuildChangeRequest,
    formula: &dyn BuildFormula,
) -> Result<PendingBuildChange> {
    let recovery = authority
        .record_for(root)
        .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
    prepare_character_build_with_recovery(tx, &recovery, node, fence, request, formula).await
}

/// Sibling writers may carry an actual pre-qualified recovery record into the
/// SQL-only semantic-pass callback. This never reconstructs a current token:
/// the record and every independent current DB fence are verified in this TX.
pub(super) async fn prepare_character_build_with_recovery(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: BuildChangeRequest,
    formula: &dyn BuildFormula,
) -> Result<PendingBuildChange> {
    if fence.character_lease_generation == 0
        || !request.cause.admits_runtime_scope(fence.runtime_scope)
        || !request.cause.admits(&request.before, &request.after)
        || !normalized(formula, &request.after)
        || request
            .pruned_stance
            .as_deref()
            .is_some_and(|stance| request.cause == BuildCause::Training || !valid_revision(stance))
        || (request.cause.converts()
            && convert_vocation(formula, &request.before, &request.after.vocation)?
                != request.after)
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    let policy_digest = formula.digest();
    let binding = command_binding(&fence, &request, &policy_digest);
    let outcome = apply_build_in_transaction(
        tx,
        node,
        &fence,
        request,
        policy_digest,
        formula.content_revision().to_owned(),
        binding,
        None,
    )
    .await??;
    let physical_transaction: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await
        .map_err(DurabilityError::from)?;
    let occurrence = match &outcome {
        BuildCommitOutcome::Committed(receipt) | BuildCommitOutcome::AlreadyCommitted(receipt) => {
            receipt.occurrence
        }
    };
    let source_transaction:Option<String>=sqlx::query_scalar(
        "SELECT created_xact_id::text FROM game_character_build_receipts WHERE build_occurrence_id=encode($1,'hex')::uuid")
        .bind(occurrence.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    Ok(PendingBuildChange {
        outcome,
        physical_transaction,
        source_transaction,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "existing build participant plus OFFLINE0 columns"
)]
pub(super) async fn apply_build_in_transaction(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterGameplayFence,
    request: BuildChangeRequest,
    policy_digest: [u8; 32],
    content_revision: String,
    binding: Vec<u8>,
    offline: Option<super::offline_training_state::OfflineChange>,
) -> std::result::Result<Result<BuildCommitOutcome>, DurabilityError> {
    apply_build_with_fence(
        tx,
        BuildFence::Gameplay(node),
        fence,
        request,
        policy_digest,
        content_revision,
        binding,
        offline,
    )
    .await
}

/// Only the native explicit House logout participant can supply this proof.
/// No general Character build caller acquires House write authority here.
#[allow(
    clippy::too_many_arguments,
    reason = "the existing build participant plus its closed House logout proof"
)]
pub(super) async fn apply_house_logout_in_transaction(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    house: super::loop_training::HouseLogout<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: BuildChangeRequest,
    policy_digest: [u8; 32],
    content_revision: String,
    binding: Vec<u8>,
    offline: super::offline_training_state::OfflineChange,
) -> std::result::Result<Result<BuildCommitOutcome>, DurabilityError> {
    if house.fence() != *fence
        || request.cause != BuildCause::Logout
        || request.before != request.after
        || request.pruned_stance.is_some()
    {
        return Ok(Err(CharacterProgressionError::InvalidInput));
    }
    apply_build_with_fence(
        tx,
        BuildFence::HouseLogout(house),
        fence,
        request,
        policy_digest,
        content_revision,
        binding,
        Some(offline),
    )
    .await
}

enum BuildFence<'a> {
    Gameplay(&'a NodeIncarnationProof),
    HouseLogout(super::loop_training::HouseLogout<'a>),
}

#[allow(
    clippy::too_many_arguments,
    reason = "the unchanged native build transaction participant"
)]
async fn apply_build_with_fence(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    authority: BuildFence<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: BuildChangeRequest,
    policy_digest: [u8; 32],
    content_revision: String,
    binding: Vec<u8>,
    offline: Option<super::offline_training_state::OfflineChange>,
) -> std::result::Result<Result<BuildCommitOutcome>, DurabilityError> {
    let stamina_training = offline.is_some_and(|change| {
        request.cause == BuildCause::Training
            && request.before == request.after
            && change.after.stamina_minutes < change.before.stamina_minutes
    });
    let stamina_resting = offline.is_some_and(|change| {
        request.cause == BuildCause::RestingStamina
            && request.before == request.after
            && change.after.stamina_minutes > change.before.stamina_minutes
    });
    if (!request.cause.admits(&request.before, &request.after)
        && !stamina_training
        && !stamina_resting)
        || binding.is_empty()
        || binding.len() > 1024
    {
        return Ok(Err(CharacterProgressionError::InvalidInput));
    }
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
                         'oteryn:character-build:' || encode($1, 'hex'), 0))",
    )
    .bind(request.occurrence.0.as_slice())
    .execute(&mut **tx)
    .await?;

    if let Some(row) = load_receipt(tx, request.occurrence).await? {
        let stored: Vec<u8> = row.try_get("command_binding")?;
        if stored != binding {
            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
        }
        let committed = decode_receipt(&row)?;
        return Ok(Ok(BuildCommitOutcome::AlreadyCommitted(committed)));
    }

    let proven = match authority {
        BuildFence::Gameplay(node) => assert_gameplay_fence(tx, fence, node).await?,
        BuildFence::HouseLogout(house) => house.fenced_root(tx).await?,
    };
    let root = match proven {
        Ok(root) => root,
        Err(error) => return Ok(Err(error)),
    };
    let character = fence.character_id.as_bytes().as_slice();
    let state = sqlx::query(
        "SELECT character_revision::text, level, profile_revision, ruleset_revision, \
                                content_revision \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(state) = state else {
        return Ok(Err(CharacterProgressionError::MissingProgressionState));
    };
    if numeric_u64(&state, "character_revision")? != root.revision {
        return Err(DurabilityError::InvalidStoredState);
    }
    if !state_matches_root(&state, &root)
        || state.try_get::<String, _>("content_revision")? != content_revision
    {
        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid)",
    )
    .bind(character)
    .fetch_one(&mut **tx)
    .await?;
    if pending {
        return Ok(Err(CharacterProgressionError::RespawnPending));
    }
    let stored = sqlx::query(concat!(
        "SELECT vocation, ",
        family_columns!("", ""),
        " FROM game_character_build_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE"
    ))
    .bind(character)
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| DurableBuildState::stored(&row, "vocation", ""))
    .transpose()?
    .unwrap_or_default();
    if stored != request.before {
        return Ok(Err(CharacterProgressionError::BuildStateMismatch));
    }
    if let Some(pruned) = &request.pruned_stance {
        let stance: Option<Option<String>> = sqlx::query_scalar(
            "SELECT stance_key FROM game_character_stance \
                              WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(character)
        .fetch_optional(&mut **tx)
        .await?;
        if stance.flatten().as_ref() != Some(pruned) {
            return Ok(Err(CharacterProgressionError::BuildStateMismatch));
        }
    }

    let offline_before =
        super::offline_training_state::load(tx, fence.character_id.as_bytes()).await?;
    let offline = offline.unwrap_or(super::offline_training_state::OfflineChange {
        before: offline_before,
        after: offline_before,
    });
    if offline.before != offline_before
        || !super::offline_training_state::admits(request.cause, offline)
    {
        return Ok(Err(CharacterProgressionError::BuildStateMismatch));
    }
    let committed_revision = root
        .revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (original, committed) = (root.revision.to_string(), committed_revision.to_string());
    for successor in [
        "UPDATE game_character_roots \
                            SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
        "UPDATE game_character_progression_state \
                            SET character_revision = $2::text::numeric(20,0) \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND character_revision = $3::text::numeric(20,0)",
    ] {
        let updated = sqlx::query(successor)
            .bind(character)
            .bind(&committed)
            .bind(&original)
            .execute(&mut **tx)
            .await?;
        if updated.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let receipt = sqlx::query(concat!(
        "INSERT INTO game_character_build_receipts(\
                           build_occurrence_id, command_binding, policy_digest, character_id, \
                           original_character_revision, committed_character_revision, cause, \
                           level_before, level_after, experience_before, experience_after, \
                           vocation_before, ",
        family_columns!("", "_before"),
        ", vocation_after, ",
        family_columns!("", "_after"),
        ", stance_before, stance_after, profile_revision, ruleset_revision, \
                           content_revision, simulation_revision, evidence_revision, \
                           declaration_revision, policy_revision, reward_revision, committed_at, \
                           stamina_minutes_before, stamina_minutes_after, offline_pool_s_before, offline_pool_s_after, offline_skill_before, offline_skill_after) \
                         SELECT encode($1,'hex')::uuid, $2, $3, s.character_id, \
                           $4::text::numeric(20,0), $5::text::numeric(20,0), $6, s.level, \
                           s.level, s.total_experience, s.total_experience, \
                           $7, ($8::bigint[])[1]::integer, ($8::bigint[])[2], ($8::bigint[])[3]::integer, ($8::bigint[])[4], ($8::bigint[])[5]::integer, ($8::bigint[])[6], ($8::bigint[])[7]::integer, ($8::bigint[])[8], ($8::bigint[])[9]::integer, ($8::bigint[])[10], ($8::bigint[])[11]::integer, ($8::bigint[])[12], ($8::bigint[])[13]::integer, ($8::bigint[])[14], ($8::bigint[])[15]::integer, ($8::bigint[])[16], \
                           $9, ($10::bigint[])[1]::integer, ($10::bigint[])[2], ($10::bigint[])[3]::integer, ($10::bigint[])[4], ($10::bigint[])[5]::integer, ($10::bigint[])[6], ($10::bigint[])[7]::integer, ($10::bigint[])[8], ($10::bigint[])[9]::integer, ($10::bigint[])[10], ($10::bigint[])[11]::integer, ($10::bigint[])[12], ($10::bigint[])[13]::integer, ($10::bigint[])[14], ($10::bigint[])[15]::integer, ($10::bigint[])[16], \
                           $11, NULL, s.profile_revision, s.ruleset_revision, \
                           s.content_revision, s.simulation_revision, s.evidence_revision, \
                           s.declaration_revision, s.policy_revision, s.reward_revision, \
                           floor(extract(epoch FROM statement_timestamp())*1000)::bigint, $13,$14,$15,$16,$17,$18 \
                           FROM game_character_progression_state s \
                          WHERE s.character_id = encode($12,'hex')::uuid"
    ))
    .bind(request.occurrence.0.as_slice())
    .bind(&binding)
    .bind(policy_digest.as_slice())
    .bind(&original)
    .bind(&committed)
    .bind(request.cause.key());
    let receipt = receipt
        .bind(request.before.vocation())
        .bind(request.before.family_values())
        .bind(request.after.vocation())
        .bind(request.after.family_values());
    let receipt = receipt
        .bind(request.pruned_stance.clone())
        .bind(character)
        .bind(i32::from(offline.before.stamina_minutes))
        .bind(i32::from(offline.after.stamina_minutes))
        .bind(
            i32::try_from(offline.before.pool_seconds)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .bind(
            i32::try_from(offline.after.pool_seconds)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .bind(offline.before.skill_code()?)
        .bind(offline.after.skill_code()?)
        .execute(&mut **tx)
        .await?;
    if receipt.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let row = sqlx::query(concat!(
        "INSERT INTO game_character_build_state(character_id, vocation, ",
        family_columns!("", ""),
        ", committed_character_revision, last_build_occurrence_id, offline_pool_s, offline_skill) \
                         VALUES (encode($1,'hex')::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, \
                           $11, $12, $13, $14, $15, $16, $17, $18, $19::text::numeric(20,0), \
                           encode($20,'hex')::uuid, $21,$22) \
                         ON CONFLICT (character_id) DO UPDATE SET (vocation, ",
        family_columns!("", ""),
        ", committed_character_revision, last_build_occurrence_id, offline_pool_s, offline_skill) = \
                           (EXCLUDED.vocation, ",
        family_columns!("EXCLUDED.", ""),
        ", EXCLUDED.committed_character_revision, \
                           EXCLUDED.last_build_occurrence_id, EXCLUDED.offline_pool_s, EXCLUDED.offline_skill)"
    ))
    .bind(character);
    let row = request
        .after
        .bind(row)
        .bind(&committed)
        .bind(request.occurrence.0.as_slice())
        .bind(
            i32::try_from(offline.after.pool_seconds)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .bind(offline.after.skill_code()?)
        .execute(&mut **tx)
        .await?;
    if row.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let level = u32::try_from(state.try_get::<i64, _>("level")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    // Crystal dawnport.lua:97-118 refills only above level 8; up to 8 the maxima do not move.
    if request.cause == BuildCause::VocationChoice
        || (request.cause == BuildCause::VocationTrial && level > 8)
    {
        let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
            request.after.vocation(),
            level,
            oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
            oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
        )
        .ok_or(DurabilityError::InvalidStoredState)?;
        // Same transaction and replay boundary as the build receipt. No position row means
        // admission already starts at maxima (0084); otherwise keep the cell/rest snapshot.
        // Advance the projection order key so pre-choice captures cannot undo the refill.
        sqlx::query(
            "UPDATE game_character_last_positions SET health = $2, mana = $3, \
             character_lease_generation = $4::text::numeric(20,0), write_sequence = write_sequence + 1 \
             WHERE character_id = encode($1,'hex')::uuid \
             AND character_lease_generation <= $4::text::numeric(20,0)"
        ).bind(character)
            .bind(i32::try_from(health).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(i32::try_from(mana).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(fence.character_lease_generation.to_string())
            .execute(&mut **tx).await?;
    }
    if request.cause == BuildCause::Training
        && offline.after.stamina_minutes < offline.before.stamina_minutes
    {
        sqlx::query(
            "SELECT game_hunting_set_stamina(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3)",
        )
        .bind(character)
        .bind(request.occurrence.0.as_slice())
        .bind(i32::from(offline.after.stamina_minutes))
        .execute(&mut **tx)
        .await?;
    }
    if request.cause == BuildCause::RestingStamina
        && offline.after.stamina_minutes > offline.before.stamina_minutes
    {
        sqlx::query(
            "SELECT game_resting_set_stamina(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3)",
        )
        .bind(character)
        .bind(request.occurrence.0.as_slice())
        .bind(i32::from(offline.after.stamina_minutes))
        .execute(&mut **tx)
        .await?;
    }
    if request.cause == BuildCause::OfflineSettlement {
        sqlx::query(
            "SELECT game_training_set_stamina(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3)",
        )
        .bind(character)
        .bind(request.occurrence.0.as_slice())
        .bind(i32::from(offline.after.stamina_minutes))
        .execute(&mut **tx)
        .await?;
    }
    if request.pruned_stance.is_some() {
        let pruned = sqlx::query(
            "UPDATE game_character_stance \
                                SET stance_key = NULL, \
                                    committed_character_revision = $2::text::numeric(20,0), \
                                    last_stance_occurrence_id = encode($3,'hex')::uuid \
                              WHERE character_id = encode($1,'hex')::uuid",
        )
        .bind(character)
        .bind(&committed)
        .bind(request.occurrence.0.as_slice())
        .execute(&mut **tx)
        .await?;
        if pruned.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }

    let committed = CommittedBuildChange {
        occurrence: request.occurrence,
        character_id: fence.character_id,
        original_character_revision: fence.expected_character_revision,
        committed_character_revision: CharacterRevision::new(committed_revision)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        cause: request.cause,
        before: request.before,
        after: request.after,
        pruned_stance: request.pruned_stance,
    };
    Ok(Ok(BuildCommitOutcome::Committed(committed)))
}

fn normalized(formula: &dyn BuildFormula, state: &DurableBuildState) -> bool {
    state
        .families()
        .into_iter()
        .enumerate()
        .all(|(family, (level, progress))| {
            let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
            let required = |level| formula.required(&state.vocation, family, level);
            (floor.saturating_add(1)..=level).all(|level| required(level).is_some())
                && (level >= MAX_BUILD_LEVEL
                    || required(level + 1).is_none_or(|required| progress < required))
        })
}
