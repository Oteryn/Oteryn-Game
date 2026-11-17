//! Closed map/tool metadata for the source paint action; registration is separate.
use super::{MISSION04, PAINT, QUEST, QUESTLINE, TARGETS, targets};
use crate::quest::world_callbacks::Callback;

/// Item 7178 becomes 7252 for 30s. Source utils_definitions.hpp:64 pins green rings 9.
pub fn definition(seal: u16) -> Result<Callback, String> {
    let target = targets::target(seal).ok_or("unknown native seal")?;
    let (x, y, floor) = target.position;
    serde_json::from_value(serde_json::json!({
        "key":format!("oteryn:quest-callback/ice-islands/paint-seal-{seal}"),
        "quest":QUEST,"trigger":"use",
        "selector":{"native":{"position":{"x":x,"y":y,"floor":floor},"ordinal":target.ordinal,"appearance":7178}},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":[
            {"track":QUESTLINE,"minimum":8,"maximum":8,"set":0,"preserve":true},
            {"track":TARGETS,"minimum":0,"maximum":(1_i64<<48)-1,"set":0,"preserve":true},
            {"track":MISSION04,"minimum":-1,"maximum":2,"set":0,"preserve":true}
        ],
        "native_rule":{"kind":"ice_seal_paint","seal":seal},
        "item_on_target":{"item":PAINT,"count":1,"replacement":null,"target_replacement":null,"reusable":true},
        "effects":{"transform":7252,"reset_after_ms":30000,"magic":{"position":{"x":x,"y":y,"floor":floor},"effect":9}},
        "offerings":[],"return_on_failure":false,"failure_message":null,
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ice_islands_quest/actions_paint.lua:15-28"
        ]
    })).map_err(|error| error.to_string())
}

pub(crate) fn admitted(candidate: &Callback, seal: u16) -> bool {
    definition(seal).is_ok_and(|expected| *candidate == expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::world_callbacks;

    #[test]
    fn every_checked_seal_has_one_closed_tool_transform_and_reset() -> Result<(), String> {
        let definitions = (0..targets::TARGET_COUNT)
            .map(definition)
            .collect::<Result<Vec<_>, _>>()?;
        world_callbacks::validate_callbacks(&definitions)?;
        assert_eq!(definitions.len(), 198);
        assert!(definition(targets::TARGET_COUNT).is_err());
        for callback in definitions {
            assert_eq!(callback.effects.transform, Some(7252));
            assert_eq!(callback.effects.reset_after_ms, Some(30000));
            assert_eq!(
                callback.effects.magic.as_ref().map(|magic| magic.effect),
                Some(9)
            );
        }
        Ok(())
    }

    #[test]
    fn source_binding_cannot_be_renamed_or_turned_into_a_different_reward() -> Result<(), String> {
        let original = serde_json::to_value(definition(0)?).map_err(|error| error.to_string())?;
        for case in 0..17 {
            let mut bad = original.clone();
            match case {
                0 => bad["key"] = "oteryn:quest-callback/ice-islands/renamed".into(),
                1 => bad["selector"]["native"]["position"]["floor"] = (-8).into(),
                2 => bad["selector"]["native"]["ordinal"] = 1.into(),
                3 => bad["selector"]["native"]["appearance"] = 7252.into(),
                4 => bad["native_rule"]["seal"] = 1.into(),
                5 => bad["item_on_target"] = serde_json::Value::Null,
                6 => bad["item_on_target"]["reusable"] = false.into(),
                7 => bad["item_on_target"]["count"] = 2.into(),
                8 => bad["item_on_target"]["item"] = "oteryn:item.tibia.i7254".into(),
                9 => bad["item_on_target"]["replacement"] = PAINT.into(),
                10 => bad["effects"]["transform"] = 7178.into(),
                11 => bad["effects"]["reset_after_ms"] = 30001.into(),
                12 => bad["effects"]["magic"]["effect"] = 18.into(),
                13 => bad["storage"][0]["minimum"] = 7.into(),
                14 => bad["sources"] = serde_json::json!([]),
                15 => bad["experience"] = 100.into(),
                _ => {
                    bad["selector"]["position"] =
                        serde_json::json!({"x":31964,"y":31203,"floor":-7})
                }
            }
            let document = serde_json::json!({"schema":1,"callbacks":[bad]}).to_string();
            assert!(
                world_callbacks::parse(&document).is_err(),
                "mutation {case}"
            );
        }
        Ok(())
    }
}
