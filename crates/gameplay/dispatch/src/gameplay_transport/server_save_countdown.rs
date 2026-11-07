//! PARITY-EVENT-CALLBACKS-1 (coordinator decision, NIGHT-W1): the server-save countdown.
//! Canary `global_server_save.lua:29,48` broadcasts once a minute from 5 to 1 minutes before
//! the server save. The Channel only reads the reset Control publishes; it never triggers,
//! moves or cancels one. Sessions with capability 75 receive each minute at most once.
use super::ComposedFreshAdmission;
use crate::durability::world_reset::PlannedWorldReset;
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::server_save_countdown::ServerSaveCountdown;

/// The planned read cadence; a minute's window is 60 s, so a notice is at most this late.
const SERVER_SAVE_COUNTDOWN_POLL: std::time::Duration = std::time::Duration::from_secs(5);

/// One countdown minute of one planned reset. `reset_at_unix_ms` keeps the receipt apart
/// from a later day's countdown that ends the same epoch (a skipped reset).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DueServerSaveCountdown {
    /// The planned reset instant this minute counts down to.
    pub reset_at_unix_ms: i64,
    /// The wire notice.
    pub notice: ServerSaveCountdown,
}

impl DueServerSaveCountdown {
    /// Whether a session whose last notice was `sent` takes this one: a new reset, or a
    /// strictly smaller minute of the same one. A clock step back never repeats a minute.
    pub fn follows(&self, sent: Option<(i64, u8)>) -> bool {
        sent.is_none_or(|(reset_at, minutes)| {
            reset_at != self.reset_at_unix_ms || self.notice.minutes_remaining < minutes
        })
    }

    /// The (planned reset instant, minute) a session keeps after sending this notice.
    pub fn receipt(&self) -> (i64, u8) {
        (self.reset_at_unix_ms, self.notice.minutes_remaining)
    }
}

/// The Channel's currently due notice. A leaf lock, never held across an await.
#[derive(Default)]
pub(crate) struct ServerSaveCountdownHub {
    due: std::sync::Mutex<Option<DueServerSaveCountdown>>,
}

impl ServerSaveCountdownHub {
    pub(crate) fn current(&self) -> Option<DueServerSaveCountdown> {
        *self
            .due
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn publish(&self, due: Option<DueServerSaveCountdown>) {
        *self
            .due
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = due;
    }
}

/// The pure pass. The first successful read fixes the serve start, so a minute whose window
/// opened before this incarnation (a restart) is never published.
#[derive(Default)]
pub(crate) struct ServerSaveCountdownPass {
    serve_started_unix_ms: Option<i64>,
}

impl ServerSaveCountdownPass {
    pub(crate) fn observe(
        &mut self,
        planned: Option<PlannedWorldReset>,
    ) -> Option<DueServerSaveCountdown> {
        let planned = planned?;
        let started = *self
            .serve_started_unix_ms
            .get_or_insert(planned.database_now_unix_ms);
        let minutes_remaining = planned.warning_due(started)?;
        Some(DueServerSaveCountdown {
            reset_at_unix_ms: planned.reset_at_unix_ms,
            notice: ServerSaveCountdown {
                ending_epoch: planned.ending_epoch,
                minutes_remaining,
            },
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Polled beside the respawn job for the serve lifetime. A failed read publishes nothing
    /// and is retried; it never stops serving.
    pub(super) async fn run_server_save_countdown(&self, shutdown: &CancellationToken) {
        let mut pass = ServerSaveCountdownPass::default();
        loop {
            let Some(read) = super::world_respawns::stopping_first(
                self.root.planned_world_reset(self.world_id),
                shutdown,
            )
            .await
            else {
                return;
            };
            match read {
                Ok(planned) => self.states.server_save.publish(pass.observe(planned)),
                Err(error) => {
                    self.states.server_save.publish(None);
                    super::operator_event(&format!(
                        "server_save_countdown_read_deferred reason={error}"
                    ));
                }
            }
            if super::world_respawns::stopping_first(
                tokio::time::sleep(SERVER_SAVE_COUNTDOWN_POLL),
                shutdown,
            )
            .await
            .is_none()
            {
                return;
            }
        }
    }
}

#[cfg(test)]
#[path = "server_save_countdown_tests.rs"]
mod tests;
