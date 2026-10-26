// Tests lifted from `crate::domain::death` (crate oteryn-gameplay-domain): they name higher-layer
// game-server modules, so they run here against the re-exported path.
#![allow(clippy::expect_used)]

#[allow(unused_imports)]
mod parent_imports {
    pub use crate::domain::character_build::{
        BuildFormula, DeathBuildState, MAX_BUILD_LEVEL, MAX_BUILD_PROGRESS, MIN_SKILL_LEVEL,
        cumulative_progress, relevel,
    };
    pub use oteryn_simulation_determinism::{
        DecisionError, DecisionOccurrenceId, ExactI64, GameplayDecisionRoot,
        deterministic_decision_u64,
    };
}
use crate::domain::death::*;
use crate::durability::character_build::{DurableBuildState, MAX_BUILD_PROGRESS};
#[allow(unused_imports)]
use parent_imports::*;

// Deliberately small requirement oracles; these are arithmetic fixtures, not
// a content formula or a new source of vocation rates.
struct LossFormula(fn(usize, u16) -> Option<u64>);

impl BuildFormula for LossFormula {
    fn required(&self, _vocation: &str, family: usize, level: u16) -> Option<u64> {
        (self.0)(family, level)
    }

    fn digest(&self) -> [u8; 32] {
        [0; 32]
    }

    fn content_revision(&self) -> &str {
        "arithmetic-fixture"
    }
}

fn loss_build(magic: (u16, u64), skills: [(u16, u64); 7]) -> DurableBuildState {
    DurableBuildState::new("knight", magic, skills).expect("bounded arithmetic fixture")
}

fn build_loss(
    formula: &dyn BuildFormula,
    before: &DurableBuildState,
    experience: i64,
    lost: i64,
) -> Result<DurableBuildState, DeathBuildLossError> {
    calculate_death_build_loss(
        formula,
        before,
        ExactI64::new(experience),
        ExactI64::new(lost),
    )
}

#[test]
fn build_loss_preserves_each_family_with_its_own_requirements() {
    let formula = LossFormula(|family, _| Some(100 * (family as u64 + 1)));
    let before = loss_build((2, 50), [(12, 50); 7]);
    let after = build_loss(&formula, &before, 3, 1).expect("valid capped loss");
    assert_eq!(after.vocation(), "knight");
    assert_eq!(after, before);
}

#[test]
fn full_experience_loss_preserves_trained_build_and_seed() {
    let formula = LossFormula(|_, _| Some(100));
    let before = loss_build((2, 50), [(12, 50); 7]);
    let after = build_loss(&formula, &before, 40, 40).expect("full capped loss");
    assert_eq!(after, before);
    assert_eq!(after.vocation(), before.vocation());
    let seed = loss_build((0, 0), [(10, 0); 7]);
    assert_eq!(build_loss(&formula, &seed, 40, 40), Ok(seed));
}

#[test]
fn zero_experience_and_rounded_zero_build_loss_preserve_exact_values() {
    let formula = LossFormula(|_, _| Some(100));
    let before = loss_build((0, 2), [(10, 2); 7]);
    for (experience, lost) in [(0, 0), (100, 0), (3, 1)] {
        assert_eq!(
            build_loss(&formula, &before, experience, lost),
            Ok(before.clone())
        );
    }
}

#[test]
fn build_loss_uses_the_actual_capped_death_experience() {
    let mut death = input(&[], 0);
    death.total_experience = ExactI64::new(40);
    let death = calculate_pve_death(&death, &ROOT, occurrence(90)).expect("capped death");
    assert_eq!(death.experience_lost.get(), 40);
    let before = loss_build((0, 50), [(10, 50); 7]);
    let after = calculate_death_build_loss(
        &LossFormula(|_, _| Some(100)),
        &before,
        ExactI64::new(40),
        death.experience_lost,
    )
    .expect("capped XP loss preserves build");
    assert_eq!(after, before);
}

