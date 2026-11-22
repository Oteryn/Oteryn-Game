#![allow(clippy::expect_used)]
use super::*;
use crate::foundation::{ChannelRuntimeV1, WorldId};
use std::sync::atomic::{AtomicU64, Ordering};

fn runtime() -> ChannelRuntimeV1 {
    let world =
        WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect("world");
    crate::monster_summon::summon_execution_fixture_for_world(world).0
}
struct Wake {
    now: AtomicU64,
    stop: CancellationToken,
    until: u64,
    world_utc_ms: Option<u64>,
}
impl OwnerClock for Wake {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(self.now.load(Ordering::SeqCst))
    }
}
impl RespawnWake for Wake {
    fn world_clock_utc_ms(&self) -> Option<u64> {
        self.world_utc_ms
    }
    async fn sleep_until(&self, due: SemanticTimeMicros) {
        self.now.store(due.get(), Ordering::SeqCst);
        if due.get() >= self.until {
            self.stop.cancel();
        }
    }
}
#[test]
fn respawn_job_runs_without_sessions_at_the_due_window() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(async {
            let (runtime, due) = waiting_respawn();
            let stop = CancellationToken::new();
            let wake = Wake {
                now: AtomicU64::new(due),
                stop: stop.clone(),
                until: due + 4_250_000,
                world_utc_ms: None,
            };
            let runtime = tokio::sync::Mutex::new(runtime);
            assert_eq!(drive(&runtime, &wake, &stop).await, RespawnExit::Shutdown);
            assert!(runtime.lock().await.spawn_point_creatures()[0].2.is_some());
        });
}
#[test]
fn respawn_job_shutdown_before_pass_and_retired_owner_and_clock_exhaustion() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(async {
            let runtime = tokio::sync::Mutex::new(runtime());
            let stop = CancellationToken::new();
            stop.cancel();
            let wake = Wake {
                now: AtomicU64::new(u64::MAX),
                stop: stop.clone(),
                until: u64::MAX,
                world_utc_ms: None,
            };
            assert_eq!(drive(&runtime, &wake, &stop).await, RespawnExit::Shutdown);
            assert_eq!(
                drive(&runtime, &wake, &CancellationToken::new()).await,
                RespawnExit::ClockExhausted
            );
            runtime.lock().await.retire_owner_cycle();
            assert_eq!(
                drive(&runtime, &wake, &CancellationToken::new()).await,
                RespawnExit::StaleOwner
            );
        });
}
#[test]
fn respawn_job_regression_and_origin_past_deadline_share_the_carrier_clock() {
    let mut runtime = runtime();
    let mut job = MonsterRespawnJob::new(&runtime).expect("current owner");
    job.pass(&mut runtime, 150_000, None).expect("pass");
    job.pass(&mut runtime, 50_000, None).expect("regression");
    assert_eq!(job.next_owner_window(50_000), Some(200_000));
    let wake = MonotonicRespawnWake::from_origin(
        std::time::Instant::now() - std::time::Duration::from_secs(10),
    );
    job.pass(&mut runtime, wake.now().get(), None)
        .expect("late installed pass");
    let same = wake.now().get();
    job.pass(&mut runtime, same, None).expect("same window");
    super::super::monster_ai_cycle::drive_owner_spawns(&mut runtime, same);
}

fn waiting_respawn() -> (ChannelRuntimeV1, u64) {
    let mut runtime = runtime();
    let room =
        crate::content::qualify_native_entry_room(runtime.binding().world_id()).expect("room");
    runtime
        .realize_activation_spawns(vec![room.spawn_source().expect("spawn").activation_facts()])
        .expect("spawns");
    let (point, _, actor) = runtime.spawn_point_creatures()[0];
    assert!(runtime.kill_creature_for_test(actor.expect("rat")));
    super::super::monster_ai_cycle::drive_owner_spawns(&mut runtime, 0);
    let due = runtime.pending_respawn(point).expect("pending").due_micros;
    (runtime, due)
}
struct AfterFirstPass {
    wake: MonotonicRespawnWake,
    stop: CancellationToken,
}
impl OwnerClock for AfterFirstPass {
    fn now(&self) -> SemanticTimeMicros {
        self.wake.now()
    }
}
impl RespawnWake for AfterFirstPass {
    async fn sleep_until(&self, _: SemanticTimeMicros) {
        self.stop.cancel();
    }
}
#[test]
fn installed_respawn_job_uses_the_session_origin_past_its_deadline() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(async {
            let (mut runtime, due) = waiting_respawn();
            super::super::monster_ai_cycle::drive_owner_spawns(&mut runtime, due);
            assert!(runtime.spawn_point_creatures()[0].2.is_none());
            let stop = CancellationToken::new();
            let wake = AfterFirstPass {
                wake: MonotonicRespawnWake::from_origin(
                    std::time::Instant::now() - std::time::Duration::from_micros(due + 4_200_000),
                ),
                stop: stop.clone(),
            };
            let runtime = tokio::sync::Mutex::new(runtime);
            assert_eq!(drive(&runtime, &wake, &stop).await, RespawnExit::Shutdown);
            let mut runtime = runtime.lock().await;
            let realized = runtime.spawn_point_creatures();
            assert!(
                realized[0].2.is_some(),
                "the installed job realized the already due spawn without a session"
            );
            super::super::monster_ai_cycle::drive_owner_spawns(&mut runtime, wake.now().get());
            assert_eq!(
                runtime.spawn_point_creatures(),
                realized,
                "the session pass shares the carrier clock and creates nothing twice"
            );
        });
}
struct RetireWake<'a> {
    runtime: &'a tokio::sync::Mutex<ChannelRuntimeV1>,
}
impl OwnerClock for RetireWake<'_> {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(0)
    }
}
impl RespawnWake for RetireWake<'_> {
    async fn sleep_until(&self, _: SemanticTimeMicros) {
        self.runtime.lock().await.retire_owner_cycle();
    }
}
#[test]
fn installed_respawn_job_rechecks_retirement_after_every_wake() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(async {
            let runtime = tokio::sync::Mutex::new(runtime());
            assert_eq!(
                drive(
                    &runtime,
                    &RetireWake { runtime: &runtime },
                    &CancellationToken::new()
                )
                .await,
                RespawnExit::StaleOwner
            );
        });
}

