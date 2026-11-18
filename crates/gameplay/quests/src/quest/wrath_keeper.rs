//! Crystal 201189f8, actions_mission03_the_keeper.lua:23-28.
use super::reward_callbacks::RewardCallback;
use super::wrath_sleeping_dragon::{QUEST, QUESTLINE};

pub const TAIL_HANDOFF: &str = "oteryn:quest-reward/wrath-of-the-emperor/keeper-tail";
pub const CORPSE: &str = "oteryn:item.tibia.i11360";
pub const CREATURE: &str = "oteryn:creature.the_keeper";
pub const MISSION: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission03";

pub(crate) fn tail_callback() -> Result<RewardCallback, String> {
    serde_json::from_value(serde_json::json!({
        "key": TAIL_HANDOFF,
        "quest": QUEST,
        "npc": "oteryn:npc.zalamon",
        "keywords": [],
        "topic": 0,
        "next_topic": 0,
        "storage": [
            {"track": QUESTLINE, "minimum": 7, "maximum": 7, "set": 8},
            {"track": MISSION, "minimum": -1, "maximum": 2, "set": 2}
        ],
        "creature_target": CREATURE,
        "fresh_corpse": true,
        "take": [],
        "give": {"item": "oteryn:item.tibia.i11367", "count": 1},
        "outfit": null,
        "achievement": null,
        "reply": [],
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission03_the_keeper.lua:23-28"]
    }))
    .map_err(|error| error.to_string())
}
