//! Crystal 201189f, actions_reliable_ram.lua:20-50 and emiliana.lua:169-182.
//! Emiliana atomically sets earned turnips=1 and mission=4. This callback admits
//! that real source state; after this receipt mission=5 and the mount is owned.
use super::world_callbacks::Callback;

pub const KEY: &str = "oteryn:quest-callback/srv7-reliable-ram";
pub const QUEST: &str = "oteryn:quest.targuna_quest";
pub const TURNIPS: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/burning_heart/turnips_reward";
pub const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/burning_heart/mission";
pub const RAM: &str = "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/reliable_ram";

pub fn callback() -> Result<Callback, String> {
    let value = serde_json::json!({"schema":1,"callbacks":[{
        "key":KEY,"quest":QUEST,"trigger":"use",
        "selector":{"position":{"x":31926,"y":31896,"floor":-7},"unique":null,"action":null,"appearance":53439,"native":{"position":{"x":31926,"y":31896,"floor":-7},"ordinal":2,"appearance":53439}},
        "lever":null,"reversible":false,"requires_world_admission":true,"passages":[],"teleport":null,
        "storage":[
            {"track":TURNIPS,"minimum":1,"maximum":1,"set":1,"preserve":true},
            {"track":MISSION,"minimum":4,"maximum":4,"set":5},
            {"track":RAM,"minimum":0,"maximum":0,"set":1}
        ],
        "offerings":[],"item_on_target":{"selected_instance":true,"item":"oteryn:item.tibia.i53073","count":1,"replacement":null,"target_replacement":null},
        "effects":{"magic":{"position":{"x":31926,"y":31896,"floor":-7},"effect":36},"mount_reward":{"mount":"oteryn:content.mount.reliable_ram","achievement":"oteryn:achievement/a_reliable_friend"}},
        "return_on_failure":false,"failure_message":null,
        "success_message":"The hungry ram enjoyed your gift and is willing to join and assist you on your journey. Take good care of it!",
        "sources":["canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:src/utils/utils_definitions.hpp:96",
                   "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/targuna/actions_reliable_ram.lua:20-50",
                   "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/emiliana.lua:169-182"]
    }]});
    super::world_callbacks::parse(&value.to_string())?
        .pop()
        .ok_or_else(|| "missing Reliable Ram callback".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_ram_binds_source_earned_turnips_and_map_target() -> Result<(), String> {
        let callback = callback()?;
        assert_eq!(callback.selector.appearance, Some(53439));
        let native = callback
            .selector
            .native
            .as_ref()
            .ok_or("exact native Ram counterpart")?;
        assert_eq!(
            (
                native.position.x,
                native.position.y,
                native.position.floor,
                native.ordinal,
                native.appearance
            ),
            (31926, 31896, -7, 2, 53439)
        );
        assert!(
            callback
                .item_on_target
                .as_ref()
                .is_some_and(|item| item.selected_instance)
        );
        assert_eq!(
            callback.effects.magic.as_ref().map(|effect| effect.effect),
            Some(36)
        );
        assert_eq!(callback.storage[0].track, TURNIPS);
        assert!(callback.storage[0].preserve);
        assert_eq!(
            (callback.storage[1].minimum, callback.storage[1].set),
            (4, 5)
        );
        assert_eq!(
            callback.item_on_target.as_ref().map(|i| i.item.as_str()),
            Some("oteryn:item.tibia.i53073")
        );
        let reward = callback.effects.mount_reward.as_ref().ok_or("mount")?;
        assert_eq!(
            reward.grant_key(),
            "oteryn:content.mount.reliable_ram/quest-grant"
        );
        Ok(())
    }
}
