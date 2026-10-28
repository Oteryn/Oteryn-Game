//! I11 parity: station baking and milling the production `cooking_conversion` admits, against
//! Crystal `201189f8` `data-global/scripts/actions/other/baking.lua` (ovens 2535/2537/2539/2541,
//! millstones 1943-1946, lines 1-73). The ids below are recorded source data.
#![allow(clippy::expect_used)]

use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use oteryn_gameplay_items::item_craft::cooking_conversion;

const OVENS: [u32; 4] = [2535, 2537, 2539, 2541];
const MILLSTONES: [u32; 4] = [1943, 1944, 1945, 1946];

fn native() -> crate::content::ContentActivationController {
    let world =
        crate::foundation::WorldId::decode(&[1, 1, 1, 1, 1, 1, 0x70, 1, 0x80, 1, 1, 1, 1, 1, 1, 1])
            .expect("world");
    activated_with_loot_tables(world, None).0
}

fn typed(state: &NativeGameplayState, id: u32) -> Option<TypedDefinitionRef> {
    let index = state.item_index()?;
    let key = format!("oteryn:item.tibia.i{id}");
    let reference = index.definition_ref("Item", &key, index.revision(&key)?)?;
    super::super::item_look::typed_definition(index, reference)
}

/// Item ids and counts the production path mints for `used` on `station`.
fn outputs(state: &NativeGameplayState, used: u32, station: u32) -> Option<Vec<(String, u32)>> {
    let conversion = cooking_conversion(state, &typed(state, used)?, &typed(state, station)?)?;
    Some(
        conversion
            .outputs
            .into_iter()
            .map(|(definition, count)| {
                let id = definition
                    .production_key
                    .trim_start_matches("oteryn:item.tibia.i")
                    .to_owned();
                (id, count)
            })
            .collect(),
    )
}

fn out(entries: &[(u32, u32)]) -> Option<Vec<(String, u32)>> {
    Some(entries.iter().map(|(id, n)| (id.to_string(), *n)).collect())
}

/// Crystal oven branch: cake and chocolate dough give id+1, garlic dough gives garlic bread,
/// lump of dough gives bread, the garlic cookie tray gives 12 garlic cookies and an empty tray.
/// Millstone branch: wheat gives flour, reed gives ground reed. All match on every station.
#[test]
fn i11_crystal_station_baking_and_milling_match() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    for oven in OVENS {
        assert_eq!(
            outputs(state, 3604, oven),
            out(&[(3600, 1)]),
            "bread {oven}"
        );
        assert_eq!(outputs(state, 6276, oven), out(&[(6277, 1)]), "cake {oven}");
        assert_eq!(
            outputs(state, 8018, oven),
            out(&[(8019, 1)]),
            "chocolate {oven}"
        );
        assert_eq!(
            outputs(state, 8196, oven),
            out(&[(8194, 1)]),
            "garlic {oven}"
        );
        assert_eq!(
            outputs(state, 8198, oven),
            out(&[(3464, 1), (8199, 12)]),
            "garlic cookies {oven}"
        );
    }
    for mill in MILLSTONES {
        assert_eq!(
            outputs(state, 3605, mill),
            out(&[(3603, 1)]),
            "flour {mill}"
        );
        assert_eq!(
            outputs(state, 30975, mill),
            out(&[(31686, 1)]),
            "reed {mill}"
        );
    }
}

/// Divergences pinned for the owner; none is changed here.
/// - MISSING: the cookie tray. Crystal bakes tray 8020 on an oven into 12 cookies 3598 and an
///   empty tray 3464; TibiaWiki "Baking Tray (Cookie)" agrees. Oteryn has no 8020 row. Crystal
///   fills the tray with lump of dough 3604, TibiaWiki with lump of cake dough 6276 (DIFF).
/// - Crystal's oven `else` branch also turns flour 3603, wheat 3605, holy water dough 8195 and
///   reed 30975 into bread; Oteryn bakes only lump of dough 3604 (Oteryn keeps the narrower rule).
/// - Carried-target rows (flour on a fluid container, cake dough on chocolate, holy water dough
///   on garlic, garlic dough on a tray, sugar cane on wheat) are authored in
///   `content/craft/recipes.json`, but the production station path does not resolve them.
#[test]
fn i11_crystal_divergences_are_pinned() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    for oven in OVENS {
        assert_eq!(outputs(state, 8020, oven), None, "cookie tray {oven}");
        for used in [3603, 3605, 8195, 30975] {
            assert_eq!(outputs(state, used, oven), None, "oven else {used} {oven}");
        }
    }
    for (used, carried) in [
        (3603, 2874),
        (6276, 6574),
        (8195, 8197),
        (8196, 3464),
        (5466, 3605),
    ] {
        assert_eq!(
            outputs(state, used, carried),
            None,
            "carried {used} on {carried}"
        );
    }
}
