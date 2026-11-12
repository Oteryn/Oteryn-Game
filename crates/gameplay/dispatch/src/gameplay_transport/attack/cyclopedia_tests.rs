//! Projection tests compare published rates with the actual production combat consumer.
use super::*;
use crate::spell::cast::CharacterCastFacts;

fn state() -> Result<PlayerSpellState, &'static str> {
    PlayerSpellState::new(
        CharacterCastFacts {
            vocation: crate::spell::Vocation::Knight,
            level: 100,
            magic_level: 0,
            max_health: 1000,
            max_mana: 500,
            max_soul: 100,
        },
        0,
        0,
    )
    .ok_or("player spell state")
}

#[test]
fn p12c_attack_display_formula_matches_crystal_values() {
    assert_eq!(attack_totals(20, 7, 50, FightMode::Balanced), (33, 33));
    assert_eq!(attack_totals(20, 7, 50, FightMode::Offensive), (35, 33));
    assert_eq!(attack_totals(20, 7, 50, FightMode::Defensive), (27, 33));
    assert_eq!(attack_totals(20, 30, 50, FightMode::Offensive), (89, 77));
    assert_eq!(attack_totals(20, 30, 50, FightMode::Defensive), (54, 77));
    assert_eq!(attack_skill(7, 50), 6);
    assert_eq!(attack_skill(30, 50), 27);
    assert_eq!(attack_skill(7, 10), -3);
}

#[test]
fn p12c_offence_rates_preserve_combat_saturation_and_unknown_sources() -> Result<(), String> {
    let state = state()?;
    let equipment = EquippedOffence::from_sums(1_000, 2_000, 3_000, 4_000);
    let offence = EquippedOffence {
        perfect_shot: [(3, 25)].into(),
        cleave: 2_000,
        ..EquippedOffence::from_sums(65_000, 65_000, 5_000, 6_000)
    };
    let wheel = WheelRuntimeEffects {
        life_leech_hundredths_percent: 100,
        mana_leech_hundredths_percent: 200,
        mitigation_milli_percent: 1_250,
        ..WheelRuntimeEffects::default()
    };
    let observation = Observation {
        state: &state,
        now_micros: 1,
        mode: FightMode::Balanced,
        weapon: SwingWeapon::Fists,
        ranged: None,
        equipment_only: &equipment,
        offence: &offence,
        wheel: Some(&wheel),
        battle_instinct: BattleInstinctBonuses::default(),
        skill_bonuses: [0; 8],
        mantra: 0,
    };
    let mut value =
        CombatStats::new(CharacterTab::Offence, 1, 1, 1).map_err(|e| format!("{e:?}"))?;
    observation
        .offence(&mut value)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(value.get(Stat::CriticalChance), Some(6_550_000));
    assert_eq!(value.get(Stat::CriticalDamage), Some(6_553_500));
    assert_eq!(value.get(Stat::CriticalChanceBase), Some(50_000));
    assert_eq!(value.get(Stat::CriticalChanceEquipment), Some(100_000));
    assert_eq!(value.get(Stat::CriticalChanceImbuement), Some(6_400_000));
    assert_eq!(value.get(Stat::LifeLeech), Some(510_000));
    assert_eq!(value.get(Stat::ManaLeech), Some(620_000));
    assert_eq!(value.get(Stat::ManaLeechWheel), Some(20_000));
    assert_eq!(value.get(Stat::Cleave), Some(200_000));
    assert_eq!(value.get(Stat::PerfectShot3), Some(25));
    assert_eq!(value.get(Stat::PerfectShot1), Some(0));
    assert_eq!(value.get(Stat::Onslaught), Some(0));
    assert_eq!(value.get(Stat::CriticalChanceConcoction), None);
    assert_eq!(value.get(Stat::CriticalChanceWheel), None);
    assert_eq!(value.get(Stat::AttackTotal), None);
    assert_eq!(value.get(Stat::AttackSkill), None);
    assert_eq!(
        value.get(Stat::Accuracy1),
        None,
        "melee is not a ranged accuracy observation"
    );
    assert_eq!(value.get(Stat::AttackElement), Some(0));
    Ok(())
}

