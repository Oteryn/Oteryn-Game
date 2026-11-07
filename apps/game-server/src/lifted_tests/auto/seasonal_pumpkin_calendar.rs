//! Actual generated Pumpkin candidate through the durable calendar owner.
use crate::bestiary_postgres_harness::{CHANNEL, Harness, TestResult, WORLD, debug, id, runtime};
use crate::content::ProductionKey;
use crate::durability::raid_schedule::{
    CalendarRaidScheduleCandidate, RaidScheduleCandidate, RaidSchedulerCheck,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use oteryn_gameplay_encounter_rules::raid::{CatalogueSchedule, RaidCatalogue};
#[test]
fn srv7_pumpkin_calendar_first_open_then_four_hours_all_three_sources_on_postgres() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let work = || -> TestResult {
                runtime()?.block_on(async {
                    let h = Harness::create(admin, "srv7_pumpkin_calendar", false).await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
            };
            work().map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Pumpkin calendar thread failed")??;
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
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')").bind(world.as_bytes().as_slice()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let key = ProductionKey::new("oteryn:raid.northern_darama_raids.the_mutated_pumpkin")
        .map_err(debug)?;
    let catalogue = RaidCatalogue::embedded()?;
    let raid = catalogue
        .get(&key)
        .ok_or("existing source Pumpkin catalogue row")?;
    assert!(
        raid.active,
        "current-Tibia Pumpkin sources must have a real active scheduler caller"
    );
    let (window, interval, repeat) = match raid.schedule {
        CatalogueSchedule::Calendar {
            window: Some(w),
            interval_ms: Some(i),
            repeat,
        } => (w.window(), i, repeat),
        _ => return Err("source Pumpkin calendar".into()),
    };
    assert!(!repeat, "seasonal annual windows do not use the Flower startup cycle");
    assert_eq!(interval, 14_400_000);
    let mut points = std::collections::BTreeSet::new();
    for seed in 80u8..120 {
        let selected = raid.select([7; 32], id(seed))?;
        assert_eq!(selected.definition.announcements.len(), 1);
        assert_eq!(
            selected.definition.announcements[0].text,
            "Oh noes! It's a mutated pumpkin!"
        );
        for area in selected.areas.values() {
            assert_eq!(area.from, area.to);
            points.insert((area.from.x, area.from.y, area.from.floor));
        }
    }
    assert_eq!(
        points,
        std::collections::BTreeSet::from([(33115, 32398, 7), (33095, 32381, 7), (33092, 32418, 7)])
    );
    let selected = raid.select([7; 32], id(81))?;
    let start = 1_793_422_800_000;
    let mut request = RaidSchedulerCheck {
        world,
        channel,
        generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
        reset_epoch: 1,
        now_ms: start - 60_000,
        bundle_digest: [7; 32],
        candidates: vec![RaidScheduleCandidate {
            key: key.clone(),
            definition: selected.definition.clone(),
            eligible: true,
            calendar: None,
            firing_id: id(81),
            global_margin_ms: 0,
            lua: None,
            calendar_window: Some(CalendarRaidScheduleCandidate {
                window,
                interval_ms: interval,
                repeat,
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
    request.now_ms = start + 1000;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings,
        vec![(id(81), key.clone())],
        "first eligible post-open check, not invented four-hour initial delay"
    );
    // Scheduler-only fixture has no native slots; acknowledge truthful failed placements.
    // Physical placement/fight is covered by the real B3/runtime source case separately.
    finish_unplaced(
        h,
        request.channel,
        request.generation,
        id(81),
        selected.definition.clone(),
        request.now_ms,
    )
    .await?;
    request.now_ms = start + 14_340_000;
    request.candidates[0].firing_id = id(82);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty(),
        "four hours have not elapsed from the actual firing"
    );
    request.now_ms = start + 14_400_000 + 1000;
    assert_eq!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings,
        vec![(id(82), key.clone())],
        "persistent actors do not suppress the next source wave"
    );
    finish_unplaced(
        h,
        request.channel,
        request.generation,
        id(82),
        selected.definition.clone(),
        request.now_ms,
    )
    .await?;
    request.now_ms = start + 3 * 86_400_000;
    request.candidates[0].firing_id = id(83);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request)
            .await?
            .firings
            .is_empty(),
        "November3 save closes Pumpkin event"
    );
    Ok(())
}
async fn finish_unplaced(
    h: &Harness,
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
    firing: [u8; 16],
    definition: oteryn_gameplay_encounter_rules::raid::RaidDefinition,
    now: i64,
) -> TestResult {
    use crate::durability::raid_runner::{CommittedRaidAction, RaidPlacementOutcome};
    for _ in 0..8 {
        let step = h
            .root
            .advance_raid_run(
                &h.node,
                firing,
                channel,
                generation,
                definition.clone(),
                now,
            )
            .await?
            .ok_or("current source raid step")?;
        let outcomes = match &step.action {
            CommittedRaidAction::Wave { creatures, .. } => creatures
                .iter()
                .map(|c| RaidPlacementOutcome {
                    identity: c.identity,
                    placed: false,
                })
                .collect(),
            _ => Vec::new(),
        };
        let complete = matches!(step.action, CommittedRaidAction::Complete { .. });
        h.root
            .acknowledge_raid_effect(&h.node, step.identity, outcomes, now)
            .await?;
        if complete {
            return Ok(());
        }
    }
    Err("source raid did not complete its bounded acknowledgement sequence".into())
}
