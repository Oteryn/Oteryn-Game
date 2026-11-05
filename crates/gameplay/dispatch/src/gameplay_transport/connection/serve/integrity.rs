//! SEC-1: bounded command work and anonymous, coalesced server refusal observations.
//! These are operational observations, never bot verdicts or sanction inputs.
use super::*;
use std::time::Duration;
use tokio::time::Instant;

// Aggregate connection ceiling, deliberately above the existing 25/s per-mechanic limits
// so legitimate combinations and short bursts fit. Mechanic clocks remain authoritative.
const COMMANDS_PER_SECOND: usize = 100;
const WINDOW: Duration = Duration::from_secs(1);

// REPORT_ONLY: eight existing refusals in a sliding second suggest a burst.
// These thresholds never decide command admission, outcomes or sanctions.
const DETECT_WINDOW: Duration = Duration::from_secs(1);
const DETECT_THRESHOLDS: [usize; 3] = [8, 8, 8]; // cadence, use/cast exhaust, reach
const DETECT_SLOTS: usize = 8;

#[derive(Default)]
struct Detector {
    recent: [[Option<Instant>; DETECT_SLOTS]; 3],
    cursors: [usize; 3],
}

impl Detector {
    fn observe(&mut self, kind: usize, now: Instant) -> bool {
        let cursor = self.cursors[kind];
        self.recent[kind][cursor] = Some(now);
        self.cursors[kind] = (cursor + 1) % DETECT_THRESHOLDS[kind];
        self.recent[kind][self.cursors[kind]]
            .is_some_and(|oldest| now.saturating_duration_since(oldest) < DETECT_WINDOW)
    }
}

#[derive(Clone, Copy, Default)]
struct Counts {
    flood: u64,
    cadence: u64,
    exhaust: u64,
    reach: u64,
    signals: [bool; 3],
}

impl Counts {
    fn any(&self) -> bool {
        self.flood != 0 || self.cadence != 0 || self.exhaust != 0 || self.reach != 0
    }

    fn emit(&self) {
        // Closed numeric schema: no account, actor, session, command payload, coordinates,
        // name or other player-supplied text. At most one line per sliding second, including connection teardown.
        crate::gameplay_transport::operator_event(&format!(
            "event=command_integrity mode=REPORT_ONLY flood={} cadence={} exhaust={} reach={} cadence_signal={} exhaust_signal={} reach_signal={}",
            self.flood,
            self.cadence,
            self.exhaust,
            self.reach,
            u8::from(self.signals[0]),
            u8::from(self.signals[1]),
            u8::from(self.signals[2])
        ));
    }
}

pub(super) struct Integrity {
    // Boxed: the window moves through several connection futures and must not grow them.
    recent: Box<[Option<Instant>; COMMANDS_PER_SECOND]>,
    cursor: usize,
    counts: Counts,
    detector: Box<Detector>,
    emitted_at: Option<Instant>,
    observed_at: Option<Instant>,
}

impl Default for Integrity {
    fn default() -> Self {
        Self {
            recent: Box::new([None; COMMANDS_PER_SECOND]),
            cursor: 0,
            counts: Counts::default(),
            detector: Box::default(),
            emitted_at: None,
            observed_at: None,
        }
    }
}

impl Integrity {
    fn due(&self, now: Instant) -> Option<Instant> {
        self.recent[self.cursor]
            .map(|at| at + WINDOW)
            .filter(|due| *due > now)
    }

    fn charge(&mut self, now: Instant) {
        self.recent[self.cursor] = Some(now);
        self.cursor = (self.cursor + 1) % COMMANDS_PER_SECOND;
    }

    /// Returns whether this network command exceeded the sliding window. Even refused
    /// commands and retained replays consume capacity, preventing a cheap-response flood.
    pub(super) async fn admit(&mut self) -> bool {
        let now = Instant::now();
        let due = self.due(now);
        if let Some(due) = due {
            self.counts.flood = self.counts.flood.saturating_add(1);
            self.flush(now);
            tokio::time::sleep_until(due).await;
        }
        self.charge(Instant::now());
        due.is_some()
    }

    pub(super) fn observe(&mut self, dispatch: &Dispatch) {
        let (counter, kind) = match dispatch {
            Dispatch::Step(outcome, _) if outcome.disposition == StepDisposition::TooEarly => {
                (&mut self.counts.cadence, 0)
            }
            Dispatch::Use(outcome) if outcome.disposition == UseDisposition::Exhausted => {
                (&mut self.counts.exhaust, 1)
            }
            Dispatch::Spell(outcome) | Dispatch::Chat(_, Some(outcome))
                if outcome.disposition == SpellCastDisposition::CoolingDown =>
            {
                (&mut self.counts.exhaust, 1)
            }
            Dispatch::Use(outcome) if outcome.disposition == UseDisposition::TooFar => {
                (&mut self.counts.reach, 2)
            }
            Dispatch::Spell(outcome) | Dispatch::Chat(_, Some(outcome))
                if outcome.disposition == SpellCastDisposition::TargetIllegal =>
            {
                (&mut self.counts.reach, 2)
            }
            Dispatch::Attack(AttackIntentDisposition::TargetNotVisible)
            | Dispatch::ItemMove(ItemMoveOutcome::TooFar, _, _)
            | Dispatch::UseItem(OpenDecision::TooFar, _) => (&mut self.counts.reach, 2),
            _ => return,
        };
        *counter = counter.saturating_add(1);
        let now = Instant::now();
        self.observed_at.get_or_insert(now);
        self.counts.signals[kind] |= self.detector.observe(kind, now);
        self.flush(now);
    }

