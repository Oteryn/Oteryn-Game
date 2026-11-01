//! Crystal `201189f` Hireling services; persistence and NPC handles stay with their owners.
pub mod catalogue;
pub mod dialogue;
pub mod food;

/// Character-wide apprenticeships (Crystal `data/libs/systems/hireling.lua:6-10,283-292`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Skill {
    Banker = 1001,
    Cook = 1002,
    Steward = 1003,
    Trader = 1004,
}

impl Skill {
    pub const fn owned(self, skills: u8) -> bool {
        skills & (1 << (self as u16 - 1001)) != 0
    }
}
