//! Pure, persistence-neutral PvE player death outcome
//! (`REFERENCE-FIRST-PLAYER-DEATH-V1` §4.3, owner decisions D58, D59, D62, D65, D66, D68).
//!
//! The caller supplies only PvE deaths (D60). The Character transaction that commits this outcome
//! and the DUR-03 item moves that follow it are separate children (DEATH-1b, DEATH-3).

use super::character_build::{
    BuildFormula, DeathBuildState, MAX_BUILD_LEVEL, MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL,
};
use oteryn_simulation_determinism::{
    DecisionError, DecisionOccurrenceId, ExactI64, GameplayDecisionRoot, deterministic_decision_u64,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathBuildLossError {
    InvalidExperienceLoss,
    InvalidBuild,
}

/// D58 / NIGHT-W1 decision 19: death preserves all eight build families exactly.
/// The caller supplies the actual capped XP loss and current content formula;
/// existing input validation still applies. Vocation, levels and progress stay
/// unchanged, so new deaths require NULL death-build fields.
pub fn calculate_death_build_loss<S: DeathBuildState>(
    formula: &dyn BuildFormula,
    before: &S,
    experience_before: ExactI64,
    experience_lost: ExactI64,
) -> Result<S, DeathBuildLossError> {
    let mut families = [(0, 0); 8];
    families[0] = before.magic();
    families[1..].copy_from_slice(&before.skills());
    let families = calculate_death_build_family_loss(
        families,
        experience_before,
        experience_lost,
        |family, next| formula.required(before.vocation(), family, next),
    )?;
    let mut skills = [(0, 0); 7];
    skills.copy_from_slice(&families[1..]);
    S::from_death_families(before.vocation(), families[0], skills)
        .ok_or(DeathBuildLossError::InvalidBuild)
}

/// Persistence-neutral value boundary of [`calculate_death_build_loss`]. Native
/// writers compiled in an independent test crate use the same arithmetic without
/// substituting a second copy of the durable Rust types for the library's types.
pub fn calculate_death_build_family_loss(
    families: [(u16, u64); 8],
    experience_before: ExactI64,
    experience_lost: ExactI64,
    required: impl Fn(usize, u16) -> Option<u64>,
) -> Result<[(u16, u64); 8], DeathBuildLossError> {
    let experience = u64::try_from(experience_before.get())
        .map_err(|_| DeathBuildLossError::InvalidExperienceLoss)?;
    let lost = u64::try_from(experience_lost.get())
        .map_err(|_| DeathBuildLossError::InvalidExperienceLoss)?;
    if lost > experience {
        return Err(DeathBuildLossError::InvalidExperienceLoss);
    }
    for (family, (level, progress)) in families.iter().enumerate() {
        let floor = if family == 0 { 0 } else { MIN_SKILL_LEVEL };
        let required = |next| required(family, next);
        // A missing requirement above the current level is the accepted ceiling,
        // but an unreachable current level or already-paid next level is invalid.
        if !(floor..=MAX_BUILD_LEVEL).contains(level)
            || *progress > MAX_BUILD_PROGRESS
            || !(floor.saturating_add(1)..=*level)
                .all(|next| required(next).is_some_and(|value| value > 0))
            || (*level < MAX_BUILD_LEVEL
                && required(*level + 1).is_some_and(|value| value == 0 || *progress >= value))
        {
            return Err(DeathBuildLossError::InvalidBuild);
        }
    }
    Ok(families)
}

/// Global has 7 regular blessings (tibia.com products 6-3.1); Twist of Fate is PvP-only.
pub const MAX_REGULAR_BLESSINGS: u8 = 7;
/// D65: no item loss up to and including this level. The post-Newhaven Global value at the
/// target date is `UNKNOWN`; this constant is the decided Reference value.
pub const ITEM_LOSS_EXEMPT_MAX_LEVEL: u32 = 8;
/// §4.3.3: the Amulet of Loss is selected only below this blessing count.
pub const AMULET_OF_LOSS_BLESSINGS_EXCLUSIVE: u8 = 5;

const ITEM_LOSS_DRAW_PURPOSE: &str = "oteryn.death.item_loss.v1";
const PER_MILLE: u128 = 1_000;
/// D62, indexed by `min(blessings, 5)`: chance in per mille that a container is lost.
pub const CONTAINER_LOSS_PER_MILLE: [u16; 6] = [1_000, 700, 450, 250, 100, 0];
/// D62, indexed by `min(blessings, 5)`: chance in per mille that another equipped item is lost.
pub const EQUIPMENT_LOSS_PER_MILLE: [u16; 6] = [100, 70, 45, 25, 10, 0];

/// Character equipment slots, in canonical order. The discriminant is the slot's draw index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EquipmentSlot {
    Head = 0,
    Necklace = 1,
    Container = 2,
    Armor = 3,
    RightHand = 4,
    LeftHand = 5,
    Legs = 6,
    Feet = 7,
    Ring = 8,
    Ammo = 9,
}

