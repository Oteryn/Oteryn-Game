//! LOG-17: why a blocked operation waits. A refused logout, a pending control-loss
//! reconciliation and a step refused for a busy durability holder report what holds or is
//! awaited as a closed name, how long the operation has waited and its attempt count. The
//! first block is reported at once, later ones at most every [`REPORT_EVERY`] or when the
//! awaited name changes. Reporting only: nothing here changes a retry, deadline or lock.
//! The session is an in-memory key only and never enters a line (logging review §4).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::actor_spell::ChannelSpellStates;
use super::operator_event;
use crate::foundation::{ExactActorRef, GameSessionId};

/// The longest silence between two reports of one continuing wait.
const REPORT_EVERY: Duration = Duration::from_secs(5);
/// A block seen after this long without one starts a new wait (the client retries a `BUSY`
/// within about a second).
const IDLE_RESET: Duration = Duration::from_secs(10);
/// Waits tracked at once; the stalest is forgotten first.
const CAPACITY: usize = 64;

/// The closed name of what a blocked operation waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Awaiting {
    ActivityReturn,
    ActorSpellReserved,
    FamiliarCast,
    ForegroundPreference,
    GourmetCheckpoint,
    ItemEffects,
    MovementPosition,
    NativeCast,
    NewhavenTravel,
    Parameters,
    PendingEnchanting,
    PendingFamiliar,
    PendingGourmet,
    PendingHirelingLamp,
    PendingItemEffect,
    PendingNative,
    PendingParameters,
    PendingStance,
    PendingTrainingCheckpoint,
    PendingWorldItems,
    ReleaseCommit,
    ReleaseFence,
    SessionBinding,
    SessionNotCurrent,
    TrainingSave,
    Unknown,
    WorldItems,
    /// The single durability holder: idle again, absent, in a semantic pass or in the
    /// readiness maintenance window ([`crate::durability::DurabilityRoot::holder_state`]).
    Holder(&'static str),
}

impl Awaiting {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::ActivityReturn => "activity_return",
            Self::ActorSpellReserved => "actor_spell_reserved",
            Self::FamiliarCast => "familiar_cast",
            Self::ForegroundPreference => "foreground_preference",
            Self::GourmetCheckpoint => "gourmet_checkpoint",
            Self::ItemEffects => "item_effects",
            Self::MovementPosition => "movement_position",
            Self::NativeCast => "native_cast",
            Self::NewhavenTravel => "newhaven_travel",
            Self::Parameters => "parameters",
            Self::PendingEnchanting => "pending_enchanting",
            Self::PendingFamiliar => "pending_familiar",
            Self::PendingGourmet => "pending_gourmet",
            Self::PendingHirelingLamp => "pending_hireling_lamp",
            Self::PendingItemEffect => "pending_item_effect",
            Self::PendingNative => "pending_native",
            Self::PendingParameters => "pending_parameters",
            Self::PendingStance => "pending_stance",
            Self::PendingTrainingCheckpoint => "pending_training_checkpoint",
            Self::PendingWorldItems => "pending_world_items",
            Self::ReleaseCommit => "release_commit",
            Self::ReleaseFence => "release_fence",
            Self::SessionBinding => "session_binding",
            Self::SessionNotCurrent => "session_not_current",
            Self::TrainingSave => "training_save",
            Self::Unknown => "unknown",
            Self::WorldItems => "world_items",
            Self::Holder(state) => state,
        }
    }
}

