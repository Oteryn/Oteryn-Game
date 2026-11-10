//! Test-only RETIRING rows and bounded MAX retirement diagnostics.
use crate::foundation::WorldId;
type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::sync::{Arc, Mutex};
use std::time::Instant;

tokio::task_local! {
    static RETIREMENT_PROFILE: ([u8; 16], Arc<Mutex<Option<RetirementReport>>>);
}

/// Observe only the selected MAX fixture root, without changing its result.
pub async fn profile_maximum_retirement<F: std::future::Future>(
    item: [u8; 16],
    operation: F,
) -> (F::Output, Option<RetirementReport>) {
    let report = Arc::new(Mutex::new(None));
    let result = RETIREMENT_PROFILE
        .scope((item, Arc::clone(&report)), operation)
        .await;
    let snapshot = report.lock().ok().and_then(|mut slot| slot.take());
    (result, snapshot)
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum RetirementPhase {
    Admission,
    TreeLoad,
    ItemLocks,
    GraphRecheck,
    Preparation,
    Reservation,
    Wrapper,
    Root,
    Commit,
}

/// A bounded snapshot; unfinished work stays separate from completed phases.
#[derive(Default)]
pub struct RetirementReport {
    phase: Option<RetirementPhase>,
    completed_us: [u64; 9],
    in_flight_us: u64,
    total_us: u64,
    descendants: usize,
    reservations_ok: usize,
    wrappers_ok: usize,
    child_index: usize,
    pre_commit_us: Option<u64>,
    commit_budget_us: Option<u64>,
    committed: bool,
}

impl std::fmt::Display for RetirementReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "phase={:?} in_flight_us={} total_us={} descendants={} reservations_ok={} wrappers_ok={} child_index={} pre_commit_us={:?} commit_budget_us={:?} committed={}",
            self.phase,
            self.in_flight_us,
            self.total_us,
            self.descendants,
            self.reservations_ok,
            self.wrappers_ok,
            self.child_index,
            self.pre_commit_us,
            self.commit_budget_us,
            self.committed
        )?;
        for (name, micros) in [
            "admission",
            "tree_load",
            "item_locks",
            "graph_recheck",
            "preparation",
            "reservation",
            "wrapper",
            "root",
            "commit",
        ]
        .into_iter()
        .zip(self.completed_us)
        {
            write!(f, " {name}_completed_us={micros}")?;
        }
        Ok(())
    }
}

/// Owned state survives cancellation of the inner semantic-pass future. The
/// probe takes the sink lock only in Drop; no lock crosses an await.
pub(crate) struct RetirementProbe {
    sink: Arc<Mutex<Option<RetirementReport>>>,
    report: RetirementReport,
    started: Instant,
    phase_started: Option<Instant>,
}

impl RetirementProbe {
    pub(crate) fn start(item: [u8; 16]) -> Option<Self> {
        let sink = RETIREMENT_PROFILE
            .try_with(|(selected, sink)| (*selected == item).then(|| Arc::clone(sink)))
            .ok()??;
        let mut probe = Self {
            sink,
            report: RetirementReport::default(),
            started: Instant::now(),
            phase_started: None,
        };
        probe.enter(RetirementPhase::Admission);
        Some(probe)
    }

    pub(crate) fn enter(&mut self, phase: RetirementPhase) {
        self.finish_phase();
        self.report.phase = Some(phase);
        self.phase_started = Some(Instant::now());
    }

    fn finish_phase(&mut self) {
        if let (Some(phase), Some(started)) = (self.report.phase, self.phase_started.take()) {
            let total = &mut self.report.completed_us[phase as usize];
            *total = total.saturating_add(micros(started.elapsed()));
        }
    }

    pub(crate) fn descendants(&mut self, count: usize) {
        self.report.descendants = count;
    }

    pub(crate) fn begin_child(&mut self) {
        self.report.child_index += 1;
        self.enter(RetirementPhase::Preparation);
    }

    pub(crate) fn reservation_finished(&mut self) {
        self.report.reservations_ok += 1;
        self.enter(RetirementPhase::Wrapper);
    }

    pub(crate) fn wrapper_finished(&mut self) {
        self.report.wrappers_ok += 1;
        self.finish_phase();
    }

    pub(crate) fn begin_commit(&mut self, deadline: Instant) {
        self.enter(RetirementPhase::Commit);
        if let Some(started) = self.phase_started {
            self.report.pre_commit_us =
                Some(micros(started.saturating_duration_since(self.started)));
            self.report.commit_budget_us =
                Some(micros(deadline.saturating_duration_since(started)));
        }
    }

    pub(crate) fn committed(&mut self) {
        self.finish_phase();
        self.report.committed = true;
    }
}

impl Drop for RetirementProbe {
    fn drop(&mut self) {
        let now = Instant::now();
        self.report.total_us = micros(now.saturating_duration_since(self.started));
        self.report.in_flight_us = self
            .phase_started
            .map_or(0, |start| micros(now.saturating_duration_since(start)));
        if let Ok(mut slot) = self.sink.lock() {
            *slot = Some(std::mem::take(&mut self.report));
        }
    }
}

fn micros(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

pub async fn seed_retiring(pool: &sqlx::PgPool, world: WorldId) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),to_char((clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'))")
            .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(pool).await?;
    sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,retirement_binding,target_map_revision,target_server_digest,target_client_digest,target_frame_digest,target_house_tiles,calendar_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8') || $2),'map-2',$2,$2,$2,'[]',1,'RETIRING')")
            .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world.as_bytes().as_slice())
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO game_world_reset_channels SELECT world_id,0,channel_id,ownership_generation FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1")
            .bind(world.as_bytes().as_slice()).execute(pool).await?;
    Ok(())
}
