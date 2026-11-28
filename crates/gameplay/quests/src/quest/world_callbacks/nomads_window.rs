//! Chosen Nomads Land s4: the two-stack arrangement proves personal progress.
//! TibiaWiki BR Arito's Task Quest:115-117 supplies the two-crate/amphora variant.
//! Crystal 201189f data/items/items.xml:5020-5024,6245-6247 identifies both Items.
use super::{Callback, GroundStackItem, Position, WorldRequirement};

pub(super) const KEY: &str = "oteryn:quest-callback/m02/nomads-land/window";
const QUEST: &str = "oteryn:quest.authored.nomads_land_quest";
const PROOF: &str = "oteryn:quest-progress/authored/nomads_land_quest/s4_window_obscured";
const WINDOW: Position = Position {
    x: 33216,
    y: 32855,
    floor: -9,
};

pub(super) fn requirements() -> Vec<WorldRequirement> {
    [WINDOW.x - 1, WINDOW.x]
        .into_iter()
        .map(|x| WorldRequirement {
            ground: None,
            position: Position {
                x,
                y: WINDOW.y - 1,
                floor: WINDOW.floor,
            },
            appearance: 0,
            ground_stack: Some(
                [
                    "oteryn:item.tibia.i2471",
                    "oteryn:item.tibia.i2471",
                    "oteryn:item.tibia.i2904",
                ]
                .into_iter()
                .map(|item| GroundStackItem {
                    item: item.into(),
                    count: 1,
                })
                .collect(),
            ),
        })
        .collect()
}

pub(super) fn callback() -> Result<Callback, String> {
    // Chosen Oteryn placement, not a claim about the original spoiler's coordinates:
    // the committed z09/x129/y128 region has this sandstone window at ordinal 1,
    // and the two cells immediately north/north-west are clear sandstone floor.
    serde_json::from_value(serde_json::json!({
        "key":KEY, "quest":QUEST, "trigger":"use",
        "selector":{"position":WINDOW,"appearance":1744,"unique":null,"action":null,
                    "native":{"position":WINDOW,"appearance":1744,"ordinal":1}},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":[{"track":PROOF,"minimum":-1,"maximum":0,"set":1}],
        "requires_world_admission":true,
        "offerings":[],"item_on_target":null,"world_requirements":requirements(),
        "return_on_failure":false,
        "failure_message":"The crates and amphorae do not block this window.",
        "success_message":"The arranged crates and amphorae block this window.",
        "sources":[
            "OTERYN_CHOSEN_RECIPE:content/quests/definitions/quests-00300-00351.json:nomads_land_quest/s4",
            "https://www.tibiawiki.com.br/Arito%27s_Task_Quest:115-117",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/items/items.xml:5020-5024,6245-6247",
            "OTERYN_CHOSEN_NATIVE_WINDOW:content/world/placements/region-z09-x129-y128.b3:33216,32855,9/ordinal1"
        ]
    })).map_err(|error|error.to_string())
}

pub(super) fn exact(callback_: &Callback) -> bool {
    callback().is_ok_and(|canonical| callback_ == &canonical)
}

/// Both strictly admitted arrangements earn the same window completion proof.
pub fn alternate_completion_receipt(
    quest: &str,
    transition: &str,
    track: &str,
    value: i64,
) -> Option<&'static str> {
    (quest == QUEST
        && transition == "oteryn:quest-callback/engenc2/nomads-land-window"
        && track == PROOF
        && value == 1)
        .then_some(KEY)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m02_nomads_window_records_the_shared_proof_and_preserves_its_source_readset()
    -> Result<(), String> {
        let callbacks = super::super::load()?;
        let window = callbacks
            .iter()
            .find(|c| c.key == KEY)
            .ok_or("registered Nomads window")?;
        assert_eq!(window, &callback()?);
        assert!(window.has_personal_state());
        assert!(window.requires_world_admission);
        let [proof] = window.storage.as_slice() else {
            return Err("one window completion track".into());
        };
        assert_eq!(
            proof.track,
            "oteryn:quest-progress/authored/nomads_land_quest/s4_window_obscured"
        );
        assert_eq!((proof.minimum, proof.maximum, proof.set), (-1, 0, 1));
        assert_eq!(window.world_requirements.len(), 2);
        for case in 0..5 {
            let mut bad = window.clone();
            match case {
                0 => bad.world_requirements[0].ground_stack.as_mut().ok_or("stack")?.swap(0, 2),
                1 => bad.world_requirements[1].position.x += 1,
                2 => bad.selector.appearance = Some(1745),
                3 => bad.world_requirements[0].ground_stack.as_mut().ok_or("stack")?[0].count = 2,
                _ => bad.storage = serde_json::from_value(serde_json::json!([{
                    "track":"oteryn:quest-progress/nomads-land/fabricated", "minimum":-1,"maximum":1,"set":1
                }])).map_err(|error|error.to_string())?,
            }
            assert!(
                super::super::parse(&serde_json::json!({"schema":1,"callbacks":[bad]}).to_string())
                    .is_err(),
                "accepted altered window rule {case}"
            );
        }
        Ok(())
    }
}
