//! Crystal 9f5a72c grimvale.lua checks stored Chance before drawing its replacement.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::raid_schedule::{RaidScheduleCandidate, RaidSchedulerCheck};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::RaidCatalogue;

#[test]
fn qn5_feroxa_unset_chance_selects_first_due_full_moon_check() -> fixture::TestResult {
    run_case(false)
}

#[test]
fn qn5_feroxa_active_clock_is_fenced_and_independent_of_pending_delivery() -> fixture::TestResult {
    run_case(true)
}

fn run_case(prove_clock: bool) -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let start = std::time::Instant::now();
                    for initial_chance in if prove_clock {
                        vec![None]
                    } else {
                        vec![None, Some(10)]
                    } {
                        let tag = if prove_clock {
                            "qn5_feroxa_clock"
                        } else {
                            "qn5_feroxa_occurrence"
                        };
                        let h = fixture::Harness::create(admin.clone(), tag, false).await?;
                        let result = exercise(&h, initial_chance, prove_clock).await;
                        h.cleanup().await?;
                        result?;
                    }
                    assert!(start.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "QN5_M52_OCCURRENCE_PG_EXECUTED elapsed={:?}",
                        start.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "Feroxa occurrence case panicked")?
        .map_err(Into::into)
}

async fn exercise(
    h: &fixture::Harness,
    initial_chance: Option<i16>,
    prove_clock: bool,
) -> fixture::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QN5_M52_OCCURRENCE_POSTGRES_READY version={version}");
    let world = fixture::scope()?.world_id();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,make_timestamptz(2031,6,13,12,0,0,'UTC'),'2031-06')")
        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    let key = ProductionKey::new("raid:edron.feroxa").map_err(fixture::debug)?;
    let source = RaidCatalogue::embedded()
        .map_err(fixture::debug)?
        .get(&key)
        .ok_or("native Feroxa")?
        .select([7; 32], fixture::id(88))
        .map_err(fixture::debug)?;
    let mut request = RaidSchedulerCheck {
        world,
        channel: ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?,
        generation: ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?,
        reset_epoch: 1,
        now_ms: 0,
        bundle_digest: [7; 32],
        candidates: vec![RaidScheduleCandidate {
            key,
            definition: source.definition,
            eligible: true,
            calendar: None,
            firing_id: fixture::id(88),
            global_margin_ms: 0,
            lua: None,
            calendar_window: None,
        }],
    };
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
    if let Some(chance) = initial_chance {
        sqlx::query("INSERT INTO game_world_feroxa_occurrences(world_id,reset_epoch,chance) VALUES(encode($1,'hex')::uuid,1,$2)")
            .bind(world.as_bytes().as_slice()).bind(chance).execute(&h.pool).await?;
    }
    let mut prior_chance = initial_chance.unwrap_or(-1);
    let mut selected = false;
    for minute in 1..=100 {
        request.now_ms = minute * 60_000;
        let chosen = h
            .root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?;
        assert_eq!(
            chosen.firings.len(),
            usize::from(prior_chance <= 5),
            "selection tests the preceding stored chance, including unset -1, before drawing its replacement"
        );
        let state: (i16, i16, Option<i64>) = sqlx::query_as("SELECT chance,phase,selected_at_ms FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
            .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert!(
            h.root
                .check_raid_schedules(&authority, &h.node, request.clone())
                .await?
                .firings
                .is_empty(),
            "same due check cannot select or draw twice"
        );
        let replay: (i16, i16, Option<i64>) = sqlx::query_as("SELECT chance,phase,selected_at_ms FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
            .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            state, replay,
            "a replay retains the exact persisted chance and Active state"
        );
        if prior_chance <= 5 {
            assert_eq!(chosen.firings[0].1.as_str(), "raid:edron.feroxa");
            assert_eq!(state, (prior_chance, 1, Some(request.now_ms)));
            selected = true;
            break;
        }
        assert!(
            (1..=10).contains(&state.0),
            "source replacement draw is inclusive 1..10"
        );
        assert_eq!((state.1, state.2), (0, None));
        prior_chance = state.0;
    }
    assert!(
        selected,
        "the fixed deterministic sequence reaches a qualifying prior chance"
    );
    let selected_at = request.now_ms;
    if prove_clock {
        exercise_clock(h, &request, selected_at).await?;
        // Hold the actual firing while the scheduler reaches its write. It must
        // not hold the occurrence row, which the runner locks after this firing.
        let mut locked = h.pool.begin().await?;
        let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *locked)
            .await?;
        sqlx::query("SELECT 1 FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(fixture::id(88).as_slice()).execute(&mut *locked).await?;
        request.now_ms = selected_at + 2_760_000;
        let scheduler = h
            .root
            .check_raid_schedules(&authority, &h.node, request.clone());
        let probe = async move {
            let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                    .bind(blocker).fetch_one(&h.pool).await?;
                if blocked {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err("scheduler did not reach held firing".into());
                }
                tokio::task::yield_now().await;
            }
            sqlx::query("SELECT 1 FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1 FOR UPDATE NOWAIT")
                .bind(request.world.as_bytes().as_slice()).execute(&mut *locked).await?;
            locked.rollback().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        // This target deliberately has no Tokio macro feature. Drive the
        // scheduler alongside the database probe without adding a dependency.
        let mut scheduler = std::pin::pin!(scheduler);
        let mut probe = std::pin::pin!(probe);
        let mut completed = None;
        std::future::poll_fn(|cx| {
            if completed.is_none()
                && let std::task::Poll::Ready(result) =
                    std::future::Future::poll(scheduler.as_mut(), cx)
            {
                completed = Some(result);
            }
            std::future::Future::poll(probe.as_mut(), cx)
        })
        .await?;
        let scheduled = match completed {
            Some(result) => result,
            None => scheduler.await,
        };
        assert!(scheduled?.firings.is_empty());
        return Ok(());
    }
    // Source Active remains set after the boss is gone, until the next reset.
    sqlx::query(
        "UPDATE game_world_raid_firings SET state=2 WHERE firing_id=encode($1,'hex')::uuid",
    )
    .bind(fixture::id(88).as_slice())
    .execute(&h.pool)
    .await?;
    request.now_ms += 60_000;
    request.candidates[0].firing_id = fixture::id(89);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request.clone())
            .await?
            .firings
            .is_empty(),
        "a completed Feroxa does not clear source Active in the same occurrence"
    );
    // A pre-migration firing has no Feroxa state row. Its admitted identity
    // still establishes Active, even if the boss completed before the upgrade.
    sqlx::query("DELETE FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
    request.now_ms += 60_000;
    request.candidates[0].firing_id = fixture::id(90);
    assert!(
        h.root
            .check_raid_schedules(&authority, &h.node, request)
            .await?
            .firings
            .is_empty(),
        "upgrading an existing occurrence cannot spawn a second Feroxa"
    );
    let upgraded: (i16, Option<i64>) = sqlx::query_as("SELECT phase,selected_at_ms FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(upgraded, (1, Some(selected_at)));
    Ok(())
}

async fn exercise_clock(
    h: &fixture::Harness,
    request: &RaidSchedulerCheck,
    selected_at: i64,
) -> fixture::TestResult {
    let mut receipt = None;
    for (elapsed, expected) in [
        (1_799_999, 1_i16),
        (1_800_000, 2),
        (2_699_999, 2),
        (2_700_000, 3),
        (1_800_000, 3),
    ] {
        if expected == 2 {
            // Change only the current ownership generation; a stale owner cannot
            // use a future timer to advance this World occurrence.
            assert!(
                h.root
                    .advance_raid_run(
                        &h.node,
                        fixture::id(88),
                        request.channel,
                        ScopeOwnershipGeneration::new(2).map_err(fixture::debug)?,
                        request.candidates[0].definition.clone(),
                        selected_at + 2_700_000
                    )
                    .await?
                    .is_none()
            );
        }
        let pending = h
            .root
            .advance_raid_run(
                &h.node,
                fixture::id(88),
                request.channel,
                request.generation,
                request.candidates[0].definition.clone(),
                selected_at + elapsed,
            )
            .await?
            .ok_or("source initial announcement")?;
        assert!(matches!(
            pending.action,
            crate::durability::raid_runner::CommittedRaidAction::Announcement { .. }
        ));
        if let Some(identity) = receipt {
            assert_eq!(
                pending.identity, identity,
                "phase clocks do not acknowledge a pending effect"
            );
        } else {
            receipt = Some(pending.identity);
        }
        let state: (i16, i16, Option<i64>) = sqlx::query_as("SELECT chance,phase,selected_at_ms FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
            .bind(request.world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            state,
            (-1, expected, Some(selected_at)),
            "source Active advances at 30/45 minutes without rerolling chance or regressing on replay"
        );
        if elapsed == 1_799_999 {
            // Upgrading a real firing with no occurrence row must reconstruct
            // its original selection clock, including while delivery is pending.
            sqlx::query("DELETE FROM game_world_feroxa_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
                .bind(request.world.as_bytes().as_slice()).execute(&h.pool).await?;
        }
    }
    Ok(())
}
