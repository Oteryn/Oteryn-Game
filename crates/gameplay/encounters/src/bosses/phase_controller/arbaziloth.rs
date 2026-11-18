//! Shield progression from Crystal 201189f8, no_rest_for_the_wicked/arbaziloth_fight.lua.
const THRESHOLDS: [i64; 4] = [80, 60, 40, 20];
const LEAVE_DELAY_MS: u64 = 3_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShieldSequence {
    cleared: usize,
    shield: Shield,
    departure_due: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Shield {
    #[default]
    Down,
    Raised,
    Cracked,
}

impl ShieldSequence {
    /// Apply this to positive incoming damage after replay qualification.
    pub fn limit_damage(&mut self, health: i64, maximum: i64, damage: i64) -> i64 {
        if self.shield != Shield::Down {
            return 0;
        }
        let Some(&percent) = THRESHOLDS.get(self.cleared) else {
            return damage;
        };
        let threshold = maximum / 100 * percent + maximum % 100 * percent / 100;
        let allowed = health.saturating_sub(threshold).max(0);
        if damage >= allowed {
            self.shield = Shield::Raised;
        }
        damage.min(allowed)
    }

    /// The map owner calls this when the exact boss steps onto the active fire.
    /// A true return consumes that fire.
    pub fn step_on_fire(&mut self) -> bool {
        if self.shield != Shield::Raised {
            return false;
        }
        self.shield = Shield::Cracked;
        true
    }

    /// The item owner supplies its roll in 1..=100 from the qualified rod use.
    pub fn use_rod(&mut self, roll: u8, now_ms: u64) -> bool {
        if self.shield != Shield::Cracked || !(51..=100).contains(&roll) {
            return false;
        }
        self.shield = Shield::Down;
        self.cleared += 1;
        if self.cleared == THRESHOLDS.len() {
            self.departure_due = now_ms.checked_add(LEAVE_DELAY_MS);
        }
        true
    }

    pub fn forgemaster_departure_due(&self, now_ms: u64) -> bool {
        self.departure_due.is_some_and(|due| now_ms >= due)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbaziloth_four_distinct_shields_qualify_timed_departure() {
        let mut sequence = ShieldSequence::default();
        for health in [100, 80, 60, 40] {
            assert_eq!(sequence.limit_damage(health, 100, 100), 20);
            assert_eq!(sequence.limit_damage(health - 20, 100, 100), 0);
            assert!(!sequence.use_rod(51, 100));
            assert!(sequence.step_on_fire());
            assert!(!sequence.step_on_fire());
            assert!(!sequence.use_rod(50, 100));
            assert!(sequence.use_rod(51, 100));
        }
        assert_eq!(sequence.limit_damage(20, 100, 20), 20);
        assert!(!sequence.forgemaster_departure_due(3_099));
        assert!(sequence.forgemaster_departure_due(3_100));
    }
}
