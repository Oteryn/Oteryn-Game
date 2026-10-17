use super::*;

const ROOT: GameplayDecisionRoot = GameplayDecisionRoot::from_bytes([7; 32]);

fn occurrence() -> DecisionOccurrenceId {
    DecisionOccurrenceId::from_bytes([1; 16])
}

fn equipment() -> Vec<EquippedItem<u8>> {
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

fn input(items: &[EquippedItem<u8>]) -> DeathInput<'_, u8> {
    DeathInput {
        ordinary: PveDeathInput {
            level: 100,
            total_experience: ExactI64::new(15_694_800),
            regular_blessings: 3,
            promotion_benefit_current: false,
            has_vocation: true,
            equipment: items,
            necklace_is_amulet_of_loss: false,
        },
        world: WorldPvpType::Open,
        damage: DeathDamage {
            total: 100_000,
            pvp: 5_000,
            final_blow: FinalBlow::Other,
        },
        skull_at_death: PvpSkull::None,
        twist_of_fate_held: false,
        adventurer_forfeited: true,
        bless_reduction_percent: 0,
    }
}

#[test]
fn four_point_999_percent_is_not_pvp_but_five_percent_is() -> Result<(), DeathError> {
    let damage = input(&[]).damage;
    assert!(!is_pvp_death(DeathDamage {
        pvp: 4_999,
        ..damage
    })?);
    assert!(is_pvp_death(damage)?);
    Ok(())
}

#[test]
fn character_and_summon_final_blow_are_independent_of_damage_ratio() -> Result<(), DeathError> {
    for final_blow in [FinalBlow::Character, FinalBlow::CharacterSummon] {
        assert!(is_pvp_death(DeathDamage {
            total: 100_000,
            pvp: 1,
            final_blow
        })?);
    }
    Ok(())
}

#[test]
fn invalid_totals_refuse_instead_of_defaulting_to_pve() {
    assert_eq!(
        is_pvp_death(DeathDamage {
            total: 0,
            pvp: 0,
            final_blow: FinalBlow::Other
        }),
        Err(DeathError::EmptyDamage)
    );
    assert_eq!(
        is_pvp_death(DeathDamage {
            total: 10,
            pvp: 11,
            final_blow: FinalBlow::Other
        }),
        Err(DeathError::PvpDamageExceedsTotal)
    );
}

#[test]
fn damage_comparison_remains_exact_at_u64_limits() -> Result<(), DeathError> {
    let damage = DeathDamage {
        total: u64::MAX,
        pvp: u64::MAX / 20,
        final_blow: FinalBlow::Other,
    };
    assert!(!is_pvp_death(damage)?);
    assert!(is_pvp_death(DeathDamage {
        pvp: damage.pvp + 1,
        ..damage
    })?);
    Ok(())
}

#[test]
fn ordinary_pvp_reuses_existing_xp_and_deterministic_item_selection() -> Result<(), DeathError> {
    let items = equipment();
    let request = input(&items);
    let expected = calculate_pve_death_with_bless(&request.ordinary, 0, &ROOT, occurrence())?;
    let actual = calculate_death(&request, &ROOT, occurrence())?;
    assert!(actual.receipt.pvp_death);
    assert_eq!(actual.experience_lost, expected.experience_lost);
    assert_eq!(actual.experience_after, expected.experience_after);
    assert_eq!(actual.lost_items, expected.lost_items);
    assert_eq!(
        actual.amulet_of_loss_consumed,
        expected.amulet_of_loss_consumed
    );
    assert_eq!(actual.grants_empty_bag, expected.grants_empty_bag);
    assert_eq!(actual.regular_blessings_consumed, 3);
    Ok(())
}

#[test]
fn twist_preserves_regular_blessings_and_requires_a_regular_blessing() -> Result<(), DeathError> {
    for count in [0, 1, 7] {
        let mut request = input(&[]);
        request.ordinary.regular_blessings = count;
        request.twist_of_fate_held = true;
        let result = calculate_death(&request, &ROOT, occurrence())?;
        assert_eq!(result.receipt.twist_of_fate_used, count > 0);
        assert_eq!(result.regular_blessings_consumed, 0);
    }
    Ok(())
}

#[test]
fn pve_does_not_use_twist_to_preserve_regular_blessings() -> Result<(), DeathError> {
    let mut request = input(&[]);
    request.damage.pvp = 4_999;
    request.twist_of_fate_held = true;
    let result = calculate_death(&request, &ROOT, occurrence())?;
    assert!(!result.receipt.pvp_death);
    assert!(!result.receipt.twist_of_fate_used);
    assert_eq!(result.regular_blessings_consumed, 3);
    Ok(())
}

#[test]
fn red_and_black_lose_every_equipped_item_and_ignore_amulet_and_twist() -> Result<(), DeathError> {
    let mut items = equipment();
    items.reverse();
    for skull in [PvpSkull::Red, PvpSkull::Black] {
        for pvp in [0, 5_000] {
            let mut request = input(&items);
            request.damage.pvp = pvp;
            request.skull_at_death = skull;
            request.ordinary.regular_blessings = 7;
            request.ordinary.necklace_is_amulet_of_loss = true;
            request.twist_of_fate_held = true;
            let result = calculate_death(&request, &ROOT, occurrence())?;
            assert_eq!(result.lost_items, equipment());
            assert_eq!(result.amulet_of_loss_consumed, None);
            assert_eq!(result.regular_blessings_consumed, 7);
            assert!(!result.receipt.twist_of_fate_used);
            assert!(result.grants_empty_bag);
        }
    }
    Ok(())
}

