impl DurabilityRoot {
    /// Commit one build change. Exact occurrence replay returns the retained receipt without
    /// reacquiring session authority; the same occurrence with another binding conflicts, and the
    /// binding is compared before replay. A new occurrence is fenced like an XP award, must start
    /// from the stored build (and stance, for a prune), and is refused while a death's respawn is
    /// pending. `after` must hold its progress within its levels under `formula`, and a vocation
    /// choice must be exactly the writer's own conversion of `before` (SKILLS-0 §3.3).
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_build(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: BuildChangeRequest,
        formula: &dyn BuildFormula,
    ) -> Result<BuildCommitOutcome> {
        self.commit_character_build_with_kit(authority, node, fence, request, formula, None)
            .await
    }

    pub(super) async fn commit_character_build_with_kit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: BuildChangeRequest,
        formula: &dyn BuildFormula,
        mut kit: Option<super::starter_grant::NewhavenKit<'_>>,
    ) -> Result<BuildCommitOutcome> {
        if kit.is_some()
            && (request.cause != BuildCause::VocationChoice
                || request.before.vocation() != NO_VOCATION)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if fence.character_lease_generation == 0
            || !request.cause.admits_runtime_scope(fence.runtime_scope)
            || !request.cause.admits(&request.before, &request.after)
            || !normalized(formula, &request.after)
            || request.pruned_stance.as_deref().is_some_and(|stance| {
                request.cause == BuildCause::Training || !valid_revision(stance)
            })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if request.cause.converts()
            && convert_vocation(formula, &request.before, &request.after.vocation)? != request.after
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let policy_digest = formula.digest();
        let content_revision = formula.content_revision().to_owned();
        let binding = command_binding(&fence, &request, &policy_digest);
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run_with_context(&mut kit, move |holder, deadline, kit| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;

                    let vocation = request.after.vocation().to_owned();
                    let outcome = apply_build_in_transaction(
                        &mut tx,
                        &node,
                        &fence,
                        request,
                        policy_digest,
                        content_revision,
                        binding,
                        None,
                    )
                    .await?;
                    if outcome.is_ok() {
                        if let Some(kit) = kit
                            && super::starter_grant::grant_newhaven_in_transaction(
                                &mut tx,
                                &recovery,
                                &node,
                                &fence.item_fence(),
                                &vocation,
                                kit,
                            )
                            .await?
                                == super::starter_grant::NewhavenStarterOutcome::NotAdmitted
                        {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        }
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(outcome)
                })
            })
            .await?
    }

    /// Resolve an ambiguous build commit by its occurrence (A13 §4.2). This proves only what
    /// committed and never reacquires gameplay authority.
    pub async fn reconcile_character_build(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        occurrence: BuildOccurrence,
    ) -> Result<Option<CommittedBuildChange>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let record = load_receipt(&mut tx, occurrence)
                        .await?
                        .as_ref()
                        .map(decode_receipt)
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(record))
                })
            })
            .await?
    }

    /// Reads this session's trained build and Platform account loyalty in one fenced
    /// transaction. Neither the skill owner nor the account projection is mutated.
    pub async fn read_loyalty_build_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &super::item_transfer::CurrentCharacterItemFence,
        command: crate::foundation::CommandRef,
        content_digest: [u8; 32],
    ) -> Result<(DurableBuildState, u32)> {
        let (build, points, _) = self
            .read_skill_display_state(authority, node, fence, command, content_digest)
            .await?;
        Ok((build, points))
    }

    /// NX-7: level/XP, trained build and account loyalty from one fenced transaction.
    pub async fn read_skill_display_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &super::item_transfer::CurrentCharacterItemFence,
        command: crate::foundation::CommandRef,
        content_digest: [u8; 32],
    ) -> Result<(DurableBuildState, u32, Option<SkillDisplayProgression>)> {
        if content_digest == [0; 32] {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let fence = *fence;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let current = match fence.runtime_scope {
                        RuntimeScopeRefV1::Channel { world_id, channel_id } => {
                            // Same lock order as the existing equipment read; activation
                            // cannot advance between its proof and the projection read.
                            sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea, 'hex')::uuid::text || ':' || encode($2::bytea, 'hex')::uuid::text, 0))")
                                .bind(world_id.as_bytes().as_slice())
                                .bind(channel_id.as_bytes().as_slice())
                                .execute(&mut *tx).await?;
                            if !super::item_transfer::character_item_fence_for_scope_is_current(
                                &mut tx, &node, &fence, command, fence.character_id, fence.runtime_scope,
                            ).await? {
                                return Ok(Err(CharacterProgressionError::AuthorityRejected));
                            }
                            let digest: Option<Vec<u8>> = sqlx::query_scalar(
                                "SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1",
                            ).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
                                .fetch_optional(&mut *tx).await?;
                            digest.as_deref() == Some(content_digest.as_slice())
                        }
                        RuntimeScopeRefV1::Instance { .. } => {
                            // Private activities use their own assignment and immutable
                            // content binding; an origin Channel never grants authority.
                            super::item_transfer::assert_activity_item_content(
                                &mut tx, &node, &fence, command, content_digest,
                            ).await?
                        }
                    };
                    if !current {
                        return Ok(Err(CharacterProgressionError::AuthorityRejected));
                    }
                    let row = sqlx::query(concat!(
                        "SELECT b.vocation, ", family_columns!("b.", ""),
                        ", COALESCE(l.loyalty_points,0)::bigint AS loyalty_points, p.level, p.total_experience, p.profile_revision, p.ruleset_revision, p.content_revision ",
                        "FROM game_character_roots r ",
                        "LEFT JOIN game_character_build_state b USING(character_id) ",
                        "LEFT JOIN game_account_loyalty_projections l ON l.account_id=r.account_id ",
                        "LEFT JOIN game_character_progression_state p USING(character_id) ",
                        "WHERE r.character_id=encode($1,'hex')::uuid"
                    )).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    let build = if row.try_get::<Option<String>, _>("vocation")?.is_some() {
                        DurableBuildState::stored(&row, "vocation", "")?
                    } else {
                        DurableBuildState::default()
                    };
                    let points = u32::try_from(row.try_get::<i64, _>("loyalty_points")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let progression = row.try_get::<Option<i64>, _>("level")?.map(|level| {
                        Ok::<_, DurabilityError>(SkillDisplayProgression {
                            level: u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?,
                            experience: u64::try_from(row.try_get::<i64, _>("total_experience")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                            profile_revision: row.try_get("profile_revision")?,
                            ruleset_revision: row.try_get("ruleset_revision")?,
                            content_revision: row.try_get("content_revision")?,
                        })
                    }).transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok((build, points, progression)))
                })
            })
            .await?
    }

    /// The admission load (A13 §4.1): the build row a new runtime actor starts from, or the seed
    /// when the Character has none. It writes nothing. A stored value outside the bounds fails
    /// closed with `Unavailable(InvalidStoredState)`.
    pub async fn read_character_build_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<DurableBuildState> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let state = sqlx::query(concat!(
                        "SELECT vocation, ",
                        family_columns!("", ""),
                        " FROM game_character_build_state \
                          WHERE character_id = encode($1,'hex')::uuid"
                    ))
                    .bind(character_id.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?
                    .map(|row| DurableBuildState::stored(&row, "vocation", ""))
                    .transpose()?
                    .unwrap_or_default();
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(state))
                })
            })
            .await?
    }
}

