//! VITALS-SOUL-1: the soul kill window (Canary `CONDITION_SOUL`). A monster kill whose applied
//! experience reaches the killer's level opens a 240 s window, or replaces the remaining one;
//! positive experience below the level closes it. While it runs, the actor gains one soul point
//! every 120 s, or every 15 s for a promoted Premium Character, up to its maximum. The gain
//! cadence survives a replaced window, as Canary's `ConditionSoul::addCondition` keeps its
//! internal ticks. Inside a protection zone no soul is gained: a gain that falls due there is
//! held, and the first owner cycle outside credits it once and restarts the cadence from that
//! cycle, as Canary's `ConditionSoul::executeCondition` keeps counting ticks in the zone and
//! resets them on its first gain outside. Runtime-only: a fresh actor and a dead one have no
//! window.
use super::cast::PlayerSpellState;

const MICROS_PER_SECOND: u64 = 1_000_000;
/// The slow and fast `gainsoulticks` of Canary's vocations, in seconds.
pub const SOUL_GAIN_SECONDS: u32 = 120;
pub const PROMOTED_SOUL_GAIN_SECONDS: u32 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoulWindow {
    ends_at_us: u64,
    next_gain_us: u64,
    promoted: bool,
    /// A gain fell due inside a protection zone and waits for the first cycle outside.
    held: bool,
}

/// What a settled kill does to the killer's window (`SoulKillHook` without `NoHook`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulWindowChange {
    Open { seconds: u32, promoted: bool },
    Close,
}

impl PlayerSpellState {
    fn soul_gain_us(&self, promoted: bool) -> u64 {
        let seconds = if promoted && self.premium_current {
            PROMOTED_SOUL_GAIN_SECONDS
        } else {
            SOUL_GAIN_SECONDS
        };
        u64::from(seconds) * MICROS_PER_SECOND
    }

    /// Applies one kill's change at `now_us`. A dead actor keeps no window.
    pub fn change_soul_window(&mut self, change: SoulWindowChange, now_us: u64) {
        self.soul_window = match change {
            SoulWindowChange::Close => None,
            SoulWindowChange::Open { .. } if self.health == 0 => None,
            SoulWindowChange::Open { seconds, promoted } => {
                let ends_at_us = now_us.saturating_add(u64::from(seconds) * MICROS_PER_SECOND);
                let (next_gain_us, held) = match self.soul_window {
                    Some(window) if window.ends_at_us >= now_us => {
                        (window.next_gain_us, window.held)
                    }
                    _ => (now_us.saturating_add(self.soul_gain_us(promoted)), false),
                };
                Some(SoulWindow {
                    ends_at_us,
                    next_gain_us,
                    promoted,
                    held,
                })
            }
        };
    }

    fn gain_soul(&mut self) {
        if self.health > 0 {
            self.soul = self.soul.saturating_add(1).min(self.facts.max_soul);
        }
    }

    /// Credits every gain due by `now_us` inside the window, then drops an ended window.
    /// Inside a protection zone a due gain is held instead; an unknown zone credits as outside.
    pub(super) fn credit_soul_window(&mut self, now_us: u64, in_protection_zone: Option<bool>) {
        let Some(mut window) = self.soul_window else {
            return;
        };
        let until = now_us.min(window.ends_at_us);
        if in_protection_zone == Some(true) {
            window.held |= window.next_gain_us <= until;
        } else {
            if window.held && now_us <= window.ends_at_us {
                self.gain_soul();
                window.next_gain_us = now_us.saturating_add(self.soul_gain_us(window.promoted));
            }
            window.held = false;
            while window.next_gain_us <= until {
                self.gain_soul();
                window.next_gain_us = window
                    .next_gain_us
                    .saturating_add(self.soul_gain_us(window.promoted));
            }
        }
        self.soul_window = (window.ends_at_us > now_us).then_some(window);
    }

