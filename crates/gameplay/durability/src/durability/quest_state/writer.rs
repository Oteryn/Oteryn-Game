use super::*;

impl DurabilityRoot {
    /// Commit one quest transition (§4-§5). Exact replay of the receipt key returns the receipt
    /// under the current recovery fence without session authority; another cause kind on the
    /// same key conflicts. A new request is fenced like an XP award, refused while a death's
    /// respawn is pending, and validated against the locked values: a refusal writes nothing to
    /// the quest relations or the revision chain. `catalogue` must be the Character's current
    /// content revision; an unknown transition is `InvalidInput`.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_quest_transition(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner(authority, node, fence, request, catalogue, None)
            .await
    }

    /// A content-bound world callback uses the same fenced quest writer and receipt.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_world_callback(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        context: super::super::quest_world_callback::QuestWorldContext,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner(
            authority,
            node,
            fence,
            request,
            catalogue,
            Some((permit, context)),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn commit_quest_inner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        mut context: Option<(
            &super::super::spell_owner_commit::SpellLanePermit,
            super::super::quest_world_callback::QuestWorldContext,
        )>,
    ) -> Result<QuestTransitionOutcome> {
        let callback = if context.is_some() {
            Some(
                oteryn_gameplay_quests::quest::world_callbacks::load()
                    .map_err(|_| CharacterProgressionError::InvalidInput)?
                    .into_iter()
                    .find(|c| c.key == request.transition_key)
                    .ok_or(CharacterProgressionError::InvalidInput)?,
            )
        } else {
            None
        };
        let achievement = super::super::quest_monk_rewards::prepare(callback.as_ref())
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        // Altar transitions cannot be invoked through a weaker generic quest caller.
        if context.is_none()
            && oteryn_gameplay_quests::quest::world_callbacks::load()
                .map_err(|_| CharacterProgressionError::InvalidInput)?
                .iter()
                .any(|c| {
                    c.key == request.transition_key
                        && (!c.offerings.is_empty()
                            || c.item_on_target.is_some()
                            || c.minimum_level.is_some()
                            || c.experience.is_some()
                            || c.monk_reward.is_some())
                })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            || !valid_quest_key(&request.transition_key)
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        // §5.3: a command cause belongs to the fenced GameSession; a still-pending one after a
        // same-GameSession reconnect commits with the current generation, and a replaced
        // session's command is refused. An obligation may come from an earlier session.
        if let QuestCause::Command(command) | QuestCause::Use(command) | QuestCause::Step(command) =
            request.cause
            && command.game_session_id() != fence.game_session_id
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let mut binding = request_binding(fence.character_id, &request);
        if let Some((_, context)) = &context {
            let bytes = serde_json::to_vec(&(context, &callback))
                .map_err(|_| CharacterProgressionError::InvalidInput)?;
            let digest = Sha256::new()
                .chain_update(binding)
                .chain_update(bytes)
                .finalize();
            binding[1..].copy_from_slice(&digest);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let character = fence.character_id;
                    lock_receipt_key(&mut tx, character, &request).await?;
                    if let Some(row) = load_receipt(&mut tx, character, &request).await? {
                        if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(QuestTransitionOutcome::AlreadyCommitted(committed)));
                    }
                    let Some(transition) = catalogue.transition(&request.transition_key) else {
                        return Ok(Err(CharacterProgressionError::InvalidInput));
                    };
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let id = character.as_bytes().as_slice();
                    let state = sqlx::query(
                        "SELECT character_revision::text, profile_revision, ruleset_revision, \
                                content_revision \
                           FROM game_character_progression_state \
                          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
                    )
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(state) = state else {
                        return Ok(Err(CharacterProgressionError::MissingProgressionState));
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let content_revision: String = state.try_get("content_revision")?;
                    if !state_matches_root(&state, &root)
                        || content_revision != catalogue.content_revision()
                    {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    let pending: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_character_pending_respawns \
                          WHERE character_id = encode($1,'hex')::uuid)",
                    )
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?;
                    if pending {
                        return Ok(Err(CharacterProgressionError::RespawnPending));
                    }
                    let obligation = match request.cause {
                        QuestCause::ClaimObligation(claim) => Some(claim),
                        _ => None,
                    };
                    if let Some(claim) = obligation {
                        let row = sqlx::query(
                            "SELECT character_id::text, transition_key, state \
                               FROM game_character_quest_obligations \
                              WHERE claim_game_session_id = encode($1,'hex')::uuid \
                                AND claim_command_id = $2::text::numeric(20,0) FOR UPDATE",
                        )
                        .bind(claim.game_session_id().as_bytes().as_slice())
                        .bind(claim.command_id().get().to_string())
                        .fetch_optional(&mut *tx)
                        .await?;
                        let Some(row) = row else {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        };
                        if uuid_text(row.try_get("character_id")?)? != *character.as_bytes()
                            || row.try_get::<String, _>("transition_key")?
                                != request.transition_key
                        {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        }
                        if row.try_get::<String, _>("state")? != "PENDING" {
                            return Ok(Ok(QuestTransitionOutcome::ObligationClosed));
                        }
                    }

                    let quest = transition.quest.as_str();
                    let current_hash = catalogue
                        .definition_hash(quest)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let stored_state = sqlx::query(
                        "SELECT pinned_content_revision, definition_hash, \
                                completed_character_revision IS NOT NULL AS completed \
                           FROM game_character_quest_states \
                          WHERE character_id = encode($1,'hex')::uuid AND quest_key = $2 \
                          FOR UPDATE",
                    )
                    .bind(id)
                    .bind(quest)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let (pin, hash, refusal) = match &stored_state {
                        Some(row) => {
                            let hash: [u8; 32] = row
                                .try_get::<Vec<u8>, _>("definition_hash")?
                                .try_into()
                                .map_err(|_| DurabilityError::InvalidStoredState)?;
                            // §6: a completed quest keeps its pin and no longer blocks.
                            let blocked =
                                hash != current_hash && !row.try_get::<bool, _>("completed")?;
                            (
                                row.try_get::<String, _>("pinned_content_revision")?,
                                hash,
                                blocked.then_some(QuestRefusal::RevisionMismatch),
                            )
                        }
                        None => {
                            let states: i64 = sqlx::query_scalar(
                                "SELECT count(*) FROM game_character_quest_states \
                                  WHERE character_id = encode($1,'hex')::uuid",
                            )
                            .bind(id)
                            .fetch_one(&mut *tx)
                            .await?;
                            let full = usize::try_from(states).unwrap_or(usize::MAX)
                                >= QUESTSTATE0_RL_05;
                            (
                                content_revision.clone(),
                                current_hash,
                                full.then_some(QuestRefusal::CapacityExceeded),
                            )
                        }
                    };
                    let final_flags: Vec<String> = if request.transition_key == oteryn_gameplay_quests::quest::monk_shrines::FINISH {
                        oteryn_gameplay_quests::quest::monk_shrines::NAMES.iter().map(|name|
                            format!("{}{name}", oteryn_gameplay_quests::quest::monk_shrines::FLAG_PREFIX)).collect()
                    } else { Vec::new() };
                    let mut keys: Vec<&str> = transition
                        .effects
                        .iter()
                        .map(|effect| effect.track.as_str())
                        .collect();
                    keys.extend(final_flags.iter().map(String::as_str));
                    keys.sort_unstable(); keys.dedup();
                    let rows = sqlx::query(
                        "SELECT track_key, quest_key, value FROM game_character_quest_tracks \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND track_key = ANY($2::text[]) \
                          ORDER BY track_key FOR UPDATE",
                    )
                    .bind(id)
                    .bind(&keys)
                    .fetch_all(&mut *tx)
                    .await?;
                    let mut stored = BTreeMap::new();
                    for row in &rows {
                        if row.try_get::<String, _>("quest_key")? != quest {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        stored.insert(row.try_get::<String, _>("track_key")?, row.try_get("value")?);
                    }
                    let refusal = refusal.or_else(|| {
                        (!final_flags.is_empty() && oteryn_gameplay_quests::quest::monk_shrines::completed_count(&stored) != 10)
                            .then_some(QuestRefusal::StageMismatch)
                    });
                    let refusal = match refusal {
                        Some(refusal) => Some(refusal),
                        None if rows.len() < keys.len() => {
                            let (tracks, has_counter, has_flags): (i64, bool, bool) = sqlx::query_as(
                                "SELECT count(*), coalesce(bool_or(track_key=$2),false), \
                                  coalesce(bool_or(left(track_key,length($3))=$3),false) \
                                  FROM game_character_quest_tracks \
                                  WHERE character_id = encode($1,'hex')::uuid",
                            )
                            .bind(id)
                            .bind(oteryn_gameplay_quests::quest::monk_shrines::COUNT_TRACK)
                            .bind(oteryn_gameplay_quests::quest::monk_shrines::FLAG_PREFIX)
                            .fetch_one(&mut *tx)
                            .await?;
                            // A flag-only character needs one additional counter in its
                            // read projection. Reserve it before committing any new row.
                            let has_counter = has_counter || keys.contains(&oteryn_gameplay_quests::quest::monk_shrines::COUNT_TRACK);
                            let has_flags = has_flags || keys.iter().any(|key| key.starts_with(oteryn_gameplay_quests::quest::monk_shrines::FLAG_PREFIX));
                            let after = usize::try_from(tracks)
                                .unwrap_or(usize::MAX)
                                .saturating_add(keys.len() - rows.len())
                                .saturating_add(usize::from(has_flags && !has_counter));
                            (after > QUESTSTATE0_RL_01.min(QUESTSTATE0_RL_08_TRACKS))
                                .then_some(QuestRefusal::CapacityExceeded)
                        }
                        None => None,
                    };
                    // QUEST-GATE-0 §5.5: the already-held character_root lock serializes all
                    // inserts and consumption of this Character's XP obligations. Count them
                    // before any write; FOR UPDATE here would require the UPDATE privilege
                    // intentionally absent from the runtime role. At RL-10 refuse whole.
                    let xp_full = match (refusal, transition.experience) {
                        (None, Some(_)) => {
                            let pending = sqlx::query(
                                "SELECT reward_occurrence_id::text \
                                   FROM game_character_quest_xp_obligations \
                                  WHERE character_id = encode($1,'hex')::uuid \
                                  ORDER BY reward_occurrence_id",
                            )
                            .bind(id)
                            .fetch_all(&mut *tx)
                            .await?;
                            pending.len() >= QUESTGATE0_RL_10
                        }
                        _ => false,
                    };
                    let now: i64 =
                        sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
                            .fetch_one(&mut *tx)
                            .await?;
                    let changes = match refusal
                        .map_or_else(|| catalogue.evaluate(transition, &stored, now), Err)
                        .and_then(|changes| {
                            if xp_full {
                                Err(QuestRefusal::OutOfRange)
                            } else {
                                Ok(changes)
                            }
                        }) {
                        Ok(changes) => changes,
                        Err(refusal) => {
                            let Some(claim) = obligation else {
                                // Dropping the transaction rolls back: nothing is written.
                                if context.is_some() {
                                    tx.rollback().await?;
                                }
                                return Ok(Ok(QuestTransitionOutcome::Refused(refusal)));
                            };
                            let next = if refusal == QuestRefusal::RevisionMismatch {
                                "WAITING_MIGRATION"
                            } else {
                                "REFUSED"
                            };
                            let updated = sqlx::query(
                                "UPDATE game_character_quest_obligations \
                                    SET state = $3, result_code = $4, \
                                        updated_at = greatest(updated_at, \
                                          floor(extract(epoch FROM statement_timestamp())*1000)::bigint) \
                                  WHERE claim_game_session_id = encode($1,'hex')::uuid \
                                    AND claim_command_id = $2::text::numeric(20,0) \
                                    AND state = 'PENDING'",
                            )
                            .bind(claim.game_session_id().as_bytes().as_slice())
                            .bind(claim.command_id().get().to_string())
                            .bind(next)
                            .bind(refusal.code())
                            .execute(&mut *tx)
                            .await?;
                            if updated.rows_affected() != 1 {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            commit_semantic_transaction(tx, deadline).await?;
                            return Ok(Ok(QuestTransitionOutcome::Refused(refusal)));
                        }
                    };

                    if let (Some(callback), Some((permit, context))) = (&callback, &context)
                        && !super::super::quest_world_callback::consume(&mut tx, permit, fence, &request, callback, context).await?
                    { return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))); }
                    let committed_revision = root
                        .revision
                        .checked_add(1)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let (original, committed) =
                        (root.revision.to_string(), committed_revision.to_string());
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
                            .bind(id)
                            .bind(&committed)
                            .bind(&original)
                            .execute(&mut *tx)
                            .await?;
                        if updated.rows_affected() != 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    let (cause_id, ordinal) = request.cause.key();
                    let track_keys: Vec<&str> =
                        changes.iter().map(|change| change.track.as_str()).collect();
                    let before: Vec<i64> = changes.iter().map(|change| change.before).collect();
                    let after: Vec<i64> = changes.iter().map(|change| change.after).collect();
                    let receipt = sqlx::query(
                        "INSERT INTO game_character_quest_receipts(\
                           character_id, cause_kind, cause_id, cause_ordinal, transition_key, \
                           quest_key, request_binding, pinned_content_revision, definition_hash, \
                           completes, track_keys, values_before, values_after, \
                           original_character_revision, committed_character_revision, \
                           level_before, level_after, experience_before, experience_after, \
                           profile_revision, ruleset_revision, content_revision, \
                           simulation_revision, evidence_revision, declaration_revision, \
                           policy_revision, reward_revision, committed_at) \
                         SELECT s.character_id, $2, encode($3,'hex')::uuid, \
                           $4::text::numeric(20,0), $5, $6, $7, $8, $9, $10, $11::text[], \
                           $12::bigint[], $13::bigint[], $14::text::numeric(20,0), \
                           $15::text::numeric(20,0), s.level, s.level, s.total_experience, \
                           s.total_experience, s.profile_revision, s.ruleset_revision, \
                           s.content_revision, s.simulation_revision, s.evidence_revision, \
                           s.declaration_revision, s.policy_revision, s.reward_revision, \
                           floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                           FROM game_character_progression_state s \
                          WHERE s.character_id = encode($1,'hex')::uuid",
                    )
                    .bind(id)
                    .bind(request.cause.kind())
                    .bind(cause_id.as_slice())
                    .bind(ordinal.to_string())
                    .bind(&request.transition_key)
                    .bind(quest)
                    .bind(binding.as_slice())
                    .bind(&pin)
                    .bind(hash.as_slice())
                    .bind(transition.completes)
                    .bind(&track_keys)
                    .bind(&before)
                    .bind(&after)
                    .bind(&original)
                    .bind(&committed)
                    .execute(&mut *tx)
                    .await?;
                    if receipt.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    for change in &changes {
                        let written = sqlx::query(
                            "INSERT INTO game_character_quest_tracks(character_id, track_key, \
                               quest_key, value, committed_character_revision) \
                             VALUES (encode($1,'hex')::uuid, $2, $3, $4, \
                               $5::text::numeric(20,0)) \
                             ON CONFLICT (character_id, track_key) DO UPDATE \
                               SET value = EXCLUDED.value, \
                                   committed_character_revision = \
                                     EXCLUDED.committed_character_revision",
                        )
                        .bind(id)
                        .bind(&change.track)
                        .bind(quest)
                        .bind(change.after)
                        .bind(&committed)
                        .execute(&mut *tx)
                        .await?;
                        if written.rows_affected() != 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    let completed = transition.completes.then_some(committed.as_str());
                    let written = if stored_state.is_some() {
                        sqlx::query(
                            "UPDATE game_character_quest_states \
                                SET committed_character_revision = $3::text::numeric(20,0), \
                                    completed_character_revision = coalesce(\
                                      completed_character_revision, $4::text::numeric(20,0)) \
                              WHERE character_id = encode($1,'hex')::uuid AND quest_key = $2",
                        )
                        .bind(id)
                        .bind(quest)
                        .bind(&committed)
                        .bind(completed)
                        .execute(&mut *tx)
                        .await?
                    } else {
                        sqlx::query(
                            "INSERT INTO game_character_quest_states(character_id, quest_key, \
                               pinned_content_revision, definition_hash, \
                               started_character_revision, completed_character_revision, \
                               committed_character_revision) \
                             VALUES (encode($1,'hex')::uuid, $2, $3, $4, \
                               $5::text::numeric(20,0), $6::text::numeric(20,0), \
                               $5::text::numeric(20,0))",
                        )
                        .bind(id)
                        .bind(quest)
                        .bind(&pin)
                        .bind(hash.as_slice())
                        .bind(&committed)
                        .bind(completed)
                        .execute(&mut *tx)
                        .await?
                    };
                    if written.rows_affected() != 1 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let experience = match transition.experience {
                        Some(amount) => Some(
                            insert_xp_obligation(
                                &mut tx,
                                character,
                                &request,
                                amount,
                                &pin,
                            )
                            .await?,
                        ),
                        None => None,
                    };
                    if let Some(claim) = obligation {
                        let consumed = sqlx::query(
                            "UPDATE game_character_quest_obligations \
                                SET state = 'CONSUMED', \
                                    updated_at = greatest(updated_at, \
                                      floor(extract(epoch FROM statement_timestamp())*1000)::bigint) \
                              WHERE claim_game_session_id = encode($1,'hex')::uuid \
                                AND claim_command_id = $2::text::numeric(20,0) \
                                AND state = 'PENDING'",
                        )
                        .bind(claim.game_session_id().as_bytes().as_slice())
                        .bind(claim.command_id().get().to_string())
                        .execute(&mut *tx)
                        .await?;
                        if consumed.rows_affected() != 1 {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                    }
                    if let (Some(callback), Some((_, context))) = (&callback, &context) {
                        super::super::quest_monk_rewards::earn(&mut tx, fence, &request, callback,
                            &context.content_revision, achievement.as_ref()).await?;
                    }
                    super::super::quest_newhaven_rewards::earn(&mut tx, fence, &request, &pin).await?;
                    let committed = CommittedQuestTransition {
                        character_id: character,
                        cause: request.cause,
                        transition_key: request.transition_key,
                        quest_key: quest.to_owned(),
                        original_character_revision: fence.expected_character_revision,
                        committed_character_revision: CharacterRevision::new(committed_revision)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        pinned_content_revision: pin,
                        definition_hash: hash,
                        completes: transition.completes,
                        changes,
                        experience,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(QuestTransitionOutcome::Committed(committed)))
                })
            })
            .await?
    }
}
