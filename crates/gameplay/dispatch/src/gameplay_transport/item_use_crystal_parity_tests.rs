//! I14 parity: potion and food values the item-use path reads from the activated content, against
//! Crystal `201189f8` `data/scripts/actions/items/potions.lua` (table at lines 44-68) and
//! `foods.lua` (regeneration = value x 12 s), with the I14G owner-selected Global overrides.
//! The ids below are recorded source data.
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

/// Matching Global/Crystal rows. Wiki revisions: Strong Health 1183253, Strong Mana
/// 1183193, Great Health 1183256, Mana 1183192, Ultimate Health 1183257,
/// Small Health 1139185, Ultimate Mana 1183206; Egg 1113787 and Fish 1113662.
/// Crystal `201189f8` potions.lua:45-59 and foods.lua:2,8.
#[test]
fn i14_matching_restore_potions_and_foods_are_admitted() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    for (id, resource, min, max, flask, level, allowed) in [
        (
            236,
            ReferenceRestoreResource::Health,
            250,
            350,
            283,
            50,
            vec![Vocation::Knight, Vocation::Paladin, Vocation::Monk],
        ),
        (
            237,
            ReferenceRestoreResource::Mana,
            115,
            185,
            283,
            50,
            vec![
                Vocation::None,
                Vocation::Knight,
                Vocation::Paladin,
                Vocation::Monk,
                Vocation::Druid,
                Vocation::Sorcerer,
            ],
        ),
        (
            239,
            ReferenceRestoreResource::Health,
            425,
            575,
            284,
            80,
            vec![Vocation::Knight],
        ),
        (
            268,
            ReferenceRestoreResource::Mana,
            75,
            125,
            285,
            1,
            vec![
                Vocation::None,
                Vocation::Knight,
                Vocation::Paladin,
                Vocation::Monk,
                Vocation::Druid,
                Vocation::Sorcerer,
            ],
        ),
        (
            7643,
            ReferenceRestoreResource::Health,
            650,
            850,
            284,
            130,
            vec![Vocation::Knight],
        ),
        (
            7876,
            ReferenceRestoreResource::Health,
            60,
            90,
            285,
            1,
            vec![
                Vocation::None,
                Vocation::Knight,
                Vocation::Paladin,
                Vocation::Monk,
                Vocation::Druid,
                Vocation::Sorcerer,
            ],
        ),
        (
            23373,
            ReferenceRestoreResource::Mana,
            425,
            575,
            284,
            130,
            vec![Vocation::Druid, Vocation::Sorcerer],
        ),
    ] {
        let policy = policy_of(state, id).unwrap_or_else(|| panic!("item {id} is admitted"));
        let semantics = &policy.record().semantics;
        let ReferenceItemField::Known(ReferenceItemConsumption::Potion(potion)) =
            &semantics.consumption
        else {
            panic!("{id} is a potion")
        };
        assert_eq!(potion.restores.len(), 1, "{id}");
        assert_eq!(
            (
                potion.restores[0].resource,
                potion.restores[0].min,
                potion.restores[0].max
            ),
            (resource, min, max),
            "{id}"
        );
        let ReferenceItemField::Known(returned) = &potion.empty_flask else {
            panic!("{id} names a flask")
        };
        assert_eq!(returned.key, format!("oteryn:item.tibia.i{flask}"));
        assert!(policy_of(state, flask).is_some());
        for vocation in [
            Vocation::None,
            Vocation::Knight,
            Vocation::Paladin,
            Vocation::Monk,
            Vocation::Druid,
            Vocation::Sorcerer,
        ] {
            assert_eq!(
                requirements_met(&semantics.use_requirements, facts_at(vocation, level)),
                allowed.contains(&vocation),
                "{id} {vocation:?}"
            );
            if level > 1 {
                assert!(
                    !requirements_met(&semantics.use_requirements, facts_at(vocation, level - 1)),
                    "{id} below level"
                );
            }
        }
    }
    for (id, seconds) in [(3606, 72), (3578, 144)] {
        let policy = policy_of(state, id).expect("food is admitted");
        let ReferenceItemField::Known(ReferenceItemConsumption::Food(food)) =
            &policy.record().semantics.consumption
        else {
            panic!("{id} is food")
        };
        assert_eq!(food.regeneration_seconds, seconds);
    }
}

