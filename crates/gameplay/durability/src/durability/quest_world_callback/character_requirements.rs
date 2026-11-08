//! M03: evaluate chosen recipe prerequisites under the quest writer's Character lock.
//! Evidence: content/quests/definitions/quests-00200-00299.json:1, record 67,
//! requirements. Promotion uses the permanent purchase owner, not a session vocation.
use crate::{domain::CharacterId, durability::DurabilityError};
use oteryn_gameplay_quests::quest::world_callbacks::CharacterRequirements;

pub(super) async fn satisfied(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    requirements: &CharacterRequirements,
) -> Result<bool, DurabilityError> {
    if requirements.promoted
        && !crate::durability::character_promotion::read_durable_promotion(tx, character).await?
    {
        return Ok(false);
    }
    let held: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid AND blessing_key=ANY($2::text[])")
        .bind(character.as_bytes().as_slice())
        .bind(&requirements.blessings)
        .fetch_one(&mut **tx)
        .await?;
    Ok(usize::try_from(held).ok() == Some(requirements.blessings.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bestiary_postgres_harness as harness;

    #[test]
    fn m03_postgres_character_requirements_read_current_transaction() -> harness::TestResult {
        let admin = harness::configured_admin().ok_or("M03 requires real PostgreSQL")?;
        harness::runtime()?.block_on(async move {
            let h = harness::Harness::create(admin, "srvenc_m03_character", true).await?;
            let result: harness::TestResult = async {
                let character = CharacterId::from_bytes(harness::id(harness::CHARACTER)).map_err(harness::debug)?;
                let mut tx = h.pool.begin().await?;
                // The production quest writer already owns this root before consume().
                sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                let mut requirements = CharacterRequirements { promoted: true, blessings: vec![] };
                assert!(!satisfied(&mut tx, character, &requirements).await?);
                // A deferred purchase FK is deliberately never committed: this fixture
                // tests observation, not the independent promotion purchase owner.
                sqlx::query("INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                    .bind(character.as_bytes().as_slice()).bind(harness::id(201).as_slice()).execute(&mut *tx).await?;
                assert!(satisfied(&mut tx, character, &requirements).await?);
                requirements.blessings = vec!["oteryn:blessing.the_spiritual_shielding".into(), "oteryn:blessing.the_spark_of_the_phoenix".into()];
                assert!(!satisfied(&mut tx, character, &requirements).await?);
                for key in &requirements.blessings {
                    sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'m03-fixture')")
                        .bind(character.as_bytes().as_slice()).bind(key).execute(&mut *tx).await?;
                }
                assert!(satisfied(&mut tx, character, &requirements).await?);
                sqlx::query("DELETE FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid AND blessing_key=$2")
                    .bind(character.as_bytes().as_slice()).bind(&requirements.blessings[0]).execute(&mut *tx).await?;
                assert!(!satisfied(&mut tx, character, &requirements).await?, "death-consumed blessing must not survive in a cached admission");
                tx.rollback().await?;
                eprintln!("M03 REAL POSTGRES: missing promotion, acquired promotion, missing blessings, all held, consumed blessing: PASS");
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
    }
}
