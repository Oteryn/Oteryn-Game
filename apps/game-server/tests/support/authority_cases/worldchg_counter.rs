//! Shared counter transaction: real runtime-role PG, current independent scope fences.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes::{
    self,
    counters::{CounterKind as K, Credit, CreditEvent, CreditOutcome},
};
use crate::foundation::{CharacterId, ScopeOwnershipGeneration};

#[test]
fn worldchg_postgres_shared_kill_counter_atomic_replay_and_save() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64<<20).spawn(move ||->Result<(),String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start=std::time::Instant::now();
            let h=fixture::Harness::create(admin,"worldchg_counter",false).await?;
            let result=exercise(&h).await;h.cleanup().await?;result?;
            assert!(start.elapsed().as_secs_f64()>1.0);
            println!("WORLDCHG-Q PG EXECUTED: generic counter8concurrent writers, full-death dedupe, rollback, stale scope/epoch, reset replay; elapsed={:?}",start.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_|"counter PG thread failed")?.map_err(Into::into)
}
async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let scope = fixture::scope()?;
    let world = scope.world_id();
    for epoch in 1..=2_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2031-06-13T12:00:00Z','2031-06')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let day = h
        .root
        .world_changes(world, 1)
        .await?
        .ok_or("initialized day")?;
    let catalogue: serde_json::Value =
        serde_json::from_str(oteryn_gameplay_world_change::owner::CATALOGUE_JSON)?;
    let fire = catalogue["entries"]
        .as_array()
        .ok_or("catalogue entries")?
        .iter()
        .find(|entry| entry["key"] == "fire_from_the_earth")
        .ok_or("fire entry")?;
    let admitted = fire["status"] == "READY" || fire["status"] == "QUALIFIED";
    let any_fire = (1..=64)
        .map(|epoch| changes::roll(world, epoch, 1_939_117_200_000))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|day| day.fire_from_the_earth);
    assert_eq!(
        any_fire, admitted,
        "HELD recipes never enter the production daily draw"
    );
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let event = |seed: u32| -> fixture::TestResult<CreditEvent> {
        let mut key = Vec::new();
        key.extend_from_slice(world.as_bytes());
        let crate::foundation::RuntimeScopeRefV1::Channel { channel_id, .. } = scope else {
            return Err("channel fixture".into());
        };
        key.extend_from_slice(channel_id.as_bytes());
        key.extend_from_slice(&1_u64.to_be_bytes());
        key.extend_from_slice(&seed.to_be_bytes());
        key.extend_from_slice(&1_u64.to_be_bytes());
        Ok(CreditEvent {
            source_kind: 1,
            occurrence: key,
            credits: vec![(K::DemonWars, Credit::Kill(0))],
            counter_generations: day.counter_generations,
            occurred_at_ms: 100,
            character: Some(
                CharacterId::decode(&fixture::id(if seed <= 8 { 80 + seed as u8 } else { 81 }))
                    .map_err(fixture::debug)?,
            ),
        })
    };
    // Match the harness's explicit writable Root columns. Generated naming keys
    // and name reservations are produced by their real database owners.
    for seed in 81..=88 {
        let name = format!("World Counter {}", char::from(b'A' + seed - 81));
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT encode($2,'hex')::uuid,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,$3,sex FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fixture::id(fixture::CHARACTER).as_slice()).bind(fixture::id(seed).as_slice()).bind(name).execute(&h.pool).await?;
    }
    assert!(h.root.world_changes(world, 1).await?.is_some());
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    let requests = (1..=8).map(event).collect::<Result<Vec<_>, _>>()?;
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(8));
    let mut tasks = Vec::new();
    for request in requests.clone() {
        let pool = h.pool.clone();
        let node = h.node.clone();
        let recovery = recovery.clone();
        let barrier = barrier.clone();
        tasks.push(async move {
            barrier.wait().await;
            let mut tx = pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            let token =
                changes::counters::authorize(&mut tx, &recovery, &node, scope, generation, 1)
                    .await?
                    .ok_or(crate::durability::DurabilityError::Unavailable)?;
            let result = changes::counters::credit(&mut tx, &token, &request).await?;
            tx.commit().await?;
            Ok::<_, crate::durability::DurabilityError>(result)
        });
    }
    // Poll all eight futures concurrently without manufacturing another actor/session fence.
    let mut pending: Vec<_> = tasks
        .into_iter()
        .map(|future| Some(Box::pin(future)))
        .collect();
    let mut results = Vec::new();
    let outcomes = std::future::poll_fn(move |context| {
        use std::future::Future;
        for slot in &mut pending {
            if let Some(future) = slot.as_mut()
                && let std::task::Poll::Ready(result) = future.as_mut().poll(context)
            {
                results.push(result);
                *slot = None;
            }
        }
        if pending.iter().all(Option::is_none) {
            std::task::Poll::Ready(std::mem::take(&mut results))
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    assert_eq!(outcomes.len(), 8);
    for outcome in outcomes {
        assert!(matches!(outcome?, CreditOutcome::Applied { .. }));
    }
    let count = || async {
        let value:serde_json::Value=sqlx::query_scalar("SELECT state FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key='demon_wars'")
            .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        Ok::<_, Box<dyn std::error::Error>>(value["counts"][0].as_u64().ok_or("counter")?)
    };
    let mut substituted = requests[0].clone();
    substituted.character = Some(CharacterId::decode(&fixture::id(82)).map_err(fixture::debug)?);
    if h.root
        .credit_world_change_event(&authority, &h.node, scope, generation, 1, substituted)
        .await?
        != CreditOutcome::Refused
    {
        return Err(
            "immutable native death principal substitution must be refused without tally writes"
                .into(),
        );
    }
    assert_eq!(count().await?, 8);
    assert_eq!(
        h.root
            .credit_world_change_event(
                &authority,
                &h.node,
                scope,
                generation,
                1,
                requests[0].clone()
            )
            .await?,
        CreditOutcome::AlreadyApplied
    );
    assert_eq!(count().await?, 8);
    assert_eq!(
        h.root
            .credit_world_change_event(&authority, &h.node, scope, generation, 2, event(20)?)
            .await?,
        CreditOutcome::Refused
    );
    assert_eq!(
        h.root
            .credit_world_change_event(
                &authority,
                &h.node,
                scope,
                ScopeOwnershipGeneration::new(2).map_err(fixture::debug)?,
                1,
                event(21)?
            )
            .await?,
        CreditOutcome::Refused
    );
    assert_eq!(count().await?, 8);
    // A projection milestone must not invalidate already captured concurrent credits.
    let mut serpent =
        oteryn_gameplay_world_change::counter::CounterState::new(K::FireFeathered, 1, false);
    serpent.counts[0] = 999;
    sqlx::query("UPDATE game_world_change_counters SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND change_key='the_fire_feathered_serpent'")
        .bind(world.as_bytes().as_slice()).bind(serde_json::to_value(serpent)?).execute(&h.pool).await?;
    for index in 100..108 {
        let mut request = event(index)?;
        request.credits = vec![(K::FireFeathered, Credit::Kill(0))];
        assert!(matches!(
            h.root
                .credit_world_change_event(&authority, &h.node, scope, generation, 1, request)
                .await?,
            CreditOutcome::Applied { .. }
        ));
    }
    let serpent:oteryn_gameplay_world_change::counter::CounterState=serde_json::from_value(sqlx::query_scalar("SELECT state FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key='the_fire_feathered_serpent'")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?)?;
    assert_eq!(serpent.counts[0], 1007);
    assert_eq!(serpent.stage, 2);
    assert_eq!(serpent.credit_generation, 1);
    // The token cannot be transplanted into a new transaction, even with matching scope.
    let mut tx = h.pool.begin().await?;
    let token = changes::counters::authorize(&mut tx, &recovery, &h.node, scope, generation, 1)
        .await?
        .ok_or("current token")?;
    tx.rollback().await?;
    let mut tx = h.pool.begin().await?;
    assert_eq!(
        changes::counters::credit(&mut tx, &token, &event(109)?).await?,
        CreditOutcome::Refused
    );
    tx.commit().await?;
    assert_eq!(count().await?, 8);
    let mut connection = h.pool.acquire().await?;
    let mut tx = sqlx::Connection::begin(&mut *connection).await?;
    let token = changes::counters::authorize(&mut tx, &recovery, &h.node, scope, generation, 1)
        .await?
        .ok_or("current token")?;
    assert!(matches!(
        changes::counters::credit(&mut tx, &token, &event(22)?).await?,
        CreditOutcome::Applied { .. }
    ));
    tx.rollback().await?;
    assert_eq!(count().await?, 8);
    let mut seed:oteryn_gameplay_world_change::counter::CounterState=serde_json::from_value(sqlx::query_scalar("SELECT state FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key='demon_wars'")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?)?;
    seed.counts[0] = 99;
    seed.validate(K::DemonWars).map_err(fixture::debug)?;
    sqlx::query("UPDATE game_world_change_counters SET state=$2 WHERE world_id=encode($1,'hex')::uuid AND change_key='demon_wars'")
        .bind(world.as_bytes().as_slice()).bind(serde_json::to_value(seed)?).execute(&h.pool).await?;
    assert!(matches!(
        h.root
            .credit_world_change_event(&authority, &h.node, scope, generation, 1, event(23)?)
            .await?,
        CreditOutcome::Applied { .. }
    ));
    assert_eq!(count().await?, 100);
    assert_eq!(
        h.root
            .world_changes(world, 1)
            .await?
            .ok_or("current day")?
            .counter_stages[K::DemonWars.index()],
        3,
        "stage changes only at save"
    );
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    let next = h.root.world_changes(world, 2).await?.ok_or("next day")?;
    assert_eq!(next.counter_stages[K::DemonWars.index()], 4);
    assert_eq!(count().await?, 0);
    assert_eq!(h.root.world_changes(world, 2).await?, Some(next));
    assert!(h.root.world_changes(world, 1).await?.is_none());
    Ok(())
}
