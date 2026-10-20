//! Authored party-buff data.
use super::SpellEffect;
use std::collections::BTreeSet;

/// How a party cast's mana is computed (C.3 steps 4 and 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartyMana {
    /// The same cost for any number of members (Enlighten Party).
    Fixed(u32),
    /// `ceil(base * falloff^(X - 1) * X)`, X = affected members including the caster.
    Scaled {
        base: u32,
        /// `falloff` as a reduced fraction `numerator / denominator` in (0, 1].
        numerator: u32,
        denominator: u32,
    },
}

/// An admitted `party_buff`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PartyBuffSpec {
    /// Tile offsets `(dx, dy)` from the caster that the area covers, on the caster's floor.
    pub(crate) area: BTreeSet<(i32, i32)>,
    /// Affected members needed, the caster included (C.3 step 3).
    pub(crate) min_affected: u32,
    pub(crate) mana: PartyMana,
    /// The Effect each affected member receives.
    pub(crate) effects: Vec<SpellEffect>,
}
