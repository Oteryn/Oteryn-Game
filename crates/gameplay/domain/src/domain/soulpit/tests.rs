use super::*;

fn participant(slot: u8, level: u32) -> EntryParticipant {
    EntryParticipant {
        slot,
        level,
        game_master: false,
    }
}

#[test]
fn entry_accepts_one_to_five_players_and_checks_every_level() {
    assert_eq!(validate_entry(0, &[participant(0, 8)], false, 1), Ok(()));
    let mut party: Vec<_> = (0..5).map(|slot| participant(slot, 8)).collect();
    assert_eq!(validate_entry(0, &party, false, 1), Ok(()));
    party[3].level = 7;
    assert_eq!(
        validate_entry(0, &party, false, 1),
        Err(SoulpitError::LevelTooLow)
    );
    party[3].game_master = true;
    assert_eq!(validate_entry(0, &party, false, 1), Ok(()));
}

#[test]
fn entry_refuses_busy_arena_missing_core_and_invalid_participants() {
    let party = [participant(0, 8)];
    assert_eq!(
        validate_entry(0, &party, true, 1),
        Err(SoulpitError::ArenaOccupied)
    );
    assert_eq!(
        validate_entry(0, &party, false, 0),
        Err(SoulpitError::NotEnoughItems)
    );
    assert_eq!(
        validate_entry(0, &[], false, 1),
        Err(SoulpitError::InvalidParticipants)
    );
    assert_eq!(
        validate_entry(5, &party, false, 1),
        Err(SoulpitError::NotOnEntryTile)
    );
    assert_eq!(
        validate_entry(1, &party, false, 1),
        Err(SoulpitError::NotOnEntryTile)
    );
    assert_eq!(
        validate_entry(0, &[participant(0, 8); 2], false, 1),
        Err(SoulpitError::InvalidParticipants)
    );
    assert_eq!(
        validate_entry(0, &[participant(0, 8); 6], false, 1),
        Err(SoulpitError::InvalidParticipants)
    );
    assert_eq!(
        validate_entry(0, &[participant(0, 8), participant(5, 8)], false, 1),
        Err(SoulpitError::InvalidParticipants)
    );
}

#[test]
fn four_waves_have_exact_donor_stack_population() -> Result<(), SoulpitError> {
    for (wave, expected) in [
        (1, [1, 1, 1, 1, 1, 1, 1]),
        (2, [1, 1, 1, 1, 5, 5, 5]),
        (3, [1, 1, 1, 1, 1, 15, 15]),
        (4, [1, 1, 1, 5, 5, 5, 40]),
    ] {
        assert_eq!(wave_stacks(wave)?.map(SoulpitStack::get), expected);
    }
    assert_eq!(wave_stacks(0), Err(SoulpitError::InvalidWave));
    assert_eq!(wave_stacks(5), Err(SoulpitError::InvalidWave));
    Ok(())
}

#[test]
fn stacks_apply_donor_health_attack_and_only_unsummoned_boss_experience() -> Result<(), SoulpitError>
{
    for (count, health, attack) in [(1, 150, 135), (5, 210, 175), (15, 360, 275), (40, 735, 525)] {
        let stack = SoulpitStack::from_count(count)?;
        assert_eq!(stack.health_percent(), health);
        assert_eq!(stack.attack_percent(), attack);
        assert_eq!(stack.scaled_health(100)?, u64::from(health));
        assert_eq!(stack.experience(7, true)?, 0);
        assert_eq!(
            stack.experience(7, false)?,
            if count == 40 { 196 } else { 0 }
        );
    }
    assert_eq!(SoulpitStack::Boss.scaled_health(1)?, 8);
    assert_eq!(SoulpitStack::from_count(2), Err(SoulpitError::InvalidStack));
    assert_eq!(
        SoulpitStack::Boss.experience(u64::MAX, false),
        Err(SoulpitError::Overflow)
    );
    assert_eq!(
        SoulpitStack::Boss.scaled_health(u64::MAX),
        Err(SoulpitError::Overflow)
    );
    Ok(())
}

fn progress() -> WaveProgress {
    WaveProgress {
        wave: 1,
        encounter_started_at_micros: 100,
        wave_started_at_micros: 100,
        now_micros: 100,
        spawn_applied: false,
        monsters_alive: 0,
        participants: 1,
    }
}

