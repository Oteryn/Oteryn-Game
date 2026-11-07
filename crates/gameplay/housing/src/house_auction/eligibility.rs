//! PhysicalHouseEligibility/v1 calculation over independently captured inputs.
//! These facts are neither a catalogue qualification nor mutation authority.

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Eligibility {
    pub active: bool,
    pub account_matches: bool,
    pub world_matches: bool,
    pub level: u32,
    pub junior: bool,
    pub slot_occupied: bool,
    pub banned: bool,
    pub premium_allowed: bool,
}

impl Eligibility {
    /// Stable audit reason: zero eligible; 1 nomination, 2 junior, 3 level,
    /// 4 personal slot, 5 ban, 6 Premium. HOUSE-OWN-0 §4 / SOCIAL-MAP §1.4.
    pub fn refusal(self) -> u32 {
        if !self.active || !self.account_matches || !self.world_matches {
            1
        } else if self.junior {
            2
        } else if self.level < 20 {
            3
        } else if self.slot_occupied {
            4
        } else if self.banned {
            5
        } else if !self.premium_allowed {
            6
        } else {
            0
        }
    }
}

#[cfg(test)]
#[path = "eligibility_tests.rs"]
mod tests;
