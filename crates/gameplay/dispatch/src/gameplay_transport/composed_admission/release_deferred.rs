//! LOGDBG-C: why an expired grace was not released on this attempt. Diagnostics only: the
//! release loop decides exactly as before, and a report never awaits or blocks it.

use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

/// A stuck release reports each reason at most once per this interval; the next report
/// carries how many were suppressed meanwhile.
const REPORT_INTERVAL: Duration = Duration::from_secs(10);

/// Closed set of the steps that put off the release of an expired grace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gameplay_transport) enum DeferReason {
    SessionMoved,
    FactsUnavailable,
    InFight,
    DeathSettlement,
    ActivityHeld,
    ActivityHandoff,
    ActivityReturn,
    ItemEffects,
    FenceWait,
    FenceRefused,
    SpellSave,
    MonkSave,
    GourmetCheckpoint,
    ConditionsSave,
    KillsDeferred,
    UnendedUnknown,
    NotExpired,
    ReleaseUnknown,
    AttemptsExhausted,
}

const REASONS: usize = DeferReason::AttemptsExhausted as usize + 1;

impl DeferReason {
    const fn token(self) -> &'static str {
        match self {
            Self::SessionMoved => "session_moved",
            Self::FactsUnavailable => "facts_unavailable",
            Self::InFight => "in_fight",
            Self::DeathSettlement => "death_settlement",
            Self::ActivityHeld => "activity_held",
            Self::ActivityHandoff => "activity_handoff",
            Self::ActivityReturn => "activity_return",
            Self::ItemEffects => "item_effects",
            Self::FenceWait => "fence_wait",
            Self::FenceRefused => "fence_refused",
            Self::SpellSave => "spell_save",
            Self::MonkSave => "monk_save",
            Self::GourmetCheckpoint => "gourmet_checkpoint",
            Self::ConditionsSave => "conditions_save",
            Self::KillsDeferred => "kills_deferred",
            Self::UnendedUnknown => "unended_unknown",
            Self::NotExpired => "not_expired",
            Self::ReleaseUnknown => "release_unknown",
            Self::AttemptsExhausted => "attempts_exhausted",
        }
    }
}

/// Last report and suppressed count per reason.
#[derive(Clone, Copy)]
struct Slot {
    last: Option<Instant>,
    suppressed: u64,
}

static LIMITER: StdMutex<[Slot; REASONS]> = StdMutex::new(
    [Slot {
        last: None,
        suppressed: 0,
    }; REASONS],
);

/// `Some(suppressed)` when `reason` may report at `now`, counting a suppressed one otherwise.
fn admit(slots: &mut [Slot; REASONS], reason: DeferReason, now: Instant) -> Option<u64> {
    let slot = slots.get_mut(reason as usize)?;
    if slot
        .last
        .is_some_and(|last| now.duration_since(last) < REPORT_INTERVAL)
    {
        slot.suppressed = slot.suppressed.saturating_add(1);
        return None;
    }
    slot.last = Some(now);
    Some(std::mem::take(&mut slot.suppressed))
}

/// The expired grace being released: its fixed deadline and the generations it ran under.
#[derive(Clone, Copy)]
pub(in crate::gameplay_transport) struct ExpiredGrace {
    pub(in crate::gameplay_transport) grace_deadline: i64,
    pub(in crate::gameplay_transport) control_loss_epoch: u64,
    pub(in crate::gameplay_transport) connection_generation: u64,
}

fn release_deferred_line(
    reason: DeferReason,
    grace: ExpiredGrace,
    attempts: u32,
    suppressed: u64,
    now_unix_ms: u64,
) -> oteryn_error_codes::Line<'static> {
    let deadline_ms = u64::try_from(grace.grace_deadline)
        .unwrap_or(0)
        .saturating_mul(1_000);
    oteryn_error_codes::Line::new(
        oteryn_error_codes::Level::Warn,
        "gameplay_transport",
        "release_deferred",
    )
    .field(
        "reason",
        oteryn_error_codes::FieldValue::Token(reason.token()),
    )
    .field(
        "waited_ms",
        oteryn_error_codes::FieldValue::U64(now_unix_ms.saturating_sub(deadline_ms)),
    )
    .field(
        "attempts",
        oteryn_error_codes::FieldValue::U64(u64::from(attempts)),
    )
    .field(
        "control_loss_epoch",
        oteryn_error_codes::FieldValue::U64(grace.control_loss_epoch),
    )
    .field(
        "connection_generation",
        oteryn_error_codes::FieldValue::U64(grace.connection_generation),
    )
    .field(
        "suppressed",
        oteryn_error_codes::FieldValue::U64(suppressed),
    )
}