#[test]
fn waves_wait_for_spawn_delay_clearance_and_minimum_check_delay() -> Result<(), SoulpitError> {
    let mut facts = progress();
    assert_eq!(plan_wave(facts)?, WaveTransition::Waiting);
    facts.now_micros += SPAWN_DELAY_MICROS - 1;
    assert_eq!(plan_wave(facts)?, WaveTransition::Waiting);
    facts.now_micros += 1;
    assert_eq!(plan_wave(facts)?, WaveTransition::Spawn(wave_stacks(1)?));
    facts.spawn_applied = true;
    assert_eq!(plan_wave(facts)?, WaveTransition::Waiting);
    facts.now_micros = 100 + WAVE_CHECK_DELAY_MICROS;
    facts.monsters_alive = 1;
    assert_eq!(plan_wave(facts)?, WaveTransition::Waiting);
    facts.monsters_alive = 0;
    assert_eq!(plan_wave(facts)?, WaveTransition::NextWave(2));
    facts.wave = 4;
    assert_eq!(plan_wave(facts)?, WaveTransition::Completed);
    Ok(())
}

#[test]
fn timeout_abandonment_and_clock_validation_cannot_award_completion() -> Result<(), SoulpitError> {
    let mut facts = progress();
    facts.now_micros += TIME_LIMIT_MICROS;
    facts.wave = 4;
    facts.spawn_applied = true;
    assert_eq!(plan_wave(facts)?, WaveTransition::TimedOut);
    facts.now_micros -= 1;
    facts.participants = 0;
    assert_eq!(plan_wave(facts)?, WaveTransition::Abandoned);
    facts.now_micros = 99;
    assert_eq!(plan_wave(facts), Err(SoulpitError::InvalidClock));
    facts = progress();
    facts.wave_started_at_micros = 99;
    assert_eq!(plan_wave(facts), Err(SoulpitError::InvalidClock));
    facts = progress();
    facts.participants = 6;
    assert_eq!(plan_wave(facts), Err(SoulpitError::InvalidParticipants));
    Ok(())
}

#[test]
fn boss_ability_rolls_and_critical_boundary_match_donor() -> Result<(), SoulpitError> {
    assert_eq!(BossAbility::from_roll(1)?, BossAbility::Overpower);
    assert_eq!(BossAbility::from_roll(2)?, BossAbility::Enrage);
    assert_eq!(BossAbility::from_roll(3)?, BossAbility::Oppressor);
    assert_eq!(BossAbility::from_roll(0), Err(SoulpitError::InvalidRoll));
    assert_eq!(BossAbility::from_roll(4), Err(SoulpitError::InvalidRoll));
    assert_eq!(
        overpower_damage(103, 50)?,
        DamageOutcome {
            amount: 128,
            critical: true
        }
    );
    assert_eq!(
        overpower_damage(103, 51)?,
        DamageOutcome {
            amount: 103,
            critical: false
        }
    );
    assert_eq!(overpower_damage(1, 0), Err(SoulpitError::InvalidRoll));
    assert_eq!(overpower_damage(u64::MAX, 1), Err(SoulpitError::Overflow));
    Ok(())
}

#[test]
fn enrage_preserves_literal_reversed_last_bound() -> Result<(), SoulpitError> {
    for (health, percent) in [
        (100, 100),
        (81, 100),
        (80, 90),
        (61, 90),
        (60, 75),
        (41, 75),
        (40, 60),
        (21, 60),
        (20, 100),
        (1, 100),
        (0, 100),
    ] {
        assert_eq!(enrage_retained_percent(health, 100)?, percent);
    }
    assert_eq!(
        enrage_retained_percent(1, 0),
        Err(SoulpitError::InvalidHealth)
    );
    assert_eq!(
        enrage_retained_percent(101, 100),
        Err(SoulpitError::InvalidHealth)
    );
    assert_eq!(enrage_retained_percent(u64::MAX, u64::MAX)?, 100);
    Ok(())
}

#[test]
fn oppressor_spells_preserve_intervals_chances_and_duration() -> Result<(), SoulpitError> {
    assert_eq!(OPPRESSOR_INTERVAL_MICROS, 2_000_000);
    assert_eq!(OPPRESSOR_CONDITION_MICROS, 3_000_000);
    for (spell, chance) in [
        (OppressorSpell::RootAndFear, 25),
        (OppressorSpell::Powerless, 30),
        (OppressorSpell::IntenseHex, 15),
    ] {
        assert!(spell.triggers(chance)?);
        assert!(!spell.triggers(chance + 1)?);
        assert_eq!(spell.triggers(101), Err(SoulpitError::InvalidRoll));
    }
    assert_eq!(INTENSE_HEX_DAMAGE_DEALT_PERCENT, 50);
    assert_eq!(INTENSE_HEX_HEALING_RECEIVED_PERCENT, 50);
    Ok(())
}

