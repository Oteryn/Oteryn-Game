//! Disposable P01 inputs. Balances are derived from Bestiary receipts.
use super::*;

const COMPLETED: [&str; 15] = [
    "oteryn:creature.acolyte_of_darkness",
    "oteryn:creature.bane_bringer",
    "oteryn:creature.bane_of_light",
    "oteryn:creature.blue_butterfly",
    "oteryn:creature.bride_of_night",
    "oteryn:creature.butterfly",
    "oteryn:creature.cake_golem",
    "oteryn:creature.cat",
    "oteryn:creature.chocolate_blob",
    "oteryn:creature.cream_blob",
    "oteryn:creature.crustacea_gigantica",
    "oteryn:creature.crystal_wolf",
    "oteryn:creature.diamond_servant",
    "oteryn:creature.dire_penguin",
    "oteryn:creature.troll",
];
pub(super) const INCOMPLETE: &str = "oteryn:creature.wolf";
const RAT: &str = "oteryn:creature.rat";

pub(super) async fn seed(clients: &SeamClients<'_>, full: bool) -> TestResult {
    // Match the character selected by connect_world, in the disposable row database.
    let character = clients.characters[1].as_slice();
    let races = full_row_p05::canonical_races()?;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = connection.begin().await?;
    let revision: String = sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots
         WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character)
    .fetch_one(&mut *tx)
    .await?;
    let prior_kills: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_bestiary_progress
         WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character)
    .fetch_one(&mut *tx)
    .await?;
    if prior_kills != 0 {
        return Err("P01 fixture requires a clean Bestiary".into());
    }
    // The normal root guard permits one successor at a time. Bulk fixture setup
    // writes the complete receipt chain atomically, as the charm PG fixture does.
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let mut points = 0;
    let mut offset = 0_i64;
    let keys: &[&str] = if full {
        &COMPLETED
    } else {
        &["oteryn:creature.troll"]
    };
    let partial: &[&str] = if full { &[RAT, INCOMPLETE] } else { &[] };
    for &key in keys.iter().chain(partial) {
        let (race, award) = races
            .iter()
            .find(|(race, _)| race.key() == key)
            .ok_or_else(|| format!("P01 fixture Bestiary definition missing: {key}"))?;
        let threshold = i64::from(race.final_kill_threshold());
        let kills = if !full {
            if threshold != 250 || *award != 5 {
                return Err("unexpected canonical Troll mastery inputs".into());
            }
            threshold - 1
        } else if key == RAT {
            if threshold != 250 || *award != 5 {
                return Err("unexpected canonical Rat mastery inputs".into());
            }
            threshold - 1
        } else if key == INCOMPLETE {
            if threshold <= 1 {
                return Err("P01 fixture requires a non-mastered refusal entry".into());
            }
            1
        } else {
            points += award;
            threshold
        };
        let written = sqlx::query(
            "WITH receipts AS (
                INSERT INTO game_character_bestiary_kill_receipts (
                    bestiary_occurrence_id, command_binding, race_digest, character_id,
                    original_character_revision, committed_character_revision,
                    level_before, level_after, experience_before, experience_after,
                    race_key, race_definition_revision, final_kill_threshold,
                    kill_count_before, kill_count_after, profile_revision, ruleset_revision,
                    content_revision, simulation_revision, evidence_revision,
                    declaration_revision, policy_revision, reward_revision, committed_at)
                SELECT game_character_uuid_v7(), decode('01','hex'), sha256('P01 fixture'),
                    s.character_id, s.character_revision + $5 + k.n - 1,
                    s.character_revision + $5 + k.n, s.level, s.level,
                    s.total_experience, s.total_experience, $2, $4, $3, k.n - 1, k.n,
                    s.profile_revision, s.ruleset_revision, s.content_revision, s.simulation_revision,
                    s.evidence_revision, s.declaration_revision, s.policy_revision, s.reward_revision, 0
                FROM game_character_progression_state s
                CROSS JOIN generate_series(1::bigint, $6::bigint) k(n)
                WHERE s.character_id = encode($1,'hex')::uuid
                RETURNING character_id, race_key, kill_count_after, committed_character_revision,
                    bestiary_occurrence_id)
            INSERT INTO game_character_bestiary_progress (
                character_id, race_key, kill_count, committed_character_revision,
                last_bestiary_occurrence_id)
            SELECT character_id, race_key, kill_count_after, committed_character_revision,
                bestiary_occurrence_id FROM receipts WHERE kill_count_after = $6",
        )
        .bind(character)
        .bind(key)
        .bind(threshold)
        .bind(race.definition_revision())
        .bind(offset)
        .bind(kills)
        .execute(&mut *tx)
        .await?;
        if written.rows_affected() != 1 {
            return Err(format!("P01 fixture did not create the Bestiary entry for {key}").into());
        }
        offset += kills;
    }
    if full && points != 400 {
        return Err(
            format!("P01 fixture needs exactly 400 points, catalogue yields {points}").into(),
        );
    }
    for statement in [
        "UPDATE game_character_roots SET character_revision = character_revision + $2
         WHERE character_id = encode($1,'hex')::uuid AND character_revision = $3::text::numeric",
        "UPDATE game_character_progression_state SET character_revision = character_revision + $2
         WHERE character_id = encode($1,'hex')::uuid AND character_revision = $3::text::numeric",
    ] {
        if sqlx::query(statement)
            .bind(character)
            .bind(offset)
            .bind(&revision)
            .execute(&mut *tx)
            .await?
            .rows_affected()
            != 1
        {
            return Err("P01 fixture progression revision mismatch".into());
        }
    }
    tx.commit().await?;
    evidence(&format!(
        "P01_FIXTURE charm_points={points} rat_kills={}",
        if full { 249 } else { 0 }
    ));
    Ok(())
}