#[test]
fn p12c_onslaught_projection_matches_consumed_fatal_thresholds() {
    for weapon_tier in 0..=11 {
        for boots_tier in 0..=11 {
            let offence = EquippedOffence {
                weapon_tier,
                boots_tier,
                ..EquippedOffence::default()
            };
            let (_, total) = onslaught_ppm(weapon_tier, boots_tier);
            // The combat consumer compares a roll in hundredths of a percent. The tab keeps
            // the additional precision of Crystal's float percentage instead of rounding it.
            let first_non_fatal = u64::try_from(total).unwrap_or(0).div_ceil(100);
            assert_eq!(offence.fatal(100, first_non_fatal), 100);
            if first_non_fatal > 0 {
                assert_eq!(offence.fatal(100, first_non_fatal - 1), 160);
            }
        }
    }
    assert_eq!(onslaught_ppm(1, 1), (5_000, 5_125));
    assert_eq!(amplification_ppm(1), 25_000);
    assert_eq!(amplification_ppm(0), 0);
}

#[test]
fn p12c_melee_attack_total_follows_live_skill_and_fight_mode() -> Result<(), String> {
    use oteryn_gameplay_combat_rules::resolution::MeleeSkill;

    let mut state = state()?;
    let build =
        crate::durability::character_build::DurableBuildState::new("knight", (0, 0), [(50, 0); 7])
            .map_err(|error| format!("{error:?}"))?;
    state
        .load_owned_build(&build)
        .map_err(|error| format!("{error:?}"))?;
    let offence = EquippedOffence::default();
    let observation = Observation {
        state: &state,
        now_micros: 1,
        mode: FightMode::Offensive,
        weapon: SwingWeapon::Melee {
            attack: 30,
            skill: MeleeSkill::Sword,
        },
        ranged: None,
        equipment_only: &offence,
        offence: &offence,
        wheel: None,
        battle_instinct: BattleInstinctBonuses::default(),
        skill_bonuses: [0; 8],
        mantra: 0,
    };
    let mut value =
        CombatStats::new(CharacterTab::Offence, 1, 1, 1).map_err(|error| format!("{error:?}"))?;
    observation
        .offence(&mut value)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(value.get(Stat::AttackTotal), Some(89));
    assert_eq!(value.get(Stat::AttackFightMode), Some(12));
    assert_eq!(value.get(Stat::AttackSkill), Some(27));
    assert_eq!(value.get(Stat::AttackSkillId), Some(8));
    assert_eq!(value.get(Stat::AttackEquipment), Some(30));
    Ok(())
}

#[test]
fn p12c_fist_attack_total_reports_the_fight_mode_delta() -> Result<(), String> {
    let mut state = state()?;
    let build =
        crate::durability::character_build::DurableBuildState::new("knight", (0, 0), [(50, 0); 7])
            .map_err(|error| format!("{error:?}"))?;
    state
        .load_owned_build(&build)
        .map_err(|error| format!("{error:?}"))?;
    let offence = EquippedOffence::default();
    for (mode, total, fight_mode) in [
        (FightMode::Balanced, 33, 0),
        (FightMode::Offensive, 35, 2),
        (FightMode::Defensive, 27, -6),
    ] {
        let observation = Observation {
            state: &state,
            now_micros: 1,
            mode,
            weapon: SwingWeapon::Fists,
            ranged: None,
            equipment_only: &offence,
            offence: &offence,
            wheel: None,
            battle_instinct: BattleInstinctBonuses::default(),
            skill_bonuses: [0; 8],
            mantra: 0,
        };
        let mut value = CombatStats::new(CharacterTab::Offence, 1, 1, 1)
            .map_err(|error| format!("{error:?}"))?;
        observation
            .offence(&mut value)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(value.get(Stat::AttackTotal), Some(total));
        assert_eq!(value.get(Stat::AttackFightMode), Some(fight_mode));
        assert_eq!(value.get(Stat::AttackSkill), Some(6));
        assert_eq!(value.get(Stat::AttackSkillId), Some(11));
    }
    Ok(())
}