fn conversion(kind: ConversionKind) -> ConversionFacts {
    ConversionFacts {
        kind,
        source_is_soul_core: true,
        target_is_soul_core: true,
        target_monster_exists: true,
        source_count: 1,
        target_count: 1,
        same_item_type: false,
        same_instance: false,
        selected_item_id: Some(47381),
        selected_item_weight: 100,
        free_capacity: 100,
        ominous_roll: 3,
    }
}

#[test]
fn fusion_consumes_two_cores_and_requires_two_in_same_item_type_source_stack()
-> Result<(), SoulpitError> {
    let mut facts = conversion(ConversionKind::Fuse);
    assert_eq!(
        plan_conversion(facts)?,
        ConversionPlan {
            output_item_id: 47381,
            consume_source: 1,
            consume_target: 1
        }
    );
    facts.same_item_type = true;
    assert_eq!(plan_conversion(facts), Err(SoulpitError::NotEnoughItems));
    facts.source_count = 2;
    facts.same_instance = true;
    assert_eq!(plan_conversion(facts)?.consume_target, 1);
    facts.free_capacity = 0;
    assert!(plan_conversion(facts).is_ok());
    facts.selected_item_id = Some(OMINOUS_SOUL_CORE_SOURCE_ID);
    assert_eq!(
        plan_conversion(facts)?.output_item_id,
        OMINOUS_SOUL_CORE_SOURCE_ID
    );
    Ok(())
}

#[test]
fn prism_ominous_branch_keeps_target_and_checks_ordinary_result_capacity_first()
-> Result<(), SoulpitError> {
    let mut facts = conversion(ConversionKind::Prism);
    facts.ominous_roll = 2;
    assert_eq!(
        plan_conversion(facts)?,
        ConversionPlan {
            output_item_id: OMINOUS_SOUL_CORE_SOURCE_ID,
            consume_source: 1,
            consume_target: 0
        }
    );
    facts.free_capacity = 99;
    assert_eq!(
        plan_conversion(facts),
        Err(SoulpitError::InsufficientCapacity)
    );
    facts.free_capacity = 100;
    facts.ominous_roll = 3;
    assert_eq!(plan_conversion(facts)?.consume_target, 1);
    Ok(())
}

#[test]
fn conversions_refuse_invalid_catalogue_state_before_consumption() {
    for kind in [
        ConversionKind::Fuse,
        ConversionKind::Prism,
        ConversionKind::Exalted,
    ] {
        let baseline = conversion(kind);
        assert_eq!(
            plan_conversion(ConversionFacts {
                source_count: 0,
                ..baseline
            }),
            Err(SoulpitError::NotEnoughItems)
        );
        assert_eq!(
            plan_conversion(ConversionFacts {
                target_count: 0,
                ..baseline
            }),
            Err(SoulpitError::NotEnoughItems)
        );
        assert_eq!(
            plan_conversion(ConversionFacts {
                target_is_soul_core: false,
                ..baseline
            }),
            Err(SoulpitError::InvalidCore)
        );
        assert_eq!(
            plan_conversion(ConversionFacts {
                selected_item_id: None,
                ..baseline
            }),
            Err(SoulpitError::NoCandidate)
        );
    }
    assert_eq!(
        plan_conversion(ConversionFacts {
            source_is_soul_core: false,
            ..conversion(ConversionKind::Fuse)
        }),
        Err(SoulpitError::InvalidCore)
    );
    assert_eq!(
        plan_conversion(ConversionFacts {
            target_monster_exists: false,
            ..conversion(ConversionKind::Prism)
        }),
        Err(SoulpitError::InvalidMonster)
    );
    assert_eq!(
        plan_conversion(ConversionFacts {
            target_monster_exists: false,
            ..conversion(ConversionKind::Exalted)
        }),
        Err(SoulpitError::InvalidMonster)
    );
    assert_eq!(
        plan_conversion(ConversionFacts {
            same_instance: true,
            ..conversion(ConversionKind::Prism)
        }),
        Err(SoulpitError::InvalidCore)
    );
    assert_eq!(
        plan_conversion(ConversionFacts {
            ominous_roll: 0,
            ..conversion(ConversionKind::Prism)
        }),
        Err(SoulpitError::InvalidRoll)
    );
}

#[test]
fn conversion_difficulty_preserves_numeric_nil_to_zero_fallback() {
    for (stars, next, previous) in [
        (0, 1, 0),
        (1, 2, 0),
        (2, 3, 1),
        (5, 6, 4),
        (6, 0, 5),
        (7, 0, 6),
        (255, 0, 0),
    ] {
        assert_eq!(
            conversion_difficulty(ConversionKind::Prism, stars),
            Some(next)
        );
        assert_eq!(
            conversion_difficulty(ConversionKind::Exalted, stars),
            Some(previous)
        );
        assert_eq!(conversion_difficulty(ConversionKind::Fuse, stars), None);
    }
}

