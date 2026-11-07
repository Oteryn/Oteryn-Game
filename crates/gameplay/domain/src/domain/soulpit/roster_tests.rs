//! Source-only candidates preserve Crystal's registry and race-map semantics;
//! they never grant a native Creature or Item identity.

use super::{
    ConversionFacts, ConversionKind, SOURCE_BESTIARY_ROSTER_SHA256, SoulpitError,
    conversion_difficulty, plan_conversion, soul_core_for_monster, source_bestiary_candidates,
    source_monster,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn source_roster_keeps_every_unique_race_in_source_order_and_exact_stars_pool() -> TestResult {
    assert_eq!(
        SOURCE_BESTIARY_ROSTER_SHA256,
        "27cfcbf31352d07df650e93e6141c179c706bf13f68b54d899e67a4f9eb47f02"
    );
    // Monsters::getMonstersByBestiaryStars traverses Game's std::map<uint16_t,...>.
    let rows: Vec<_> = source_bestiary_candidates().collect();
    assert_eq!(rows.len(), 819);
    let ids: Vec<_> = rows
        .iter()
        .map(|row| row.race_id.ok_or("candidate race ID missing"))
        .collect::<Result<_, _>>()?;
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    let counts: Vec<_> = (0..=6)
        .map(|stars| rows.iter().filter(|row| row.stars == stars).count())
        .collect();
    assert_eq!(counts, [12, 66, 159, 303, 209, 70, 0]);
    // The configuration has difficulty6, but no source monster has six stars.
    assert_eq!(conversion_difficulty(ConversionKind::Prism, 5), Some(6));
    assert!(!rows.iter().any(|row| row.stars == 6));
    Ok(())
}

#[test]
fn source_roster_contains_cake_golem_despite_the_native_entry_gap() -> TestResult {
    // quests/a_piece_of_cake/cake_golem.lua:1,10-19,122 registers this creature.
    let cake = source_monster("cAkE gOlEm").ok_or("literal case-insensitive source registry")?;
    assert_eq!(cake.registration_name, "Cake Golem");
    assert_eq!(cake.name, "Cake Golem");
    assert_eq!(cake.race_id, Some(680));
    assert_eq!((cake.class, cake.race, cake.stars), ("Construct", 4, 2));
    assert_eq!(soul_core_for_monster(cake.name), Some(47691));
    assert!(source_bestiary_candidates().any(|row| row.race_id == Some(680)));
    assert!(source_monster("CakeGolem").is_none());
    Ok(())
}

#[test]
fn source_registry_target_facts_do_not_imply_bestiary_draw_membership() -> TestResult {
    // These types are valid MonsterType targets without any raceId registration.
    let crypt = source_monster("Crypt Warrior").ok_or("source monster missing")?;
    assert_eq!(crypt.race_id, None);
    assert_eq!((crypt.class, crypt.race, crypt.stars), ("Undead", 0, 4));
    assert_eq!(soul_core_for_monster(crypt.name), Some(47984));
    for name in [
        "Crypt Warrior",
        "Lion Knight",
        "Muglex Clan Footman",
        "Muglex Clan Assassin",
    ] {
        let row = source_monster(name).ok_or("source monster missing")?;
        assert_eq!(row.race_id, None);
        assert!(!source_bestiary_candidates().any(|candidate| candidate.registration_name == name));
        if name != "Crypt Warrior" {
            assert_eq!((row.class, row.race, row.stars), ("", 0, 0));
        }
    }
    Ok(())
}

#[test]
fn source_numeric_race_is_independent_of_bestiary_class_text() -> TestResult {
    // getMonstersByRace filters Bestiary.race; normalizing the human-readable
    // class silently removes these exact source mismatches from their real pool.
    for (name, class, race, stars) in [
        ("Tainted Soul", "Magical", 9, 2),
        ("Naga Archer", "Reptile", 1, 4),
        ("Naga Warrior", "Reptile", 1, 4),
        ("Lavafungus", "Plant", 20, 4),
        ("Terrified Elephant", "Mammal", 0, 2),
        ("Haunted Dragon", "Dragon", 0, 4),
    ] {
        let row = source_monster(name).ok_or("source monster missing")?;
        assert_eq!(
            (row.class, row.race, row.stars),
            (class, race, stars),
            "{name}"
        );
        assert!(source_bestiary_candidates().any(|candidate| candidate.race_id == row.race_id));
    }
    let unwelcome = source_monster("The Unwelcome").ok_or("source monster missing")?;
    assert_eq!(unwelcome.race_id, Some(1868));
    assert_eq!(
        (unwelcome.class, unwelcome.race, unwelcome.stars),
        ("", 0, 0)
    );
    assert!(source_bestiary_candidates().any(|row| row.race_id == Some(1868)));
    Ok(())
}

#[test]
fn source_duplicate_races_keep_one_draw_with_equivalent_observable_output() -> TestResult {
    // Game::addBestiaryList first-insertion wins. Its loader's filesystem order
    // is unspecified, but these duplicate groups have equal W07 output behavior.
    for name in ["Night Harpy", "Day Night Harpy"] {
        let row = source_monster(name).ok_or("source monster missing")?;
        assert_eq!(row.name, "Night Harpy");
        assert_eq!(row.race_id, Some(2764));
        assert_eq!((row.class, row.race, row.stars), ("Bird", 3, 4));
        assert_eq!(soul_core_for_monster(row.name), Some(52813));
    }
    assert_eq!(
        source_bestiary_candidates()
            .filter(|row| row.race_id == Some(2764))
            .count(),
        1
    );
    for name in ["Mushroom", "Pillar of Dark Energy"] {
        let row = source_monster(name).ok_or("source monster missing")?;
        assert_eq!(row.race_id, Some(0));
        assert_eq!((row.class, row.race, row.stars), ("", 0, 0));
        assert_eq!(soul_core_for_monster(row.name), None);
    }
    let zeros: Vec<_> = source_bestiary_candidates()
        .filter(|row| row.race_id == Some(0))
        .collect();
    assert_eq!(zeros.len(), 1);
    assert_eq!(soul_core_for_monster(zeros[0].name), None);
    Ok(())
}

#[test]
fn source_draw_uses_final_display_name_while_target_resolution_uses_registry_name() -> TestResult {
    // register_monster_type.name changes getName(), not the registry key.
    // A blue Nomad draw consequently yields the basic Nomad core in this donor.
    for (registration, display, output) in [
        ("Nomad Blue", "Nomad", 47558),
        ("Nomad Female", "Nomad", 47558),
        ("Brown Horse", "Horse", 48961),
        ("Grey Horse", "Horse", 48961),
        ("Blue Butterfly", "Butterfly", 47503),
        ("Red Butterfly", "Butterfly", 47503),
        ("Day Night Harpy", "Night Harpy", 52813),
    ] {
        let row = source_monster(registration).ok_or("source monster missing")?;
        assert_eq!(row.registration_name, registration);
        assert_eq!(row.name, display);
        assert_eq!(soul_core_for_monster(row.name), Some(output));
        // registerMonsterType iterates parsers with Lua pairs. Whether raceId is
        // processed before or after the display rename, registry resolution has
        // the same candidate class/stars/output for every source race alias.
        let display_type =
            source_monster(display).ok_or("renamed source registry target missing")?;
        assert_eq!(
            (row.class, row.race, row.stars),
            (display_type.class, display_type.race, display_type.stars)
        );
    }
    assert_eq!(soul_core_for_monster("Nomad Blue"), Some(48965));
    assert_eq!(soul_core_for_monster("Brown Horse"), Some(48962));
    assert_eq!(soul_core_for_monster("Red Butterfly"), Some(47504));
    Ok(())
}

#[test]
fn source_selected_missing_core_refuses_before_any_ominous_replacement_or_debit() -> TestResult {
    let selected = source_bestiary_candidates()
        .find(|row| row.race_id == Some(0))
        .ok_or("source race0 candidate missing")?;
    assert_eq!(selected.stars, 0);
    // Exalted on a one-star target follows Lua's numeric-key fallback to zero.
    assert_eq!(conversion_difficulty(ConversionKind::Exalted, 1), Some(0));
    for kind in [ConversionKind::Prism, ConversionKind::Exalted] {
        assert_eq!(
            plan_conversion(ConversionFacts {
                kind,
                source_is_soul_core: false,
                target_is_soul_core: true,
                target_monster_exists: true,
                source_count: 1,
                target_count: 1,
                same_item_type: false,
                same_instance: false,
                selected_item_id: soul_core_for_monster(selected.name),
                selected_item_weight: 0,
                free_capacity: u64::MAX,
                ominous_roll: 1,
            }),
            Err(SoulpitError::NoCandidate)
        );
    }
    Ok(())
}
