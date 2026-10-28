//! I14 parity: potion and food values the item-use path reads from the activated content, against
//! Crystal `201189f8` `data/scripts/actions/items/potions.lua` (table at lines 44-68) and
//! `foods.lua` (regeneration = value x 12 s). The ids below are recorded source data.
#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use crate::content::ReferenceRestoreResource;
use crate::content::native_gameplay::test_support::activated_with_loot_tables;

fn native() -> crate::content::ContentActivationController {
    let world =
        crate::foundation::WorldId::decode(&[1, 1, 1, 1, 1, 1, 0x70, 1, 0x80, 1, 1, 1, 1, 1, 1, 1])
            .expect("world");
    activated_with_loot_tables(world, None).0
}

/// The policy the item-use path (`record`) resolves for a held `oteryn:item.tibia.i<id>`.
fn policy_of(
    state: &NativeGameplayState,
    id: u32,
) -> Option<crate::content::native_gameplay::QualifiedItemPolicy<'_>> {
    let key = format!("oteryn:item.tibia.i{id}");
    let revision = state.item_index()?.revision(&key)?;
    state.item_policy(&key, revision)
}

fn facts_at(vocation: Vocation, level: u32) -> CharacterCastFacts {
    CharacterCastFacts {
        vocation,
        level,
        magic_level: 0,
        max_health: 150,
        max_mana: 50,
        max_soul: 100,
    }
}

/// Health potion 266 (Crystal: health 125-175, no level or vocation gate, flask 285) and meat
/// 3577 (Crystal value 15 x 12 = 180 s) are the admitted consumables; both match.
#[test]
fn i14_admitted_consumables_match_crystal() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    let potion = policy_of(state, 266).expect("health potion is admitted");
    let semantics = &potion.record().semantics;
    let ReferenceItemField::Known(ReferenceItemConsumption::Potion(potion)) =
        &semantics.consumption
    else {
        panic!("266 is a potion");
    };
    let restores: Vec<_> = potion
        .restores
        .iter()
        .map(|restore| (restore.resource, restore.min, restore.max))
        .collect();
    assert_eq!(restores, vec![(ReferenceRestoreResource::Health, 125, 175)]);
    let ReferenceItemField::Known(flask) = &potion.empty_flask else {
        panic!("266 names its flask");
    };
    assert_eq!(flask.key, "oteryn:item.tibia.i285");
    assert!(policy_of(state, 285).is_some(), "the flask is admitted");
    for vocation in [
        Vocation::None,
        Vocation::Druid,
        Vocation::Sorcerer,
        Vocation::Knight,
        Vocation::Paladin,
        Vocation::Monk,
    ] {
        assert!(requirements_met(
            &semantics.use_requirements,
            facts_at(vocation, 1)
        ));
    }
    let meat = policy_of(state, 3577).expect("meat is admitted");
    let ReferenceItemField::Known(ReferenceItemConsumption::Food(food)) =
        &meat.record().semantics.consumption
    else {
        panic!("3577 is food");
    };
    assert_eq!(food.regeneration_seconds, 180);
}

/// Crystal consumables the active generation does not admit: the item-use path resolves no
/// record for them, so use answers `NothingToUse`. Pinned until their profiles are admitted.
#[test]
fn i14_unadmitted_crystal_consumables_are_pinned() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    for id in [
        // Potions: strong/great/ultimate health and mana, spirit, small health, antidote,
        // supreme health, ultimate mana and ultimate spirit.
        236, 237, 238, 239, 268, 7642, 7643, 7644, 7876, 23373, 23374, 23375,
        // Food: egg, fish, carrot, energy bar.
        3606, 3578, 3250, 23535,
    ] {
        assert!(policy_of(state, id).is_none(), "item {id}");
    }
}