impl ChannelSpellStates {
    /// Which retained commit [`Self::has_pending_spell_commit_before_actor_end`] finds first,
    /// in its order.
    pub(super) fn pending_spell_commit_awaiting(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Awaiting {
        let pending =
            if self.pending_item_effects.iter().any(|memo| {
                memo.actor == actor && memo.request.command.game_session_id() == session
            }) {
                Some(Awaiting::PendingItemEffect)
            } else if self
                .pending_gourmet
                .iter()
                .any(|pending| pending.actor == actor && pending.session == session)
            {
                Some(Awaiting::PendingGourmet)
            } else if self
                .pending_hireling_lamps
                .iter()
                .any(|pending| pending.actor == actor && pending.session == session)
            {
                Some(Awaiting::PendingHirelingLamp)
            } else if self.enchanting_pending.contains(&(actor, session)) {
                Some(Awaiting::PendingEnchanting)
            } else if self.has_pending_native(actor, session) {
                Some(Awaiting::PendingNative)
            } else if self.has_pending_world_items(actor, session) {
                Some(Awaiting::PendingWorldItems)
            } else if self.has_pending_parameters(actor, session) {
                Some(Awaiting::PendingParameters)
            } else if self.has_pending_familiar_cast(actor, session) {
                Some(Awaiting::PendingFamiliar)
            } else {
                self.actors
                    .iter()
                    .find(|(a, s, _)| *a == actor && *s == session)
                    .and_then(|(_, _, state)| {
                        if state.pending_stance().is_some() {
                            Some(Awaiting::PendingStance)
                        } else if state.pending_training_checkpoint().is_some() {
                            Some(Awaiting::PendingTrainingCheckpoint)
                        } else {
                            None
                        }
                    })
            };
        pending.unwrap_or(Awaiting::Unknown)
    }
}

/// What a block at the closed `stage` waits for. `None` (a logout refused by its spell
/// reconciliation) takes the name that reconciliation recorded.
fn stage_awaiting(stage: &str) -> Option<Awaiting> {
    Some(match stage {
        "activity_scope" => Awaiting::ActivityReturn,
        "actor_unreserved" => Awaiting::ActorSpellReserved,
        "admitted_binding" => Awaiting::SessionBinding,
        "familiar_cast" => Awaiting::FamiliarCast,
        "familiar_logout" => Awaiting::PendingFamiliar,
        "foreground_demand" => Awaiting::ForegroundPreference,
        "gourmet_checkpoint" => Awaiting::GourmetCheckpoint,
        "item_effects" => Awaiting::ItemEffects,
        "native_cast" => Awaiting::NativeCast,
        "newhaven_travel" => Awaiting::NewhavenTravel,
        "parameters" => Awaiting::Parameters,
        "release_fence" => Awaiting::ReleaseFence,
        "release_not_applicable" => Awaiting::SessionNotCurrent,
        "release_retry_lifted" => Awaiting::ReleaseCommit,
        "spell_reconcile" => return None,
        "training_save" => Awaiting::TrainingSave,
        "world_items" => Awaiting::WorldItems,
        "zones_after_reconcile" | "zones_before_reconcile" => Awaiting::MovementPosition,
        _ => Awaiting::Unknown,
    })
}

struct Wait {
    session: GameSessionId,
    event: &'static str,
    since: Instant,
    seen: Instant,
    reported: Instant,
    attempts: u64,
    awaiting: Awaiting,
}

#[derive(Default)]
struct WaitLog(Vec<Wait>);

impl WaitLog {
    /// Count one block of `event` (`event=<name>`) at `stage` and return the line to write, if
    /// any. `None` as `awaiting` takes the name another event of this session last recorded
    /// (a logout blocked by a pending control-loss reconciliation).
    fn observe(
        &mut self,
        session: GameSessionId,
        event: &'static str,
        stage: Option<&'static str>,
        awaiting: Option<Awaiting>,
        now: Instant,
    ) -> Option<String> {
        let awaiting = awaiting.unwrap_or_else(|| {
            self.0
                .iter()
                .filter(|wait| wait.session == session && wait.event != event)
                .max_by_key(|wait| wait.seen)
                .map_or(Awaiting::Unknown, |wait| wait.awaiting)
        });
        let index = self
            .0
            .iter()
            .position(|wait| wait.session == session && wait.event == event);
        let report = match index.and_then(|index| self.0.get_mut(index)) {
            Some(wait) if now.saturating_duration_since(wait.seen) < IDLE_RESET => {
                wait.seen = now;
                wait.attempts = wait.attempts.saturating_add(1);
                let report = wait.awaiting != awaiting
                    || now.saturating_duration_since(wait.reported) >= REPORT_EVERY;
                wait.awaiting = awaiting;
                if report {
                    wait.reported = now;
                }
                report.then_some((wait.since, wait.attempts))
            }
            Some(wait) => {
                *wait = Wait {
                    session,
                    event,
                    since: now,
                    seen: now,
                    reported: now,
                    attempts: 1,
                    awaiting,
                };
                Some((now, 1))
            }
            None => {
                if self.0.len() >= CAPACITY
                    && let Some(stalest) = self
                        .0
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, wait)| wait.seen)
                        .map(|(index, _)| index)
                {
                    self.0.swap_remove(stalest);
                }
                self.0.push(Wait {
                    session,
                    event,
                    since: now,
                    seen: now,
                    reported: now,
                    attempts: 1,
                    awaiting,
                });
                Some((now, 1))
            }
        };
        report.map(|(since, attempts)| {
            let stage = stage.map_or_else(String::new, |stage| format!(" stage={stage}"));
            format!(
                "{event}{stage} awaiting={} waited_ms={} attempts={attempts}",
                awaiting.as_str(),
                now.saturating_duration_since(since).as_millis()
            )
        })
    }
}