/// Reports one deferral of an expired grace release, rate-limited per reason. `waited_ms`
/// counts from the grace deadline, so it keeps growing across the lifecycle's re-runs.
pub(in crate::gameplay_transport) fn report_release_deferred(
    reason: DeferReason,
    grace: Option<ExpiredGrace>,
    attempts: u32,
) {
    let Some(grace) = grace else {
        return;
    };
    let suppressed = {
        let Ok(mut slots) = LIMITER.lock() else {
            return;
        };
        admit(&mut slots, reason, Instant::now())
    };
    let Some(suppressed) = suppressed else {
        return;
    };
    let now_unix_ms = oteryn_error_codes::unix_ms();
    let trace = super::super::connection::diagnostic_connection_trace()
        .map(super::super::format_connection_trace);
    let mut line = release_deferred_line(reason, grace, attempts, suppressed, now_unix_ms);
    if let Some(trace) = trace.as_deref() {
        line = line.trace(trace);
    }
    oteryn_error_codes::emit("oteryn-game-server", &line);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> [Slot; REASONS] {
        [Slot {
            last: None,
            suppressed: 0,
        }; REASONS]
    }

    #[test]
    fn a_stuck_reason_reports_once_per_interval_with_the_suppressed_count() {
        let mut slots = empty();
        let start = Instant::now();
        assert_eq!(admit(&mut slots, DeferReason::FenceWait, start), Some(0));
        for step in 1..=3 {
            let now = start + Duration::from_secs(step);
            assert_eq!(admit(&mut slots, DeferReason::FenceWait, now), None);
        }
        // Another reason is limited on its own.
        assert_eq!(admit(&mut slots, DeferReason::InFight, start), Some(0));
        let later = start + REPORT_INTERVAL;
        assert_eq!(admit(&mut slots, DeferReason::FenceWait, later), Some(3));
        assert_eq!(admit(&mut slots, DeferReason::FenceWait, later), None);
    }

    #[test]
    fn every_reason_has_its_own_slot_and_a_distinct_token() {
        let all = [
            DeferReason::SessionMoved,
            DeferReason::FactsUnavailable,
            DeferReason::InFight,
            DeferReason::DeathSettlement,
            DeferReason::ActivityHeld,
            DeferReason::ActivityHandoff,
            DeferReason::ActivityReturn,
            DeferReason::ItemEffects,
            DeferReason::FenceWait,
            DeferReason::FenceRefused,
            DeferReason::SpellSave,
            DeferReason::MonkSave,
            DeferReason::GourmetCheckpoint,
            DeferReason::ConditionsSave,
            DeferReason::KillsDeferred,
            DeferReason::UnendedUnknown,
            DeferReason::NotExpired,
            DeferReason::ReleaseUnknown,
            DeferReason::AttemptsExhausted,
        ];
        assert_eq!(all.len(), REASONS);
        let mut slots = empty();
        let now = Instant::now();
        let mut tokens = std::collections::BTreeSet::new();
        for reason in all {
            assert_eq!(admit(&mut slots, reason, now), Some(0));
            assert!(tokens.insert(reason.token()));
        }
    }

    #[test]
    fn the_line_names_the_reason_wait_and_generations_without_identities() {
        let grace = ExpiredGrace {
            grace_deadline: 1_700_000_000,
            control_loss_epoch: 4,
            connection_generation: 7,
        };
        let line = release_deferred_line(DeferReason::FenceWait, grace, 3, 2, 1_700_000_012_345);
        let text = line.render("oteryn-game-server", 0);
        for expected in [
            "event=release_deferred",
            "reason=fence_wait",
            "waited_ms=12345",
            "attempts=3",
            "control_loss_epoch=4",
            "connection_generation=7",
            "suppressed=2",
        ] {
            assert!(text.contains(expected), "{expected} missing in {text}");
        }
    }
}
