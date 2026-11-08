//! Native credited Fire from Earth death receipt grants the canonical account achievement.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes::{
    self,
    counters::{CounterKind, Credit, CreditEvent, CreditOutcome},
};
use crate::foundation::{CharacterId, RuntimeScopeRefV1, ScopeOwnershipGeneration};

#[test]
fn worldchg_postgres_fire_earth_fifty_active_kills_grant_once() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(),String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start=std::time::Instant::now();
            let h=fixture::Harness::create(admin,"worldchg_fire_earth",false).await?;
            let result=exercise(&h).await; h.cleanup().await?; result?;
            assert!(start.elapsed().as_secs_f64()>1.0);
            println!("WORLDCHG-Q PG EXECUTED: Fire Earth personal49/50, atomic rollback, replay, canonical achievement; elapsed={:?}",start.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_|"Fire Earth PG thread failed")?.map_err(Into::into)
}
async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let scope = fixture::scope()?;
    let world = scope.world_id();
    let epoch = 1_u64;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query(
        "INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric)",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .execute(&h.pool)
    .await?;
    // Seed the active immutable fixture before initialization: production selection
    // stays HELD until its physical producer is qualified independently.
    let mut fixture_day = changes::roll(world, epoch, 0)?;
    fixture_day.fire_from_the_earth = true;
    fixture_day.world_counters = true;
    for kind in CounterKind::ALL {
        let state = oteryn_gameplay_world_change::counter::CounterState::new(kind, epoch, true);
        sqlx::query("INSERT INTO game_world_change_counters(world_id,change_key,reset_epoch,state) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4)")
            .bind(world.as_bytes().as_slice()).bind(kind.key()).bind(epoch.to_string()).bind(serde_json::to_value(state)?).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_change_days(world_id,reset_epoch,decision) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(serde_json::to_value(fixture_day)?).execute(&h.pool).await?;
    let day = h.root.world_changes(world, epoch).await?.ok_or("day")?;
    assert_eq!(day.counter_stages[CounterKind::FireEarth.index()], 1);
    let character =
        CharacterId::decode(&fixture::id(fixture::CHARACTER)).map_err(fixture::debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    let event = |ordinal: u32| -> fixture::TestResult<CreditEvent> {
        let RuntimeScopeRefV1::Channel { channel_id, .. } = scope else {
            return Err("channel fixture".into());
        };
        let mut occurrence = world.as_bytes().to_vec();
        occurrence.extend_from_slice(channel_id.as_bytes());
        occurrence.extend_from_slice(&generation.get().to_be_bytes());
        occurrence.extend_from_slice(&ordinal.to_be_bytes());
        occurrence.extend_from_slice(&1_u64.to_be_bytes());
        assert!(occurrence.len() <= 64);
        Ok(CreditEvent {
            source_kind: 1,
            occurrence,
            credits: vec![(CounterKind::FireEarth, Credit::Kill(0))],
            counter_generations: day.counter_generations,
            occurred_at_ms: 100,
            character: Some(character),
        })
    };
    for ordinal in 1..=49 {
        assert!(matches!(
            h.root
                .credit_world_change_event(
                    &authority,
                    &h.node,
                    scope,
                    generation,
                    epoch,
                    event(ordinal)?
                )
                .await?,
            CreditOutcome::Applied { .. }
        ));
    }
    let grants = || async {
        Ok::<i64,Box<dyn std::error::Error>>(sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/fire_from_the_earth'").fetch_one(&h.pool).await?)
    };
    assert_eq!(grants().await?, 0);
    // The fiftieth death and grant are visible only together: aborting its receipt aborts value.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let token = changes::counters::authorize(&mut tx, &recovery, &h.node, scope, generation, epoch)
        .await?
        .ok_or("current token")?;
    assert!(matches!(
        changes::counters::credit(&mut tx, &token, &event(50)?).await?,
        CreditOutcome::Applied { .. }
    ));
    let inside:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/fire_from_the_earth'").fetch_one(&mut *tx).await?;
    assert_eq!(inside, 1);
    tx.rollback().await?;
    assert_eq!(grants().await?, 0);
    assert!(matches!(
        h.root
            .credit_world_change_event(&authority, &h.node, scope, generation, epoch, event(50)?)
            .await?,
        CreditOutcome::Applied { .. }
    ));
    assert_eq!(grants().await?, 1);
    assert_eq!(
        h.root
            .credit_world_change_event(&authority, &h.node, scope, generation, epoch, event(50)?)
            .await?,
        CreditOutcome::AlreadyApplied
    );
    assert!(matches!(
        h.root
            .credit_world_change_event(&authority, &h.node, scope, generation, epoch, event(51)?)
            .await?,
        CreditOutcome::Applied { .. }
    ));
    assert_eq!(grants().await?, 1);
    let requests:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/fire_from_the_earth'").fetch_one(&h.pool).await?;
    assert_eq!(requests, 1);
    Ok(())
}
