//! Source-authorized Flower Month uses the actual durable World raid scheduler.
use crate::bestiary_postgres_harness::{CHANNEL, Harness, TestResult, WORLD, debug, id, runtime};
use crate::content::ProductionKey;
use crate::durability::raid_schedule::{
    CalendarRaidScheduleCandidate, RaidScheduleCandidate, RaidSchedulerCheck,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use oteryn_gameplay_encounter_rules::raid::{CatalogueSchedule, RaidCatalogue};

#[test]
fn world_src_flower_postgres_repeats_donor_cycle_without_duplicate_firing() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    for active_june in [false, true] {
                        let h = Harness::create(
                            admin.clone(),
                            if active_june {
                                "world_src_flower_june"
                            } else {
                                "world_src_flower_may"
                            },
                            false,
                        )
                        .await?;
                        let result = exercise(&h, active_june).await;
                        let cleanup = h.cleanup().await;
                        result?;
                        cleanup?;
                    }
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "flower case thread")?
        .map_err(Box::<dyn std::error::Error>::from)?;
    eprintln!("POSTGRES_CASE_COMPLETED WORLD-SRC flower-cycle repeat-and-replay");
    Ok(())
}
async fn exercise(h: &Harness, active_june: bool) -> TestResult {
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let key = ProductionKey::new("oteryn:raid.raid.dryad_invasion").map_err(debug)?;
    let raid = catalogue.get(&key).ok_or("flower record")?;
    assert!(
        raid.active,
        "Flower donor cycle must reach the production scheduler"
    );
    let selected = raid.select([7; 32], id(81)).map_err(debug)?;
    assert_eq!(selected.definition.waves.len(), 4);
    for (index, wave) in selected.definition.waves.iter().enumerate() {
        assert_eq!(wave.delay_ms, index as u32 * 900000);
        assert_eq!(wave.entries[0].count, 25);
        assert_eq!(wave.entries[0].creature.as_str(), "oteryn:creature.dryad");
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($3::double precision/1000.0),$4)").bind(world.as_bytes().as_slice()).bind([7u8;32].as_slice()).bind(if active_june {1780286400000_i64} else {1780200000000_i64}).bind(if active_june {"2026-06"} else {"2026-05"}).execute(&h.pool).await?;
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
        return Err("flower calendar".into());
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
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty()
    );
    request.now_ms += 60000;
    if !active_june {
        assert!(
            h.root
                .check_raid_schedules(&authority, &h.node, request.clone())
                .await?
                .firings
                .is_empty(),
            "wall June cannot activate a still-committed May31epoch"
        );
        return Ok(());
    }
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .len(),
        1
    );
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty()
    );
    // A completed source cycle keeps its ordinary creatures but does not suppress the next cycle.
    sqlx::query("UPDATE game_world_raid_firings SET state=2 WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    request.now_ms += 3600000;
    request.candidates[0].firing_id = id(82);
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .len(),
        1
    );
    let firings: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(firings, 2);
    Ok(())
}