impl EquipmentSlot {
    pub const COUNT: usize = 10;

    const fn draw_index(self) -> u64 {
        self as u64
    }

    const fn loss_per_mille(self, ladder_step: usize) -> u16 {
        match self {
            Self::Container => CONTAINER_LOSS_PER_MILLE[ladder_step],
            _ => EQUIPMENT_LOSS_PER_MILLE[ladder_step],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquippedItem<I> {
    pub slot: EquipmentSlot,
    pub item: I,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PveDeathInput<'a, I> {
    pub level: u32,
    pub total_experience: ExactI64,
    /// Held regular blessings, `0..=MAX_REGULAR_BLESSINGS`.
    pub regular_blessings: u8,
    /// Promoted AND Premium current at the death transaction (D66, Premium decision D76).
    pub promotion_benefit_current: bool,
    pub has_vocation: bool,
    /// Occupied equipment slots, at most one item per slot, in any order.
    pub equipment: &'a [EquippedItem<I>],
    /// The item in the necklace slot is an Amulet of Loss.
    pub necklace_is_amulet_of_loss: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PveDeathOutcome<I> {
    pub experience_lost: ExactI64,
    pub experience_after: ExactI64,
    /// Every held regular blessing is consumed (§4.3.2).
    pub blessings_consumed: u8,
    /// The Amulet of Loss selected for a DUR-03 destroy (§4.3.3, §4.4).
    pub amulet_of_loss_consumed: Option<I>,
    /// The lost-item set in canonical slot order; a container is lost with its contents.
    pub lost_items: Vec<EquippedItem<I>>,
    /// The character lost the item in its container slot, so it receives an empty bag there
    /// (tibia.com manual §5.1.11). The bag is minted by a DUR-03 operation after the item moves.
    pub grants_empty_bag: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PveDeathError {
    InvalidLevel,
    NegativeExperience,
    TooManyBlessings,
    InvalidBlessReduction,
    DuplicateSlot,
    AmuletOfLossNotWorn,
    Overflow,
    Decision(DecisionError),
}

impl From<DecisionError> for PveDeathError {
    fn from(value: DecisionError) -> Self {
        Self::Decision(value)
    }
}

/// D58/D59/D68: `floor(((L+50)/100) × 50 × (L² − 5L + 8) × (1 − 0.08·b − 0.30·p))`, before the
/// cap at the current experience. Exact integer arithmetic; the same formula from level 1.
pub fn death_experience_loss(
    level: u32,
    regular_blessings: u8,
    promotion_benefit_current: bool,
) -> Result<u128, PveDeathError> {
    death_experience_loss_with_bless(level, regular_blessings, promotion_benefit_current, 0)
}

/// Bless reduces the residual death experience loss by 6%, 9% or 12%.
/// Zero explicitly retains the death calculation without a current Bless assignment.
/// Apply the multiplier after blessings and promotion, with the existing D68 floor once,
/// before capping the loss at the character's current experience.
pub fn death_experience_loss_with_bless(
    level: u32,
    regular_blessings: u8,
    promotion_benefit_current: bool,
    bless_reduction_percent: u8,
) -> Result<u128, PveDeathError> {
    if level == 0 {
        return Err(PveDeathError::InvalidLevel);
    }
    if regular_blessings > MAX_REGULAR_BLESSINGS {
        return Err(PveDeathError::TooManyBlessings);
    }
    if !matches!(bless_reduction_percent, 0 | 6 | 9 | 12) {
        return Err(PveDeathError::InvalidBlessReduction);
    }
    let level = u128::from(level);
    // L² − 5L + 8 = L(L − 5) + 8 is positive for every L ≥ 1.
    let span = (level * level + 8)
        .checked_sub(5 * level)
        .ok_or(PveDeathError::Overflow)?;
    let percent_kept =
        100 - 8 * u128::from(regular_blessings) - if promotion_benefit_current { 30 } else { 0 };
    (level + 50)
        .checked_mul(50)
        .and_then(|value| value.checked_mul(span))
        .and_then(|value| value.checked_mul(percent_kept))
        .and_then(|value| value.checked_mul(100 - u128::from(bless_reduction_percent)))
        .map(|value| value / 1_000_000)
        .ok_or(PveDeathError::Overflow)
}

pub fn calculate_pve_death<I: Clone>(
    input: &PveDeathInput<'_, I>,
    decision_root: &GameplayDecisionRoot,
    death_occurrence: DecisionOccurrenceId,
) -> Result<PveDeathOutcome<I>, PveDeathError> {
    calculate_pve_death_with_bless(input, 0, decision_root, death_occurrence)
}

/// Uses the same PvE death outcome, with a current Bless reduction applied to experience only.
/// The owning transaction must resolve the reduction from the assignment to the lethal creature.
pub fn calculate_pve_death_with_bless<I: Clone>(
    input: &PveDeathInput<'_, I>,
    bless_reduction_percent: u8,
    decision_root: &GameplayDecisionRoot,
    death_occurrence: DecisionOccurrenceId,
) -> Result<PveDeathOutcome<I>, PveDeathError> {
    let experience = input.total_experience.get();
    if experience < 0 {
        return Err(PveDeathError::NegativeExperience);
    }
    let raw_loss = death_experience_loss_with_bless(
        input.level,
        input.regular_blessings,
        input.promotion_benefit_current,
        bless_reduction_percent,
    )?;
    let experience_lost = i64::try_from(raw_loss).unwrap_or(i64::MAX).min(experience);

    let mut equipment: Vec<EquippedItem<I>> = input.equipment.to_vec();
    equipment.sort_by_key(|entry| entry.slot);
    if equipment
        .windows(2)
        .any(|pair| pair[0].slot == pair[1].slot)
    {
        return Err(PveDeathError::DuplicateSlot);
    }
    let necklace = equipment
        .iter()
        .find(|entry| entry.slot == EquipmentSlot::Necklace);
    if input.necklace_is_amulet_of_loss && necklace.is_none() {
        return Err(PveDeathError::AmuletOfLossNotWorn);
    }

    let amulet_of_loss_consumed = if input.necklace_is_amulet_of_loss
        && input.regular_blessings < AMULET_OF_LOSS_BLESSINGS_EXCLUSIVE
    {
        necklace.map(|entry| entry.item.clone())
    } else {
        None
    };
    let exempt = input.level <= ITEM_LOSS_EXEMPT_MAX_LEVEL || !input.has_vocation;
    let lost_items = if exempt || amulet_of_loss_consumed.is_some() {
        Vec::new()
    } else {
        let ladder_step = usize::from(input.regular_blessings.min(5));
        let mut lost = Vec::new();
        for entry in equipment {
            let chance = entry.slot.loss_per_mille(ladder_step);
            if chance == 0 {
                continue;
            }
            let draw = deterministic_decision_u64(
                decision_root,
                death_occurrence,
                ITEM_LOSS_DRAW_PURPOSE,
                entry.slot.draw_index(),
            )?;
            if per_mille_roll(draw) < chance {
                lost.push(entry);
            }
        }
        lost
    };

    let grants_empty_bag = lost_items
        .iter()
        .any(|lost| lost.slot == EquipmentSlot::Container);
    Ok(PveDeathOutcome {
        experience_lost: ExactI64::new(experience_lost),
        experience_after: ExactI64::new(experience - experience_lost),
        blessings_consumed: input.regular_blessings,
        amulet_of_loss_consumed,
        lost_items,
        grants_empty_bag,
    })
}

/// Maps a uniform `u64` onto `0..1000` by multiply-shift, without modulo bias.
pub fn per_mille_roll(draw: u64) -> u16 {
    // The product is below 1000 · 2^64, so the shifted value is below 1000.
    u16::try_from((u128::from(draw) * PER_MILLE) >> 64).unwrap_or(u16::MAX)
}
