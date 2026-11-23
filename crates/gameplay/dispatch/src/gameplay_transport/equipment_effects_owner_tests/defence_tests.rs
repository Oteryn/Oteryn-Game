//! SHIELD-DEFENCE-1: the hands' defence read from the pinned profiles.

use super::*;
use oteryn_gameplay_combat_rules::attack::formulas::DefenceSource;

const SWORD: &str = "oteryn:item.tibia.i3264";
const SHIELD: &str = "oteryn:item.tibia.i3412";
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