#[test]
fn item_exemptions_precede_skull_loss_and_amulet_consumption() -> Result<(), DeathError> {
    let items = equipment();
    for skull in [PvpSkull::None, PvpSkull::Red, PvpSkull::Black] {
        for (level, vocation) in [(8, true), (100, false)] {
            let mut request = input(&items);
            request.ordinary.level = level;
            request.ordinary.has_vocation = vocation;
            request.ordinary.necklace_is_amulet_of_loss = true;
            request.skull_at_death = skull;
            let result = calculate_death(&request, &ROOT, occurrence())?;
            assert!(result.lost_items.is_empty());
            assert_eq!(result.amulet_of_loss_consumed, None);
            assert!(!result.grants_empty_bag);
        }
    }
    Ok(())
}

#[test]
fn adventurer_precedes_xp_items_amulet_and_blessing_consumption() -> Result<(), DeathError> {
    let items = equipment();
    let mut request = input(&items);
    request.ordinary.level = 20;
    request.ordinary.necklace_is_amulet_of_loss = true;
    request.adventurer_forfeited = false;
    request.twist_of_fate_held = true;
    let result = calculate_death(&request, &ROOT, occurrence())?;
    assert!(result.receipt.adventurer_applied);
    assert_eq!(result.experience_lost, ExactI64::new(0));
    assert_eq!(result.experience_after, request.ordinary.total_experience);
    assert!(result.lost_items.is_empty());
    assert_eq!(result.amulet_of_loss_consumed, None);
    assert_eq!(result.regular_blessings_consumed, 0);
    assert!(!result.receipt.twist_of_fate_used);
    Ok(())
}

#[test]
fn adventurer_requires_open_world_level_at_most_20_and_not_forfeited() -> Result<(), DeathError> {
    for (world, level, forfeited) in [
        (WorldPvpType::Optional, 20, false),
        (WorldPvpType::Hardcore, 20, false),
        (WorldPvpType::Open, 21, false),
        (WorldPvpType::Open, 20, true),
    ] {
        let mut request = input(&[]);
        request.world = world;
        request.ordinary.level = level;
        request.adventurer_forfeited = forfeited;
        let result = calculate_death(&request, &ROOT, occurrence())?;
        assert!(!result.receipt.adventurer_applied);
        assert!(result.experience_lost.get() > 0);
    }
    Ok(())
}

#[test]
fn contradictory_adventurer_and_skull_facts_are_not_silently_resolved() {
    for skull in [PvpSkull::Red, PvpSkull::Black] {
        let mut request = input(&[]);
        request.ordinary.level = 20;
        request.adventurer_forfeited = false;
        request.skull_at_death = skull;
        assert_eq!(
            calculate_death(&request, &ROOT, occurrence()),
            Err(DeathError::ContradictoryAdventurerSkull)
        );
    }
}

#[test]
fn black_respawn_uses_original_death_fields_after_current_skull_changes() -> Result<(), DeathError>
{
    let mut request = input(&[]);
    request.skull_at_death = PvpSkull::Black;
    let original = calculate_death(&request, &ROOT, occurrence())?.receipt;
    request.skull_at_death = PvpSkull::None;
    assert_eq!(
        original.respawn_vitals(),
        RespawnVitals::BlackSkull { hp: 40, mana: 0 }
    );
    assert_eq!(
        calculate_death(&request, &ROOT, occurrence())?
            .receipt
            .respawn_vitals(),
        RespawnVitals::Full
    );
    assert_eq!(original.unfair_fight_milli, 0);
    Ok(())
}

#[test]
fn native_bless_reduction_is_retained_for_ordinary_and_skull_deaths() -> Result<(), DeathError> {
    for skull in [PvpSkull::None, PvpSkull::Red, PvpSkull::Black] {
        let mut request = input(&[]);
        request.skull_at_death = skull;
        let without = calculate_death(&request, &ROOT, occurrence())?;
        request.bless_reduction_percent = 12;
        let with = calculate_death(&request, &ROOT, occurrence())?;
        assert!(with.experience_lost.get() < without.experience_lost.get());
        assert_eq!(
            with.regular_blessings_consumed,
            without.regular_blessings_consumed
        );
    }
    Ok(())
}

#[test]
fn malformed_loss_inputs_fail_independently_before_any_outcome() {
    let mut request = input(&[]);
    request.ordinary.level = 0;
    assert_eq!(
        calculate_death(&request, &ROOT, occurrence()),
        Err(DeathError::Ordinary(PveDeathError::InvalidLevel))
    );
    request = input(&[]);
    request.bless_reduction_percent = 5;
    assert_eq!(
        calculate_death(&request, &ROOT, occurrence()),
        Err(DeathError::Ordinary(PveDeathError::InvalidBlessReduction))
    );
    request = input(&[]);
    request.ordinary.total_experience = ExactI64::new(-1);
    assert_eq!(
        calculate_death(&request, &ROOT, occurrence()),
        Err(DeathError::Ordinary(PveDeathError::NegativeExperience))
    );
    request = input(&[]);
    request.ordinary.regular_blessings = 8;
    assert_eq!(
        calculate_death(&request, &ROOT, occurrence()),
        Err(DeathError::Ordinary(PveDeathError::TooManyBlessings))
    );
    request = input(&[]);
    request.ordinary.necklace_is_amulet_of_loss = true;
    assert_eq!(
        calculate_death(&request, &ROOT, occurrence()),
        Err(DeathError::Ordinary(PveDeathError::AmuletOfLossNotWorn))
    );
    let duplicated = [
        EquippedItem {
            slot: EquipmentSlot::Head,
            item: 1,
        },
        EquippedItem {
            slot: EquipmentSlot::Head,
            item: 2,
        },
    ];
    assert_eq!(
        calculate_death(&input(&duplicated), &ROOT, occurrence()),
        Err(DeathError::Ordinary(PveDeathError::DuplicateSlot))
    );
}
