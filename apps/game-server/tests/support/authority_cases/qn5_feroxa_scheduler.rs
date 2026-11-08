//! Feroxa is a full-moon encounter; its boss lifetime cannot lease the legacy raid queue.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::raid_runner::{CommittedRaidAction, RaidPlacementOutcome};
use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{
    RaidAction, RaidCatalogue, RaidDefinition, RaidPolicy, RaidSchedule, Wave, WaveEntry,
};

#[test]
fn qn5_feroxa_lifetime_does_not_serialize_ordinary_legacy_raids() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start = std::time::Instant::now();
            for (day, expire_offline) in [(12, false), (13, false), (13, true)] {
                let h = fixture::Harness::create(admin.clone(), "qn5_feroxa_scheduler", false).await?;
                let result = exercise(&h, day, expire_offline).await;
                h.cleanup().await?;
                result?;
            }
            assert!(start.elapsed().as_secs_f64() > 0.01);
            eprintln!("QN5_M52_PG_EXECUTED independent Feroxa and serialized legacy queue duration={:?}", start.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_| "Feroxa scheduler case panicked")?.map_err(Into::into)
}

async fn exercise(h: &fixture::Harness, day: i32, expire_offline: bool) -> fixture::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QN5_M52_POSTGRES_READY version={version} day={day}");
    let world = fixture::scope()?.world_id();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,$3,12,0,0,'UTC'),'2031-06')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).bind(day).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let changes = h.root.world_changes(world, 1).await?.ok_or("World day")?;
    assert_eq!(changes.grimvale_full_moon, day == 13);
    sqlx::query("INSERT INTO game_world_raid_scheduler_clocks(world_id,reset_epoch,last_end_at_ms) VALUES(encode($1,'hex')::uuid,1,0)")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let key = ProductionKey::new("raid:edron.feroxa").map_err(fixture::debug)?;
    let catalogue = RaidCatalogue::embedded().map_err(fixture::debug)?;
    let source = catalogue
        .get(&key)
        .ok_or("Feroxa native source")?
        .select([7; 32], fixture::id(88))
        .map_err(fixture::debug)?;
    // Keep the native Feroxa probability/definition. Seed only its validated
    // pre-winning schedule so this case isolates queue ownership in one due pass.
    let mut before = RaidSchedule::initial(1, 0).map_err(fixture::debug)?;
    let (before, now) = (1..100_000_i64)
        .find_map(|ordinal| {
            let now = ordinal * 60_000;
            let transition = before
                .plan(
                    world,
                    &key,
                    &source.definition,
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
        .ok_or("native Feroxa winning check")?;
    before.validate().map_err(fixture::debug)?;
    let feroxa_definition = source.definition.clone();
    let ordinary = RaidDefinition {
        policy: RaidPolicy::Random {
            mean_interval_s: 60,
            min_gap_s: 0,
        },
        despawn_after_ms: None,
        announcements: vec![],
        waves: vec![Wave {
            delay_ms: 0,
            entries: vec![WaveEntry {
                creature: ProductionKey::new("oteryn:creature.badger").map_err(fixture::debug)?,
                count: 1,
                area_anchor: ProductionKey::new("raid-anchor:qn5").map_err(fixture::debug)?,
            }],
        }],
    };
    ordinary.validate().map_err(fixture::debug)?;
    let mut candidates = vec![];
    for (key, definition, prior, firing) in [
        (key, source.definition, before, 88),
        (
            ProductionKey::new("raid:qn5.ordinary_a").map_err(fixture::debug)?,
            ordinary.clone(),
            RaidSchedule::initial(1, 0).map_err(fixture::debug)?,
            89,
        ),
        (
            ProductionKey::new("raid:qn5.ordinary_b").map_err(fixture::debug)?,
            ordinary,
            RaidSchedule::initial(1, 0).map_err(fixture::debug)?,
            90,
        ),
    ] {
        sqlx::query("INSERT INTO game_world_raid_schedules(world_id,raid_key,revision,reset_epoch,check_ordinal,next_firing_ordinal,next_check_at_ms,last_fired_at_ms,last_fixed_slot_ms,retired) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,1,$4::text::numeric,1,$5,NULL,NULL,false)")
            .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(prior.revision.to_string()).bind(prior.check_ordinal.to_string()).bind(prior.next_check_at_ms).execute(&h.pool).await?;
        candidates.push(RaidScheduleCandidate {
            key,
            definition,
            eligible: true,
            calendar: None,
            firing_id: fixture::id(firing),
            global_margin_ms: if firing == 88 { u32::MAX } else { 0 },
            lua: None,
            calendar_window: None,
        });
    }
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let request = RaidSchedulerCheck {
        world,
        channel: ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?,
        generation: ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?,
        reset_epoch: 1,
        now_ms: now,
        bundle_digest: [7; 32],
        candidates,
    };
    let result = h
        .root
        .check_raid_schedules(&authority, &h.node, request.clone())
        .await?;
    let keys: Vec<_> = result.firings.iter().map(|(_, key)| key.as_str()).collect();
    assert!(
        keys.contains(&"raid:qn5.ordinary_a"),
        "the live full-moon boss must not lease the legacy queue: {keys:?}"
    );
    assert!(
        !keys.contains(&"raid:qn5.ordinary_b"),
        "ordinary legacy raids remain exclusive"
    );
    assert_eq!(
        keys.contains(&"raid:edron.feroxa"),
        day == 13,
        "the independent gate still requires the current full moon"
    );
    // The next pass reloads the persisted running set. Close only the ordinary
    // fixture; the real Feroxa firing stays active across both scheduler passes.
    sqlx::query("UPDATE game_world_raid_firings SET state=2 WHERE world_id=encode($1,'hex')::uuid AND raid_key='raid:qn5.ordinary_a'")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    let mut next = request.clone();
    next.now_ms += 60_000;
    next.candidates
        .retain(|candidate| candidate.key.as_str() == "raid:qn5.ordinary_b");
    let second = h
        .root
        .check_raid_schedules(&authority, &h.node, next)
        .await?;
    assert_eq!(
        second.firings.len(),
        1,
        "persisted Feroxa must not block the next ordinary raid"
    );
    assert_eq!(second.firings[0].1.as_str(), "raid:qn5.ordinary_b");
    // Leave only Feroxa active when measuring its completion clock effect.
    sqlx::query("UPDATE game_world_raid_firings SET state=2 WHERE world_id=encode($1,'hex')::uuid AND raid_key='raid:qn5.ordinary_b'")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    if day == 13 {
        let last_wave_ms = feroxa_definition
            .waves
            .iter()
            .map(|wave| wave.delay_ms)
            .max()
            .ok_or("Feroxa wave")?;
        let expired_at = now + i64::from(last_wave_ms) + 72_000_000;
        if expire_offline {
            let mut expiry = request;
            expiry.now_ms = expired_at;
            expiry.candidates.clear();
            h.root
                .check_raid_schedules(&authority, &h.node, expiry)
                .await?;
        } else {
            for _ in 0..32 {
                let step = h
                    .root
                    .advance_raid_run(
                        &h.node,
                        fixture::id(88),
                        request.channel,
                        request.generation,
                        feroxa_definition.clone(),
                        expired_at,
                    )
                    .await?
                    .ok_or("Feroxa due lifecycle step")?;
                let outcomes = match &step.action {
                    CommittedRaidAction::Wave { creatures, .. } => creatures
                        .iter()
                        .map(|creature| RaidPlacementOutcome {
                            identity: creature.identity,
                            placed: true,
                        })
                        .collect(),
                    _ => vec![],
                };
                let finished = matches!(step.action, CommittedRaidAction::Complete { .. });
                h.root
                    .acknowledge_raid_effect(&h.node, step.identity, outcomes, expired_at)
                    .await?;
                if finished {
                    break;
                }
            }
        }
        let state: i16 = sqlx::query_scalar(
            "SELECT state FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid",
        )
        .bind(fixture::id(88).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(state, 2, "both ACK and offline deadline close Feroxa");
        let margin: i64 = sqlx::query_scalar("SELECT last_end_at_ms FROM game_world_raid_scheduler_clocks WHERE world_id=encode($1,'hex')::uuid")
            .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            margin, 0,
            "Feroxa completion must not delay ordinary raids; offline={expire_offline}"
        );
    }
    Ok(())
}
