//! A vocationless maximum must stay an absolute saved pool across the first choice.
use super::{PlayerSpellState, Vocation};

impl PlayerSpellState {
    /// The existing position projection saves explicit vocationless pools even at maxima.
    /// A capture made before the choice can therefore never refill the new class on restart.
    /// Classed actors at both maxima retain CHAR-VITALS-1's compact NULL representation.
    pub const fn saved_vitals(&self) -> Option<(u32, u32)> {
        if !matches!(self.facts.vocation, Vocation::None)
            && self.health == self.facts.max_health
            && self.mana == self.facts.max_mana
        {
            None
        } else {
            Some((self.health, self.mana))
        }
    }
}