static WAITS: Mutex<WaitLog> = Mutex::new(WaitLog(Vec::new()));

/// Report one logout refused `BUSY` at the closed `stage`.
pub(super) fn logout_busy(session: GameSessionId, stage: &'static str) {
    blocked(
        session,
        "event=logout_busy",
        Some(stage),
        stage_awaiting(stage),
    );
}

/// Report one control-loss reconciliation left pending at the closed `stage`.
pub(super) fn reconcile_pending(session: GameSessionId, stage: &'static str) {
    blocked(
        session,
        "event=control_loss_reconcile_pending",
        Some(stage),
        stage_awaiting(stage),
    );
}

/// [`reconcile_pending`] with the awaited name the caller resolved.
pub(super) fn reconcile_pending_awaiting(
    session: GameSessionId,
    stage: &'static str,
    awaiting: Awaiting,
) {
    blocked(
        session,
        "event=control_loss_reconcile_pending",
        Some(stage),
        Some(awaiting),
    );
}

/// Report one block of `event` (`event=<name>`), rate-limited per session and event.
pub(super) fn blocked_event(
    session: GameSessionId,
    event: &'static str,
    stage: Option<&'static str>,
    awaiting: Awaiting,
) {
    blocked(session, event, stage, Some(awaiting));
}

fn blocked(
    session: GameSessionId,
    event: &'static str,
    stage: Option<&'static str>,
    awaiting: Option<Awaiting>,
) {
    let report = WAITS
        .lock()
        .ok()
        .and_then(|mut waits| waits.observe(session, event, stage, awaiting, Instant::now()));
    if let Some(report) = report {
        operator_event(&report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECONCILE: &str = "event=control_loss_reconcile_pending";
    const LOGOUT: &str = "event=logout_busy";

    fn session(byte: u8) -> Result<GameSessionId, Box<dyn std::error::Error>> {
        let mut bytes = [byte; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        Ok(GameSessionId::decode(&bytes)?)
    }

    #[test]
    fn first_block_reports_then_rate_limited_with_wait_and_attempts()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut log = WaitLog::default();
        let one = session(1)?;
        let start = Instant::now();
        let stage = Some("pending_spell_commit");
        let pending = Some(Awaiting::PendingNative);
        assert_eq!(
            log.observe(one, RECONCILE, stage, pending, start)
                .as_deref(),
            Some(
                "event=control_loss_reconcile_pending stage=pending_spell_commit \
                 awaiting=pending_native waited_ms=0 attempts=1"
            )
        );
        let mut at = start;
        for _ in 0..9 {
            at += Duration::from_millis(500);
            assert_eq!(log.observe(one, RECONCILE, stage, pending, at), None);
        }
        at += Duration::from_millis(500);
        assert_eq!(
            log.observe(one, RECONCILE, stage, pending, at).as_deref(),
            Some(
                "event=control_loss_reconcile_pending stage=pending_spell_commit \
                 awaiting=pending_native waited_ms=5000 attempts=11"
            )
        );
        // A changed awaited name is reported at once.
        at += Duration::from_millis(500);
        let stance = Some(Awaiting::PendingStance);
        assert_eq!(
            log.observe(one, RECONCILE, stage, stance, at).as_deref(),
            Some(
                "event=control_loss_reconcile_pending stage=pending_spell_commit \
                 awaiting=pending_stance waited_ms=5500 attempts=12"
            )
        );
        // A logout refused by that reconciliation names what it awaits, on its own count.
        let reconcile = Some("spell_reconcile");
        assert_eq!(
            log.observe(
                one,
                LOGOUT,
                reconcile,
                stage_awaiting("spell_reconcile"),
                at
            )
            .as_deref(),
            Some(
                "event=logout_busy stage=spell_reconcile awaiting=pending_stance \
                 waited_ms=0 attempts=1"
            )
        );
        // Another session and a wait resumed after the idle gap start their own counts.
        assert!(
            log.observe(session(2)?, RECONCILE, stage, pending, at)
                .is_some_and(|line| line.ends_with("waited_ms=0 attempts=1"))
        );
        at += IDLE_RESET;
        assert!(
            log.observe(one, RECONCILE, stage, pending, at)
                .is_some_and(|line| line.ends_with("waited_ms=0 attempts=1"))
        );
        Ok(())
    }

    #[test]
    fn every_reported_stage_names_what_it_awaits() -> Result<(), Box<dyn std::error::Error>> {
        let mut stages = 0;
        for (source, call) in [
            (
                include_str!("control_loss_reconciliation.rs"),
                "reconcile_pending(",
            ),
            (include_str!("logout.rs"), "logout_busy("),
        ] {
            for (at, _) in source.match_indices(call) {
                let rest = source.get(at..).ok_or("call site")?;
                let Some(stage) = rest.split('"').nth(1) else {
                    continue;
                };
                stages += 1;
                assert_eq!(
                    stage_awaiting(stage).is_some_and(|name| name != Awaiting::Unknown),
                    stage != "spell_reconcile",
                    "{stage}"
                );
            }
        }
        assert_eq!(stages, 20);
        Ok(())
    }

    #[test]
    fn reports_carry_only_closed_names_and_numbers() -> Result<(), Box<dyn std::error::Error>> {
        let mut log = WaitLog::default();
        let one = session(0xab)?;
        let now = Instant::now();
        let mut lines = Vec::new();
        for (event, stage, awaiting) in [
            (
                LOGOUT,
                Some("foreground_demand"),
                Some(Awaiting::ForegroundPreference),
            ),
            (
                "event=step_rejected_holder_busy",
                None,
                Some(Awaiting::Holder("semantic_pass")),
            ),
            (
                RECONCILE,
                Some("native_cast"),
                stage_awaiting("native_cast"),
            ),
        ] {
            lines.extend(log.observe(one, event, stage, awaiting, now));
        }
        assert_eq!(lines.len(), 3);
        let identity = format!("{one:?}");
        for line in &lines {
            assert!(!line.contains("171") && !line.contains(&identity), "{line}");
            for field in line.split(' ') {
                let (key, value) = field.split_once('=').ok_or("field without a value")?;
                assert!(
                    ["event", "stage", "awaiting", "waited_ms", "attempts"].contains(&key),
                    "{line}"
                );
                assert!(
                    value
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                    "{line}"
                );
            }
        }
        Ok(())
    }
}
