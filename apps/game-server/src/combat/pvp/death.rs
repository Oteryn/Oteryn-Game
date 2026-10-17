//! PVP-DEATH-1 leaf: PARTY-PVP0 §10 and DEATH-COMPLETE0 §4.3/§4.6 outcomes.
//!
//! Inputs and results are data, not current authority or proof of a committed death.
//! The owning death transaction must independently qualify the lethal-time ledger,
//! World, skull, blessings, equipment and current fences, bind every semantic input,
//! and persist the result once. D1105 leaves that shared writer with #2193 until merge.
//! This leaf neither writes SQL nor grants respawn/controller authority.

use super::legality::WorldPvpType;
use crate::domain::death::{
    EquipmentSlot, EquippedItem, ITEM_LOSS_EXEMPT_MAX_LEVEL, PveDeathError, PveDeathInput,
    calculate_pve_death_with_bless, death_experience_loss_with_bless,
};
use crate::durability::pvp_state::PvpSkull;
use oteryn_simulation_determinism::{DecisionOccurrenceId, ExactI64, GameplayDecisionRoot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FinalBlow {
    Other,
    Character,
    CharacterSummon,
}

/// Complete five-minute totals, including the lethal hit, supplied by PVP-RT.
/// This leaf does not invent missing buckets or choose their unresolved endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeathDamage {
    pub(crate) total: u64,
    pub(crate) pvp: u64,
    pub(crate) final_blow: FinalBlow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeathInput<'a, I> {
    pub(crate) ordinary: PveDeathInput<'a, I>,
    pub(crate) world: WorldPvpType,
    pub(crate) damage: DeathDamage,
    pub(crate) skull_at_death: PvpSkull,
    pub(crate) twist_of_fate_held: bool,
    pub(crate) adventurer_forfeited: bool,
    /// Current admitted Bless assignment, using the native 0/6/9/12 validation.
    pub(crate) bless_reduction_percent: u8,
}

/// Semantic fields to be bound/persisted by the sole receipt writer; no new codec/schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReceiptFields {
    pub(crate) pvp_death: bool,
    pub(crate) skull_at_death: PvpSkull,
    pub(crate) twist_of_fate_used: bool,
    pub(crate) adventurer_applied: bool,
    /// PARTY-PVP0 §10.2: no sourced unfair-fight formula; always zero.
    pub(crate) unfair_fight_milli: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RespawnVitals {
    Full,
    BlackSkull { hp: u32, mana: u32 },
}

