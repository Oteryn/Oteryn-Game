//! Accepted W15 daily predicate gates the unmodified native Stampede raid.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes;
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{RaidAction, RaidCatalogue, RaidSchedule};
use oteryn_gameplay_world_change::owner::is_active;

#[test]
fn worldchg_postgres_stampede_daily_gate_reaches_existing_raid_runner() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start = std::time::Instant::now();
            let h = fixture::Harness::create(admin, "worldchg_stampede", false).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            println!("WORLDCHG-Q PG EXECUTED: Stampede inactive/active epochs, native raid RNG,16-elephant wave,reload,inactive Furyosa denied; elapsed={:?}", start.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_| "stampede PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    use crate::durability::raid_runner::CommittedRaidAction;
    use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck};
    use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
    let world = fixture::scope()?.world_id();
    // Locate adjacent real deterministic days, without changing selection or stored decisions.
    let inactive = (1..1000_u64).find(|epoch| {
        matches!((changes::roll(world,*epoch,0), changes::roll(world,*epoch+1,0)), (Ok(a),Ok(b)) if !a.stampede && b.stampede && a.fury_gates.is_none() && b.fury_gates.is_none())
    }).ok_or("adjacent inactive/active days")?;
    let active = inactive + 1;
    for epoch in [inactive, active] {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query(
        "INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric)",
    )
    .bind(world.as_bytes().as_slice())
    .bind(inactive.to_string())
    .execute(&h.pool)
    .await?;
    let day = h
        .root
        .world_changes(world, inactive)
        .await?
        .ok_or("inactive snapshot")?;
    assert!(!is_active(&day, "stampede"));
    let key = ProductionKey::new("raid:porthope.stampede").map_err(fixture::debug)?;
    let catalogue = RaidCatalogue::embedded().map_err(fixture::debug)?;
    let selected = catalogue
        .get(&key)
        .ok_or("Stampede raid")?
        .select([7; 32], fixture::id(88))
        .map_err(fixture::debug)?;
    assert_eq!(selected.definition.waves[0].entries[0].count, 16);
    // Compute the native RNG's first winning check in memory. Seed only the
    // independently validated pre-check fixture, so PostgreSQL needs one due pass.
    // The sourced definition, random policy, 24h interval and 6h margin are unchanged.
    let mut before = RaidSchedule::initial(active, 0).map_err(fixture::debug)?;
    let (before, now) = (0..100_000)
        .find_map(|ordinal| {
            let now = 21_600_000 + i64::from(ordinal) * 60_000;
            let transition = before
                .plan(
                    world,
                    &key,
                    &selected.definition,
                    RaidAction::Check {
                        now_ms: now,
                        eligible: true,
                        calendar: None,
                        firing_id: fixture::id(88),
                        bundle_digest: [7; 32],
                    },
                )
                .ok()?;
            if transition.after().latest_firing.is_some() {
                Some((before.clone(), now))
            } else {
                before = transition.after().clone();
                None
            }
        })
        .ok_or("native winning check")?;
    before.validate().map_err(fixture::debug)?;
    let channel = ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?;
    let mut request = RaidSchedulerCheck {
        world,
        channel,
        generation: ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?,
        reset_epoch: inactive,
        now_ms: now,
        bundle_digest: [7; 32],
        candidates: vec![RaidScheduleCandidate {
            key: key.clone(),
            definition: selected.definition.clone(),
            eligible: true,
            calendar: None,
            firing_id: fixture::id(88),
            global_margin_ms: 21_600_000,
            lua: None,
            calendar_window: None,
        }],
    };
    // The accepted Furyosa W6 precondition stays false while its current static layer is inactive.
    // Its real sourced definition precedes Stampede to catch any ungated schedule mutation.
    let fury_key = ProductionKey::new("raid:furyfate.furyosa").map_err(fixture::debug)?;
    let fury = catalogue
        .get(&fury_key)
        .ok_or("Furyosa source")?
        .select([7; 32], fixture::id(89))
        .map_err(fixture::debug)?;
    request.candidates.insert(
        0,
        RaidScheduleCandidate {
            key: fury_key.clone(),
            definition: fury.definition,
            eligible: true,
            calendar: None,
            firing_id: fixture::id(89),
            global_margin_ms: 21_600_000,
            lua: None,
            calendar_window: None,
        },
    );
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty()
    );
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_world_raid_schedules WHERE world_id=encode($1,'hex')::uuid AND raid_key IN($2,$3)")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(fury_key.as_str()).fetch_one(&h.pool).await?;
    assert_eq!(
        count, 0,
        "inactive daily change must stop before schedule/RNG mutation"
    );
    sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$2::text::numeric WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).bind(active.to_string()).execute(&h.pool).await?;
    let day = h
        .root
        .world_changes(world, active)
        .await?
        .ok_or("active snapshot")?;
    assert!(is_active(&day, "stampede"));
    assert_eq!(h.root.world_changes(world, active).await?, Some(day));
    assert!(h.root.world_changes(world, inactive).await?.is_none());
    sqlx::query("UPDATE game_world_raid_scheduler_clocks SET reset_epoch=$2::text::numeric,last_end_at_ms=0 WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).bind(active.to_string()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_schedules(world_id,raid_key,revision,reset_epoch,check_ordinal,next_firing_ordinal,next_check_at_ms,last_fired_at_ms,last_fixed_slot_ms,retired) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4::text::numeric,$5::text::numeric,1,$6,NULL,NULL,false)")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(before.revision.to_string()).bind(active.to_string()).bind(before.check_ordinal.to_string()).bind(before.next_check_at_ms).execute(&h.pool).await?;
    request.reset_epoch = active;
    let outcome = h
        .root
        .check_raid_schedules(&authority, &h.node, request.clone())
        .await?;
    assert_eq!(outcome.firings, vec![(fixture::id(88), key)]);
    let fury_schedules:i64=sqlx::query_scalar("SELECT count(*) FROM game_world_raid_schedules WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
        .bind(world.as_bytes().as_slice()).bind(fury_key.as_str()).fetch_one(&h.pool).await?;
    assert_eq!(
        fury_schedules, 0,
        "inactive Furyosa must not consume a check or create a schedule"
    );
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request)
            .await?
            .firings
            .is_empty()
    );
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let notice = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(88),
            channel,
            generation,
            selected.definition.clone(),
            now + 1000,
        )
        .await?
        .ok_or("source announcement")?;
    assert!(matches!(
        notice.action,
        CommittedRaidAction::Announcement { .. }
    ));
    h.root
        .acknowledge_raid_effect(&h.node, notice.identity, vec![], now + 1000)
        .await?;
    let wave = h
        .root
        .advance_raid_run(
            &h.node,
            fixture::id(88),
            channel,
            generation,
            selected.definition,
            now + 2000,
        )
        .await?
        .ok_or("native wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = wave.action else {
        return Err("wave action".into());
    };
    assert_eq!(creatures.len(), 16);
    assert!(creatures.iter().all(
        |c| c.creature.as_str() == "oteryn:creature.terrified_elephant"
            && c.area_anchor.as_str() == "raid-anchor:32825_32667_7-32888_32675_7"
    ));
    drop(authority);
    drop(seal);
    Ok(())
}
