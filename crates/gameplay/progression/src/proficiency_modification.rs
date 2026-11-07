//! Pure proficiency shaping identities and registered usage bounds.
use oteryn_gameplay_domain::domain::weapon_proficiency::ProficiencyModificationCost;

/// `DUR03-RL-03-PROF` and `DUR03-RL-06-PROF` (PROFICIENCY-1B §7.2): the value lines, participants
/// and effect work units of one `perk_modification` transaction. The dust shape is one value line
/// (the dust entry), one participant (the Character) and four work units (the receipt, its track
/// line, its modification line and the dust entry); a free operation has no value line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProficiencyModificationUsage {
    pub value_lines: u8,
    pub participants: u8,
    pub work_units: u8,
}

impl ProficiencyModificationUsage {
    /// The registered maxima.
    pub const MAX: Self = Self {
        value_lines: 1,
        participants: 1,
        work_units: 4,
    };

    /// The usage of a plan with this bound cost.
    pub const fn of(cost: ProficiencyModificationCost) -> Self {
        let value_lines = if cost.dust > 0 { 1 } else { 0 };
        Self {
            value_lines,
            participants: 1,
            work_units: 3 + value_lines,
        }
    }

    /// Within every registered maximum; checked before any write.
    pub const fn admitted(self) -> bool {
        self.value_lines <= Self::MAX.value_lines
            && self.participants <= Self::MAX.participants
            && self.work_units <= Self::MAX.work_units
    }
}

/// The shaping definition of a Proficiency definition (PROFICIENCY-1B §3.1):
/// `oteryn:proficiency.tibia.p<Id>` is shaped by `oteryn:proficiency-shaping.tibia.p<Id>`.
pub fn proficiency_shaping_key(definition_key: &str) -> Option<String> {
    definition_key
        .strip_prefix("oteryn:proficiency.")
        .map(|rest| format!("oteryn:proficiency-shaping.{rest}"))
}
