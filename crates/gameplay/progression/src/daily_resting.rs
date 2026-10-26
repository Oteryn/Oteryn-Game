//! Daily-reward streak benefits that are eligible while a player is in a resting area.

/// The benefits unlocked by the current streak. Days four through seven are Premium-only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RestingBenefits {
    pub health: bool,
    pub mana: bool,
    pub stamina: bool,
    pub double_health: bool,
    pub double_mana: bool,
    pub soul: bool,
}

impl RestingBenefits {
    /// Stable status bits used by the daily-reward wall.
    pub const fn bits(self) -> u8 {
        (self.health as u8)
            | ((self.mana as u8) << 1)
            | ((self.stamina as u8) << 2)
            | ((self.double_health as u8) << 3)
            | ((self.double_mana as u8) << 4)
            | ((self.soul as u8) << 5)
    }
}

/// Resolves only streak/Premium eligibility. The caller additionally proves that the current
/// map tile is a resting area before applying these benefits.
pub const fn benefits(streak: u32, premium: bool) -> RestingBenefits {
    RestingBenefits {
        health: streak >= 2,
        mana: streak >= 3,
        stamina: premium && streak >= 4,
        double_health: premium && streak >= 5,
        double_mana: premium && streak >= 6,
        soul: premium && streak >= 7,
    }
}

/// Global restores one stamina minute every three minutes below 39 hours, otherwise every six.
pub const fn stamina_interval_seconds(stamina_minutes: u16) -> u32 {
    if stamina_minutes < 39 * 60 { 180 } else { 360 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_and_premium_gate_match_global() {
        assert_eq!(benefits(1, true), RestingBenefits::default());
        assert_eq!(benefits(2, false).bits(), 1);
        assert_eq!(benefits(3, false).bits(), 3);
        assert_eq!(benefits(7, false).bits(), 3);
        assert_eq!(benefits(4, true).bits(), 7);
        assert_eq!(benefits(7, true).bits(), 63);
    }

    #[test]
    fn stamina_uses_the_global_three_and_six_minute_intervals() {
        assert_eq!(stamina_interval_seconds(2_339), 180);
        assert_eq!(stamina_interval_seconds(2_340), 360);
    }
}
