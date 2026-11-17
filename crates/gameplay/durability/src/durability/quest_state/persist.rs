//! Shared persistence for already validated Quest transitions. Eligibility and
//! transaction ownership remain with the calling normal or native writer.
use super::*;

pub(super) struct QuestWrite<'a> {
    pub character: CharacterId,
    pub root_revision: u64,
    pub request: &'a QuestTransitionRequest,
    pub transition: &'a oteryn_gameplay_quests::quest::QuestTransition,
    pub pin: &'a str,
    pub hash: &'a [u8; 32],
    pub changes: &'a [QuestTrackChange],
    pub binding: &'a [u8; 33],
    pub state_exists: bool,
}

pub(super) async fn persist(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    write: QuestWrite<'_>,
) -> StoredResult<u64> {
    let character = write.character;
    let id = character.as_bytes().as_slice();
    let committed_revision = write
        .root_revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let (original, committed) = (
        write.root_revision.to_string(),
        committed_revision.to_string(),
    );
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
            .execute(&mut **tx)
            .await?;
        if updated.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let (cause_id, ordinal) = write.request.cause.key();
    let track_keys: Vec<&str> = write
        .changes
        .iter()
        .map(|change| change.track.as_str())
        .collect();
    let before: Vec<i64> = write.changes.iter().map(|change| change.before).collect();
    let after: Vec<i64> = write.changes.iter().map(|change| change.after).collect();
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
    .bind(write.request.cause.kind())
    .bind(cause_id.as_slice())
    .bind(ordinal.to_string())
    .bind(&write.request.transition_key)
    .bind(write.transition.quest.as_str())
    .bind(write.binding.as_slice())
    .bind(write.pin)
    .bind(write.hash.as_slice())
    .bind(write.transition.completes)
    .bind(&track_keys)
    .bind(&before)
    .bind(&after)
    .bind(&original)
    .bind(&committed)
    .execute(&mut **tx)
    .await?;
    if receipt.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    for change in write.changes {
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
        .bind(write.transition.quest.as_str())
        .bind(change.after)
        .bind(&committed)
        .execute(&mut **tx)
        .await?;
        if written.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let completed = write.transition.completes.then_some(committed.as_str());
    let written = if write.state_exists {
        sqlx::query(
            "UPDATE game_character_quest_states \
            SET committed_character_revision = $3::text::numeric(20,0), \
                completed_character_revision = coalesce(\
                  completed_character_revision, $4::text::numeric(20,0)) \
          WHERE character_id = encode($1,'hex')::uuid AND quest_key = $2",
        )
        .bind(id)
        .bind(write.transition.quest.as_str())
        .bind(&committed)
        .bind(completed)
        .execute(&mut **tx)
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
        .bind(write.transition.quest.as_str())
        .bind(write.pin)
        .bind(write.hash.as_slice())
        .bind(&committed)
        .bind(completed)
        .execute(&mut **tx)
        .await?
    };
    if written.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(committed_revision)
}
