use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, kill_reward_e2e_fixtures::scenario_thread};
use crate::durability::DurabilityRoot;
use crate::monster_owner_cycle::respawn_job::RespawnExit;
use oteryn_foundation::CancellationToken;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Wake, Waker};
use std::time::Duration;

#[derive(Default)]
struct CadenceWake(AtomicBool);

impl Wake for CadenceWake {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Relaxed);
    }
}

#[test]
fn soulpit_cadence_wakes_when_the_durable_holder_returns_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_thread::run_async(async move |_| {
        let h = Harness::create_with_admission_seed(admin, "soulpit_cadence", true, false).await?;
        {
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let admission = deps.owner(&h, &authority);
            let holder = h.root.try_issue_semantic_pass().map_err(debug)?;
            assert!(!h.root.is_ready(), "discovery must encounter a busy holder");

            // Keep Tokio from auto-advancing to the fallback timer during PostgreSQL I/O.
            let (release, released) = std::sync::mpsc::channel::<()>();
            let watchdog = tokio::task::spawn_blocking(move || {
                let _ = released.recv_timeout(Duration::from_secs(10));
            });
            tokio::time::pause();
            let started = tokio::time::Instant::now();
            let stop = CancellationToken::new();
            let mut cadence = std::pin::pin!(DurabilityRoot::with_background_holder(
                admission.run_soulpits(&stop)
            ));
            let woken = Arc::new(CadenceWake::default());
            let waker = Waker::from(woken.clone());
            assert!(
                cadence
                    .as_mut()
                    .poll(&mut Context::from_waker(&waker))
                    .is_pending()
            );
            assert!(
                !woken.0.load(Ordering::Relaxed),
                "cadence must park while the holder is busy"
            );
            let returned = holder.run(|_, _| Box::pin(async { Ok(()) })).await;
            let woke_on_return = woken.0.load(Ordering::Relaxed);
            let elapsed = started.elapsed();
            tokio::time::resume();
            drop(release);
            watchdog.await?;
            returned.map_err(debug)?;
            assert!(h.root.is_ready());
            assert_eq!(elapsed, Duration::ZERO);
            assert!(
                woke_on_return,
                "returning the holder must wake the parked cadence"
            );

            stop.cancel();
            assert_eq!(cadence.await, RespawnExit::Shutdown);
        }
        h.cleanup().await
    })
}
