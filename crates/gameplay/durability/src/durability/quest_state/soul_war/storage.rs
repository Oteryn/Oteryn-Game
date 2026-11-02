use super::*;
use super::super::super::character_progression::{
    FencedCharacterRoot, numeric_u64, state_matches_root,
};
use crate::domain::CharacterId;
use sqlx::{Postgres, Row, Transaction};

pub(super) async fn database_time(tx: &mut Transaction<'_, Postgres>) -> StoredResult<i64> {
    Ok(sqlx::query_scalar("SELECT floor(extract(epoch FROM transaction_timestamp()))::bigint")
        .fetch_one(&mut **tx).await?)
}

pub(super) async fn load(tx: &mut Transaction<'_, Postgres>, character: CharacterId) -> StoredResult<SoulWarState> {
    let row = sqlx::query("SELECT teleport_access, defeated_bosses, first_taint_at, megalomania_completed, final_reward_claimed FROM game_character_soul_war WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(SoulWarState::default()); };
    let state = SoulWarState {
        teleport_access: row.try_get("teleport_access")?,
        defeated_bosses: u8::try_from(row.try_get::<i16, _>("defeated_bosses")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        first_taint_at: row.try_get("first_taint_at")?,
        megalomania_completed: row.try_get("megalomania_completed")?,
        final_reward_claimed: row.try_get("final_reward_claimed")?,
    };
    state.validate().map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(state)
}

pub(super) fn binding(transition: SoulWarTransition) -> StoredResult<Vec<u8>> {
    serde_json::to_vec(&(1_u8, transition)).map_err(|_| DurabilityError::InvalidStoredState)
}

pub(super) async fn lock_occurrence(tx: &mut Transaction<'_, Postgres>, character: CharacterId, occurrence: &[u8; 32]) -> StoredResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:soul-war:' || encode($1,'hex') || encode($2,'hex'),0))")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_slice())
        .execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn replay(
    tx: &mut Transaction<'_, Postgres>, character: CharacterId,
    occurrence: &[u8; 32], binding: &[u8],
) -> StoredResult<Result<Option<SoulWarCommandOutcome>>> {
    let row = sqlx::query("SELECT request_binding,result,coalesce(committed_character_revision,original_character_revision)::text AS character_revision FROM game_character_soul_war_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence=$2")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_slice())
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(Ok(None)); };
    if row.try_get::<Vec<u8>, _>("request_binding")? != binding {
        return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
    }
    let (state, result): (SoulWarState, std::result::Result<(), SoulWarRefusal>) =
        serde_json::from_slice(&row.try_get::<Vec<u8>, _>("result")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    state.validate().map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Ok(Some(SoulWarCommandOutcome {
        state, result,
        character_revision: CharacterRevision::new(numeric_u64(&row, "character_revision")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        replayed: true,
    })))
}

/// Caller holds the current session fence, Character root and occurrence lock.
/// A death source is required for boss gains; NPC transitions cannot manufacture it.
pub(super) async fn commit_locked(
    tx: &mut Transaction<'_, Postgres>, character: CharacterId,
    root: &FencedCharacterRoot, occurrence: &[u8; 32],
    transition: SoulWarTransition, source_death: Option<&[u8; 52]>, source_attempt: Option<&[u8; 32]>,
) -> StoredResult<SoulWarCommandOutcome> {
    let id = character.as_bytes().as_slice();
    let state = sqlx::query("SELECT character_revision::text,level,profile_revision,ruleset_revision,content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(id).fetch_one(&mut **tx).await?;
    if numeric_u64(&state, "character_revision")? != root.revision || !state_matches_root(&state, root) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let before = load(tx, character).await?;
    let now = database_time(tx).await?;
    let (after, result) = match before.apply(transition, now) {
        Ok(after) => (after, Ok(())),
        Err(refusal) => (before, Err(refusal)),
    };
    let changed = before != after;
    let next = root.revision.checked_add(u64::from(changed))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let response = serde_json::to_vec(&(after, result))
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let binding = binding(transition)?;
    let boss_key = match transition {
        SoulWarTransition::BossDefeated(boss) => Some(match boss {
            oteryn_gameplay_quests::quest::soul_war::SoulWarBoss::Malice => "malice",
            oteryn_gameplay_quests::quest::soul_war::SoulWarBoss::Hatred => "hatred",
            oteryn_gameplay_quests::quest::soul_war::SoulWarBoss::Spite => "spite",
            oteryn_gameplay_quests::quest::soul_war::SoulWarBoss::Cruelty => "cruelty",
            oteryn_gameplay_quests::quest::soul_war::SoulWarBoss::Greed => "greed",
            oteryn_gameplay_quests::quest::soul_war::SoulWarBoss::Megalomania => "megalomania",
        }),
        _ => None,
    };
    if boss_key.is_some() != source_death.is_some()
        || matches!(transition, SoulWarTransition::FailedMegalomania { .. }) != source_attempt.is_some()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let receipt = sqlx::query("INSERT INTO game_character_soul_war_receipts(character_id,occurrence,request_binding,result,source_death_key,boss_key,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,source_attempt) SELECT s.character_id,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,s.character_revision,$12::text::numeric,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,$13 FROM game_character_progression_state s WHERE s.character_id=encode($1,'hex')::uuid")
        .bind(id).bind(occurrence.as_slice()).bind(&binding).bind(response)
        .bind(source_death.map(|key| key.as_slice())).bind(boss_key)
        .bind(after.teleport_access).bind(i16::from(after.defeated_bosses))
        .bind(after.first_taint_at).bind(after.megalomania_completed).bind(after.final_reward_claimed)
        .bind(changed.then(|| next.to_string())).bind(source_attempt.map(|key| key.as_slice()))
        .execute(&mut **tx).await?;
    if receipt.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
    if changed {
        for query in [
            "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
            "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
        ] {
            if sqlx::query(query).bind(id).bind(next.to_string()).bind(root.revision.to_string())
                .execute(&mut **tx).await?.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        sqlx::query("INSERT INTO game_character_soul_war(character_id,teleport_access,defeated_bosses,first_taint_at,megalomania_completed,final_reward_claimed) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6) ON CONFLICT(character_id) DO UPDATE SET teleport_access=EXCLUDED.teleport_access,defeated_bosses=EXCLUDED.defeated_bosses,first_taint_at=EXCLUDED.first_taint_at,megalomania_completed=EXCLUDED.megalomania_completed,final_reward_claimed=EXCLUDED.final_reward_claimed")
            .bind(id).bind(after.teleport_access).bind(i16::from(after.defeated_bosses))
            .bind(after.first_taint_at).bind(after.megalomania_completed).bind(after.final_reward_claimed)
            .execute(&mut **tx).await?;
    }
    if !before.megalomania_completed && after.megalomania_completed {
        super::reward::earn(tx, character, occurrence).await?;
    }
    Ok(SoulWarCommandOutcome {
        state: after, result,
        character_revision: CharacterRevision::new(next).map_err(|_| DurabilityError::InvalidStoredState)?,
        replayed: false,
    })
}
