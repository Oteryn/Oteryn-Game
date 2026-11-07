//! W05d: the real generated Halloween candidate on the durable server-save calendar.
use crate::bestiary_postgres_harness::{CHANNEL, Harness, TestResult, WORLD, debug, id, runtime};
use crate::content::ProductionKey;
use crate::durability::raid_schedule::{
    CalendarRaidScheduleCandidate, RaidScheduleCandidate, RaidSchedulerCheck,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use oteryn_gameplay_encounter_rules::raid::{CatalogueSchedule, RaidCatalogue};

#[test]
fn srv7_halloween_raid_waits_for_server_save_and_fires_once_on_postgres() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                runtime()?.block_on(async {
                    let h = Harness::create(admin, "srv7_halloween_calendar", false).await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "seasonal raid calendar thread failed")??;
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
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let key =
        ProductionKey::new("oteryn:raid.kazordoon_raids.the_halloween_hare").map_err(debug)?;
    let catalogue = RaidCatalogue::embedded()?;
    let raid = catalogue.get(&key).ok_or("source Halloween raid")?;
    let (window, interval_ms) = match raid.schedule {
        CatalogueSchedule::Calendar {
            window: Some(window),
            interval_ms: Some(interval),
        } => (window.window(), interval),
        _ => return Err("source Halloween calendar".into()),
    };
    let selected = raid.select([7; 32], id(81))?;
    // Oct 31 2026 06:00 Berlin = 05:00 UTC. The preceding wall date is already
    // October31, but belongs to the October30 reset and cannot fire Halloween.
    let start = 1_793_422_800_000;
    let mut request = RaidSchedulerCheck {
        world,
        channel,
        generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        reset_epoch: 1,
        now_ms: start - 120_000,
        bundle_digest: [7; 32],
        candidates: vec![RaidScheduleCandidate {
            key: key.clone(),
            definition: selected.definition,
            eligible: true,
            calendar: None,
            firing_id: id(81),
            global_margin_ms: 0,
            lua: None,
            calendar_window: Some(CalendarRaidScheduleCandidate {
                window,
                interval_ms,
            }),
        }],
    };
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty()
    );
    request.now_ms = start - 60_000;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty(),
        "Halloween cannot fire before the October31 server save"
    );
    request.now_ms = start;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings,
        vec![(id(81), key)]
    );
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty(),
        "duplicate check replays without another firing"
    );
    request.now_ms += 60_000;
    request.candidates[0].firing_id = id(82);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty(),
        "annual source is one wave in the reset epoch"
    );
    request.now_ms = start + 86_400_000 - 1;
    request.candidates[0].firing_id = id(83);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty(),
        "midnight does not create a second wave before the next server save"
    );
    request.now_ms = start + 86_400_000;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty(),
        "November1 save is outside the source Halloween Hare day"
    );
    let mut setup = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *setup)
        .await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,2,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($3::double precision/1000.0),'2027-10')").bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).bind(1824958800000_i64).execute(&mut *setup).await?;
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&mut *setup)
    .await?;
    setup.commit().await?;
    request.reset_epoch = 2;
    request.now_ms = 1824958800000;
    request.candidates[0].firing_id = id(84);
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .len(),
        1,
        "the next annual occurrence is reachable after its new save epoch"
    );
    request.candidates[0].firing_id = id(85);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request)
            .await?
            .firings
            .is_empty()
    );
    let persisted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(persisted, 2);
    eprintln!(
        "SRV7 Halloween real generated candidate committed one firing and rejected pre-save/replay"
    );
    Ok(())
}
