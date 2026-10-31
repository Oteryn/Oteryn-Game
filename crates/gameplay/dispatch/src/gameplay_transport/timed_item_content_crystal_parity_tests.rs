//! I05 parity: timed equipment budgets and paired forms the timed-item path reads, against
//! Crystal `201189f8` `data/items/items.xml` (`duration` seconds, `charges`, `transformEquipTo`,
//! `transformDeEquipTo`, `decayTo`). Canary `47dfd51f` carries the same values for these ids.
#![allow(clippy::expect_used, clippy::panic)]

use super::*;
use crate::content::native_gameplay::test_support::activated_with_loot_tables;

fn native() -> crate::content::ContentActivationController {
    let world =
        crate::foundation::WorldId::decode(&[1, 1, 1, 1, 1, 1, 0x70, 1, 0x80, 1, 1, 1, 1, 1, 1, 1])
            .expect("world");
    activated_with_loot_tables(world, None).0
}

fn typed_of(native: &NativeGameplayState, id: u32) -> Option<TypedDefinitionRef> {
    let key = format!("oteryn:item.tibia.i{id}");
    let revision = native.item_index()?.revision(&key)?;
    Some(TypedDefinitionRef {
        family: "Item".into(),
        production_key: key,
        revision_ref: revision.into(),
    })
}

fn key(id: u32) -> String {
    format!("oteryn:item.tibia.i{id}")
}

fn native_state(controller: &crate::content::ContentActivationController) -> &NativeGameplayState {
    controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay")
}

/// Energy ring 3051 -> 3088 (600 s, no decay item) and soft boots 6529 -> 3549 (14,400 s, decays
/// to worn soft boots 6530) are admitted with Crystal's paired form, budget and decay target.
#[test]
fn i05_crystal_timed_equipment_budgets_match() {
    let controller = native();
    let native = native_state(&controller);
    for (base, active, seconds, decay) in
        [(3051, 3088, 600, None), (6529, 3549, 14_400, Some(6530))]
    {
        let definition = typed_of(native, base).expect("admitted");
        let equipped = equipped_definition(native, &definition).expect("equipped form");
        assert_eq!(equipped.production_key, key(active), "{base}");
        let (facts, target) = facts(native, &definition).expect("timed facts");
        assert_eq!(facts.full.charges(), None, "{base}");
        assert_eq!(facts.full.remaining_ms(), Some(seconds * 1_000), "{base}");
        assert_eq!(
            target.map(|target| target.definition.production_key),
            decay.map(key),
            "{base}"
        );
    }
}

/// Stone skin amulet 3081: Crystal `charges` 5, no duration, removed when spent.
#[test]
fn i05_crystal_charged_amulet_matches() {
    let controller = native();
    let native = native_state(&controller);
    let definition = typed_of(native, 3081).expect("admitted");
    let (facts, target) = facts(native, &definition).expect("timed facts");
    assert_eq!(facts.full.charges(), Some(5));
    assert_eq!(facts.full.remaining_ms(), None);
    assert!(target.is_none());
}

/// Time ring 3053 -> 3090 (600 s) and dwarven ring 3097 -> 3099 (3,600 s, Crystal
/// `items.xml:7458-7489`, decayTo 0) are admitted with their active-time budgets and no decay
/// item. Stealth, life and healing rings are pinned below until their invisibility and
/// regeneration effects are production-ready.
#[test]
fn i05_time_ring_is_admitted_and_remaining_rings_are_pinned() {
    let controller = native();
    let native = native_state(&controller);
    for (base, active, seconds) in [(3053, 3090, 600), (3097, 3099, 3_600)] {
        let definition = typed_of(native, base).expect("ring admitted");
        assert_eq!(
            equipped_definition(native, &definition)
                .expect("active form")
                .production_key,
            key(active),
            "{base}"
        );
        let (ring_facts, target) = facts(native, &definition).expect("timed facts");
        assert_eq!(
            ring_facts.full.remaining_ms(),
            Some(seconds * 1_000),
            "{base}"
        );
        assert_eq!(ring_facts.full.charges(), None, "{base}");
        assert!(target.is_none(), "{base}");
    }

    for base in [3049, 3052, 3098] {
        let definition = typed_of(native, base);
        assert!(
            definition
                .as_ref()
                .and_then(|definition| equipped_definition(native, definition))
                .is_none(),
            "{base}"
        );
        assert!(
            definition
                .as_ref()
                .and_then(|definition| facts(native, definition))
                .is_none(),
            "{base}"
        );
    }
}
