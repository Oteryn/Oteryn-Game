use super::*;

fn ordinary_death() -> EchoDeathFacts {
    EchoDeathFacts {
        is_warden: false,
        is_raid_spawned: false,
        is_summon: false,
        is_reward_boss: false,
        has_boss_race: false,
        forge_stacks: 0,
        bestiary_stars: 1,
    }
}

#[test]
fn portal_chance_has_exact_inclusive_donor_boundaries() -> Result<(), EchoWardenError> {
    let facts = ordinary_death();
    assert_eq!(death_effect(facts, 1)?, EchoDeathEffect::SchedulePortal);
    assert_eq!(death_effect(facts, 100)?, EchoDeathEffect::SchedulePortal);
    assert_eq!(death_effect(facts, 101)?, EchoDeathEffect::None);
    assert_eq!(death_effect(facts, 200_000)?, EchoDeathEffect::None);
    assert_eq!(death_effect(facts, 0), Err(EchoWardenError::InvalidDraw));
    assert_eq!(
        death_effect(facts, 200_001),
        Err(EchoWardenError::InvalidDraw)
    );
    let successes = (1..=SPAWN_CHANCE_DEN)
        .filter(|draw| death_effect(facts, *draw) == Ok(EchoDeathEffect::SchedulePortal))
        .count();
    assert_eq!(successes, 100);
    Ok(())
}

#[test]
fn portal_refuses_every_donor_guard_before_using_the_draw() -> Result<(), EchoWardenError> {
    let base = ordinary_death();
    for facts in [
        EchoDeathFacts {
            is_raid_spawned: true,
            ..base
        },
        EchoDeathFacts {
            is_summon: true,
            ..base
        },
        EchoDeathFacts {
            is_reward_boss: true,
            ..base
        },
        EchoDeathFacts {
            has_boss_race: true,
            ..base
        },
        EchoDeathFacts {
            forge_stacks: 1,
            ..base
        },
        EchoDeathFacts {
            bestiary_stars: 0,
            ..base
        },
        EchoDeathFacts {
            bestiary_stars: 3,
            ..base
        },
    ] {
        assert_eq!(death_effect(facts, 1)?, EchoDeathEffect::None);
        // The donor does not request any random value for an excluded creature.
        assert_eq!(death_effect(facts, 0)?, EchoDeathEffect::None);
    }
    assert_eq!(
        death_effect(
            EchoDeathFacts {
                bestiary_stars: 2,
                ..base
            },
            100,
        )?,
        EchoDeathEffect::SchedulePortal
    );
    Ok(())
}

#[test]
fn tagged_warden_rewards_participants_before_any_portal_exclusion() -> Result<(), EchoWardenError> {
    let facts = EchoDeathFacts {
        is_warden: true,
        is_raid_spawned: true,
        is_summon: true,
        is_reward_boss: true,
        has_boss_race: true,
        forge_stacks: 1,
        bestiary_stars: 5,
    };
    assert_eq!(death_effect(facts, 0)?, EchoDeathEffect::FirstSpeciesReward);
    Ok(())
}

#[test]
fn portal_ttl_starts_when_it_appears_not_when_the_creature_dies() {
    let expected = EchoPortalTiming {
        appears_at_ms: 30_011,
        expires_at_ms: 150_011,
    };
    assert_eq!(portal_timing(11), Some(expected));
    assert!(!expected.is_visible_at(30_010));
    assert!(expected.is_visible_at(30_011));
    assert!(expected.is_visible_at(150_010));
    assert!(!expected.is_visible_at(150_011));
    assert_eq!(portal_timing(u64::MAX - 149_999), None);
    assert_eq!(portal_timing(u64::MAX), None);
}

