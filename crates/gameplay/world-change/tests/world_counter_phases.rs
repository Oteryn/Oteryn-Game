#![allow(
    clippy::expect_used,
    reason = "authored focused phase fixtures must be valid"
)]
use oteryn_gameplay_world_change::counter::{BINDINGS_JSON, Bindings, CounterKind as K};

fn bindings() -> Bindings {
    serde_json::from_str(BINDINGS_JSON).expect("authored bindings")
}
fn stages(kind: K, stage: u8) -> [u8; 8] {
    let mut values = [1; 8];
    values[kind.index()] = stage;
    values
}

#[test]
fn worldchg_phase_serpents_replace_only_seacrest_grounds_sources() {
    let b = bindings();
    for stage in [1, 2] {
        assert_eq!(
            b.spawn_projection(
                "oteryn:creature.shark",
                (33530, 31630, 14),
                stages(K::FireFeathered, stage)
            ),
            Some(Some("oteryn:creature.seacrest_serpent"))
        );
    }
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.shark",
            (33530, 31630, 14),
            stages(K::FireFeathered, 3)
        ),
        Some(Some("oteryn:creature.renegade_quara_constrictor"))
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.seacrest_serpent",
            (33393, 31855, 14),
            stages(K::FireFeathered, 3)
        ),
        None,
        "Fury source belongs to WVAR"
    );
}
#[test]
fn worldchg_phase_mage_and_deepling_bosses_follow_current_stage() {
    let b = bindings();
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.energized_raging_mage",
            (33144, 31529, 2),
            stages(K::MagesTower, 2)
        ),
        Some(Some("oteryn:creature.raging_mage"))
    );
    for stage in [3, 4] {
        assert_eq!(
            b.spawn_projection(
                "oteryn:creature.energized_raging_mage",
                (33144, 31529, 2),
                stages(K::MagesTower, stage)
            ),
            Some(None)
        );
    }
    for stage in [1, 2] {
        assert_eq!(
            b.spawn_projection(
                "oteryn:creature.jaul",
                (33545, 31274, 11),
                stages(K::Deeplings, stage)
            ),
            Some(None)
        );
    }
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.jaul",
            (33545, 31274, 11),
            stages(K::Deeplings, 3)
        ),
        Some(Some("oteryn:creature.jaul"))
    );
}
#[test]
fn worldchg_phase_hive_membership_excludes_vandura_outpost() {
    let b = bindings();
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.kollos",
            (32300, 31250, 7),
            stages(K::HiveBorn, 1)
        ),
        None
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.kollos",
            (33450, 31250, 7),
            stages(K::HiveBorn, 1)
        ),
        Some(None)
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.kollos",
            (33450, 31250, 7),
            stages(K::HiveBorn, 2)
        ),
        Some(Some("oteryn:creature.kollos"))
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.spidris_elite",
            (33450, 31250, 7),
            stages(K::HiveBorn, 2)
        ),
        Some(None)
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.spidris_elite",
            (33450, 31250, 7),
            stages(K::HiveBorn, 3)
        ),
        Some(Some("oteryn:creature.spidris_elite"))
    );
}
#[test]
fn worldchg_phase_population_and_eruption_suppress_dormant_points() {
    let b = bindings();
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.starving_wolf",
            (32596, 31745, 7),
            stages(K::Overhunting, 1)
        ),
        Some(None)
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.starving_wolf",
            (32596, 31745, 7),
            stages(K::Overhunting, 2)
        ),
        Some(Some("oteryn:creature.starving_wolf"))
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.deer",
            (32596, 31745, 7),
            stages(K::Overhunting, 2)
        ),
        Some(None)
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.fire_elemental",
            (32060, 32620, 5),
            stages(K::FireEarth, 0)
        ),
        Some(None)
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.fire_elemental",
            (32060, 32620, 5),
            stages(K::FireEarth, 1)
        ),
        Some(Some("oteryn:creature.fire_elemental"))
    );
    assert_eq!(
        b.spawn_projection(
            "oteryn:creature.fire_elemental",
            (100, 100, 7),
            stages(K::FireEarth, 0)
        ),
        None
    );
}