    #[must_use]
    pub const fn soul_window(&self) -> Option<SoulWindow> {
        self.soul_window
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::Vocation;
    use super::super::actor_conditions::stage_source_owner_cycle;
    use super::super::cast::CharacterCastFacts;
    use super::*;
    use oteryn_simulation_determinism::SemanticTimeMicros;

    const S: u64 = MICROS_PER_SECOND;

    fn actor(soul: u32) -> PlayerSpellState {
        let mut state = PlayerSpellState::new(
            CharacterCastFacts {
                vocation: Vocation::Knight,
                level: 20,
                magic_level: 1,
                max_health: 300,
                max_mana: 100,
                max_soul: 100,
            },
            0,
            0,
        )
        .expect("actor");
        state.soul = soul;
        state
    }

    fn open(promoted: bool) -> SoulWindowChange {
        SoulWindowChange::Open {
            seconds: 240,
            promoted,
        }
    }

    /// One owner cycle at `second`; the soul it leaves.
    fn cycle(state: &mut PlayerSpellState, second: u64) -> u32 {
        let now = SemanticTimeMicros::from_micros(second * S);
        if let Some(cycle) = stage_source_owner_cycle(state, now, None, None).expect("cycle") {
            *state = cycle.next;
        }
        state.soul
    }

    /// One owner cycle at `second` with a known zone; the soul it leaves.
    fn zoned(state: &mut PlayerSpellState, second: u64, in_protection_zone: bool) -> u32 {
        let now = SemanticTimeMicros::from_micros(second * S);
        let facts = oteryn_gameplay_foundation::foundation::condition::TickFacts {
            in_protection_zone,
            standing_on_field: None,
        };
        if let Some(cycle) = stage_source_owner_cycle(state, now, Some(facts), None).expect("cycle")
        {
            *state = cycle.next;
        }
        state.soul
    }

    #[test]
    fn a_window_gains_one_soul_every_120_seconds_until_it_ends() {
        let mut state = actor(10);
        state.change_soul_window(open(false), 0);
        assert_eq!(cycle(&mut state, 119), 10);
        assert_eq!(cycle(&mut state, 120), 11);
        assert_eq!(cycle(&mut state, 240), 12);
        assert_eq!(state.soul_window(), None);
        assert_eq!(cycle(&mut state, 400), 12);
    }

    #[test]
    fn promotion_gains_every_15_seconds_only_with_premium() {
        let mut slow = actor(0);
        slow.change_soul_window(open(true), 0);
        assert_eq!(cycle(&mut slow, 240), 2);

        let mut fast = actor(0);
        fast.premium_current = true;
        fast.change_soul_window(open(true), 0);
        assert_eq!(cycle(&mut fast, 240), 16);
    }

    #[test]
    fn a_replaced_window_keeps_its_cadence_and_a_close_ends_it() {
        let mut state = actor(0);
        state.change_soul_window(open(false), 0);
        assert_eq!(cycle(&mut state, 100), 0);
        state.change_soul_window(open(false), 100);
        // The gain stays due at 120 s; the window now ends at 340 s.
        assert_eq!(cycle(&mut state, 120), 1);
        assert_eq!(cycle(&mut state, 340), 2);
        state.change_soul_window(open(false), 400);
        state.change_soul_window(SoulWindowChange::Close, 401);
        assert_eq!(cycle(&mut state, 600), 2);
    }

    #[test]
    fn soul_stops_at_the_maximum_and_a_dead_actor_opens_nothing() {
        let mut full = actor(100);
        full.change_soul_window(open(false), 0);
        assert_eq!(cycle(&mut full, 240), 100);

        let mut dead = actor(5);
        dead.health = 0;
        dead.change_soul_window(open(false), 0);
        assert_eq!(dead.soul_window(), None);
    }

    #[test]
    fn a_protection_zone_holds_the_due_gain_until_the_first_cycle_outside() {
        let mut state = actor(10);
        state.change_soul_window(
            SoulWindowChange::Open {
                seconds: 600,
                promoted: false,
            },
            0,
        );
        assert_eq!(zoned(&mut state, 120, true), 10);
        assert_eq!(zoned(&mut state, 150, true), 10);
        // One gain on leaving, and the cadence restarts from that cycle.
        assert_eq!(zoned(&mut state, 200, false), 11);
        assert_eq!(zoned(&mut state, 319, false), 11);
        assert_eq!(zoned(&mut state, 320, false), 12);
        assert_eq!(zoned(&mut state, 440, false), 13);
    }

    #[test]
    fn a_held_gain_survives_a_replaced_window_and_is_lost_when_the_window_ends() {
        let mut replaced = actor(0);
        replaced.change_soul_window(open(false), 0);
        assert_eq!(zoned(&mut replaced, 130, true), 0);
        replaced.change_soul_window(open(false), 135);
        assert_eq!(zoned(&mut replaced, 140, false), 1);

        let mut ended = actor(0);
        ended.change_soul_window(open(false), 0);
        assert_eq!(zoned(&mut ended, 120, true), 0);
        assert_eq!(zoned(&mut ended, 240, true), 0);
        assert_eq!(ended.soul_window(), None);
        assert_eq!(zoned(&mut ended, 241, false), 0);
    }
}
