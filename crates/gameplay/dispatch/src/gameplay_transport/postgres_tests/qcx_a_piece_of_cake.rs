//! Brutus's cupcake offer uses the persisted annual errand.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};

#[test]
fn brutus_offers_cupcake_only_after_the_five_extractors() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Cake PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "qcx_cake_brutus", true, false).await?;
        let result = exercise(&h).await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup?;
        eprintln!("POSTGRES_CASE_COMPLETED brutus_offers_cupcake_only_after_the_five_extractors {:?}", started.elapsed());
        Ok(())
    })
}

async fn exercise(h: &Harness) -> TestResult {
    reset();
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let deps = Dependencies::new()?;
    let admitted = &deps.admitted;
    let mut owner = deps.owner(h, &authority);
    deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
    let [brutus] = install(&owner, admitted, ["oteryn:npc.brutus_the_baker"]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_cake_world_progress VALUES(encode($1,'hex')::uuid,2026,200,200)")
        .bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_cake_phases(world_id,event_year,stage,layer,brutus_present) VALUES(encode($1,'hex')::uuid,2026,3,4,true)")
        .bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_cake_errands(world_id,event_year,character_id,cities) VALUES(encode($1,'hex')::uuid,2026,encode($2,'hex')::uuid,15)")
        .bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;

    talk(&mut owner, admitted, brutus, 1, "hi").await?;
    assert_eq!(
        talk(&mut owner, admitted, brutus, 2, "cupcake").await?,
        ["I am sorry, but you did not earn your reward yet."]
    );
    sqlx::query("UPDATE game_cake_errands SET cities=31 WHERE world_id=encode($1,'hex')::uuid AND event_year=2026 AND character_id=encode($2,'hex')::uuid")
        .bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
    let offer = talk(&mut owner, admitted, brutus, 3, "cupcake").await?;
    assert!(offer.iter().any(|line| line.contains("blueberry cupcake") && line.contains("strawberry cupcake") && line.contains("lemon cupcake")));

    sqlx::query("UPDATE game_cake_errands SET cupcake_claimed=true WHERE world_id=encode($1,'hex')::uuid AND event_year=2026 AND character_id=encode($2,'hex')::uuid")
        .bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
    assert_eq!(
        talk(&mut owner, admitted, brutus, 4, "cupcake").await?,
        ["Sorry, but you already got a cupcake this year."]
    );
    Ok(())
}
