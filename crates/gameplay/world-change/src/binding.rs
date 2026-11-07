//! Declarative inputs selected by an admitted NPC or native World selector.
//! Clients do not supply event identity, action, cost, time or level policy.
use crate::timed::{EventKind, Input};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub event: EventKind,
    pub action: Input,
    #[serde(default)]
    pub minimum_level: u32,
    #[serde(default)]
    pub maximum_level: Option<u32>,
}
impl Binding {
    pub fn valid(&self) -> bool {
        if self
            .maximum_level
            .is_some_and(|max| max < self.minimum_level)
        {
            return false;
        }
        matches!(
            (self.event, self.action),
            (_, Input::Observe)
                | (
                    EventKind::SpringIntoLife,
                    Input::BeginObject(1..=2) | Input::Reward
                )
                | (
                    EventKind::RiseOfDevovorga | EventKind::Kingsday,
                    Input::Reward
                )
                | (EventKind::DevovorgasEssence, Input::Enter(0..=4))
                | (
                    EventKind::BankRobbery,
                    Input::ClaimObject(0) | Input::Recover
                )
                | (
                    EventKind::InsectoidInvasion,
                    Input::Fill(0) | Input::Extinguish(1..=3)
                )
        )
    }
    pub fn level_admitted(&self, level: u32) -> bool {
        level >= self.minimum_level && self.maximum_level.is_none_or(|max| level <= max)
    }
    /// Authored fallback price. A paid caller must debit this exact source item
    /// in the same Quest transaction before invoking the World transition.
    pub fn cost(&self) -> Option<(u32, u16)> {
        matches!(
            (self.event, self.action),
            (EventKind::DevovorgasEssence, Input::Enter(_))
                | (EventKind::SpringIntoLife, Input::Observe)
        )
        .then_some((3043, 1))
    }
}
