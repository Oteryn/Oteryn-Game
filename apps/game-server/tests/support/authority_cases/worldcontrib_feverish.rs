//! WCONTRIB: medicine's saved disease projection gates the existing native raid.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes;
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{RaidAction, RaidCatalogue, RaidSchedule};
use oteryn_gameplay_world_change::owner::ContributionKind;

#[test]
fn worldchg_postgres_swamp_epidemic_gates_native_feverish_raid() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start = std::time::Instant::now();
            let h = fixture::Harness::create(admin, "wcontrib_feverish", false).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            println!("WCONTRIB PG EXECUTED: saved medicine cure denies raid schedule, epidemic reaches native120-citizen wave; elapsed={:?}",start.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_| "feverish PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    use crate::durability::raid_runner::CommittedRaidAction;
    use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck};
    use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
    let world = fixture::scope()?.world_id();
    for epoch in 1..=3_i64 {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    h.root
        .world_changes(world, 1)
        .await?
        .ok_or("initial snapshot")?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let recovery = authority.record_for(&h.root).map_err(fixture::debug)?;
    let mut tx = h.pool.begin().await?;
    assert_eq!(
        changes::contribute(
            &mut tx,
            &recovery,
            world,
            1,
            [80; 16],
            ContributionKind::Medicine,
            100
        )
        .await?,
        changes::ContributionWrite::Applied(100)
    );
    tx.commit().await?;
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=2 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    h.root.world_changes(world, 2).await?.ok_or("saved cure")?;
    assert!(
        !h.root
            .shared_world_changes(world, 2)
            .await?
            .ok_or("shared cure")?
            .swamp_epidemic
    );
    let key = ProductionKey::new("raid:venore.feverishcitizens").map_err(fixture::debug)?;
    let catalogue = RaidCatalogue::embedded().map_err(fixture::debug)?;
    let selected = catalogue
        .get(&key)
        .ok_or("native feverish raid")?
        .select([7; 32], fixture::id(88))
        .map_err(fixture::debug)?;
    assert_eq!(selected.definition.waves[0].entries[0].count, 120);
    let mut before = RaidSchedule::initial(3, 0).map_err(fixture::debug)?;
    let (before, now) = (0..100_000)
        .find_map(|ordinal| {
            let now = 10_800_000 + i64::from(ordinal) * 60_000;
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
    let channel = ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let mut request = RaidSchedulerCheck {
        world,
        channel,
        generation,
        reset_epoch: 2,
        now_ms: now,
        bundle_digest: [7; 32],
        candidates: vec![RaidScheduleCandidate {
            key: key.clone(),
            definition: selected.definition.clone(),
            eligible: true,
            calendar: None,
            firing_id: fixture::id(88),
            global_margin_ms: 10_800_000,
            lua: None,
            calendar_window: None,
        }],
    };
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty()
    );
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_world_raid_schedules WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).fetch_one(&h.pool).await?;
    assert_eq!(
        count, 0,
        "cured day must not consume a native raid schedule/check"
    );
    sqlx::query(
        "UPDATE game_world_reset_active SET reset_epoch=3 WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    h.root
        .world_changes(world, 3)
        .await?
        .ok_or("saved epidemic")?;
    assert!(
        h.root
            .shared_world_changes(world, 3)
            .await?
            .ok_or("shared epidemic")?
            .swamp_epidemic
    );
    sqlx::query("UPDATE game_world_raid_scheduler_clocks SET reset_epoch=3,last_end_at_ms=0 WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_schedules(world_id,raid_key,revision,reset_epoch,check_ordinal,next_firing_ordinal,next_check_at_ms,last_fired_at_ms,last_fixed_slot_ms,retired) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,3,$4::text::numeric,1,$5,NULL,NULL,false)")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(before.revision.to_string())
        .bind(before.check_ordinal.to_string()).bind(before.next_check_at_ms).execute(&h.pool).await?;
    request.reset_epoch = 3;
    let outcome = h
        .root
        .check_raid_schedules(&authority, &h.node, request.clone())
        .await?;
    assert_eq!(outcome.firings, vec![(fixture::id(88), key)]);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request)
            .await?
            .firings
            .is_empty()
    );
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
        .ok_or("native disease announcement")?;
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
        .ok_or("native disease wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = wave.action else {
        return Err("wave action".into());
    };
    assert_eq!(creatures.len(), 120);
    assert!(creatures.iter().all(
        |c| c.creature.as_str() == "oteryn:creature.feverish_citizen"
            && c.area_anchor.as_str() == "raid-anchor:32861_32052_6-33012_32126_6"
    ));
    Ok(())
}