#[test]
fn build_loss_rejects_negative_or_uncapped_experience_loss() {
    let formula = LossFormula(|_, _| Some(100));
    let before = DurableBuildState::default();
    for (experience, lost) in [(-1, 0), (1, -1), (0, 1), (1, 2)] {
        assert_eq!(
            build_loss(&formula, &before, experience, lost),
            Err(DeathBuildLossError::InvalidExperienceLoss)
        );
    }
}

#[test]
fn build_loss_rejects_unreachable_or_unnormalized_builds() {
    let before = loss_build((2, 50), [(12, 50); 7]);
    for invalid_family in 0..8 {
        // Each mutation targets one family of otherwise reachable build state.
        struct MissingFamily(usize);
        impl BuildFormula for MissingFamily {
            fn required(&self, _: &str, family: usize, _: u16) -> Option<u64> {
                (family != self.0).then_some(100)
            }
            fn digest(&self) -> [u8; 32] {
                [0; 32]
            }
            fn content_revision(&self) -> &str {
                "arithmetic-fixture"
            }
        }
        assert_eq!(
            build_loss(&MissingFamily(invalid_family), &before, 10, 1),
            Err(DeathBuildLossError::InvalidBuild)
        );
    }
    for formula in [LossFormula(|_, _| Some(0)), LossFormula(|_, _| Some(50))] {
        assert_eq!(
            build_loss(&formula, &before, 10, 1),
            Err(DeathBuildLossError::InvalidBuild)
        );
    }
}

#[test]
fn build_loss_preserves_saturated_progress_even_when_experience_is_lost() {
    let formula = LossFormula(|_, _| Some(MAX_BUILD_PROGRESS - 1));
    let before = loss_build((2, 0), [(12, 0); 7]);
    assert_eq!(build_loss(&formula, &before, 100, 0), Ok(before.clone()));
    let after = build_loss(&formula, &before, 2, 1).expect("saturated cumulative sum");
    assert_eq!(after, before);
}

#[test]
fn build_loss_handles_signed_storage_maxima_without_overflow() {
    let formula = LossFormula(|_, _| Some(MAX_BUILD_PROGRESS));
    let before = loss_build(
        (0, MAX_BUILD_PROGRESS - 1),
        [(10, MAX_BUILD_PROGRESS - 1); 7],
    );
    let after = build_loss(&formula, &before, i64::MAX, i64::MAX - 1)
        .expect("exact product wider than u64");
    assert_eq!(after, before);
    let ceiling = LossFormula(|_, level| (level < 12).then_some(100));
    let before = loss_build((1, 50), [(11, MAX_BUILD_PROGRESS); 7]);
    let after = build_loss(&ceiling, &before, 1, 1).expect("unreachable next level is a ceiling");
    assert_eq!(after, before);
}

#[test]
fn trained_death_keeps_all_build_progress_while_d58_experience_drops() {
    let formula = LossFormula(|_, _| Some(100));
    let before = loss_build((3, 50), [(14, 50); 7]);
    for blessings in 0..=MAX_REGULAR_BLESSINGS {
        for promoted in [false, true] {
            let mut death = input(&[], blessings);
            death.level = 50;
            death.total_experience = ExactI64::new(1_000_000);
            death.promotion_benefit_current = promoted;
            let outcome = calculate_pve_death(&death, &ROOT, occurrence(91)).expect("death");
            let expected =
                112_900 * (100 - 8 * i64::from(blessings) - if promoted { 30 } else { 0 }) / 100;
            assert_eq!(outcome.experience_lost.get(), expected);
            assert_eq!(outcome.experience_after.get(), 1_000_000 - expected);
            assert_eq!(
                calculate_death_build_loss(
                    &formula,
                    &before,
                    death.total_experience,
                    outcome.experience_lost
                ),
                Ok(before.clone())
            );
        }
    }
}

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
fn bless_reduces_the_residual_loss_at_each_stage() {
    // Independent fixed oracles: the 7 blessings and promotion retain 14% first.
    for (reduction, at_level_100, at_level_180) in [
        (6, 93_843, 476_842),
        (9, 90_848, 461_623),
        (12, 87_853, 446_405),
    ] {
        assert_eq!(
            death_experience_loss_with_bless(100, 7, true, reduction),
            Ok(at_level_100)
        );
        assert_eq!(
            death_experience_loss_with_bless(180, 7, true, reduction),
            Ok(at_level_180)
        );
    }
    assert_eq!(
        death_experience_loss_with_bless(100, 0, false, 12),
        Ok(627_528)
    );
    for invalid in [1, 5, 10, 13, 100, 255] {
        assert_eq!(
            death_experience_loss_with_bless(100, 0, false, invalid),
            Err(PveDeathError::InvalidBlessReduction)
        );
    }
    assert!(death_experience_loss_with_bless(u32::MAX, 0, false, 12).is_ok());
}

