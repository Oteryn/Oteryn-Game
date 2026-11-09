//! Production Ahau binding and read-only prerequisite matrix; no victory is fabricated.
#![cfg(feature = "test-support")]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;
#[path = "support/m01c_admission_probe.rs"]
mod admission_probe;
#[allow(
    dead_code,
    reason = "reuse canonical PostgreSQL session-generation harness"
)]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod fixture;

#[test]
fn m01c_ancient_aucar_requires_completed_parent_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let started = std::time::Instant::now();
            let h = fixture::Harness::create(admin, "questpc3_m01c_aucar", true).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            assert!(started.elapsed().as_secs_f64() > 0.01);
            eprintln!("M01C_AUCAR POSTGRES_CASE_COMPLETED duration_ms={} query_component_only=true", started.elapsed().as_millis());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_| "Aucar PG thread")?.map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        version,
        oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
    );
    eprintln!("M01C_AUCAR POSTGRES_CONNECTED version={version}");
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("aucar-pg")
        .map_err(fixture::debug)?;
    let catalogue = loaded.catalogue();
    let quest = "oteryn:quest.ancient_aucar_outfits_quest";
    assert!(admission_probe::requires_premium(catalogue, quest));
    let counter = oteryn_gameplay_quests::quest::native_kills::counter_for(
        catalogue,
        "oteryn:quest-transition/native-kills/m01c-ancient-aucar-ahau",
    )
    .ok_or("Ahau binding")?;
    assert_eq!(counter.creature, "oteryn:creature.ahau");
    let parent = "oteryn:quest.adventures_of_galthen_quest";
    let character = fixture::id(fixture::CHARACTER);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character.as_slice()).execute(&mut *tx).await?;
    assert!(!admission_probe::prerequisites_hold(&mut tx, &character, catalogue, quest).await?);
    sqlx::query("SAVEPOINT incomplete_parent")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,completed_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'aucar-pg',decode(repeat('33',32),'hex'),2,NULL,2)")
        .bind(character.as_slice()).bind(parent).execute(&mut *tx).await?;
    assert!(!admission_probe::prerequisites_hold(&mut tx, &character, catalogue, quest).await?);
    sqlx::query("ROLLBACK TO SAVEPOINT incomplete_parent")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,completed_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'aucar-pg',decode(repeat('33',32),'hex'),2,2,2)")
        .bind(character.as_slice()).bind(parent).execute(&mut *tx).await?;
    assert!(admission_probe::prerequisites_hold(&mut tx, &character, catalogue, quest).await?);
    tx.rollback().await?;
    let persisted: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(character.as_slice()).bind(parent).fetch_one(&h.pool).await?;
    assert_eq!(
        persisted, 0,
        "unearned prerequisite fixtures must never persist"
    );
    Ok(())
}
