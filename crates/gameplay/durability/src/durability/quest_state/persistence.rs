//! Shared persistence for a plan already evaluated by the mature Quest owner.
//! Callers retain Character/root locks, definition-pin checks and storage capacity admission.
use super::{CharacterId, DurabilityError, QuestTrackChange, QuestTransitionRequest};
use sqlx::PgConnection;
pub(super) struct EvaluatedWrite<'a> {
    pub character: CharacterId,
    pub request: &'a QuestTransitionRequest,
    pub quest: &'a str,
    pub binding: &'a [u8],
    pub pin: &'a str,
    pub hash: &'a [u8; 32],
    pub completes: bool,
    pub changes: &'a [QuestTrackChange],
    pub original_revision: u64,
    pub committed_revision: u64,
    pub state_exists: bool,
}
pub(super) async fn write_evaluated(
    tx: &mut PgConnection,
    input: &EvaluatedWrite<'_>,
) -> Result<(), DurabilityError> {
    let EvaluatedWrite {
        character,
        request,
        quest,
        binding,
        pin,
        hash,
        completes,
        changes,
        original_revision,
        committed_revision,
        state_exists,
    } = input;
    let character = *character;
    let completes = *completes;
    let state_exists = *state_exists;
    let id = character.as_bytes().as_slice();
    let original = original_revision.to_string();
    let committed = committed_revision.to_string();
    let (cause_id, ordinal) = request.cause.key();
    let track_keys: Vec<&str> = changes.iter().map(|change| change.track.as_str()).collect();
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
    .bind(*binding)
    .bind(pin)
    .bind(hash.as_slice())
    .bind(completes)
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
    write_state(
        tx,
        &EvaluatedStateWrite {
            character,
            quest,
            pin,
            hash,
            completes,
            changes,
            committed_revision: *committed_revision,
            state_exists,
        },
    )
    .await
}

/// Called only after a mature Quest receipt or exact current Source-NPC child
/// receipt has been admitted. The same immutable definition pin and storage
/// capacity proofs are required by either owner before this function.
pub(super) struct EvaluatedStateWrite<'a> {
    pub character: CharacterId,
    pub quest: &'a str,
    pub pin: &'a str,
    pub hash: &'a [u8; 32],
    pub completes: bool,
    pub changes: &'a [QuestTrackChange],
    pub committed_revision: u64,
    pub state_exists: bool,
}
pub(super) async fn write_state(
    tx: &mut PgConnection,
    input: &EvaluatedStateWrite<'_>,
) -> Result<(), DurabilityError> {
    let EvaluatedStateWrite {
        character,
        quest,
        pin,
        hash,
        completes,
        changes,
        committed_revision,
        state_exists,
    } = input;
    let id = character.as_bytes().as_slice();
    let committed = committed_revision.to_string();
    let completes = *completes;
    let state_exists = *state_exists;
    write_tracks(tx, *character, quest, changes, *committed_revision).await?;
    let completed = completes.then_some(committed.as_str());
    let written = if state_exists {
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
        .bind(pin)
        .bind(hash.as_slice())
        .bind(&committed)
        .bind(completed)
        .execute(&mut *tx)
        .await?
    };
    if written.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

/// Shared bounded physical track write; the caller's mature receipt or exact
/// source child independently proves owner, before-values and next revision.
pub(super) async fn write_tracks(
    tx: &mut PgConnection,
    character: CharacterId,
    quest: &str,
    changes: &[QuestTrackChange],
    committed_revision: u64,
) -> Result<(), DurabilityError> {
    let id = character.as_bytes().as_slice();
    let committed = committed_revision.to_string();
    for change in changes {
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
    Ok(())
}