/// The complete intent (A13 §4.2 "Occurrence key"): character, expected revision, occurrence,
/// cause, before and after values, the stance fields and the formula table digest. Retry-local
/// authority (session, connection, lease and scope generations) is excluded so a replay after a
/// lost response matches.
fn command_binding(
    fence: &CurrentCharacterGameplayFence,
    request: &BuildChangeRequest,
    policy_digest: &[u8; 32],
) -> Vec<u8> {
    let mut semantic = vec![COMMAND_BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(fence.character_id.as_bytes());
    semantic.extend_from_slice(&fence.expected_character_revision.get().to_be_bytes());
    semantic.extend_from_slice(request.cause.key().as_bytes());
    semantic.push(0);
    request.before.encode(&mut semantic);
    request.after.encode(&mut semantic);
    match &request.pruned_stance {
        None => semantic.push(0),
        Some(stance) => {
            semantic.push(1);
            semantic.push(u8::try_from(stance.len()).unwrap_or(u8::MAX));
            semantic.extend_from_slice(stance.as_bytes());
        }
    }
    semantic.extend_from_slice(policy_digest);
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(COMMAND_BINDING_VERSION);
    binding.extend_from_slice(&digest);
    binding
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    occurrence: BuildOccurrence,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(concat!(
        "SELECT build_occurrence_id::text, command_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, cause, \
                vocation_before, ",
        "ARRAY[",
        family_columns!("", "_before"),
        "]::bigint[] AS families_before",
        ", vocation_after, ",
        "ARRAY[",
        family_columns!("", "_after"),
        "]::bigint[] AS families_after",
        ", stance_before FROM game_character_build_receipts \
          WHERE build_occurrence_id = encode($1,'hex')::uuid"
    ))
    .bind(occurrence.0.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedBuildChange, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    Ok(CommittedBuildChange {
        occurrence: BuildOccurrence(uuid_text(row.try_get("build_occurrence_id")?)?),
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(invalid)?,
        original_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "original_character_revision",
        )?)
        .map_err(invalid)?,
        committed_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "committed_character_revision",
        )?)
        .map_err(invalid)?,
        cause: BuildCause::parse(row.try_get("cause")?)
            .ok_or(DurabilityError::InvalidStoredState)?,
        before: DurableBuildState::stored_array(row, "vocation_before", "families_before")?
            .ok_or(DurabilityError::InvalidStoredState)?,
        after: DurableBuildState::stored_array(row, "vocation_after", "families_after")?
            .ok_or(DurabilityError::InvalidStoredState)?,
        pruned_stance: row.try_get("stance_before")?,
    })
}
