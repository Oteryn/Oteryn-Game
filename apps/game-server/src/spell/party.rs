//! Party area buffs, `native_behavior` key `party_buff`
//! (`docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` part C.3, owner S27).
//!
//! Picks the party members one cast affects and computes the cast's mana. The party itself comes
//! from a [`PartyWorld`]; no party service exists yet, so the live adapter is [`SoloParty`] and
//! every party cast fails as for a caster without a party (C.3 step 1). The member condition is an
//! ordinary Effect, applied by its owner.

use std::collections::{BTreeMap, BTreeSet};

use super::SpellEffect;
use super::chain::{ChainCreature, TilePosition};

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

/// World facts one party cast reads.
pub(crate) trait PartyWorld {
    fn caster(&self) -> &ChainCreature;
    /// The caster's party (leader and members, the caster included); `None` without a party.
    fn party(&self) -> Option<&[ChainCreature]>;
}

/// The live adapter while no party service exists: every caster is solo.
pub(crate) struct SoloParty(pub(crate) ChainCreature);

impl PartyWorld for SoloParty {
    fn caster(&self) -> &ChainCreature {
        &self.0
    }

    fn party(&self) -> Option<&[ChainCreature]> {
        None
    }
}

/// Why a party cast failed before anything was spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartyFailure {
    /// No party, or fewer than `min_affected` members in the area ("No party members in range.").
    NoMembersInRange,
    /// The scaled cost does not fit the exact integer computation.
    CostOverflow,
}

impl PartyBuffSpec {
    /// The members one cast affects, ordered by creature id: party members on the caster's floor
    /// inside the area. The caster must be listed in its own party.
    pub(crate) fn affected<'a>(
        &self,
        world: &'a dyn PartyWorld,
    ) -> Result<Vec<&'a ChainCreature>, PartyFailure> {
        let caster = world.caster();
        let party = world.party().ok_or(PartyFailure::NoMembersInRange)?;
        if !party.iter().any(|member| member.id == caster.id) {
            return Err(PartyFailure::NoMembersInRange);
        }
        let affected: BTreeMap<u64, &ChainCreature> = party
            .iter()
            .filter(|member| self.covers(caster.position, member.position))
            .map(|member| (member.id, member))
            .collect();
        if affected.len() < usize::try_from(self.min_affected).unwrap_or(usize::MAX) {
            return Err(PartyFailure::NoMembersInRange);
        }
        Ok(affected.into_values().collect())
    }

    fn covers(&self, centre: TilePosition, tile: TilePosition) -> bool {
        tile.floor == centre.floor && self.area.contains(&(tile.x - centre.x, tile.y - centre.y))
    }

    /// Mana of a cast affecting `members` creatures, rounded up in exact integer arithmetic.
    pub(crate) fn mana_cost(&self, members: usize) -> Result<u32, PartyFailure> {
        let (base, numerator, denominator) = match self.mana {
            PartyMana::Fixed(mana) => return Ok(mana),
            PartyMana::Scaled {
                base,
                numerator,
                denominator,
            } => (base, numerator, denominator),
        };
        let members = u32::try_from(members).map_err(|_| PartyFailure::CostOverflow)?;
        let exponent = members.saturating_sub(1);
        let power = |value: u32| u128::from(value).checked_pow(exponent);
        let top = power(numerator)
            .and_then(|power| power.checked_mul(u128::from(base)))
            .and_then(|value| value.checked_mul(u128::from(members)))
            .ok_or(PartyFailure::CostOverflow)?;
        let bottom = power(denominator).ok_or(PartyFailure::CostOverflow)?;
        u32::try_from(top.div_ceil(bottom)).map_err(|_| PartyFailure::CostOverflow)
    }
}
