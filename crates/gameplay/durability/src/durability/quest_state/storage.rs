use super::*;

impl DurabilityRoot {
    /// Resolve an ambiguous quest commit by its receipt key. This proves only what committed and
    /// never reacquires gameplay authority.
    pub async fn reconcile_character_quest_transition(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        request: QuestTransitionRequest,
    ) -> Result<Option<CommittedQuestTransition>> {
        let binding = request_binding(character, &request);
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_receipt_key(&mut tx, character, &request).await?;
                    let Some(row) = load_receipt(&mut tx, character, &request).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
                        return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                    }
                    let committed = decode_receipt(&row)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(committed)))
                })
            })
            .await?
    }

    /// The admission load (§7, §12.3): tracks, quest states and open obligations of one
    /// Character. It writes nothing. Over `QUESTSTATE0-RL-01`, `-05`, `-07` or `-08`, or with a
    /// stored value out of shape, it fails closed with `Unavailable(InvalidStoredState)`.
    pub async fn read_character_quest_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<QuestStateCopy> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let id = character.as_bytes().as_slice();
                    // One coherent copy: every quest writer and every claim that inserts an
                    // obligation holds `character_root` FOR UPDATE until it commits, so with the
                    // root held FOR SHARE no quest relation of this Character can change between
                    // the reads below.
                    let root = sqlx::query(
                        "SELECT character_revision::text FROM game_character_roots \
                          WHERE character_id = encode($1,'hex')::uuid FOR SHARE",
                    )
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let bound = |limit: usize| i64::try_from(limit + 1).unwrap_or(i64::MAX);
                    let over = |rows: &[sqlx::postgres::PgRow], limit: usize| rows.len() > limit;
                    let tracks = sqlx::query(
                        "SELECT track_key, value FROM game_character_quest_tracks \
                          WHERE character_id = encode($1,'hex')::uuid \
                          ORDER BY track_key LIMIT $2",
                    )
                    .bind(id)
                    .bind(bound(QUESTSTATE0_RL_01.min(QUESTSTATE0_RL_08_TRACKS)))
                    .fetch_all(&mut *tx)
                    .await?;
                    let states = sqlx::query(
                        "SELECT quest_key, pinned_content_revision, definition_hash, \
                                completed_character_revision IS NOT NULL AS completed \
                           FROM game_character_quest_states \
                          WHERE character_id = encode($1,'hex')::uuid \
                          ORDER BY quest_key LIMIT $2",
                    )
                    .bind(id)
                    .bind(bound(QUESTSTATE0_RL_05.min(QUESTSTATE0_RL_08_STATES)))
                    .fetch_all(&mut *tx)
                    .await?;
                    let obligations = sqlx::query(
                        "SELECT claim_game_session_id::text, claim_command_id::text, \
                                transition_key, state \
                           FROM game_character_quest_obligations \
                          WHERE character_id = encode($1,'hex')::uuid \
                            AND state IN ('PENDING', 'WAITING_MIGRATION') \
                          ORDER BY created_at, claim_game_session_id, claim_command_id \
                          LIMIT $2",
                    )
                    .bind(id)
                    .bind(bound(QUESTSTATE0_RL_07))
                    .fetch_all(&mut *tx)
                    .await?;
                    let xp_obligations = sqlx::query(
                        "SELECT reward_occurrence_id::text, amount \
                           FROM game_character_quest_xp_obligations \
                          WHERE character_id = encode($1,'hex')::uuid \
                          ORDER BY created_at, reward_occurrence_id LIMIT $2",
                    )
                    .bind(id)
                    .bind(bound(QUESTGATE0_RL_10))
                    .fetch_all(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    if over(&tracks, QUESTSTATE0_RL_01.min(QUESTSTATE0_RL_08_TRACKS))
                        || over(&states, QUESTSTATE0_RL_05.min(QUESTSTATE0_RL_08_STATES))
                        || over(&obligations, QUESTSTATE0_RL_07)
                        || over(&xp_obligations, QUESTGATE0_RL_10)
                    {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let mut copy = QuestStateCopy {
                        character_revision: root
                            .as_ref()
                            .map(|row| {
                                CharacterRevision::new(numeric_u64(row, "character_revision")?)
                                    .map_err(|_| DurabilityError::InvalidStoredState)
                            })
                            .transpose()?,
                        ..QuestStateCopy::default()
                    };
                    for row in &tracks {
                        copy.tracks
                            .insert(row.try_get("track_key")?, row.try_get("value")?);
                    }
                    oteryn_gameplay_quests::quest::monk_shrines::derive_count(&mut copy.tracks);
                    if copy.tracks.len() > QUESTSTATE0_RL_08_TRACKS {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    for row in &states {
                        copy.states.insert(
                            row.try_get("quest_key")?,
                            QuestStateRecord {
                                pinned_content_revision: row.try_get("pinned_content_revision")?,
                                definition_hash: row
                                    .try_get::<Vec<u8>, _>("definition_hash")?
                                    .try_into()
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                                completed: row.try_get("completed")?,
                            },
                        );
                    }
                    for row in &obligations {
                        copy.obligations.push(QuestObligationRecord {
                            claim: CommandRef::new(
                                GameSessionId::decode(&uuid_text(
                                    row.try_get("claim_game_session_id")?,
                                )?)
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                                CommandId::new(numeric_u64(row, "claim_command_id")?)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                            ),
                            transition_key: row.try_get("transition_key")?,
                            waiting_migration: row.try_get::<String, _>("state")?
                                == "WAITING_MIGRATION",
                        });
                    }
                    for row in &xp_obligations {
                        copy.xp_obligations.push(QuestXpObligation {
                            occurrence: ExperienceRewardOccurrence::from_bytes(uuid_text(
                                row.try_get("reward_occurrence_id")?,
                            )?)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                            amount: row.try_get("amount")?,
                        });
                    }
                    Ok(Ok(copy))
                })
            })
            .await?
    }
}