/// Owner decision I14G (2026-10-30): Global wins the five conflicts.
/// TibiaWiki revisions are recorded next to each exact range/duration.
#[test]
fn i14_global_consumables_are_admitted() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    use ReferenceRestoreResource::{Health, Mana};
    for (id, restores, level, allowed) in [
        // Great Spirit Potion rev 1183259.
        (
            7642,
            vec![(Health, 250, 350), (Mana, 125, 185)],
            80,
            vec![Vocation::Paladin, Vocation::Monk],
        ),
        // Ultimate Spirit Potion rev 1183261.
        (
            23374,
            vec![(Health, 420, 580), (Mana, 150, 250)],
            130,
            vec![Vocation::Paladin, Vocation::Monk],
        ),
        // Supreme Health Potion rev 1183258.
        (
            23375,
            vec![(Health, 875, 1175)],
            200,
            vec![Vocation::Knight],
        ),
    ] {
        let policy = policy_of(state, id).unwrap_or_else(|| panic!("item {id} is admitted"));
        let semantics = &policy.record().semantics;
        let ReferenceItemField::Known(ReferenceItemConsumption::Potion(potion)) =
            &semantics.consumption
        else {
            panic!("{id} is a potion")
        };
        assert_eq!(
            potion
                .restores
                .iter()
                .map(|r| (r.resource, r.min, r.max))
                .collect::<Vec<_>>(),
            restores,
            "{id}"
        );
        let ReferenceItemField::Known(flask) = &potion.empty_flask else {
            panic!("{id} names its flask")
        };
        assert_eq!(flask.key, "oteryn:item.tibia.i284");
        assert!(policy_of(state, 284).is_some());
        for vocation in [
            Vocation::None,
            Vocation::Knight,
            Vocation::Paladin,
            Vocation::Monk,
            Vocation::Druid,
            Vocation::Sorcerer,
        ] {
            assert_eq!(
                requirements_met(&semantics.use_requirements, facts_at(vocation, level)),
                allowed.contains(&vocation),
                "{id} {vocation:?}"
            );
            assert!(
                !requirements_met(&semantics.use_requirements, facts_at(vocation, level - 1)),
                "{id} below level"
            );
        }
    }
    for (id, seconds) in [
        (3250, 96),   // The Carrot of Doom rev 1016926.
        (23535, 300), // Energy Bar rev 1011115.
    ] {
        let policy = policy_of(state, id).expect("food is admitted");
        let ReferenceItemField::Known(ReferenceItemConsumption::Food(food)) =
            &policy.record().semantics.consumption
        else {
            panic!("{id} is food")
        };
        assert_eq!(food.regeneration_seconds, seconds, "{id}");
    }
}

/// Great Mana consumption still needs its canonical correction stage. E12 admits its
/// physical Trader item (Crystal hireling.lua:273,1720; items.xml:410), without granting USE.
/// The separate condition potion 7644 remains outside the native item closure.
#[test]
fn i14_unresolved_crystal_consumables_remain_unadmitted() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    let policy = policy_of(state, 238).expect("the source Trader sells physical item 238");
    assert!(policy.record().admission.materializable);
    let item =
        record(state, &production_ref(policy.record())).expect("the exact native USE lookup");
    // These are the two action branches consulted before any item-use burn. Unknown
    // consumption cannot select a Food/Potion effect, and Unknown fluid cannot drink.
    assert!(matches!(
        item.semantics.consumption,
        ReferenceItemField::Unknown
    ));
    assert!(matches!(item.semantics.fluid, ReferenceItemField::Unknown));
    assert!(policy_of(state, 7644).is_none(), "item 7644");
}

/// I03: Crystal `data/scripts/actions/items/spellbook.lua:25` registers these spellbook ids; using
/// one opens a text dialog listing the user's vocation instant spells with a level or magic-level
/// requirement, grouped and sorted by level. Oteryn exposes the same server-authored content on
/// the existing text-bearing LOOK result for the admitted source spellbook.
#[test]
fn i03_crystal_spellbook_has_vocation_filtered_spell_list() -> Result<(), &'static str> {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    let mut admitted = Vec::new();
    for id in [
        3059, 6120, 8072, 8073, 8074, 8075, 8076, 8090, 11691, 14769, 16107, 20088, 21400, 22755,
        25699, 29431, 20089, 20090, 34153,
    ] {
        if let Some(policy) = policy_of(state, id) {
            assert!(
                !matches!(
                    policy.record().semantics.consumption,
                    ReferenceItemField::Known(_)
                ),
                "spellbook {id} carries no consumption the item-use path acts on"
            );
            admitted.push(id);
        }
    }
    // E12 adds exactly the five source Trader spellbooks: Crystal hireling.lua:667-670
    // and 501,1776. The basic spellbook 3059 and the remaining source ids stay absent.
    assert_eq!(admitted, vec![6120, 8072, 8073, 8074, 8075, 25699]);
    let text = crate::gameplay_transport::item_look::spellbook_text(
        state.spell_book(),
        crate::spell::Vocation::Knight,
    )
    .ok_or("bounded spellbook text")?;
    assert!(text.starts_with("Spells for Level "));
    assert!(text.contains(" - "));
    Ok(())
}