fn drop_facts() -> DropFacts {
    DropFacts {
        fiendish: true,
        recipient_can_receive_loot: true,
        same_species_roll: 15,
        core_roll: 4,
        prism_roll: 3,
        same_species_core: Some(47381),
        category_core: Some(47382),
        prism_available: true,
    }
}

#[test]
fn fiendish_drop_effective_probabilities_are_four_and_three_percent() -> Result<(), SoulpitError> {
    let base = drop_facts();
    let mut cores = 0;
    let mut prisms = 0;
    for roll in 1..=100 {
        let plan = plan_drop(DropFacts {
            core_roll: roll,
            prism_roll: roll,
            ..base
        })?;
        cores += u32::from(plan.soul_core.is_some());
        prisms += u32::from(plan.soul_prism);
    }
    assert_eq!((cores, prisms), (4, 3));
    assert_eq!(plan_drop(base)?.soul_core, Some(47381));
    assert_eq!(
        plan_drop(DropFacts {
            same_species_roll: 16,
            ..base
        })?
        .soul_core,
        Some(47382)
    );
    Ok(())
}

#[test]
fn missing_selected_core_suppresses_prism_without_falling_back_to_category()
-> Result<(), SoulpitError> {
    let base = drop_facts();
    assert_eq!(
        plan_drop(DropFacts {
            same_species_core: None,
            ..base
        })?,
        DropPlan::NONE
    );
    assert_eq!(
        plan_drop(DropFacts {
            same_species_roll: 16,
            category_core: None,
            ..base
        })?,
        DropPlan::NONE
    );
    assert_eq!(
        plan_drop(DropFacts {
            same_species_core: None,
            core_roll: 5,
            ..base
        })?,
        DropPlan {
            soul_core: None,
            soul_prism: true
        }
    );
    assert_eq!(
        plan_drop(DropFacts {
            fiendish: false,
            ..base
        })?,
        DropPlan::NONE
    );
    assert_eq!(
        plan_drop(DropFacts {
            recipient_can_receive_loot: false,
            ..base
        })?,
        DropPlan::NONE
    );
    assert!(
        !plan_drop(DropFacts {
            prism_available: false,
            ..base
        })?
        .soul_prism
    );
    assert_eq!(
        plan_drop(DropFacts {
            core_roll: 0,
            ..base
        }),
        Err(SoulpitError::InvalidRoll)
    );
    Ok(())
}

