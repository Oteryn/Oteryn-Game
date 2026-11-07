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
}
impl OwnerClock for Wake {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(self.now.load(Ordering::SeqCst))
    }
}
impl RespawnWake for Wake {
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
    job.pass(&mut runtime, 150_000).expect("pass");
    job.pass(&mut runtime, 50_000).expect("regression");
    assert_eq!(job.next_owner_window(50_000), Some(200_000));
    let wake = MonotonicRespawnWake::from_origin(
        std::time::Instant::now() - std::time::Duration::from_secs(10),
    );
    job.pass(&mut runtime, wake.now().get())
        .expect("late installed pass");
    let same = wake.now().get();
    job.pass(&mut runtime, same).expect("same window");
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