pub(super) async fn insert_xp_obligation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &QuestTransitionRequest,
    amount: i64,
    pin: &str,
) -> StoredResult<QuestXpObligation> {
    let bytes: Vec<u8> = sqlx::query_scalar(
        "SELECT substring(int8send(\
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint) FROM 3) \
         || substring(uuid_send(gen_random_uuid()) FROM 7)",
    )
    .fetch_one(&mut **tx)
    .await?;
    let mut occurrence: [u8; 16] = bytes
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    occurrence[6] = 0x70 | (occurrence[6] & 0x0f);
    occurrence[8] = 0x80 | (occurrence[8] & 0x3f);
    let occurrence = ExperienceRewardOccurrence::from_bytes(occurrence)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let (cause_id, ordinal) = request.cause.key();
    let written = sqlx::query(
        "INSERT INTO game_character_quest_xp_obligations(reward_occurrence_id, character_id, \
           cause_id, cause_ordinal, transition_key, amount, pinned_content_revision, created_at) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           $4::text::numeric(20,0), $5, $6, $7, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(occurrence.as_bytes().as_slice())
    .bind(character.as_bytes().as_slice())
    .bind(cause_id.as_slice())
    .bind(ordinal.to_string())
    .bind(&request.transition_key)
    .bind(amount)
    .bind(pin)
    .execute(&mut **tx)
    .await?;
    if written.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(QuestXpObligation { occurrence, amount })
}

/// Version byte plus SHA-256 over the request only (§5.1): the Character, the cause (kind, id,
/// ordinal) and the transition key. The fence, the revision and the definition are not inputs,
/// so a retry after a reconnect, a revision move or a content change replays.
pub(super) fn request_binding(
    character: CharacterId,
    request: &QuestTransitionRequest,
) -> [u8; 33] {
    let (id, ordinal) = request.cause.key();
    let mut semantic = b"oteryn:quest-transition:request-binding\0".to_vec();
    semantic.push(REQUEST_BINDING_VERSION);
    semantic.extend_from_slice(character.as_bytes());
    semantic.extend_from_slice(request.cause.kind().as_bytes());
    semantic.push(0);
    semantic.extend_from_slice(&id);
    semantic.extend_from_slice(&ordinal.to_be_bytes());
    // Checked keys cannot contain NUL.
    semantic.extend_from_slice(request.transition_key.as_bytes());
    semantic.push(0);
    let mut binding = [REQUEST_BINDING_VERSION; 33];
    binding[1..].copy_from_slice(&Sha256::digest(&semantic));
    binding
}

pub(super) async fn lock_receipt_key(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &QuestTransitionRequest,
) -> StoredResult<()> {
    let (id, ordinal) = request.cause.key();
    let mut key = character.as_bytes().to_vec();
    key.extend_from_slice(&id);
    key.extend_from_slice(&ordinal.to_be_bytes());
    key.extend_from_slice(request.transition_key.as_bytes());
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:character-quest-receipt:' || encode($1, 'hex'), 0))",
    )
    .bind(key.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(super) async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    request: &QuestTransitionRequest,
) -> StoredResult<Option<sqlx::postgres::PgRow>> {
    let (id, ordinal) = request.cause.key();
    // The receipt and, while pending, the quest XP obligation it wrote (QUEST-GATE-0 §5.5).
    Ok(sqlx::query(
        "SELECT r.character_id::text, r.cause_kind, r.cause_id::text, \
                r.cause_ordinal::text, r.transition_key, r.quest_key, r.request_binding, \
                r.pinned_content_revision, r.definition_hash, r.completes, r.track_keys, \
                r.values_before, r.values_after, r.original_character_revision::text, \
                r.committed_character_revision::text, \
                x.reward_occurrence_id::text AS xp_occurrence, x.amount AS xp_amount \
           FROM game_character_quest_receipts r \
           LEFT JOIN game_character_quest_xp_obligations x \
             ON x.character_id = r.character_id AND x.cause_id = r.cause_id \
            AND x.cause_ordinal = r.cause_ordinal AND x.transition_key = r.transition_key \
          WHERE r.character_id = encode($1,'hex')::uuid \
            AND r.cause_id = encode($2,'hex')::uuid \
            AND r.cause_ordinal = $3::text::numeric(20,0) AND r.transition_key = $4",
    )
    .bind(character.as_bytes().as_slice())
    .bind(id.as_slice())
    .bind(ordinal.to_string())
    .bind(&request.transition_key)
    .fetch_optional(&mut **tx)
    .await?)
}

pub(super) fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> StoredResult<CommittedQuestTransition> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    let keys: Vec<String> = row.try_get("track_keys")?;
    let before: Vec<i64> = row.try_get("values_before")?;
    let after: Vec<i64> = row.try_get("values_after")?;
    if before.len() != keys.len() || after.len() != keys.len() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let original = CharacterRevision::new(numeric_u64(row, "original_character_revision")?)
        .map_err(invalid)?;
    let committed = CharacterRevision::new(numeric_u64(row, "committed_character_revision")?)
        .map_err(invalid)?;
    if original.get().checked_add(1) != Some(committed.get()) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let experience = match (
        row.try_get::<Option<&str>, _>("xp_occurrence")?,
        row.try_get::<Option<i64>, _>("xp_amount")?,
    ) {
        (Some(occurrence), Some(amount)) => Some(QuestXpObligation {
            occurrence: ExperienceRewardOccurrence::from_bytes(uuid_text(occurrence)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            amount,
        }),
        (None, None) => None,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(CommittedQuestTransition {
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(invalid)?,
        cause: QuestCause::stored(
            row.try_get("cause_kind")?,
            uuid_text(row.try_get("cause_id")?)?,
            numeric_u64(row, "cause_ordinal")?,
        )?,
        transition_key: row.try_get("transition_key")?,
        quest_key: row.try_get("quest_key")?,
        original_character_revision: original,
        committed_character_revision: committed,
        pinned_content_revision: row.try_get("pinned_content_revision")?,
        definition_hash: row
            .try_get::<Vec<u8>, _>("definition_hash")?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        completes: row.try_get("completes")?,
        changes: keys
            .into_iter()
            .zip(before)
            .zip(after)
            .map(|((track, before), after)| QuestTrackChange {
                track,
                before,
                after,
            })
            .collect(),
        experience,
    })
}

/// The production session copy (QUEST-STATE-0 §7).
impl oteryn_gameplay_quests::quest::log::QuestLogFacts for QuestStateCopy {
    fn track_value(&self, track: &str) -> Option<i64> {
        self.tracks().get(track).copied()
    }

    fn quest_state(
        &self,
        quest: &str,
    ) -> Option<oteryn_gameplay_quests::quest::log::QuestLogState<'_>> {
        self.states()
            .get(quest)
            .map(|state| oteryn_gameplay_quests::quest::log::QuestLogState {
                pinned_content_revision: &state.pinned_content_revision,
                completed: state.completed,
            })
    }
}
