//! SHIELD-DEFENCE-1: the hands' defence read from the pinned profiles.

use super::*;
use oteryn_gameplay_combat_rules::attack::formulas::DefenceSource;

const SWORD: &str = "oteryn:item.tibia.i3264";
const SHIELD: &str = "oteryn:item.tibia.i3412";
const SPELLBOOK: &str = "oteryn:item.tibia.i21400";
const FEET: &str = "oteryn:item.tibia.i3549";

fn held(content: &NativeGameplayState, items: &[(&str, u8)]) -> EquipmentSnapshot {
    let mut equipment = snapshot(content, items[0].0, items[0].1);
    equipment.items = items
        .iter()
        .flat_map(|(key, slot)| snapshot(content, key, *slot).items)
        .collect();
    equipment
}

fn defence(source: DefenceSource, value: u32, skill_index: usize) -> Option<Option<HandDefence>> {
    Some(Some(HandDefence {
        source,
        value,
        skill_index: Some(skill_index),
    }))
}

#[test]
fn the_shield_defends_over_the_weapon_and_bare_hands_with_the_fist() -> TestResult {
    let content = native(&super::input()?)?;
    let read = |items: &[(&str, u8)]| equipped_defence(&held(&content, items), &content);
    assert_eq!(read(&[(FEET, 4)]), defence(DefenceSource::Fist, 7, 0));
    assert_eq!(read(&[(SHIELD, 6)]), defence(DefenceSource::Shield, 19, 5));
    assert_eq!(read(&[(SWORD, 5)]), defence(DefenceSource::Weapon, 13, 2));
    assert_eq!(
        read(&[(SWORD, 5), (SHIELD, 6)]),
        defence(DefenceSource::Shield, 20, 5),
        "the shield adds the weapon's extra defence"
    );
    Ok(())
}

#[test]
fn a_spellbook_defends_alone_and_with_the_attack_weapon() -> TestResult {
    use crate::content::ReferenceItemField::Known;

    let content = native(&super::input()?)?;
    let book = content
        .item_policy(SPELLBOOK, "definition-r1")
        .ok_or("spellbook")?;
    let Known(book_weapon) = &book.record().semantics.weapon else {
        return Err("spellbook weapon facts unavailable".into());
    };
    assert_eq!(
        book_weapon.weapon_type,
        Known(crate::content::ReferenceWeaponType::Spellbook)
    );
    let Known(base) = book_weapon.defense else {
        return Err("spellbook defence unavailable".into());
    };
    let book_value = u32::try_from(base.0)?;
    let sword = content.item_policy(SWORD, "definition-r1").ok_or("sword")?;
    let Known(sword_weapon) = &sword.record().semantics.weapon else {
        return Err("sword weapon facts unavailable".into());
    };
    let Known(extra) = sword_weapon.extra_defense else {
        return Err("sword extra defence unavailable".into());
    };
    let shield = content
        .item_policy(SHIELD, "definition-r1")
        .ok_or("shield")?;
    let Known(shield_weapon) = &shield.record().semantics.weapon else {
        return Err("shield weapon facts unavailable".into());
    };
    let Known(shield_base) = shield_weapon.defense else {
        return Err("shield defence unavailable".into());
    };
    assert!(shield_base.0 > base.0);
    let read = |items: &[(&str, u8)]| equipped_defence(&held(&content, items), &content);
    assert_eq!(
        read(&[(SPELLBOOK, 6)]),
        defence(DefenceSource::Shield, book_value, 5)
    );
    assert_eq!(
        read(&[(SWORD, 5), (SPELLBOOK, 6)]),
        defence(
            DefenceSource::Shield,
            book_value + u32::try_from(extra.0)?,
            5
        )
    );
    assert_eq!(
        read(&[(SPELLBOOK, 5), (SHIELD, 6)]),
        defence(DefenceSource::Shield, u32::try_from(shield_base.0)?, 5)
    );
    Ok(())
}

#[test]
fn twin_hooks_with_unauthored_extra_defence_contribute_to_wheel_mitigation() -> TestResult {
    use crate::durability::character_build::DurableBuildState;
    use crate::spell::Vocation;
    use crate::spell::cast::{CharacterCastFacts, PlayerSpellState};

    let content = native(&super::input()?)?;
    let equipment = held(&content, &[("oteryn:item.tibia.i10392", 5)]);
    let hands = equipped_wheel_hands(&equipment, &content);
    assert!(
        hands.and_then(|hands| hands.weapon_hand).is_some(),
        "Twin Hooks must resolve to a Wheel weapon hand"
    );

    let mut state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Knight,
            level: 20,
            magic_level: 0,
            max_health: 100,
            max_mana: 100,
            max_soul: 100,
        },
        0,
        0,
    )
    .ok_or("Knight spell state")?;
    let build = DurableBuildState::new("knight", (0, 0), [(10, 0); 7])?;
    state
        .load_owned_build(&build)
        .map_err(|error| format!("{error:?}"))?;
    state.install_wheel_equipment(hands);
    assert!(
        state
            .player_mitigation_percent(0)
            .is_some_and(|rate| rate > 0.0)
    );
    Ok(())
}

#[test]
fn unknown_weapon_facts_defend_with_nothing_and_another_digest_refuses() -> TestResult {
    let content = native(&super::input()?)?;
    assert_eq!(
        equipped_defence(&held(&content, &[(FEET, 5)]), &content),
        Some(None)
    );
    let mut equipment = held(&content, &[(SHIELD, 6)]);
    equipment.content_digest = [9; 32];
    assert_eq!(equipped_defence(&equipment, &content), None);
    Ok(())
}