#[test]
fn p12c_defence_observes_loaded_live_skills_and_owned_equipment() -> Result<(), String> {
    use oteryn_gameplay_combat_rules::attack::formulas::DefenceSource;
    use oteryn_gameplay_combat_rules::resolution::player_defence::HandDefence;
    let mut state = state()?;
    let build =
        crate::durability::character_build::DurableBuildState::new("knight", (0, 0), [(50, 0); 7])
            .map_err(|e| format!("{e:?}"))?;
    state
        .load_owned_build(&build)
        .map_err(|e| format!("{e:?}"))?;
    let hand = HandDefence {
        source: DefenceSource::Shield,
        value: 40,
        skill_index: Some(5),
    };
    state.install_equipped_armor(32);
    state.install_equipped_defence(Some(hand));
    let offence = EquippedOffence {
        reflect_flat: 7,
        ..EquippedOffence::default()
    };
    let wheel = WheelRuntimeEffects {
        mitigation_milli_percent: 1_250,
        sockets: oteryn_gameplay_progression::wheel_effects::WheelSocketSeam {
            gem_values_milli: [
                (("dodge".to_owned(), String::new()), 280),
                (("momentum_chance".to_owned(), String::new()), 330),
            ]
            .into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let observation = Observation {
        state: &state,
        now_micros: 1,
        mode: FightMode::Balanced,
        weapon: SwingWeapon::Fists,
        ranged: None,
        equipment_only: &offence,
        offence: &offence,
        wheel: Some(&wheel),
        battle_instinct: BattleInstinctBonuses::default(),
        skill_bonuses: [0, 0, 0, 0, 0, 3, 0, 0],
        mantra: 9,
    };
    let mut value =
        CombatStats::new(CharacterTab::Defence, 1, 1, 1).map_err(|e| format!("{e:?}"))?;
    observation
        .defence(&mut value)
        .map_err(|e| format!("{e:?}"))?;
    let constants = AttackConstants::checked_in().ok_or("constants")?;
    let (_, defence) =
        player_defence_bounds(constants, hand, 50, FightMode::Balanced, true).ok_or("bounds")?;
    assert_eq!(value.get(Stat::DefenceTotal), Some(i64::from(defence)));
    assert_eq!(
        value.get(Stat::ShieldingSkill),
        Some(50),
        "incoming defence does not consume attack-only proficiency bonuses"
    );
    assert_eq!(value.get(Stat::DefenceEquipment), Some(40));
    assert_eq!(value.get(Stat::Armor), Some(32));
    assert_eq!(value.get(Stat::Mantra), Some(9));
    assert_eq!(value.get(Stat::PhysicalReflection), Some(7));
    assert_eq!(value.get(Stat::Dodge), Some(2_800));
    assert_eq!(value.get(Stat::DodgeWheel), Some(2_800));
    assert_eq!(value.get(Stat::MitigationWheel), Some(12_500));
    for key in [
        Stat::Mitigation,
        Stat::MagicShieldCapacity,
        Stat::DefenceWheel,
    ] {
        assert_eq!(
            value.get(key),
            None,
            "unsupported field {key:?} is not zero"
        );
    }
    let mut misc =
        CombatStats::new(CharacterTab::Misc, 1, 1, 1).map_err(|error| format!("{error:?}"))?;
    observation
        .misc(&mut misc)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(misc.get(Stat::Momentum), Some(3_300));
    assert_eq!(misc.get(Stat::MomentumWheel), Some(3_300));
    let boosted = Observation {
        battle_instinct: BattleInstinctBonuses {
            melee: 3,
            shielding: 6,
        },
        ..observation
    };
    boosted.defence(&mut value).map_err(|e| format!("{e:?}"))?;
    let (_, boosted_defence) =
        player_defence_bounds(constants, hand, 56, FightMode::Balanced, true).ok_or("bounds")?;
    assert_eq!(value.get(Stat::ShieldingSkill), Some(56));
    assert_eq!(
        value.get(Stat::DefenceTotal),
        Some(i64::from(boosted_defence))
    );
    Ok(())
}
