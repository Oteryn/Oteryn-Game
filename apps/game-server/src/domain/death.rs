//! Pure, persistence-neutral PvE player death outcome
//! (`REFERENCE-FIRST-PLAYER-DEATH-V1` §4.3, owner decisions D58, D59, D62, D65, D66, D68).
//!
//! The caller supplies only PvE deaths (D60). The Character transaction that commits this outcome
//! and the DUR-03 item moves that follow it are separate children (DEATH-1b, DEATH-3).

use oteryn_simulation_determinism::{
    DecisionError, DecisionOccurrenceId, ExactI64, GameplayDecisionRoot, deterministic_decision_u64,
};

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
const CONTAINER_LOSS_PER_MILLE: [u16; 6] = [1_000, 700, 450, 250, 100, 0];
/// D62, indexed by `min(blessings, 5)`: chance in per mille that another equipped item is lost.
const EQUIPMENT_LOSS_PER_MILLE: [u16; 6] = [100, 70, 45, 25, 10, 0];

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PveDeathError {
    InvalidLevel,
    NegativeExperience,
    TooManyBlessings,
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
    if level == 0 {
        return Err(PveDeathError::InvalidLevel);
    }
    if regular_blessings > MAX_REGULAR_BLESSINGS {
        return Err(PveDeathError::TooManyBlessings);
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
        .map(|value| value / 10_000)
        .ok_or(PveDeathError::Overflow)
}

pub fn calculate_pve_death<I: Clone>(
    input: &PveDeathInput<'_, I>,
    decision_root: &GameplayDecisionRoot,
    death_occurrence: DecisionOccurrenceId,
) -> Result<PveDeathOutcome<I>, PveDeathError> {
    let experience = input.total_experience.get();
    if experience < 0 {
        return Err(PveDeathError::NegativeExperience);
    }
    let raw_loss = death_experience_loss(
        input.level,
        input.regular_blessings,
        input.promotion_benefit_current,
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

    Ok(PveDeathOutcome {
        experience_lost: ExactI64::new(experience_lost),
        experience_after: ExactI64::new(experience - experience_lost),
        blessings_consumed: input.regular_blessings,
        amulet_of_loss_consumed,
        lost_items,
    })
}

/// Maps a uniform `u64` onto `0..1000` by multiply-shift, without modulo bias.
fn per_mille_roll(draw: u64) -> u16 {
    // The product is below 1000 · 2^64, so the shifted value is below 1000.
    u16::try_from((u128::from(draw) * PER_MILLE) >> 64).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    const ROOT: GameplayDecisionRoot = GameplayDecisionRoot::from_bytes([7; 32]);

    fn occurrence(seed: u16) -> DecisionOccurrenceId {
        let mut bytes = [0_u8; 16];
        bytes[..2].copy_from_slice(&seed.to_be_bytes());
        DecisionOccurrenceId::from_bytes(bytes)
    }

    fn full_equipment() -> Vec<EquippedItem<u8>> {
        [
            EquipmentSlot::Head,
            EquipmentSlot::Necklace,
            EquipmentSlot::Container,
            EquipmentSlot::Armor,
            EquipmentSlot::RightHand,
            EquipmentSlot::LeftHand,
            EquipmentSlot::Legs,
            EquipmentSlot::Feet,
            EquipmentSlot::Ring,
            EquipmentSlot::Ammo,
        ]
        .into_iter()
        .map(|slot| EquippedItem {
            slot,
            item: slot as u8,
        })
        .collect()
    }

    fn input(equipment: &[EquippedItem<u8>], blessings: u8) -> PveDeathInput<'_, u8> {
        PveDeathInput {
            level: 100,
            total_experience: ExactI64::new(15_694_800),
            regular_blessings: blessings,
            promotion_benefit_current: false,
            has_vocation: true,
            equipment,
            necklace_is_amulet_of_loss: false,
        }
    }

    /// `XPThreshold(L)` = 50 (L³ − 6L² + 17L − 12) / 3, the Global level table.
    fn threshold(level: u128) -> u128 {
        50 * (level * level * level + 17 * level - 6 * level * level - 12) / 3
    }

    #[test]
    fn experience_loss_matches_the_d58_formula_at_the_revalidation_levels() {
        let expected: [(u32, [u128; 5], u128); 9] = [
            (1, [102, 93, 85, 77, 69], 14),
            (23, [15_403, 14_170, 12_938, 11_706, 10_474], 2_156),
            (24, [17_168, 15_794, 14_421, 13_047, 11_674], 2_403),
            (50, [112_900, 103_868, 94_836, 85_804, 76_772], 15_806),
            (100, [713_100, 656_052, 599_004, 541_956, 484_908], 99_834),
            (
                150,
                [2_175_800, 2_001_736, 1_827_672, 1_653_608, 1_479_544],
                304_612,
            ),
            (
                180,
                [3_623_420, 3_333_546, 3_043_672, 2_753_799, 2_463_925],
                507_278,
            ),
            (
                200,
                [4_876_000, 4_485_920, 4_095_840, 3_705_760, 3_315_680],
                682_640,
            ),
            (
                500,
                [68_064_700, 62_619_524, 57_174_348, 51_729_172, 46_283_996],
                9_529_058,
            ),
        ];
        for (level, by_blessings, all_reductions) in expected {
            for (blessings, loss) in (0_u8..).zip(by_blessings) {
                assert_eq!(death_experience_loss(level, blessings, false), Ok(loss));
            }
            assert_eq!(
                death_experience_loss(level, MAX_REGULAR_BLESSINGS, true),
                Ok(all_reductions)
            );
        }
    }

    #[test]
    fn experience_loss_equals_the_level_factor_times_the_previous_level_span() {
        for level in 2_u32..=1_000 {
            let wide = u128::from(level);
            let span = threshold(wide) - threshold(wide - 1);
            assert_eq!(
                death_experience_loss(level, 0, false),
                Ok((wide + 50) * span / 100)
            );
        }
    }

    #[test]
    fn experience_loss_rounds_down_and_rejects_invalid_inputs() {
        // 180: 3,623,420 × 0.14 = 507,278.8 → 507,278 (D68).
        assert_eq!(death_experience_loss(180, 7, true), Ok(507_278));
        assert_eq!(death_experience_loss(100, 0, true), Ok(499_170));
        assert_eq!(
            death_experience_loss(0, 0, false),
            Err(PveDeathError::InvalidLevel)
        );
        assert_eq!(
            death_experience_loss(100, 8, false),
            Err(PveDeathError::TooManyBlessings)
        );
        assert!(death_experience_loss(u32::MAX, 0, false).is_ok());
    }

    #[test]
    fn experience_never_drops_below_zero_and_blessings_are_consumed() {
        let mut death = input(&[], 3);
        death.level = 1;
        death.total_experience = ExactI64::new(40);
        let outcome = calculate_pve_death(&death, &ROOT, occurrence(0)).expect("a valid PvE death");
        assert_eq!(outcome.experience_lost, ExactI64::new(40));
        assert_eq!(outcome.experience_after, ExactI64::new(0));
        assert_eq!(outcome.blessings_consumed, 3);

        death.level = 100;
        death.total_experience = ExactI64::new(15_694_800);
        let outcome = calculate_pve_death(&death, &ROOT, occurrence(0)).expect("a valid PvE death");
        assert_eq!(outcome.experience_lost, ExactI64::new(541_956));
        assert_eq!(outcome.experience_after, ExactI64::new(15_152_844));

        death.total_experience = ExactI64::new(-1);
        assert_eq!(
            calculate_pve_death(&death, &ROOT, occurrence(0)),
            Err(PveDeathError::NegativeExperience)
        );
    }

    #[test]
    fn item_loss_follows_the_d62_ladder_per_blessing_count() {
        let equipment = full_equipment();
        let deaths = 4_000_u16;
        for blessings in 0_u8..=MAX_REGULAR_BLESSINGS {
            let step = usize::from(blessings.min(5));
            let mut container_lost = 0_u32;
            let mut others_lost = 0_u32;
            for seed in 0..deaths {
                let outcome =
                    calculate_pve_death(&input(&equipment, blessings), &ROOT, occurrence(seed))
                        .expect("a valid PvE death");
                for lost in outcome.lost_items {
                    if lost.slot == EquipmentSlot::Container {
                        container_lost += 1;
                    } else {
                        others_lost += 1;
                    }
                }
            }
            let container_expected = u32::from(CONTAINER_LOSS_PER_MILLE[step]) * 4;
            let others_expected = u32::from(EQUIPMENT_LOSS_PER_MILLE[step]) * 4 * 9;
            match step {
                0 => assert_eq!(container_lost, 4_000),
                5 => {
                    assert_eq!(container_lost, 0);
                    assert_eq!(others_lost, 0);
                }
                _ => assert!(container_lost.abs_diff(container_expected) <= 120),
            }
            assert!(
                others_lost.abs_diff(others_expected) <= 250,
                "blessings {blessings}: {others_lost} lost, about {others_expected} expected"
            );
        }
    }

    #[test]
    fn low_level_and_vocationless_characters_lose_no_items() {
        let equipment = full_equipment();
        let mut death = input(&equipment, 0);
        death.level = ITEM_LOSS_EXEMPT_MAX_LEVEL;
        let outcome = calculate_pve_death(&death, &ROOT, occurrence(1)).expect("a valid PvE death");
        assert!(outcome.lost_items.is_empty());
        assert!(outcome.experience_lost.get() > 0);

        death.level = ITEM_LOSS_EXEMPT_MAX_LEVEL + 1;
        let outcome = calculate_pve_death(&death, &ROOT, occurrence(1)).expect("a valid PvE death");
        assert!(
            outcome
                .lost_items
                .iter()
                .any(|lost| lost.slot == EquipmentSlot::Container)
        );

        death.level = 100;
        death.has_vocation = false;
        let outcome = calculate_pve_death(&death, &ROOT, occurrence(1)).expect("a valid PvE death");
        assert!(outcome.lost_items.is_empty());
    }

    #[test]
    fn amulet_of_loss_is_selected_only_below_five_blessings() {
        let equipment = full_equipment();
        for blessings in 0_u8..=MAX_REGULAR_BLESSINGS {
            let mut death = input(&equipment, blessings);
            death.necklace_is_amulet_of_loss = true;
            let outcome =
                calculate_pve_death(&death, &ROOT, occurrence(2)).expect("a valid PvE death");
            assert!(outcome.lost_items.is_empty());
            let expected = (blessings < AMULET_OF_LOSS_BLESSINGS_EXCLUSIVE)
                .then_some(EquipmentSlot::Necklace as u8);
            assert_eq!(outcome.amulet_of_loss_consumed, expected);
        }

        let without_necklace: Vec<_> = full_equipment()
            .into_iter()
            .filter(|entry| entry.slot != EquipmentSlot::Necklace)
            .collect();
        let mut death = input(&without_necklace, 0);
        death.necklace_is_amulet_of_loss = true;
        assert_eq!(
            calculate_pve_death(&death, &ROOT, occurrence(2)),
            Err(PveDeathError::AmuletOfLossNotWorn)
        );
    }

    #[test]
    fn the_lost_set_is_deterministic_canonical_and_order_independent() {
        let equipment = full_equipment();
        let mut shuffled = equipment.clone();
        shuffled.reverse();
        shuffled.swap(0, 4);
        for seed in 0..64 {
            let first = calculate_pve_death(&input(&equipment, 1), &ROOT, occurrence(seed))
                .expect("a valid PvE death");
            let again = calculate_pve_death(&input(&equipment, 1), &ROOT, occurrence(seed))
                .expect("a valid PvE death");
            let reordered = calculate_pve_death(&input(&shuffled, 1), &ROOT, occurrence(seed))
                .expect("a valid PvE death");
            assert_eq!(first, again);
            assert_eq!(first, reordered);
            assert!(
                first
                    .lost_items
                    .windows(2)
                    .all(|pair| pair[0].slot < pair[1].slot)
            );
        }
        let differs = (0..64).any(|seed| {
            calculate_pve_death(&input(&equipment, 1), &ROOT, occurrence(seed))
                .expect("a valid PvE death")
                .lost_items
                != calculate_pve_death(&input(&equipment, 1), &ROOT, occurrence(seed + 64))
                    .expect("a valid PvE death")
                    .lost_items
        });
        assert!(differs);
    }

    #[test]
    fn duplicate_slots_are_rejected() {
        let equipment = [
            EquippedItem {
                slot: EquipmentSlot::Ring,
                item: 1_u8,
            },
            EquippedItem {
                slot: EquipmentSlot::Ring,
                item: 2,
            },
        ];
        assert_eq!(
            calculate_pve_death(&input(&equipment, 0), &ROOT, occurrence(3)),
            Err(PveDeathError::DuplicateSlot)
        );
    }

    #[test]
    fn per_mille_roll_covers_the_full_range() {
        assert_eq!(per_mille_roll(0), 0);
        assert_eq!(per_mille_roll(u64::MAX), 999);
    }
}
