use super::TestResult;

pub(super) fn callback() -> TestResult<oteryn_gameplay_quests::quest::world_callbacks::Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/svargrond-arena/concede-pit-1",
        "quest": "oteryn:quest.barbarian_arena_quest",
        "trigger": "step_in",
        "selector": {
            "action": 25300,
            "native": { "position": {"x":32209,"y":31104,"floor":-7}, "ordinal":1, "appearance":1949 }
        },
        "reversible": false,
        "passages": [],
        "storage": [],
        "offerings": [],
        "return_on_failure": false,
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/svargrond_arena/movements_arena_pit.lua:1-21,70-72"]
    }))?)
}