    fn take(&mut self, now: Instant) -> Option<Counts> {
        // Coalesce the initial refusal burst so its signal can accompany the first line.
        if self.emitted_at.is_none()
            && self.counts.flood == 0
            && !self.counts.signals.into_iter().any(|signal| signal)
            && self
                .observed_at
                .is_some_and(|at| now.saturating_duration_since(at) < WINDOW)
        {
            return None;
        }
        if !self.counts.any()
            || self
                .emitted_at
                .is_some_and(|at| now.saturating_duration_since(at) < WINDOW)
        {
            return None;
        }
        self.emitted_at = Some(now);
        Some(std::mem::take(&mut self.counts))
    }

    fn flush(&mut self, now: Instant) {
        if let Some(event) = self.take(now) {
            event.emit();
        }
    }
}

impl Drop for Integrity {
    fn drop(&mut self) {
        // Flush a short first burst, but never bypass a previous line's rate cap.
        if self.emitted_at.is_none() && self.counts.any() {
            self.counts.emit();
        } else {
            self.flush(Instant::now());
        }
    }
}

// No new wire codes: use the mechanic's existing typed refusal, or Foundation REJECTED
// for commands without an exhaust disposition. No authority method or side effect runs.
pub(super) fn refusal(session: &ServeState, command_type: u32) -> Dispatch {
    if !session.selected_capabilities.command_selected(command_type) {
        return Dispatch::Reply(CommandStatus::Rejected, Vec::new());
    }
    let payload = match command_type {
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT => encode_step_outcome(
            session.selected_capabilities.as_slice(),
            StepDisposition::TooEarly,
            0,
        ),
        oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT => {
            encode_use_result(UseDisposition::Exhausted).unwrap_or_default()
        }
        oteryn_protocol_oteryn::actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT => {
            encode_spell_cast_result(SpellCastDisposition::CoolingDown)
        }
        _ => Vec::new(),
    };
    Dispatch::Reply(CommandStatus::Rejected, payload)
}

// Test-only access to the real observer; no detector API is exposed to production callers.
#[cfg(test)]
impl Dispatch {
    pub(in crate::gameplay_transport::connection) fn sec1_detect_probe()
    -> impl FnMut(&Dispatch) -> ([bool; 3], usize) {
        let mut gate = Integrity::default();
        // Retain the coalesced report for assertions rather than writing it to stderr.
        gate.emitted_at = Some(Instant::now());
        move |dispatch| {
            gate.observe(dispatch);
            (
                gate.counts.signals,
                std::mem::size_of_val(&gate)
                    + std::mem::size_of_val(&*gate.recent)
                    + std::mem::size_of_val(&*gate.detector),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec1_existing_typed_refusals_are_coalesced_without_flagging_success()
    -> Result<(), &'static str> {
        // Keep observe's real clock behind the explicit test clock.
        let now = Instant::now() + Duration::from_secs(3600);
        let mut gate = Integrity::default();
        gate.emitted_at = Some(now);
        gate.observe(&Dispatch::Step(StepOutcome::too_early(), 0));
        for disposition in [UseDisposition::Exhausted, UseDisposition::TooFar] {
            gate.observe(&Dispatch::Use(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            }));
        }
        for disposition in [
            SpellCastDisposition::CoolingDown,
            SpellCastDisposition::TargetIllegal,
        ] {
            gate.observe(&Dispatch::Spell(SpellCastOutcome {
                disposition,
                vitals: None,
            }));
        }
        gate.observe(&Dispatch::ItemMove(
            ItemMoveOutcome::TooFar,
            Vec::new(),
            None,
        ));
        gate.observe(&Dispatch::UseItem(OpenDecision::TooFar, None));
        gate.observe(&Dispatch::Attack(AttackIntentDisposition::TargetNotVisible));
        gate.observe(&Dispatch::Spell(SpellCastOutcome {
            disposition: SpellCastDisposition::Cast,
            vitals: None,
        }));
        gate.observe(&Dispatch::Unregistered);
        let event = gate.take(now + WINDOW).ok_or("coalesced refusals")?;
        assert_eq!(
            (event.flood, event.cadence, event.exhaust, event.reach),
            (0, 1, 2, 5)
        );
        assert!(!gate.counts.any());
        Ok(())
    }

    #[test]
    fn sec1_sliding_window_and_audit_are_bounded() -> Result<(), &'static str> {
        // Keep observe's real clock behind the explicit test clock.
        let now = Instant::now() + Duration::from_secs(3600);
        let mut gate = Integrity::default();
        for _ in 0..COMMANDS_PER_SECOND {
            assert!(gate.due(now).is_none());
            gate.charge(now);
        }
        assert_eq!(gate.due(now), Some(now + WINDOW));
        assert!(gate.due(now + WINDOW).is_none());
        gate.counts.flood = 1;
        assert_eq!(gate.take(now).ok_or("first event")?.flood, 1);
        gate.counts.exhaust = u64::MAX;
        gate.observe(&Dispatch::Spell(SpellCastOutcome {
            disposition: SpellCastDisposition::CoolingDown,
            vitals: None,
        }));
        assert!(gate.take(now).is_none());
        assert_eq!(
            gate.take(now + WINDOW).ok_or("coalesced")?.exhaust,
            u64::MAX
        );
        assert!(gate.take(now + WINDOW + WINDOW).is_none());
        Ok(())
    }
}
