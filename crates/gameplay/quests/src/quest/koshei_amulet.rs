//! Crystal 201189f8, action_boss.lua:4-8; storages.lua:934 (RewardDoor=41351).
use super::world_callbacks::Callback;

pub const KEY: &str = "oteryn:quest-callback/koshei-amulet";
pub const SUNDIAL: &str = "oteryn:quest-callback/koshei-sundial";
pub const TOMB_ENTRY: &str = "oteryn:quest-callback/koshei/tomb-entry";
pub const TOMB_ENTERED: &str = "oteryn:quest-progress/koshei/tomb-entered";
pub const TOMB_SEAL: &str = "oteryn:quest-reward/koshei/tomb-seal";
pub const TRACK: &str = "oteryn:quest-progress/quest/u8_1/koshei_the_deathless/reward_door";

pub fn callback() -> Result<Callback, String> {
    super::world_callbacks::parse(&serde_json::json!({"schema":1,"callbacks":[{
        "key":KEY,"quest":"oteryn:quest.koshei_the_deathless_quest","trigger":"use",
        "selector":{"target_definition":"oteryn:item.tibia.i7535"},
        "lever":null,"reversible":false,"requires_world_admission":true,
        "passages":[],"teleport":null,
        "storage":[{"track":TRACK,"minimum":-1,"maximum":0,"set":1}],
        "effects":{"target_magic":14},
        "offerings":[],"item_on_target":{"selected_instance":true,"item":"oteryn:item.tibia.i7532","count":1},
        "return_on_failure":false,"failure_message":null,
        "success_message":"At last... I have it back, my precious amulet. I am glad you didn't use it! I allow you to ...ahh... enter door ...ahh",
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/koshei_the_deathless_quest/action_boss.lua:4-8",
                   "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/core/storages.lua:934"]
    }]}).to_string())?.pop().ok_or_else(|| "missing Koshei callback".into())
}

/// The retained catacombs have the closed coffin and five unturned statues.
pub(super) fn catacomb_access() -> Result<Vec<Callback>, String> {
    let donor = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/koshei_the_deathless_quest/action_switch.lua:1-40";
    let statues = [
        (33276, 32444, 2062),
        (33278, 32450, 2062),
        (33281, 32442, 2063),
        (33284, 32450, 2062),
        (33286, 32444, 2063),
    ];
    let mut callbacks = Vec::new();
    for (x, y, _) in statues {
        callbacks.push(serde_json::json!({
            "key": format!("oteryn:quest-callback/koshei-statue-{x}-{y}"),
            "quest": "oteryn:quest.koshei_the_deathless_quest",
            "trigger": "use",
            "selector": {"native": {
                "position": {"x": x, "y": y, "floor": -8}, "ordinal": 1, "appearance": 2063
            }},
            "lever": [2063, 2062],
            "reversible": true,
            "passages": [], "teleport": null, "storage": [], "offerings": [],
            "return_on_failure": false, "failure_message": null, "success_message": null,
            "sources": [donor]
        }));
    }
    let coffin = serde_json::json!({"x": 33273, "y": 32458, "floor": -8});
    let mut world_requirements: Vec<_> = statues
        .map(|(x, y, appearance)| {
            serde_json::json!({
                "position": {"x": x, "y": y, "floor": -8}, "appearance": appearance
            })
        })
        .into();
    world_requirements.push(serde_json::json!({"position": coffin, "appearance": 162}));
    callbacks.push(serde_json::json!({
        "key": SUNDIAL,
        "quest": "oteryn:quest.koshei_the_deathless_quest",
        "trigger": "use",
        "selector": {"native": {
            "position": {"x": 33281, "y": 32447, "floor": -8}, "ordinal": 1, "appearance": 7534
        }},
        "lever": null, "reversible": false,
        "passages": [{"position": coffin, "appearance": 162, "replacement": 167}],
        "effects": {"passages_reset_after_ms": 120000},
        "world_requirements": world_requirements,
        "teleport": null, "storage": [], "offerings": [],
        "return_on_failure": false, "failure_message": "Nothing happens", "success_message": "CLICK",
        "sources": [donor]
    }));
    super::world_callbacks::parse(
        &serde_json::json!({
            "schema": 1, "callbacks": callbacks
        })
        .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn koshei_sundial_requires_all_five_statues_and_opens_coffin_for_two_minutes()
    -> Result<(), String> {
        let callbacks = catacomb_access()?;
        let (sundial, statues) = callbacks.split_last().ok_or("Koshei sundial")?;
        assert_eq!(statues.len(), 5);
        assert_eq!(sundial.effects.passages_reset_after_ms, Some(120_000));
        assert_eq!(sundial.world_requirements.len(), 6);
        let [coffin] = sundial.passages.as_slice() else {
            return Err("one Koshei coffin".into());
        };
        assert_eq!((coffin.appearance, coffin.replacement), (162, Some(167)));
        for (statue, requirement) in statues.iter().zip(&sundial.world_requirements) {
            let binding = statue.selector.native.as_ref().ok_or("native statue")?;
            assert_eq!(binding.position, requirement.position);
            assert_eq!(statue.lever, Some([2063, 2062]));
            assert!(statue.reversible);
        }
        assert!(callbacks.iter().all(|callback| !callback.has_state()));
        Ok(())
    }
}
