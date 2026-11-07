//! SRV7: the actual fenced World owner, calendar occurrence and restart identity.
use crate::bestiary_postgres_harness::{CHANNEL, Harness, TestResult, WORLD, debug, id, runtime};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};

#[test]
fn srv7_cake_world_owner_starts_once_and_rejects_stale_channel_on_postgres() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                runtime()?.block_on(async {
                    let h = Harness::create(admin, "srv7_cake_world", false).await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "cake world test thread failed")??;
    Ok(())
}

async fn exercise(h: &Harness) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let start = 1_771_650_000;
    assert!(
        h.root
            .refresh_cake_world(&authority, &h.node, world, channel, generation, start - 1)
            .await?
            .is_none()
    );
    let first = h
        .root
        .refresh_cake_world(&authority, &h.node, world, channel, generation, start)
        .await?
        .ok_or("scheduled World occurrence was not started")?;
    assert_eq!(
        (first.event_start, first.event_end, first.state.stage),
        (start, 1_772_168_400, 1)
    );
    assert_eq!(
        (
            first.state.substances,
            first.state.golems,
            first.state.layer,
            first.state.layer_progress
        ),
        (0, 0, 1, 0)
    );
    let restarted = h
        .root
        .refresh_cake_world(
            &authority,
            &h.node,
            world,
            channel,
            generation,
            start + 3600,
        )
        .await?
        .ok_or("restart occurrence")?;
    assert_eq!(
        restarted, first,
        "an owner restart cannot reset progress or change occurrence identity"
    );
    assert!(
        h.root
            .refresh_cake_world(
                &authority,
                &h.node,
                world,
                channel,
                ScopeOwnershipGeneration::new(2).map_err(debug)?,
                start + 3600
            )
            .await
            .is_err()
    );
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_seasonal_cake_worlds WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(rows, 1);
    let ended = h
        .root
        .refresh_cake_world(
            &authority,
            &h.node,
            world,
            channel,
            generation,
            first.event_end,
        )
        .await?
        .ok_or("ended occurrence")?;
    assert_eq!(ended.state.stage, 0);
    Ok(())
}

#[test]
fn srv7_cake_world_runtime_sql_cannot_forge_completion_on_postgres() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        let run = || -> TestResult {
            runtime()?.block_on(async {
                let h = Harness::create(admin, "srv7_cake_raw_world", false).await?;
                let result: TestResult = async {
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
                    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
                    h.root.refresh_cake_world(&authority,&h.node,world,channel,
                        ScopeOwnershipGeneration::new(1).map_err(debug)?,1_771_650_100).await?.ok_or("actual World occurrence")?;
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
                    let updated = sqlx::query("UPDATE game_seasonal_cake_worlds SET state=jsonb_set(jsonb_set(jsonb_set(state,'{stage}','3'),'{layer}','4'),'{layer_progress}','1116') WHERE world_id=encode($1,'hex')::uuid")
                        .bind(world.as_bytes().as_slice()).execute(&mut *tx).await;
                    let rejected = if updated.is_err() { tx.rollback().await?; true } else { tx.commit().await.is_err() };
                    if !rejected { return Err("runtime SQL forged valuable Cake completion without source receipts".into()); }
                    Ok(())
                }.await;
                let cleanup = h.cleanup().await;
                result?; cleanup
            })
        };
        run().map_err(|error| error.to_string())
    })?.join().map_err(|_| "cake World SQL guard test thread failed")??;
    Ok(())
}

#[test]
fn srv7_cake_golem_raid_first_hour_late_catchup_and_durable_replay_on_postgres() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        let run = || -> TestResult {
            runtime()?.block_on(async {
                let h = Harness::create(admin, "srv7_cake_first_raid", false).await?;
                let result: TestResult = async {
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
                    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
                    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
                    let start = 1_771_650_000;
                    h.root.refresh_cake_world(&authority,&h.node,world,channel,generation,start).await?.ok_or("real initial occurrence")?;
                    assert_eq!(h.root.claim_cake_raid_slot(&authority,&h.node,world,channel,generation,start+240).await?,None,"current Wiki first Cake Golem raid is server save plus one hour, not donor four minutes");
                    assert_eq!(h.root.claim_cake_raid_slot(&authority,&h.node,world,channel,generation,start+3599).await?,None);
                    assert_eq!(h.root.claim_cake_raid_slot(&authority,&h.node,world,channel,generation,start+3900).await?,Some((start,0)),"late boot catches current due hour outside its first minute");
                    assert_eq!(h.root.claim_cake_raid_slot(&authority,&h.node,world,channel,generation,start+7199).await?,None,"replay or later tick cannot repeat an already claimed hour");
                    assert_eq!(h.root.claim_cake_raid_slot(&authority,&h.node,world,channel,generation,start+7200).await?,Some((start,1)));
                    assert!(h.root.claim_cake_raid_slot(&authority,&h.node,world,channel,ScopeOwnershipGeneration::new(2).map_err(debug)?,start+10800).await.is_err(),"stale owner cannot claim a new due hour");
                    let rows:i64=sqlx::query_scalar("SELECT count(*) FROM game_seasonal_cake_raid_slots WHERE world_id=encode($1,'hex')::uuid")
                        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                    assert_eq!(rows,2);
                    Ok(())
                }.await;
                let cleanup=h.cleanup().await;
                result?; cleanup
            })
        };
        run().map_err(|error|error.to_string())
    })?.join().map_err(|_|"cake first raid test thread failed")??;
    Ok(())
}
