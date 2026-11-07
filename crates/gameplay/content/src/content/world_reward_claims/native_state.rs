//! Exact source choices for fluid contents and explicit one-charge rewards.
use super::*;

type StateChoice = (
    &'static [u64],
    &'static [(usize, u16, u8)],
    &'static [(usize, u16)],
    Option<u16>,
);

pub(super) fn reconciled(definition: &serde_json::Map<String, Value>) -> bool {
    let key = definition
        .get("identity")
        .and_then(|v| v.get("key"))
        .and_then(Value::as_str);
    let Some(native) = definition.get("native_contents") else {
        return false;
    };
    let difference = native.get("source_difference").and_then(Value::as_str);
    // (UIDs, exact per-child fluid/charge values, exact root charge).
    let (uids, fluids, charges, root_charge): StateChoice = match (key, difference) {
        (Some("oteryn:reward-claim.kv.goblintemple2"), Some("WIKI_MILK_OVERRIDES_DONOR_WATER")) => {
            (&[6119], &[(2, 2874, 9)], &[], None)
        }
        (
            Some("oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests_wine"),
            Some("WIKI_WINE_OVERRIDES_DONOR_WATER"),
        ) => (&[6197, 6198, 6199, 6200], &[(1, 2874, 2)], &[], None),
        (Some("oteryn:reward-claim.kv.present_box"), Some("CRYSTAL_PRESENT_WATER_SUBTYPES")) => {
            (&[6113], &[(0, 2882, 1), (2, 2881, 1)], &[], None)
        }
        (
            Some("oteryn:reward-claim.kv.deeperfibula4"),
            Some("EXACT_SOURCE_CHARGES_NOT_DEFINITION_DEFAULTS"),
        ) => (&[6011], &[], &[], Some(3082)),
        (
            Some(
                "oteryn:reward-claim.quest.u7_2.the_queen_of_the_banshees.reward.stone_skin_amulet",
            ),
            Some("EXACT_SOURCE_CHARGES_NOT_DEFINITION_DEFAULTS"),
        ) => (&[6056], &[], &[], Some(3081)),
        (
            Some("oteryn:reward-claim.quest.u15_24.targuna.silver_amulet_chest"),
            Some("EXACT_SOURCE_CHARGES_NOT_DEFINITION_DEFAULTS"),
        ) => (&[14093], &[], &[], Some(3054)),
        (
            Some("oteryn:reward-claim.quest.pre_u6_0.ornamented_shield.rewards.ornamented_shield"),
            Some("EXACT_SOURCE_CHARGES_NOT_DEFINITION_DEFAULTS"),
        ) => (&[6062], &[], &[(6, 3085), (7, 3048)], None),
        (
            Some("oteryn:reward-claim.kv.ornamentedshield"),
            Some("EXACT_SOURCE_CHARGES_NOT_DEFINITION_DEFAULTS"),
        ) => (&[6063], &[], &[(2, 3083)], None),
        _ => return false,
    };
    let Some(placements) = definition.get("placements").and_then(Value::as_array) else {
        return false;
    };
    let Some(recipes) = native.get("placements").and_then(Value::as_array) else {
        return false;
    };
    if placements.len() != uids.len() || recipes.len() != uids.len() {
        return false;
    }
    if !fluids.is_empty()
        && definition
            .get("source_variant")
            .and_then(|v| v.get("source_data"))
            != Some(&serde_json::json!({"status":"COMPLETE","missing":[]}))
    {
        return false;
    }
    let mut expected_holds = Vec::new();
    for (index, ((source, recipe), uid)) in placements.iter().zip(recipes).zip(uids).enumerate() {
        if recipe.get("unique_id").and_then(Value::as_u64) != Some(*uid)
            || !source["source_binding"]["legacy_unique_ids"]
                .as_array()
                .is_some_and(|ids| {
                    ids.iter()
                        .any(|v| v["server"] == "crystalserver" && v["unique_id"] == *uid)
                })
        {
            return false;
        }
        let Some(children) = recipe.get("contents").and_then(Value::as_array) else {
            return false;
        };
        if recipe.get("root_fluid_subtype").is_some() {
            return false;
        }
        if let Some(number) = root_charge {
            if recipe["root"]["key"] != format!("oteryn:item.tibia.i{number}")
                || recipe.get("root_charges_remaining").and_then(Value::as_u64) != Some(1)
            {
                return false;
            }
        } else if recipe.get("root_charges_remaining").is_some() {
            return false;
        }
        for (ordinal, child) in children.iter().enumerate() {
            let fluid = fluids.iter().find(|(i, _, _)| *i == ordinal);
            let charge = charges.iter().find(|(i, _)| *i == ordinal);
            match fluid {
                Some((_, number, subtype))
                    if child["item"]["key"] == format!("oteryn:item.tibia.i{number}")
                        && child["fluid_subtype"].as_u64() == Some(u64::from(*subtype)) => {}
                None if child.get("fluid_subtype").is_none() => {}
                _ => return false,
            }
            match charge {
                Some((_, number))
                    if child["item"]["key"] == format!("oteryn:item.tibia.i{number}")
                        && child["charges_remaining"].as_u64() == Some(1) => {}
                None if child.get("charges_remaining").is_none() => {}
                _ => return false,
            }
        }
        for (ordinal, number, _) in fluids {
            if children.get(*ordinal).is_none() {
                return false;
            }
            expected_holds.push(serde_json::json!({"category":"item","code":"ITEM_SEMANTICS_MISSING","item":format!("oteryn:item.tibia.i{number}"),"placement_index":index,"reason":"waiting","reward_field":"items","reward_index":ordinal}));
        }
        for (ordinal, _) in charges {
            if children.get(*ordinal).is_none() {
                return false;
            }
            expected_holds.push(serde_json::json!({"category":"source","code":"SOURCE_CHARGE_MISMATCH","placement_index":index,"reward_field":"items","reward_index":ordinal}));
        }
    }
    match definition.get("data_holds") {
        None => root_charge.is_some(),
        Some(Value::Array(holds)) => holds.is_empty() || holds == &expected_holds,
        _ => false,
    }
}