#[test]
fn bless_pve_outcome_changes_only_experience() {
    let equipment = full_equipment();
    let death = input(&equipment, 0);
    let plain = calculate_pve_death(&death, &ROOT, occurrence(17)).expect("a valid PvE death");
    let blessed = calculate_pve_death_with_bless(&death, 12, &ROOT, occurrence(17))
        .expect("a valid death with Bless");
    assert_eq!(blessed.experience_lost, ExactI64::new(627_528));
    assert_eq!(blessed.experience_after, ExactI64::new(15_067_272));
    assert_eq!(blessed.blessings_consumed, plain.blessings_consumed);
    assert_eq!(
        blessed.amulet_of_loss_consumed,
        plain.amulet_of_loss_consumed
    );
    assert_eq!(blessed.lost_items, plain.lost_items);
    assert_eq!(blessed.grants_empty_bag, plain.grants_empty_bag);
}

#[test]
fn bless_pve_outcome_rounds_once_before_the_experience_cap() {
    let mut death = input(&[], 7);
    death.level = 180;
    death.promotion_benefit_current = true;
    let blessed = calculate_pve_death_with_bless(&death, 6, &ROOT, occurrence(18))
        .expect("a valid death with Bless");
    // Flooring the pre-Bless loss first would incorrectly produce 476,841.
    assert_eq!(blessed.experience_lost, ExactI64::new(476_842));

    death.level = 100;
    death.total_experience = ExactI64::new(80_000);
    let blessed = calculate_pve_death_with_bless(&death, 12, &ROOT, occurrence(19))
        .expect("a valid death with Bless");
    // Capping first and reducing that cap would incorrectly leave 9,600 XP.
    assert_eq!(blessed.experience_lost, ExactI64::new(80_000));
    assert_eq!(blessed.experience_after, ExactI64::new(0));
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
        let outcome = calculate_pve_death(&death, &ROOT, occurrence(2)).expect("a valid PvE death");
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
fn an_empty_bag_is_granted_only_when_the_container_is_lost() {
    let equipment = full_equipment();
    let mut granted = 0;
    for seed in 0..64 {
        let outcome = calculate_pve_death(&input(&equipment, 2), &ROOT, occurrence(seed))
            .expect("a valid PvE death");
        let container_lost = outcome
            .lost_items
            .iter()
            .any(|lost| lost.slot == EquipmentSlot::Container);
        assert_eq!(outcome.grants_empty_bag, container_lost);
        granted += u32::from(outcome.grants_empty_bag);
    }
    assert!(granted > 0 && granted < 64);

    // No container equipped, or no item loss at all: no bag.
    let without_container: Vec<_> = equipment
        .iter()
        .copied()
        .filter(|entry| entry.slot != EquipmentSlot::Container)
        .collect();
    let outcome = calculate_pve_death(&input(&without_container, 0), &ROOT, occurrence(9))
        .expect("a valid PvE death");
    assert!(!outcome.grants_empty_bag);
    let mut protected = input(&equipment, 0);
    protected.level = ITEM_LOSS_EXEMPT_MAX_LEVEL;
    let outcome = calculate_pve_death(&protected, &ROOT, occurrence(9)).expect("a valid PvE death");
    assert!(!outcome.grants_empty_bag);
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