#[test]
fn runtime_catalogue_keeps_prism_output_and_distinguishes_action_inputs() -> Result<(), SoulpitError>
{
    assert_eq!(SOUL_CORE_SOURCE_IDS.len(), 835);
    assert!(
        SOUL_CORE_SOURCE_IDS
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
    for id in SOUL_CORE_SOURCE_IDS {
        assert!(is_soul_core_output(*id));
        assert!(soul_core_name(*id).is_some());
    }
    // Crystal items.xml:82458 literally names source49038 "white lion" although its primary
    // type is soul cores. Preserve that catalogue member and its failed Lua name parser.
    assert_eq!(
        SOUL_CORE_SOURCE_IDS
            .iter()
            .copied()
            .filter(|id| soul_core_target_monster(*id).is_none())
            .collect::<Vec<_>>(),
        vec![49038, 49164]
    );
    assert_eq!(soul_core_name(49038), Some("white lion"));
    assert_eq!(soul_core_name(49163), Some("ominous soul core"));
    assert_eq!(soul_core_target_monster(49163), Some("ominous"));
    assert!(is_soul_core_output(SOUL_PRISM_SOURCE_ID));
    assert!(!is_soul_core(SOUL_PRISM_SOURCE_ID));
    assert!(is_soul_core(49038));
    assert!(!is_fusible_soul_core(49038));
    assert!(!is_fusible_soul_core(SOUL_PRISM_SOURCE_ID));
    assert!(is_fusible_soul_core(47899));
    assert_eq!(soul_core_name(47899), Some("falcon knight soul core"));
    assert!(is_fusible_soul_core(53142));
    assert_eq!(
        plan_conversion(ConversionFacts {
            selected_item_id: Some(SOUL_PRISM_SOURCE_ID),
            ..conversion(ConversionKind::Fuse)
        })?
        .output_item_id,
        SOUL_PRISM_SOURCE_ID
    );
    assert!(!is_soul_core(EXALTED_CORE_SOURCE_ID));
    assert_eq!(soul_core_target_monster(0), None);
    assert_eq!(
        soul_core_monster_name("horse soul core (taupe)"),
        Some("horse")
    );
    assert_eq!(soul_core_monster_name("a soul prism"), None);
    Ok(())
}

#[test]
fn conversion_outputs_use_source_names_and_exact_variations() {
    assert_eq!(soul_core_for_monster("Dragon"), Some(47411));
    assert_eq!(soul_core_for_monster("dragon"), Some(47411));
    assert_eq!(
        soul_core_for_monster("Butterfly"),
        soul_core_for_monster("Blue Butterfly")
    );
    assert!(soul_core_for_monster("Butterfly").is_some());
    assert!(soul_core_for_monster("Horse").is_some());
    assert_ne!(
        soul_core_for_monster("Horse"),
        soul_core_for_monster("Brown Horse")
    );
    assert_eq!(soul_core_for_monster("White Lion"), None);
    assert_eq!(soul_core_for_monster("Soul Prism"), None);
    assert_eq!(soul_core_for_monster("unknown creature"), None);
    assert_eq!(
        soulpit_entry_monster_name(49038).as_deref(),
        Some("white lion")
    );
    assert_eq!(
        soulpit_entry_monster_name(48962).as_deref(),
        Some("Brown Horse")
    );
    assert_eq!(
        soulpit_entry_monster_name(47503).as_deref(),
        Some("Blue Butterfly")
    );
    assert_eq!(soulpit_entry_monster_name(SOUL_PRISM_SOURCE_ID), None);
}

#[test]
fn native_positions_follow_staging_and_donor_arena_bounds() -> Result<(), SoulpitError> {
    for slot in 0..5 {
        assert_eq!(
            entry_slot(TilePos {
                x: 32375,
                y: 31158 + u16::from(slot),
                floor: -8
            }),
            Some(slot)
        );
        assert_eq!(
            entry_destination(slot)?,
            TilePos {
                x: 32373 + u16::from(slot),
                y: 31151,
                floor: -8
            }
        );
    }
    assert_eq!(
        entry_slot(TilePos {
            x: 32375,
            y: 31158,
            floor: -7
        }),
        None
    );
    assert_eq!(
        entry_slot(TilePos {
            x: 32374,
            y: 31158,
            floor: -8
        }),
        None
    );
    assert_eq!(entry_destination(5), Err(SoulpitError::NotOnEntryTile));
    assert!(is_arena_position(TilePos {
        x: 32362,
        y: 31132,
        floor: -8
    }));
    assert!(is_arena_position(TilePos {
        x: 32390,
        y: 31153,
        floor: -8
    }));
    assert!(!is_arena_position(EXIT_POSITION));
    assert!(!is_arena_position(OBELISK_POSITION));
    Ok(())
}

#[test]
fn encounter_advances_four_waves_and_rewards_survivors_once() -> Result<(), SoulpitError> {
    let mut fight = SoulpitEncounter::new(0, &[0, 1, 4])?;
    let mut now = 0;
    for wave in 1..=4 {
        now += SPAWN_DELAY_MICROS;
        let ids = std::array::from_fn(|index| u64::from(wave) * 100 + index as u64 + 1);
        let spawn = EncounterEffect::SpawnWave {
            wave,
            stacks: wave_stacks(wave)?,
        };
        assert_eq!(fight.advance(now)?, spawn);
        assert_eq!(fight.advance(now)?, spawn);
        assert!(fight.spawned(wave, ids.map(Some), now)?);
        let acknowledged = fight.clone();
        assert!(!fight.spawned(wave, ids.map(Some), now)?);
        assert_eq!(fight, acknowledged);
        if wave == 1 {
            assert!(fight.leave(4, now)?);
            assert!(!fight.leave(4, now)?);
        }
        for (id, stack) in ids.into_iter().zip(wave_stacks(wave)?) {
            assert_eq!(
                fight.defeated(wave, id)?,
                Some(SoulpitMonster {
                    stack,
                    is_summon: false
                })
            );
            assert_eq!(fight.defeated(wave, id)?, None);
        }
        assert_eq!(fight.advance(now)?, EncounterEffect::NoChange);
        now += WAVE_CHECK_DELAY_MICROS - SPAWN_DELAY_MICROS;
        let expected = if wave < 4 {
            EncounterEffect::WaveStarted { wave: wave + 1 }
        } else {
            EncounterEffect::Completed {
                surviving_slots: vec![0, 1],
            }
        };
        assert_eq!(fight.advance(now)?, expected);
        assert_eq!(fight.defeated(wave, ids[0])?, None);
        assert_eq!(fight.advance(now)?, EncounterEffect::NoChange);
    }
    assert_eq!(fight.snapshot().phase, EncounterPhase::Completed);
    assert_eq!(fight.advance(TIME_LIMIT_MICROS)?, EncounterEffect::NoChange);
    assert_eq!(
        fight.interrupt(TIME_LIMIT_MICROS)?,
        EncounterEffect::NoChange
    );
    Ok(())
}

#[test]
fn summons_keep_wave_open_and_replayed_summons_never_resurrect() -> Result<(), SoulpitError> {
    let mut fight = SoulpitEncounter::new(0, &[0])?;
    let ids = [1, 2, 3, 4, 5, 6, 7];
    assert!(fight.spawned(1, ids.map(Some), SPAWN_DELAY_MICROS)?);
    assert!(fight.summoned(1, 8, SoulpitStack::Boss)?);
    assert!(!fight.summoned(1, 8, SoulpitStack::Boss)?);
    for id in ids {
        assert!(fight.defeated(1, id)?.is_some());
    }
    assert_eq!(
        fight.advance(WAVE_CHECK_DELAY_MICROS)?,
        EncounterEffect::NoChange
    );
    let summoned = fight.defeated(1, 8)?;
    assert_eq!(
        summoned,
        Some(SoulpitMonster {
            stack: SoulpitStack::Boss,
            is_summon: true
        })
    );
    assert_eq!(
        summoned.map(|monster| monster.stack.experience(100, monster.is_summon)),
        Some(Ok(0))
    );
    assert!(!fight.summoned(1, 8, SoulpitStack::Boss)?);
    assert_eq!(fight.monster(8), None);
    assert_eq!(
        fight.advance(WAVE_CHECK_DELAY_MICROS)?,
        EncounterEffect::WaveStarted { wave: 2 }
    );
    assert!(!fight.summoned(1, 9, SoulpitStack::Normal)?);
    Ok(())
}

#[test]
fn refused_encounter_transitions_preserve_state() -> Result<(), SoulpitError> {
    assert_eq!(
        SoulpitEncounter::new(0, &[]),
        Err(SoulpitError::InvalidParticipants)
    );
    assert_eq!(
        SoulpitEncounter::new(0, &[0, 0]),
        Err(SoulpitError::InvalidParticipants)
    );
    assert_eq!(
        SoulpitEncounter::new(0, &[0, 5]),
        Err(SoulpitError::InvalidParticipants)
    );
    let mut fight = SoulpitEncounter::new(10, &[0])?;
    let baseline = fight.clone();
    assert_eq!(fight.advance(9), Err(SoulpitError::InvalidClock));
    assert_eq!(
        fight.spawned(1, [1, 2, 3, 4, 5, 6, 7].map(Some), 100),
        Err(SoulpitError::InvalidState)
    );
    assert_eq!(
        fight.spawned(1, [1, 1, 3, 4, 5, 6, 7].map(Some), 10 + SPAWN_DELAY_MICROS),
        Err(SoulpitError::InvalidMonster)
    );
    assert_eq!(
        fight.spawned(1, [0, 2, 3, 4, 5, 6, 7].map(Some), 10 + SPAWN_DELAY_MICROS),
        Err(SoulpitError::InvalidMonster)
    );
    assert_eq!(fight.leave(2, 100), Err(SoulpitError::InvalidParticipants));
    assert_eq!(fight.defeated(1, 1), Err(SoulpitError::InvalidState));
    assert_eq!(
        fight.summoned(1, 8, SoulpitStack::Normal),
        Err(SoulpitError::InvalidState)
    );
    assert_eq!(fight, baseline);
    fight.spawned(1, [1, 2, 3, 4, 5, 6, 7].map(Some), 10 + SPAWN_DELAY_MICROS)?;
    let baseline = fight.clone();
    assert_eq!(
        fight.spawned(1, [8, 2, 3, 4, 5, 6, 7].map(Some), 10 + SPAWN_DELAY_MICROS),
        Err(SoulpitError::ConflictingReplay)
    );
    assert_eq!(fight.defeated(2, 1), Err(SoulpitError::InvalidState));
    assert_eq!(fight.defeated(1, 99), Err(SoulpitError::InvalidMonster));
    assert_eq!(
        fight.summoned(1, 1, SoulpitStack::Normal),
        Err(SoulpitError::ConflictingReplay)
    );
    assert_eq!(
        fight.summoned(1, 0, SoulpitStack::Normal),
        Err(SoulpitError::InvalidMonster)
    );
    assert_eq!(fight, baseline);
    Ok(())
}

#[test]
fn death_logout_timeout_and_restart_never_create_survival_reward() -> Result<(), SoulpitError> {
    let mut fight = SoulpitEncounter::new(0, &[0, 1])?;
    assert!(fight.leave(0, 10)?);
    assert_eq!(fight.advance(10)?, EncounterEffect::NoChange);
    assert!(fight.rejoin(0, 11)?);
    assert!(!fight.rejoin(0, 11)?);
    assert!(fight.leave(0, 12)?);
    assert!(fight.leave(1, 12)?);
    assert_eq!(fight.advance(12)?, EncounterEffect::Abandoned);
    assert_eq!(fight.advance(12)?, EncounterEffect::NoChange);
    assert_eq!(fight.rejoin(0, 13), Err(SoulpitError::InvalidState));
    let mut fight = SoulpitEncounter::new(0, &[0])?;
    fight.spawned(1, [1, 2, 3, 4, 5, 6, 7].map(Some), SPAWN_DELAY_MICROS)?;
    assert_eq!(fight.advance(TIME_LIMIT_MICROS)?, EncounterEffect::TimedOut);
    assert_eq!(fight.snapshot().monsters_alive, 0);
    assert_eq!(fight.defeated(1, 1)?, None);
    assert_eq!(fight.advance(TIME_LIMIT_MICROS)?, EncounterEffect::NoChange);
    let mut fight = SoulpitEncounter::new(0, &[0])?;
    assert_eq!(fight.interrupt(1)?, EncounterEffect::Interrupted);
    assert_eq!(fight.interrupt(1)?, EncounterEffect::NoChange);
    assert_eq!(fight.rejoin(0, 2), Err(SoulpitError::InvalidState));
    Ok(())
}

#[test]
fn encounter_checkpoint_roundtrip_retains_death_replay_and_external_effect_format()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fight = SoulpitEncounter::new(0, &[0, 1])?;
    fight.spawned(1, [1, 2, 3, 4, 5, 6, 7].map(Some), SPAWN_DELAY_MICROS)?;
    fight.summoned(1, 8, SoulpitStack::Normal)?;
    fight.defeated(1, 1)?;
    fight.leave(1, SPAWN_DELAY_MICROS)?;
    let encoded = serde_json::to_value(fight.checkpoint())?;
    let checkpoint = serde_json::from_value(encoded)?;
    let mut restored = SoulpitEncounter::restore(checkpoint, 8)?;
    assert_eq!(restored, fight);
    assert_eq!(restored.defeated(1, 1)?, None);
    assert!(!restored.summoned(1, 8, SoulpitStack::Normal)?);
    assert_eq!(restored.snapshot().surviving_slots, vec![0]);
    assert_eq!(restored.snapshot().monsters_alive, 7);
    assert_eq!(
        serde_json::to_value(EncounterEffect::Completed {
            surviving_slots: vec![0, 4]
        })?,
        serde_json::json!({"Completed":{"surviving_slots":[0,4]}})
    );
    assert_eq!(
        SoulpitEncounter::restore(fight.checkpoint(), 7),
        Err(SoulpitError::InvalidState)
    );
    Ok(())
}

