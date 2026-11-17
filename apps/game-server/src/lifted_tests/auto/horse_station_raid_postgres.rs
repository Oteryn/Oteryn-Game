use crate::bestiary_postgres_harness::{CHANNEL, Harness, TestResult, WORLD, debug, id, runtime};
use crate::content::ProductionKey;
use crate::durability::raid_runner::{CommittedRaidAction, RaidPlacementOutcome};
use crate::durability::raid_schedule::{
    CalendarRaidScheduleCandidate, RaidScheduleCandidate, RaidSchedulerCheck,
};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId};
use oteryn_gameplay_encounter_rules::raid::{CatalogueSchedule, RaidCatalogue};

#[test]
fn horse_station_postgres_scheduler_fires_only_while_loose() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let h = Harness::create(admin, "horse_station_raid", false).await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "horse station raid thread")?
        .map_err(Box::<dyn std::error::Error>::from)?;
    println!(
        "PASS horse_station_postgres_scheduler_fires_only_while_loose elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
async fn exercise(h: &Harness) -> TestResult {
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let key = ProductionKey::new("oteryn:raid.horse_station").map_err(debug)?;
    let raid = catalogue.get(&key).ok_or("horse station raid")?;
    assert!(
        raid.active,
        "Horse Station must reach the production scheduler"
    );
    let selected = raid.select([7; 32], id(81)).map_err(debug)?;
    assert_eq!(selected.areas.len(), 2);
    assert_eq!(selected.definition.waves.len(), 1);
    let wild: u16 = selected.definition.waves[0]
        .entries
        .iter()
        .filter(|entry| entry.creature.as_str() == "oteryn:creature.wild_horse")
        .map(|entry| entry.count)
        .sum();
    assert_eq!(wild, 6);
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($3::double precision/1000.0),$4)").bind(world.as_bytes().as_slice()).bind([7u8;32].as_slice()).bind(1780286400000_i64).bind("2026-06").execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let CatalogueSchedule::Calendar {
        window,
        interval_ms,
        repeat,
    } = &raid.schedule
    else {
        return Err("horse station calendar".into());
    };
    let mut request = RaidSchedulerCheck {
        world,
        channel,
        generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        reset_epoch: 1,
        now_ms: 1780315200000,
        bundle_digest: [7; 32],
        candidates: vec![RaidScheduleCandidate {
            key,
            definition: selected.definition,
            eligible: true,
            calendar: None,
            firing_id: id(81),
            global_margin_ms: 0,
            lua: None,
            calendar_window: Some(CalendarRaidScheduleCandidate {
                window: window.ok_or("window")?.window(),
                interval_ms: interval_ms.ok_or("interval")?,
                repeat: *repeat,
            }),
        }],
    };
    for _ in 0..2 {
        assert!(
            h.root
                .check_raid_schedules(&authority, &h.node, request.clone())
                .await?
                .firings
                .is_empty()
        );
        request.now_ms += 60_000;
    }
    sqlx::query("INSERT INTO game_horse_stations(world_id,loose_until_ms) VALUES(encode($1,'hex')::uuid,$2)")
        .bind(world.as_bytes().as_slice())
        .bind(request.now_ms + 4 * 24 * 3_600_000)
        .execute(&h.pool).await?;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .len(),
        1
    );
    let firing = request.candidates[0].firing_id;
    let wave = h
        .root
        .advance_raid_run(
            &h.node,
            firing,
            channel,
            request.generation,
            request.candidates[0].definition.clone(),
            request.now_ms,
        )
        .await?
        .ok_or("wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = wave.action else {
        return Err("first effect must be the horse wave".into());
    };
    assert_eq!(creatures.len(), 24);
    assert_eq!(
        creatures
            .iter()
            .filter(|plan| plan.creature.as_str() == "oteryn:creature.wild_horse")
            .count(),
        6
    );
    let outcomes = creatures
        .into_iter()
        .map(|plan| RaidPlacementOutcome {
            identity: plan.identity,
            placed: false,
        })
        .collect();
    h.root
        .acknowledge_raid_effect(&h.node, wave.identity, outcomes, request.now_ms)
        .await?;
    let complete = h
        .root
        .advance_raid_run(
            &h.node,
            firing,
            channel,
            request.generation,
            request.candidates[0].definition.clone(),
            request.now_ms,
        )
        .await?
        .ok_or("completion")?;
    assert_eq!(
        complete.action,
        CommittedRaidAction::Complete { cleanup: false }
    );
    h.root
        .acknowledge_raid_effect(&h.node, complete.identity, Vec::new(), request.now_ms)
        .await?;
    request.candidates[0].firing_id = id(82);
    request.now_ms += 10_740_000;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty()
    );
    request.now_ms += 60_000;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .len(),
        1
    );
    let stable_key = ProductionKey::new("oteryn:raid.horse_station_stables").map_err(debug)?;
    let stables = catalogue.get(&stable_key).ok_or("generated stable wave")?;
    let stable_firing = id(83);
    request.candidates[0].key = stable_key;
    request.candidates[0].definition = stables
        .select([7; 32], stable_firing)
        .map_err(debug)?
        .definition;
    request.candidates[0].firing_id = stable_firing;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty()
    );
    sqlx::query("UPDATE game_horse_stations SET loose_until_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint+345600000 WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    for draw in [1, 0] {
        assert!(
            h.root
                .settle_horse_lure(
                    &authority,
                    &h.node,
                    RuntimeScopeRefV1::channel(world, channel),
                    request.generation,
                    request.reset_epoch,
                    draw,
                )
                .await?
        );
        let confined: bool = sqlx::query_scalar(
            "SELECT loose_until_ms IS NULL FROM game_horse_stations WHERE world_id=encode($1,'hex')::uuid",
        )
        .bind(world.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(confined, draw == 0);
    }
    request.now_ms = sqlx::query_scalar(
        "SELECT confined_at_ms FROM game_horse_stations WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .len(),
        1
    );
    let stables = h
        .root
        .advance_raid_run(
            &h.node,
            stable_firing,
            channel,
            request.generation,
            request.candidates[0].definition.clone(),
            request.now_ms,
        )
        .await?
        .ok_or("stable wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = stables.action else {
        return Err("expected the stable repopulation wave".into());
    };
    assert_eq!(creatures.len(), 9);
    assert!(
        creatures
            .iter()
            .all(|plan| plan.creature.as_str() != "oteryn:creature.wild_horse")
    );
    sqlx::query(
        "UPDATE game_horse_stations SET loose_until_ms=$2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .bind(request.now_ms + 4 * 24 * 3_600_000)
    .execute(&h.pool)
    .await?;
    let cleanup = h
        .root
        .claim_lost_raid_run(
            &h.node,
            stable_firing,
            channel,
            request.generation,
            request.now_ms,
        )
        .await?
        .ok_or("stable cleanup on escape")?;
    assert_eq!(cleanup.action, CommittedRaidAction::Lost);
    Ok(())
}