impl ReceiptFields {
    /// At placement use the original receipt's skull, never today's effective skull.
    pub(crate) fn respawn_vitals(self) -> RespawnVitals {
        match self.skull_at_death {
            PvpSkull::Black => RespawnVitals::BlackSkull { hp: 40, mana: 0 },
            PvpSkull::None | PvpSkull::Red => RespawnVitals::Full,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeathOutcome<I> {
    pub(crate) receipt: ReceiptFields,
    pub(crate) experience_lost: ExactI64,
    pub(crate) experience_after: ExactI64,
    pub(crate) regular_blessings_consumed: u8,
    pub(crate) amulet_of_loss_consumed: Option<I>,
    pub(crate) lost_items: Vec<EquippedItem<I>>,
    pub(crate) grants_empty_bag: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeathError {
    EmptyDamage,
    PvpDamageExceedsTotal,
    /// A red/black skull and an un-forfeited Adventurer benefit cannot be reconciled here.
    ContradictoryAdventurerSkull,
    Ordinary(PveDeathError),
}

impl From<PveDeathError> for DeathError {
    fn from(value: PveDeathError) -> Self {
        Self::Ordinary(value)
    }
}

pub(crate) fn is_pvp_death(damage: DeathDamage) -> Result<bool, DeathError> {
    if damage.total == 0 {
        return Err(DeathError::EmptyDamage);
    }
    if damage.pvp > damage.total {
        return Err(DeathError::PvpDamageExceedsTotal);
    }
    Ok(damage.final_blow != FinalBlow::Other
        || u128::from(damage.pvp) * 100 >= u128::from(damage.total) * 5)
}

pub(crate) fn calculate_death<I: Clone>(
    input: &DeathInput<'_, I>,
    decision_root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
) -> Result<DeathOutcome<I>, DeathError> {
    let pvp_death = is_pvp_death(input.damage)?;
    let ordinary = &input.ordinary;
    if ordinary.total_experience.get() < 0 {
        return Err(PveDeathError::NegativeExperience.into());
    }
    // Reuse the existing D58/D59/D68 exact XP formula, including input validation.
    let raw_loss = death_experience_loss_with_bless(
        ordinary.level,
        ordinary.regular_blessings,
        ordinary.promotion_benefit_current,
        input.bless_reduction_percent,
    )?;
    let mut equipment = ordinary.equipment.to_vec();
    equipment.sort_by_key(|entry| entry.slot);
    if equipment
        .windows(2)
        .any(|pair| pair[0].slot == pair[1].slot)
    {
        return Err(PveDeathError::DuplicateSlot.into());
    }
    if ordinary.necklace_is_amulet_of_loss
        && !equipment
            .iter()
            .any(|item| item.slot == EquipmentSlot::Necklace)
    {
        return Err(PveDeathError::AmuletOfLossNotWorn.into());
    }
    let adventurer_applied = pvp_death
        && input.world == WorldPvpType::Open
        && ordinary.level <= 20
        && !input.adventurer_forfeited;
    let skull_loss = input.skull_at_death != PvpSkull::None;
    if adventurer_applied && skull_loss {
        return Err(DeathError::ContradictoryAdventurerSkull);
    }
    let exempt = ordinary.level <= ITEM_LOSS_EXEMPT_MAX_LEVEL || !ordinary.has_vocation;
    let twist_of_fate_used = pvp_death
        && !skull_loss
        && !adventurer_applied
        && input.twist_of_fate_held
        && ordinary.regular_blessings > 0;
    let receipt = ReceiptFields {
        pvp_death,
        skull_at_death: input.skull_at_death,
        twist_of_fate_used,
        adventurer_applied,
        unfair_fight_milli: 0,
    };
    // Forced no-loss/all-item outcomes happen before AoL selection or any draw.
    if adventurer_applied || skull_loss || exempt {
        let lost_items = if skull_loss && !exempt {
            equipment
        } else {
            Vec::new()
        };
        let experience_lost = if adventurer_applied {
            0
        } else {
            i64::try_from(raw_loss)
                .unwrap_or(i64::MAX)
                .min(ordinary.total_experience.get())
        };
        return Ok(DeathOutcome {
            receipt,
            experience_lost: ExactI64::new(experience_lost),
            experience_after: ExactI64::new(ordinary.total_experience.get() - experience_lost),
            regular_blessings_consumed: if adventurer_applied || twist_of_fate_used {
                0
            } else {
                ordinary.regular_blessings
            },
            amulet_of_loss_consumed: None,
            grants_empty_bag: lost_items
                .iter()
                .any(|item| item.slot == EquipmentSlot::Container),
            lost_items,
        });
    }
    // Ordinary PvP keeps the canonical PvE sampler, draw purpose, slot order and XP.
    let loss = calculate_pve_death_with_bless(
        ordinary,
        input.bless_reduction_percent,
        decision_root,
        occurrence,
    )?;
    Ok(DeathOutcome {
        receipt,
        experience_lost: loss.experience_lost,
        experience_after: loss.experience_after,
        regular_blessings_consumed: if twist_of_fate_used {
            0
        } else {
            loss.blessings_consumed
        },
        amulet_of_loss_consumed: loss.amulet_of_loss_consumed,
        lost_items: loss.lost_items,
        grants_empty_bag: loss.grants_empty_bag,
    })
}

#[cfg(test)]
#[path = "death_tests.rs"]
mod tests;