#[test]
fn raid_always_starts_one_warden_and_seven_to_twelve_minions() -> Result<(), EchoWardenError> {
    for minions in 7..=12 {
        let raid = EchoRaidPlan::new(minions)?;
        assert_eq!(raid.minion_count(), minions);
        assert_eq!(raid.spawn_count(), minions + 1);
        assert_eq!(raid.variant_at(0), Some(EchoVariant::Warden));
        assert_eq!(raid.spawn_delay_ms(0), Some(0));
        for index in 1..=minions {
            assert_eq!(raid.variant_at(index), Some(EchoVariant::Minion));
            assert_eq!(raid.spawn_delay_ms(index), Some(u64::from(index - 1) * 400));
        }
        assert_eq!(raid.variant_at(minions + 1), None);
        assert_eq!(raid.spawn_delay_ms(minions + 1), None);
    }
    for invalid in [0, 6, 13, u8::MAX] {
        assert_eq!(
            EchoRaidPlan::new(invalid),
            Err(EchoWardenError::InvalidDraw)
        );
    }
    Ok(())
}

#[test]
fn cpp_warden_application_refuses_each_existing_variant() {
    let fresh = EchoWardenFacts {
        already_warden: false,
        is_summon: false,
        is_boss: false,
        forge_stacks: 0,
        is_soulpit: false,
    };
    assert!(can_apply_warden(fresh));
    for facts in [
        EchoWardenFacts {
            already_warden: true,
            ..fresh
        },
        EchoWardenFacts {
            is_summon: true,
            ..fresh
        },
        EchoWardenFacts {
            is_boss: true,
            ..fresh
        },
        EchoWardenFacts {
            forge_stacks: 1,
            ..fresh
        },
        EchoWardenFacts {
            is_soulpit: true,
            ..fresh
        },
    ] {
        assert!(!can_apply_warden(facts));
    }
}

#[test]
fn warden_and_minion_stats_preserve_donor_scaling_and_rounding() {
    assert_eq!(EchoVariant::Warden.health(101), Some(404));
    assert_eq!(EchoVariant::Minion.health(101), Some(151));
    assert_eq!(EchoVariant::Minion.health(102), Some(153));
    assert_eq!(EchoVariant::Warden.attack_percent(), 150);
    // The Lua aura raises health; it does not apply the Warden attack multiplier to minions.
    assert_eq!(EchoVariant::Minion.attack_percent(), 100);
    assert_eq!(EchoVariant::Warden.health(u64::MAX), None);
    assert_eq!(EchoVariant::Minion.health(u64::MAX), None);
    assert_eq!(
        EchoVariant::Minion.health(u64::MAX / 2),
        Some(13_835_058_055_282_163_710)
    );
}

#[test]
fn aura_uses_a_same_floor_square_and_never_enlists_another_warden() {
    let candidate = EchoAuraFacts {
        same_species: true,
        is_warden: false,
        same_floor: true,
        delta_x: 5,
        delta_y: 5,
    };
    assert!(aura_can_enlist(candidate));
    for facts in [
        EchoAuraFacts {
            same_species: false,
            ..candidate
        },
        EchoAuraFacts {
            is_warden: true,
            ..candidate
        },
        EchoAuraFacts {
            same_floor: false,
            ..candidate
        },
        EchoAuraFacts {
            delta_x: 6,
            ..candidate
        },
        EchoAuraFacts {
            delta_y: 6,
            ..candidate
        },
    ] {
        assert!(!aura_can_enlist(facts));
    }
}

#[test]
fn first_species_reward_is_major_charm_points_by_difficulty() {
    for (stars, expected) in [(0, 1), (1, 2), (2, 5), (3, 10), (4, 15), (5, 30)] {
        assert_eq!(first_species_charm_points(stars), expected);
    }
    assert_eq!(first_species_charm_points(6), 0);
    assert_eq!(first_species_charm_points(u8::MAX), 0);
}

#[test]
fn guaranteed_scroll_draw_accepts_exactly_the_twenty_four_basic_scrolls() {
    for item in 53_751..=53_774 {
        assert_eq!(basic_scroll_item_id(item), Some(item));
    }
    assert_eq!(basic_scroll_item_id(53_750), None);
    assert_eq!(basic_scroll_item_id(53_775), None);
    assert_eq!(basic_scroll_item_id(0), None);
}
