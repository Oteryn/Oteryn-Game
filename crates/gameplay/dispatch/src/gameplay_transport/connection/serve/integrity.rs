//! SEC-1: bounded command work and anonymous, coalesced server refusal observations.
//! These are operational observations, never bot verdicts or sanction inputs.
use super::*;
use std::time::Duration;
use tokio::time::Instant;

// Aggregate connection ceiling, deliberately above the existing 25/s per-mechanic limits
// so legitimate combinations and short bursts fit. Mechanic clocks remain authoritative.
const COMMANDS_PER_SECOND: usize = 100;
const WINDOW: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Default)]
struct Counts {
    flood: u64,
    cadence: u64,
    exhaust: u64,
    reach: u64,
}

impl Counts {
    fn any(&self) -> bool {
        self.flood != 0 || self.cadence != 0 || self.exhaust != 0 || self.reach != 0
    }

    fn emit(&self) {
        // Closed numeric schema: no account, actor, session, command payload, coordinates,
        // name or other player-supplied text. At most one line/window plus a final flush.
        crate::gameplay_transport::operator_event(&format!(
            "event=command_integrity flood={} cadence={} exhaust={} reach={}",
            self.flood, self.cadence, self.exhaust, self.reach
        ));
    }
}

pub(super) struct Integrity {
    // Boxed: the window moves through several connection futures and must not grow them.
    recent: Box<[Option<Instant>; COMMANDS_PER_SECOND]>,
    cursor: usize,
    counts: Counts,
    emitted_at: Option<Instant>,
}

impl Default for Integrity {
    fn default() -> Self {
        Self {
            recent: Box::new([None; COMMANDS_PER_SECOND]),
            cursor: 0,
            counts: Counts::default(),
            emitted_at: None,
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
        let counter = match dispatch {
            Dispatch::Step(outcome, _) if outcome.disposition == StepDisposition::TooEarly => {
                &mut self.counts.cadence
            }
            Dispatch::Use(outcome) if outcome.disposition == UseDisposition::Exhausted => {
                &mut self.counts.exhaust
            }
            Dispatch::Spell(outcome) | Dispatch::Chat(_, Some(outcome))
                if outcome.disposition == SpellCastDisposition::CoolingDown =>
            {
                &mut self.counts.exhaust
            }
            Dispatch::Use(outcome) if outcome.disposition == UseDisposition::TooFar => {
                &mut self.counts.reach
            }
            Dispatch::Spell(outcome) | Dispatch::Chat(_, Some(outcome))
                if outcome.disposition == SpellCastDisposition::TargetIllegal =>
            {
                &mut self.counts.reach
            }
            Dispatch::Attack(AttackIntentDisposition::TargetNotVisible)
            | Dispatch::ItemMove(ItemMoveOutcome::TooFar, _, _)
            | Dispatch::UseItem(OpenDecision::TooFar, _) => &mut self.counts.reach,
            _ => return,
        };
        *counter = counter.saturating_add(1);
        self.flush(Instant::now());
    }

    fn take(&mut self, now: Instant) -> Option<Counts> {
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
        if self.counts.any() {
            self.counts.emit();
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