#[test]
fn encounter_checkpoint_rejects_tampered_masks_clocks_and_monster_membership()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fight = SoulpitEncounter::new(10, &[0, 1])?;
    fight.spawned(1, [1, 2, 3, 4, 5, 6, 7].map(Some), 10 + SPAWN_DELAY_MICROS)?;
    let baseline = serde_json::to_value(fight.checkpoint())?;
    for (field, value) in [
        ("version", serde_json::json!(2)),
        ("original_slots", serde_json::json!(32)),
        ("surviving_slots", serde_json::json!(4)),
        ("wave", serde_json::json!(0)),
        ("wave", serde_json::json!(5)),
        ("wave_started_at_micros", serde_json::json!(9)),
        ("last_observed_at_micros", serde_json::json!(9)),
        ("spawned_ids", serde_json::json!([1, 1, 3, 4, 5, 6, 7])),
        ("spawned_ids", serde_json::Value::Null),
        ("phase", serde_json::json!("Preparing")),
        ("phase", serde_json::json!("Completed")),
        ("phase", serde_json::json!("TimedOut")),
        ("phase", serde_json::json!("Abandoned")),
    ] {
        let mut bad = baseline.clone();
        bad[field] = value;
        assert_eq!(
            SoulpitEncounter::restore(serde_json::from_value(bad)?, 100),
            Err(SoulpitError::InvalidState),
            "{field}"
        );
    }
    let mut bad = baseline.clone();
    bad["live_monsters"]["99"] = serde_json::json!({"stack":"Normal","is_summon":false});
    assert_eq!(
        SoulpitEncounter::restore(serde_json::from_value(bad)?, 100),
        Err(SoulpitError::InvalidState)
    );
    let mut bad = baseline;
    bad["seen_monsters"]["1"] = serde_json::json!([1,{"stack":"Boss","is_summon":false}]);
    assert_eq!(
        SoulpitEncounter::restore(serde_json::from_value(bad)?, 100),
        Err(SoulpitError::InvalidState)
    );
    Ok(())
}