#[test]
fn an_unchanged_boss_point_waits_before_its_next_check() {
    let now = tokio::time::Instant::now();
    let due = unchanged_boss_recheck(now).expect("recheck");
    assert_eq!(due.duration_since(now), UNCHANGED_BOSS_RECHECK);
    // The pass skips a point whose timer is still in the future.
    let timers = std::collections::BTreeMap::from([(String::from("boss"), Some(due))]);
    assert!(
        timers
            .get("boss")
            .is_some_and(|due| due.is_none_or(|due| due > now))
    );
}

async fn drive_spawn_window(runtime: &tokio::sync::Mutex<ChannelRuntimeV1>, now: u64, minute: u64) {
    let stop = CancellationToken::new();
    let wake = Wake {
        now: AtomicU64::new(now),
        stop: stop.clone(),
        until: now + 1,
        world_utc_ms: Some(minute * crate::world_clock::TIBIAN_MINUTE_MS),
    };
    assert_eq!(drive(runtime, &wake, &stop).await, RespawnExit::Shutdown);
}

#[test]
fn bundle_spawn_starts_full_and_respawn_checks_period_before_warnings() {
    use crate::map::boot::table_activation_spawns_with_eligibility;
    use oteryn_gameplay_content::content::{ProjectV2SpawnEligibility, ProjectV2SpawnPeriod};
    use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};

    const DAY: u64 = 12 * 60;
    const NIGHT: u64 = 24 * 60 + 5 * 60 + 59;
    const SUNRISE: u64 = NIGHT + 1;
    const SUNSET_END: u64 = 18 * 60;
    const DELAY_MS: u32 = 60_000;
    const DELAY: u64 = DELAY_MS as u64 * 1_000;
    const WARNING: u64 = 4_200_000;
    let table = Table {
        creatures: vec![Creature {
            key: "oteryn:creature.night_harpy".into(),
            period: Period::Night,
        }],
        sources: [-7, -8]
            .into_iter()
            .map(|floor| Source {
                key: format!("oteryn:spawn/night_harpy/{}", -floor),
                floor,
                x: 33_828,
                y: 32_242,
                points: vec![Point {
                    creature: 0,
                    x: 33_828,
                    y: 32_242,
                    direction: Direction::North,
                    respawn_ms: DELAY_MS,
                    boss_policy: None,
                }],
            })
            .collect(),
    };
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    executor.block_on(async {
        for ignore_period_underground in [false, true] {
            let sources = table_activation_spawns_with_eligibility(
                &table,
                |_| Some((14_000, false)),
                |_| {
                    Some(ProjectV2SpawnEligibility {
                        period: ProjectV2SpawnPeriod::Night,
                        ignore_period_underground,
                        blocked_by_nearby_players: false,
                    })
                },
            )
            .expect("spawns");
            let mut channel = runtime();
            assert_eq!(channel.begin_activation_spawn_sources(sources), Ok(2));
            let progress = channel
                .realize_activation_spawn_window(0)
                .expect("startup activation");
            assert_eq!(progress.remaining, 0);
            assert_eq!(progress.realized, 2, "Crystal initially fills both points");
            let points = channel.spawn_point_creatures();
            let surface = points[0].0;
            for (_, _, actor) in points {
                assert!(channel.kill_creature_for_test(actor.expect("startup creature")));
            }

            let channel = tokio::sync::Mutex::new(channel);
            drive_spawn_window(&channel, 0, DAY).await;
            drive_spawn_window(&channel, DELAY, SUNSET_END).await;
            {
                let owner = channel.lock().await;
                let pending = owner
                    .pending_respawn(surface)
                    .expect("deferred surface point");
                assert_eq!(
                    pending.due_micros,
                    2 * DELAY,
                    "18:00 is still daytime for Crystal respawns and restarts the full delay"
                );
            }
            drive_spawn_window(&channel, DELAY + WARNING, SUNSET_END).await;
            assert_eq!(
                channel.lock().await.spawn_point_creatures()[1].2.is_some(),
                ignore_period_underground,
                "daytime respawn also requires the underground flag"
            );

            drive_spawn_window(&channel, 2 * DELAY, NIGHT).await;
            let warning_due = 2 * DELAY + WARNING;
            assert_eq!(
                channel
                    .lock()
                    .await
                    .pending_respawn(surface)
                    .expect("night warning")
                    .due_micros,
                warning_due
            );
            drive_spawn_window(&channel, warning_due, SUNRISE).await;
            let owner = channel.lock().await;
            assert!(
                owner
                    .spawn_point_creatures()
                    .iter()
                    .all(|point| point.2.is_some()),
                "a warning sequence that started at night finishes even after sunrise"
            );
            assert!(owner.pending_respawn(surface).is_none());
        }
    });
}
