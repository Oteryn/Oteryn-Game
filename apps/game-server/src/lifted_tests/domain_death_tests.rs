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
fn trained_death_keeps_all_build_progress_while_experience_drops() {
    let formula = LossFormula(|_, _| Some(100));
    let before = loss_build((3, 50), [(14, 50); 7]);
    for blessings in 0..=MAX_REGULAR_BLESSINGS {
        for promoted in [false, true] {
            let mut death = input(&[], blessings);
            death.level = 50;
            death.total_experience = ExactI64::new(1_000_000);
            death.promotion_benefit_current = promoted;
            let outcome = calculate_pve_death(&death, &ROOT, occurrence(91)).expect("death");
            // Canonical level 50 span: 1965000 − 1847300 = 117700.
            let expected =
                117_700 * (100 - 8 * i64::from(blessings) - if promoted { 30 } else { 0 }) / 100;
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
fn experience_loss_matches_the_canonical_table_at_the_revalidation_levels() {
    let expected: [(u32, [u128; 5], u128); 9] = [
        // Level 1: 100 − 0 = 100; × (100 − 8b)/100, or × 14/100.
        (1, [100, 92, 84, 76, 68], 14),
        // Level 23: 179_400 − 156_200 = 23_200; × (100 − 8b)/100, or × 14/100.
        (23, [23_200, 21_344, 19_488, 17_632, 15_776], 3_248),
        // Level 24: 204_800 − 179_400 = 25_400; × (100 − 8b)/100, or × 14/100.
        (24, [25_400, 23_368, 21_336, 19_304, 17_272], 3_556),
        // Level 50: 1_965_000 − 1_847_300 = 117_700; × (100 − 8b)/100, or × 14/100.
        (50, [117_700, 108_284, 98_868, 89_452, 80_036], 16_478),
        // Level 100: 16_180_000 − 15_694_800 = 485_200; × (100 − 8b)/100, or × 14/100.
        (100, [485_200, 446_384, 407_568, 368_752, 329_936], 67_928),
        // Level 150: 55_145_000 − 54_042_300 = 1_102_700; × (100 − 8b)/100, or × 14/100.
        (
            150,
            [1_102_700, 1_014_484, 926_268, 838_052, 749_836],
            154_378,
        ),
        // Level 180: 95_604_000 − 94_010_800 = 1_593_200; × (100 − 8b)/100, or × 14/100.
        (
            180,
            [1_593_200, 1_465_744, 1_338_288, 1_210_832, 1_083_376],
            223_048,
        ),
        // Level 200: 131_360_000 − 129_389_800 = 1_970_200; × (100 − 8b)/100, or × 14/100.
        (
            200,
            [1_970_200, 1_812_584, 1_654_968, 1_497_352, 1_339_736],
            275_828,
        ),
        // Level 500: 2_070_900_000 − 2_058_474_800 = 12_425_200; × (100 − 8b)/100, or × 14/100.
        (
            500,
            [12_425_200, 11_431_184, 10_437_168, 9_443_152, 8_449_136],
            1_739_528,
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
    // Canonical spans: level 100 = 485200, level 180 = 1593200.
    // Floor(span × 14/100 × (100 − Bless%)/100), with 7 blessings and promotion.
    for (reduction, at_level_100, at_level_180) in [
        (6, 63_852, 209_665),
        (9, 61_814, 202_973),
        (12, 59_776, 196_282),
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
        Ok(426_976)
    );
    for invalid in [1, 5, 10, 13, 100, 255] {
        assert_eq!(
            death_experience_loss_with_bless(100, 0, false, invalid),
            Err(PveDeathError::InvalidBlessReduction)
        );
    }
    assert_eq!(
        death_experience_loss_with_bless(u32::MAX, 0, false, 12),
        Err(PveDeathError::InvalidLevel)
    );
}

#[test]
fn bless_pve_outcome_changes_only_experience() {
    let equipment = full_equipment();
    let death = input(&equipment, 0);
    let plain = calculate_pve_death(&death, &ROOT, occurrence(17)).expect("a valid PvE death");
    let blessed = calculate_pve_death_with_bless(&death, 12, &ROOT, occurrence(17))
        .expect("a valid death with Bless");
    assert_eq!(blessed.experience_lost, ExactI64::new(426_976));
    assert_eq!(blessed.experience_after, ExactI64::new(15_267_824));
    assert_eq!(blessed.blessings_consumed, plain.blessings_consumed);
    assert_eq!(
        blessed.amulet_of_loss_consumed,
        plain.amulet_of_loss_consumed
    );
    assert_eq!(blessed.lost_items, plain.lost_items);
    assert_eq!(blessed.grants_empty_bag, plain.grants_empty_bag);
}

#[test]
fn bless_pve_outcome_applies_before_the_experience_cap() {
    let mut death = input(&[], 7);
    death.level = 180;
    death.promotion_benefit_current = true;
    let blessed = calculate_pve_death_with_bless(&death, 6, &ROOT, occurrence(18))
        .expect("a valid death with Bless");
    // Floor((1593200 × 14 × 94)/10000) = floor(209665.12) = 209665.
    assert_eq!(blessed.experience_lost, ExactI64::new(209_665));

    death.level = 100;
    death.total_experience = ExactI64::new(50_000);
    let blessed = calculate_pve_death_with_bless(&death, 12, &ROOT, occurrence(19))
        .expect("a valid death with Bless");
    // Floor(485200 × 14/100 × 88/100) = 59776, capped at 50000.
    // Capping first and reducing that cap would incorrectly leave 6000 XP.
    assert_eq!(blessed.experience_lost, ExactI64::new(50_000));
    assert_eq!(blessed.experience_after, ExactI64::new(0));
}

#[test]
fn experience_loss_equals_the_next_level_span() {
    for level in 2_u32..=1_000 {
        let wide = u128::from(level);
        let span = threshold(wide + 1) - threshold(wide);
        assert_eq!(death_experience_loss(level, 0, false), Ok(span));
    }
}

#[test]
fn experience_loss_rounds_down_and_rejects_invalid_inputs() {
    // Level 180: (95604000 − 94010800) × 14/100 = 223048.
    assert_eq!(death_experience_loss(180, 7, true), Ok(223_048));
    assert_eq!(death_experience_loss(100, 0, true), Ok(339_640));
    assert_eq!(
        death_experience_loss(0, 0, false),
        Err(PveDeathError::InvalidLevel)
    );
    assert_eq!(
        death_experience_loss(100, 8, false),
        Err(PveDeathError::TooManyBlessings)
    );
    assert_eq!(
        death_experience_loss(u32::MAX, 0, false),
        Err(PveDeathError::InvalidLevel)
    );
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
    assert_eq!(outcome.experience_lost, ExactI64::new(368_752));
    assert_eq!(outcome.experience_after, ExactI64::new(15_326_048));

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