#[test]
fn partial_and_empty_spawn_attempts_replay_and_clear_without_inventing_monsters()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fight = SoulpitEncounter::new(0, &[0])?;
    let ids = [Some(1), None, None, Some(4), None, None, Some(7)];
    assert!(fight.spawned(1, ids, SPAWN_DELAY_MICROS)?);
    assert_eq!(fight.snapshot().monsters_alive, 3);
    assert!(!fight.spawned(1, ids, SPAWN_DELAY_MICROS)?);
    assert_eq!(
        fight.spawned(1, [None; 7], SPAWN_DELAY_MICROS),
        Err(SoulpitError::ConflictingReplay)
    );
    for id in [1, 4, 7] {
        assert!(fight.defeated(1, id)?.is_some());
    }
    let mut fight = SoulpitEncounter::restore(
        serde_json::from_value(serde_json::to_value(fight.checkpoint())?)?,
        3,
    )?;
    assert_eq!(
        fight.advance(WAVE_CHECK_DELAY_MICROS)?,
        EncounterEffect::WaveStarted { wave: 2 }
    );
    assert!(fight.spawned(2, [None; 7], WAVE_CHECK_DELAY_MICROS + SPAWN_DELAY_MICROS)?);
    assert_eq!(fight.snapshot().monsters_alive, 0);
    assert_eq!(
        fight.advance(2 * WAVE_CHECK_DELAY_MICROS - 1)?,
        EncounterEffect::NoChange
    );
    assert_eq!(
        fight.advance(2 * WAVE_CHECK_DELAY_MICROS)?,
        EncounterEffect::WaveStarted { wave: 3 }
    );
    assert!(SoulpitEncounter::restore(fight.checkpoint(), 3).is_ok());
    Ok(())
}

