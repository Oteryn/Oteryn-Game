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

pub static FIELD_CENSUS: PhaseUsage = PhaseUsage::new();
pub static FIELD_ITERATION: PhaseUsage = PhaseUsage::new();
pub static FIELD_CREATURE_SNAPSHOT: PhaseUsage = PhaseUsage::new();
pub static FIELD_TILE_PREFETCH: PhaseUsage = PhaseUsage::new();
pub static FIELD_TILE_READ: PhaseUsage = PhaseUsage::new();
pub static FIELD_TILE_ADDRESS: PhaseUsage = PhaseUsage::new();
pub static FIELD_TILE_CACHE_WRITE: PhaseUsage = PhaseUsage::new();
pub static FIELD_ZONE_LOOKUP: PhaseUsage = PhaseUsage::new();
pub static FIELD_CONTENT_LOOKUP: PhaseUsage = PhaseUsage::new();
pub static FIELD_CONDITION_PREVIEW: PhaseUsage = PhaseUsage::new();
pub static FIELD_SOURCE_PUBLISH: PhaseUsage = PhaseUsage::new();
pub static FIELD_MAP_VALIDATION: PhaseUsage = PhaseUsage::new();
pub static FIELD_ENTRY_COMMIT: PhaseUsage = PhaseUsage::new();
pub static FIELD_HEALTH_NAMES: PhaseUsage = PhaseUsage::new();
pub static FIELD_DAMAGE_CONDITIONS: PhaseUsage = PhaseUsage::new();
pub static FIELD_POSTGRES: PhaseUsage = PhaseUsage::new();

static FIELD_CREATURES_VISITED: AtomicU64 = AtomicU64::new(0);
static FIELD_ITEMS_VISITED: AtomicU64 = AtomicU64::new(0);
static FIELD_FIELDS_VISITED: AtomicU64 = AtomicU64::new(0);
static FIELD_DUE: AtomicU64 = AtomicU64::new(0);

/// Accumulate in the pass and publish once, including work before an early return.
#[derive(Default)]
pub struct FieldVisits {
    pub creatures: u64,
    pub items: u64,
    pub fields: u64,
    pub due: u64,
}

impl Drop for FieldVisits {
    fn drop(&mut self) {
        FIELD_CREATURES_VISITED.fetch_add(self.creatures, Ordering::Relaxed);
        FIELD_ITEMS_VISITED.fetch_add(self.items, Ordering::Relaxed);
        FIELD_FIELDS_VISITED.fetch_add(self.fields, Ordering::Relaxed);
        FIELD_DUE.fetch_add(self.due, Ordering::Relaxed);
    }
}

pub fn field_visit_counts() -> [(&'static str, u64); 4] {
    [
        (
            "field_creatures_visited_count",
            FIELD_CREATURES_VISITED.load(Ordering::Relaxed),
        ),
        (
            "field_items_visited_count",
            FIELD_ITEMS_VISITED.load(Ordering::Relaxed),
        ),
        (
            "field_fields_visited_count",
            FIELD_FIELDS_VISITED.load(Ordering::Relaxed),
        ),
        ("field_due_count", FIELD_DUE.load(Ordering::Relaxed)),
    ]
}

tokio::task_local! {
    static AI_PROFILE_ACTIVE: Cell<bool>;
    static FIELD_TICK_ACTIVE: ();
}

pub fn scope<T>(future: impl Future<Output = T>) -> impl Future<Output = T> {
    AI_PROFILE_ACTIVE.scope(Cell::new(false), future)
}

pub fn field_tick_scope<T>(future: impl Future<Output = T>) -> impl Future<Output = T> {
    FIELD_TICK_ACTIVE.scope((), future)
}

/// Limit shared Map and SQL instrumentation to calls made by the field pass.
pub fn start_field_work(phase: &PhaseUsage) -> Option<PhaseTimer<'_>> {
    FIELD_TICK_ACTIVE.try_with(|()| phase.start()).ok()
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
    let _field_timer = start_field_work(&FIELD_POSTGRES);
    future.await
}

pub fn start_map_validation() -> Option<PhaseTimer<'static>> {
    active().then(|| MAP_VALIDATION.start())
}
