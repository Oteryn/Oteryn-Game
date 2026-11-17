//! Whole-scenario bound includes setup, source operations and cleanup.
use super::TestResult;
use std::sync::{Arc, Mutex, mpsc};
#[derive(Clone)]
pub(super) struct Stage(Arc<Mutex<&'static str>>);
impl Stage {
    pub(super) fn set(&self, name: &'static str) {
        if let Ok(mut stage) = self.0.lock() {
            *stage = name;
        }
    }
}
pub(super) fn run(
    name: &'static str,
    body: impl FnOnce(Stage) -> TestResult + Send + 'static,
) -> TestResult {
    let stage = Stage(Arc::new(Mutex::new("setup")));
    let watched = stage.clone();
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(64 << 20)
        .spawn(move || {
            let result = body(stage).map_err(|e| e.to_string());
            let _ = tx.send(result);
        })?;
    rx.recv_timeout(std::time::Duration::from_secs(120))
        .map_err(|e| {
            format!(
                "{name} watchdog: stage={} ({e})",
                watched.0.lock().map(|v| *v).unwrap_or("stage_lock")
            )
        })?
        .map_err(Into::into)
}