#[test]
fn physical_events_never_run_global_wave_clearance_or_completion() -> Result<(), SoulpitError> {
    let mut fight = SoulpitEncounter::new(0, &[0, 1])?;
    for wave in 1..=4 {
        let start = fight.snapshot().wave_started_at_micros;
        assert_eq!(
            fight.observe_terminal(start + SPAWN_DELAY_MICROS)?,
            EncounterEffect::NoChange
        );
        assert_eq!(fight.snapshot().phase, EncounterPhase::Preparing);
        let ids = std::array::from_fn(|index| Some(u64::from(wave - 1) * 7 + index as u64 + 1));
        fight.spawned(wave, ids, start + SPAWN_DELAY_MICROS)?;
        for id in ids.into_iter().flatten() {
            assert!(fight.defeated(wave, id)?.is_some());
        }
        let now = start + WAVE_CHECK_DELAY_MICROS + 100_000;
        assert_eq!(fight.observe_terminal(now)?, EncounterEffect::NoChange);
        assert_eq!(fight.snapshot().wave, wave);
        assert_eq!(fight.snapshot().phase, EncounterPhase::Fighting);
        assert_eq!(fight.snapshot().last_observed_at_micros, now);
        let baseline = fight.clone();
        assert_eq!(
            fight.observe_terminal(now - 1),
            Err(SoulpitError::InvalidClock)
        );
        assert_eq!(fight, baseline);
        let expected = if wave == 4 {
            EncounterEffect::Completed {
                surviving_slots: vec![0, 1],
            }
        } else {
            EncounterEffect::WaveStarted { wave: wave + 1 }
        };
        assert_eq!(fight.advance(now)?, expected);
    }
    let mut abandoned = SoulpitEncounter::new(0, &[0])?;
    abandoned.leave(0, 1)?;
    assert_eq!(abandoned.observe_terminal(1)?, EncounterEffect::Abandoned);
    let mut timed_out = SoulpitEncounter::new(0, &[0])?;
    timed_out.leave(0, TIME_LIMIT_MICROS)?;
    assert_eq!(
        timed_out.observe_terminal(TIME_LIMIT_MICROS)?,
        EncounterEffect::TimedOut
    );
    Ok(())
}
