use super::fixture::{Harness, id};

use super::TestResult;

use crate::domain::bestiary::BestiaryRace;

pub(super) fn source_limits() -> crate::content::ProjectEvidenceLimits {
    crate::content::ProjectEvidenceLimits {
        max_documents: 5,
        max_document_bytes: 2_500_000,
        max_total_bytes: 8_000_000,
        max_json_depth: 10,
        max_decoded_fields: 65_000,
        max_string_bytes: 1_000_000,
        max_locator_bytes: 128,
        max_locator_segments: 5,
        max_reference_records: 4096,
        max_import_records: 1,
        max_reimport_states: 1,
    }
}

pub(super) async fn seed_completed(h: &Harness, races: &[(BestiaryRace, u32)]) -> TestResult {
    // Test fixture only. Every projected kill has a matching receipt/revision, so subsequent
    // real Charm writes still pass the normal recovery and Character integrity checks.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let mut kills_before = 0_i64;
    for (race, _) in races {
        sqlx::query(
            "WITH receipts AS (\
              INSERT INTO game_character_bestiary_kill_receipts(\
                bestiary_occurrence_id,command_binding,race_digest,character_id,\
                original_character_revision,committed_character_revision,\
                level_before,level_after,experience_before,experience_after,\
                race_key,race_definition_revision,final_kill_threshold,\
                kill_count_before,kill_count_after,profile_revision,ruleset_revision,\
                content_revision,simulation_revision,evidence_revision,\
                declaration_revision,policy_revision,reward_revision,committed_at) \
              SELECT game_character_uuid_v7(),decode('01','hex'),sha256('seed'),\
                s.character_id,s.character_revision+$5+k.n-1,s.character_revision+$5+k.n,\
                s.level,s.level,s.total_experience,s.total_experience,$2,$4,$3,k.n-1,k.n,\
                s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,\
                s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,0 \
              FROM game_character_progression_state s \
              CROSS JOIN generate_series(1::bigint,$3::bigint) k(n) \
              WHERE s.character_id=encode($1,'hex')::uuid \
              RETURNING character_id,race_key,kill_count_after,committed_character_revision,bestiary_occurrence_id) \
             INSERT INTO game_character_bestiary_progress(character_id,race_key,kill_count,\
                committed_character_revision,last_bestiary_occurrence_id) \
             SELECT character_id,race_key,kill_count_after,committed_character_revision,bestiary_occurrence_id \
             FROM receipts WHERE kill_count_after=$3",
        )
        .bind(id(41).as_slice()).bind(race.key()).bind(i64::from(race.final_kill_threshold()))
        .bind(race.definition_revision()).bind(kills_before).execute(&mut *tx).await?;
        kills_before += i64::from(race.final_kill_threshold());
    }
    for sql in [
        "UPDATE game_character_roots SET character_revision=character_revision+$2 WHERE character_id=encode($1,'hex')::uuid",
        "UPDATE game_character_progression_state SET character_revision=character_revision+$2 WHERE character_id=encode($1,'hex')::uuid",
    ] {
        sqlx::query(sql)
            .bind(id(41).as_slice())
            .bind(kills_before)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}
