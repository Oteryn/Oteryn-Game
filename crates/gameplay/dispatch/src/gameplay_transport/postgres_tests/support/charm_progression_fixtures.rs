#![allow(clippy::expect_used)]

use super::*;

use crate::domain::bestiary::BestiaryRace;

pub(super) async fn seed(h: &Harness, races: &[&BestiaryRace]) -> TestResult {
    // Explicit fixture seed, never a product writer. Runtime assertions below use the
    // normal restricted runtime role, including fee audit and revision constraints.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
        .execute(&mut *tx)
        .await?;
    // Fund the level * 100 unassignment fee, in addition to the five carried coins.
    let bank_gold: i64 = sqlx::query_scalar(
        "SELECT level * 100 + 2000000 FROM game_character_progression_state \
         WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&mut *tx)
    .await?;
    let uuid = |n| crate::durability::item_fee_burn::test_support::uuid(n);
    let seed = format!(
        "INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES({}, {}, {},1,sha256('seed'),{},{},{},{},1,{bank_gold},0,1,sha256('seed'),0,{},{},{}); INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES({},{},{},{},1,{bank_gold},0,{bank_gold},{}); INSERT INTO game_account_bank_balances VALUES({},{},{bank_gold},{});",
        uuid(80),
        uuid(81),
        uuid(82),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(43),
        uuid(83),
        uuid(84),
        uuid(70),
        uuid(85),
        uuid(81),
        uuid(40),
        uuid(42),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(85)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    let seed = format!(
        "{}{} INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES({},{},{},{}); INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES({},{},{},{},1,{});",
        crate::durability::item_fee_burn::test_support::item(
            70,
            "oteryn:item.tibia.i2854",
            "definition-r1",
            1
        ),
        crate::durability::item_fee_burn::test_support::item(
            72,
            "oteryn:item.tibia.i3031",
            "definition-r1",
            5
        ),
        uuid(41),
        uuid(70),
        uuid(42),
        uuid(74),
        uuid(41),
        uuid(70),
        uuid(72),
        uuid(42),
        uuid(75)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    // Completed Bestiary entries need their entire kill receipt chain, including one
    // CharacterRevision per kill. A counter alone can be queried, but the next real
    // Character write correctly rejects it as an inconsistent projection.
    let mut kills_before = 0_i64;
    for race in races {
        sqlx::query(
            "WITH receipts AS (\
               INSERT INTO game_character_bestiary_kill_receipts(\
                 bestiary_occurrence_id, command_binding, race_digest, character_id, \
                 original_character_revision, committed_character_revision, \
                 level_before, level_after, experience_before, experience_after, \
                 race_key, race_definition_revision, final_kill_threshold, \
                 kill_count_before, kill_count_after, profile_revision, ruleset_revision, \
                 content_revision, simulation_revision, evidence_revision, \
                 declaration_revision, policy_revision, reward_revision, committed_at) \
               SELECT game_character_uuid_v7(), decode('01','hex'), sha256('seed'), \
                 s.character_id, s.character_revision + $5 + k.n - 1, \
                 s.character_revision + $5 + k.n, s.level, s.level, \
                 s.total_experience, s.total_experience, $2, $4, $3, k.n - 1, k.n, \
                 s.profile_revision, s.ruleset_revision, s.content_revision, \
                 s.simulation_revision, s.evidence_revision, s.declaration_revision, \
                 s.policy_revision, s.reward_revision, 0 \
               FROM game_character_progression_state s \
               CROSS JOIN generate_series(1::bigint, $3::bigint) k(n) \
               WHERE s.character_id = encode($1,'hex')::uuid \
               RETURNING character_id, race_key, kill_count_after, \
                 committed_character_revision, bestiary_occurrence_id) \
             INSERT INTO game_character_bestiary_progress(\
               character_id, race_key, kill_count, committed_character_revision, \
               last_bestiary_occurrence_id) \
             SELECT character_id, race_key, kill_count_after, committed_character_revision, \
               bestiary_occurrence_id FROM receipts WHERE kill_count_after = $3",
        )
        .bind(id(41).as_slice())
        .bind(race.key())
        .bind(i64::from(race.final_kill_threshold()))
        .bind(race.definition_revision())
        .bind(kills_before)
        .execute(&mut *tx)
        .await?;
        kills_before += i64::from(race.final_kill_threshold());
    }
    sqlx::query(
        "UPDATE game_character_roots SET character_revision = character_revision + $2 \
         WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .bind(kills_before)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE game_character_progression_state \
         SET character_revision = character_revision + $2 \
         WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .bind(kills_before)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
