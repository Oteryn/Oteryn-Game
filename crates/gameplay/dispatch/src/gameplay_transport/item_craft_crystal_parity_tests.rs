//! I11 parity: station baking and milling the production `cooking_conversion` admits, against
//! Crystal `201189f8` `data-global/scripts/actions/other/baking.lua` (ovens 2535/2537/2539/2541,
//! millstones 1943-1946, lines 1-73). The ids below are recorded source data.
#![allow(clippy::expect_used)]

use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use oteryn_gameplay_items::item_craft::{
    carried_cooking_conversion, carried_cooking_conversion_with_sub_type, cooking_conversion,
};

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

fn carried_outputs(
    state: &NativeGameplayState,
    used: u32,
    target: u32,
) -> Option<Vec<(String, u32)>> {
    let definition = |id| {
        typed(state, id).or_else(|| {
            Some(TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{id}"),
                revision_ref: "definition-r1".into(),
            })
        })
    };
    let conversion = carried_cooking_conversion(state, &definition(used)?, &definition(target)?)?;
    Some(
        conversion
            .outputs
            .into_iter()
            .map(|(definition, count)| {
                (
                    definition
                        .production_key
                        .trim_start_matches("oteryn:item.tibia.i")
                        .to_owned(),
                    count,
                )
            })
            .collect(),
    )
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

/// Cookie tray: Crystal `baking.lua` and TibiaWiki "Baking Tray (Cookie)" (revision 1008370:
/// "Can be used with an oven to get 12 Cookies (and an empty, reusable Baking Tray)") bake tray
/// 8020 on every oven into 12 cookies 3598 and the empty tray 3464.
#[test]
fn i11_cookie_tray_bakes_twelve_cookies_and_an_empty_tray() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    for oven in OVENS {
        assert_eq!(
            outputs(state, 8020, oven),
            out(&[(3464, 1), (3598, 12)]),
            "cookie tray {oven}"
        );
    }
    for mill in MILLSTONES {
        assert_eq!(outputs(state, 8020, mill), None, "tray on millstone {mill}");
    }
}

/// Crystal `data-global/scripts/actions/other/baking.lua:24-37,49-51`: every non-fluid
/// carried-ingredient conversion is admitted by the production resolver.
#[test]
fn i11_carried_ingredients_make_chocolate_garlic_trays_and_sugar_oat() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    assert_eq!(carried_outputs(state, 6276, 6574), out(&[(8018, 1)]));
    assert_eq!(carried_outputs(state, 8195, 8197), out(&[(8196, 1)]));
    assert_eq!(carried_outputs(state, 8196, 3464), out(&[(8198, 1)]));
    // The donor result exists, but the pinned Item definition marks sugar oat 12802
    // non-materializable, so the server must not mint it through the fallback path.
    assert_eq!(carried_outputs(state, 5466, 3605), None);
}

#[test]
fn i11_flour_and_carried_water_or_milk_make_the_donor_dough() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    let flour = typed(state, 3603).expect("flour");
    let vessel = typed(state, 2874).expect("vessel");
    let outputs = |sub_type| {
        carried_cooking_conversion_with_sub_type(state, &flour, &vessel, Some(sub_type)).map(
            |conversion| {
                conversion
                    .outputs
                    .into_iter()
                    .map(|(definition, count)| {
                        (
                            definition
                                .production_key
                                .trim_start_matches("oteryn:item.tibia.i")
                                .to_owned(),
                            count,
                        )
                    })
                    .collect()
            },
        )
    };
    assert_eq!(outputs(1), out(&[(2874, 1), (3604, 1)]));
    assert_eq!(outputs(9), out(&[(2874, 1), (6276, 1)]));
}

/// Divergences pinned for the owner.
/// - The cookie tray fill step. TibiaWiki (Lump of Cake Dough, revision 1013648) fills the empty
///   tray 3464 with lump of cake dough 6276 to get tray 8020; Crystal uses lump of dough 3604.
///   Oteryn follows Global (6276). The fill is a carried-target row (below), so it is MISSING.
/// - Crystal's oven `else` branch also turns flour 3603, wheat 3605, holy water dough 8195 and
///   reed 30975 into bread; Oteryn bakes only lump of dough 3604 (Oteryn keeps the narrower rule).
#[test]
fn i11_crystal_divergences_are_pinned() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    for oven in OVENS {
        for used in [3603, 3605, 8195, 30975] {
            assert_eq!(outputs(state, used, oven), None, "oven else {used} {oven}");
        }
    }
}

#[test]
fn i11_output_without_materializable_admission_is_not_minted() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .expect("native gameplay");
    let unadmitted = crate::content::ProjectV2DefinitionRef {
        family: crate::content::ProjectV2Family::Item,
        key: "oteryn:item.tibia.i12802".into(),
        revision: "definition-r1".into(),
    };
    assert!(oteryn_gameplay_items::item_craft::output_definition(state, &unadmitted).is_none());
}

#[test]
fn i11_carried_ingredient_path_keeps_death_and_spell_idle_guards() {
    assert!(!super::carried_ingredient_action_allowed(true, true));
    assert!(!super::carried_ingredient_action_allowed(false, false));
    assert!(super::carried_ingredient_action_allowed(false, true));
}

#[test]
fn m20_cursed_crystal_beeswax_requires_exact_world_oven() {
    let controller = native();
    let state = controller
        .active()
        .and_then(|a| a.native_gameplay())
        .expect("native");
    for oven in OVENS {
        assert_eq!(outputs(state, 5902, oven), out(&[(9106, 1)]));
        assert_eq!(carried_outputs(state, 5902, oven), None);
        assert_eq!(outputs(state, oven, 5902), None);
    }
    for station in MILLSTONES {
        assert_eq!(outputs(state, 5902, station), None);
    }
}
