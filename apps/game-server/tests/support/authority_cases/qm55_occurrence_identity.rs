//! The WorldCounter owner persists quest lifetimes independently of save epochs.
use crate::bestiary_postgres_harness as f;
use oteryn_gameplay_world_change::counter::CounterKind as K;

#[test]
fn qm55_counter_occurrence_identity_persists_across_saves_and_legacy_reload() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qm55_occurrence_identity", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M55_PG_EXECUTED occurrence_identity duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M55 occurrence worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let world = f::scope()?.world_id();
    for epoch in 1..=6_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    h.root
        .world_changes(world, 1)
        .await?
        .ok_or("initialized counters")?;
    // Retain old serialized rows: a rollout cannot require rewriting reset history.
    sqlx::query("UPDATE game_world_change_counters SET state=jsonb_set(jsonb_set(state,'{stage}','3'),'{entered_epoch}','1')-'occurrence_generation' WHERE world_id=encode($1,'hex')::uuid AND change_key IN ('hive_born','deeplings')")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let original = h
        .root
        .world_changes(world, 1)
        .await?
        .ok_or("legacy counter projection")?;
    for kind in [K::HiveBorn, K::Deeplings] {
        assert_eq!(original.counter_quest_occurrences[kind.index()], 1);
    }
    assert_eq!(
        original.counter_quest_occurrences[K::DemonWars.index()],
        0,
        "unqualified lifecycles cannot authorize quest progress"
    );
    for epoch in 2..=6_i64 {
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2 WHERE world_id=encode($1,'hex')::uuid").bind(world.as_bytes().as_slice()).bind(epoch).execute(&h.pool).await?;
        let day = h
            .root
            .world_changes(world, epoch as u64)
            .await?
            .ok_or("next day")?;
        let expected = if epoch < 6 { 1 } else { 2 };
        for kind in [K::HiveBorn, K::Deeplings] {
            assert_eq!(day.counter_quest_occurrences[kind.index()], expected);
            assert_eq!(day.counter_generations[kind.index()], 1);
        }
        assert_eq!(h.root.world_changes(world, epoch as u64).await?, Some(day));
    }
    let persisted: Vec<(String, String)> = sqlx::query_as("SELECT change_key,state->>'occurrence_generation' FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key IN ('hive_born','deeplings') ORDER BY change_key LIMIT 3")
        .bind(world.as_bytes().as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(
        persisted,
        vec![
            ("deeplings".into(), "2".into()),
            ("hive_born".into(), "2".into())
        ]
    );
    assert!(h.root.world_changes(world, 5).await?.is_none());
    let rewritten: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_change_days WHERE world_id=encode($1,'hex')::uuid AND decision ? 'counter_quest_occurrences')")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(
        !rewritten,
        "live quest scopes must not rewrite immutable daily decisions"
    );
    Ok(())
}
