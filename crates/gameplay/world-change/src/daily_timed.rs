//! New timed activations are part of the immutable reset snapshot. Old stored
//! days decode inactive; publishing a new rule never rerolls an existing day.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default)]
pub struct Candidates {
    pub bank: bool,
    pub spring: bool,
    pub rise: bool,
    pub essence_permille: u16,
    pub kingsday_permille: u16,
    pub insectoid_permille: u16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    #[serde(default)]
    pub bank: bool,
    pub spring: bool,
    pub rise: bool,
    pub essence: bool,
    pub kingsday: bool,
    pub insectoid: bool,
}

impl Candidates {
    pub fn select(self, month: u8, day: u8, mut draw: impl FnMut(&str) -> u64) -> Selection {
        let mut roll = |purpose, chance| {
            chance > 0
                && chance <= 1000
                && (u128::from(draw(purpose)) * 1000) >> 64 < u128::from(chance)
        };
        Selection {
            bank: self.bank,
            spring: self.spring && month == 4 && (16..23).contains(&day),
            rise: self.rise && month == 9 && (1..8).contains(&day),
            essence: roll("wevents-essence", self.essence_permille),
            kingsday: roll("wevents-kingsday", self.kingsday_permille),
            insectoid: roll("wevents-insectoid", self.insectoid_permille),
        }
    }
}
