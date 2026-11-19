//! Crystal 9f5a72c, action_boss.lua:4-8; storages.lua:934 (RewardDoor=41351).
use super::world_callbacks::Callback;

pub const KEY: &str = "oteryn:quest-callback/koshei-amulet";
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
        "sources":["crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/koshei_the_deathless_quest/action_boss.lua:4-8",
                   "crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/lib/core/storages.lua:934"]
    }]}).to_string())?.pop().ok_or_else(|| "missing Koshei callback".into())
}
