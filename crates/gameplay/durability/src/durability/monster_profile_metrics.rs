//! Cumulative wall-clock timings for the monster profile pass.
use std::{
    cell::Cell,
    future::Future,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

#[derive(Default)]
pub struct PhaseUsage {
    elapsed_ns: AtomicU64,
    count: AtomicU64,
}

impl PhaseUsage {
    pub const fn new() -> Self {
        Self {
            elapsed_ns: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> (u64, u64) {
        (
            self.elapsed_ns.load(Ordering::Relaxed),
            self.count.load(Ordering::Relaxed),
        )
    }

    pub async fn measure<T>(&self, future: impl Future<Output = T>) -> T {
        let _timer = self.start();
        future.await
    }

    pub fn start(&self) -> PhaseTimer<'_> {
        PhaseTimer(self, Instant::now())
    }
}

pub struct PhaseTimer<'a>(&'a PhaseUsage, Instant);

impl Drop for PhaseTimer<'_> {
    fn drop(&mut self) {
        let elapsed_ns = u64::try_from(self.1.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.0.elapsed_ns.fetch_add(elapsed_ns, Ordering::Relaxed);
        self.0.count.fetch_add(1, Ordering::Relaxed);
    }
}

pub static POSTGRES: PhaseUsage = PhaseUsage::new();
pub static MAP_VALIDATION: PhaseUsage = PhaseUsage::new();

tokio::task_local! {
    static AI_PROFILE_ACTIVE: Cell<bool>;
}

pub fn scope<T>(future: impl Future<Output = T>) -> impl Future<Output = T> {
    AI_PROFILE_ACTIVE.scope(Cell::new(false), future)
}

pub struct AiMeasurements;

pub fn start_ai_measurements() -> AiMeasurements {
    let _ = AI_PROFILE_ACTIVE.try_with(|active| active.set(true));
    AiMeasurements
}

impl Drop for AiMeasurements {
    fn drop(&mut self) {
        let _ = AI_PROFILE_ACTIVE.try_with(|active| active.set(false));
    }
}

fn active() -> bool {
    AI_PROFILE_ACTIVE.try_with(Cell::get).unwrap_or(false)
}

/// Time one SQL statement at its execution await. Cached reads and early
/// rejections issue no statement. Multi-statement readers count each attempt.
pub async fn query<T>(future: impl Future<Output = T>) -> T {
    let _timer = active().then(|| POSTGRES.start());
    future.await
}

pub fn start_map_validation() -> Option<PhaseTimer<'static>> {
    active().then(|| MAP_VALIDATION.start())
}
