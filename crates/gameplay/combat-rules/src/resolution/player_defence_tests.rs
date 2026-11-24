#![allow(clippy::expect_used)]

use super::*;

fn constants() -> &'static AttackConstants {
    AttackConstants::checked_in().expect("constants")
}

const SHIELD: HandDefence = HandDefence {
    source: DefenceSource::Shield,
    value: 20,
    skill_index: Some(SHIELDING),
};

#[test]
fn a_shield_defends_with_the_scaled_canary_formula() {
    let table = constants();
    assert_eq!(
        player_defence_bounds(table, SHIELD, 50, FightMode::Defensive, true),
        Some((23, 47))
    );
    let offensive = player_defence_bounds(table, SHIELD, 50, FightMode::Offensive, true);
    let rested = player_defence_bounds(table, SHIELD, 50, FightMode::Offensive, false);
    assert!(
        offensive < rested,
        "an offensive swing halves the defence factor"
    );
}

#[test]
fn no_defending_skill_defends_with_the_fight_modes_fixed_value() {
    let table = constants();
    let wand = HandDefence {
        source: DefenceSource::Weapon,
        value: 30,
        skill_index: None,
    };
    for hand in [wand, SHIELD] {
        let skill = if hand.skill_index.is_some() { 0 } else { 80 };
        assert_eq!(
            player_defence_bounds(table, hand, skill, FightMode::Balanced, true),
            Some(zero_skill_defence_bounds(table, FightMode::Balanced))
        );
        assert_eq!(
            player_defence_bounds(table, hand, skill, FightMode::Defensive, false),
            Some(zero_skill_defence_bounds(table, FightMode::Defensive))
        );
    }
}

#[test]
fn defence_draws_within_its_bounds_and_never_below_zero() {
    assert_eq!(player_defence_mitigated(100, (23, 47), 0), 77);
    assert_eq!(player_defence_mitigated(100, (23, 47), 24), 53);
    assert_eq!(player_defence_mitigated(100, (23, 47), 25), 77);
    assert_eq!(player_defence_mitigated(30, (23, 47), 24), 0);
    assert_eq!(player_defence_mitigated(30, (0, 0), u64::MAX), 30);
}

#[test]
fn hand_defence_uses_base_value_when_extra_defence_is_omitted() {
    let world = oteryn_protocol_oteryn::WorldId::decode(&[
        1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, 8,
    ])
    .expect("test WorldId");
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/spells.manifest.json");
    let native =
        oteryn_gameplay_content::content::native_gameplay::retained_callback_native_fixture_at(
            world, &manifest,
        );
    for (key, value, skill_index) in [
        ("oteryn:item.tibia.i10392", 22, 2),
        ("oteryn:item.tibia.i10406", 15, 3),
        ("oteryn:item.tibia.i21176", 18, 3),
        ("oteryn:item.tibia.i22727", 28, 3),
        ("oteryn:item.tibia.i50181", 14, 0),
        ("oteryn:item.tibia.i50182", 15, 0),
    ] {
        assert_eq!(
            hand_defence(&native, constants(), |slot| {
                (slot == 5).then_some((key, "definition-r1"))
            }),
            Some(HandDefence {
                source: DefenceSource::Weapon,
                value,
                skill_index: Some(skill_index),
            }),
            "{key}",
        );
    }
}
