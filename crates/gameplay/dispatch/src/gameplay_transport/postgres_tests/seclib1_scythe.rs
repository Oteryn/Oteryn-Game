//! Crystal register_actions.lua:1035-1059. Quest content is supplied by the test catalogue.
use super::{TestResult, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

const MONUMENT: Position = Position { x: 32177, y: 31925, floor: -7 };
const DESTINATION: Position = Position { x: 32515, y: 32535, floor: -12 };
const SCYTHE: &str = "oteryn:item.tibia.i3453";
const ALTERNATE: &str = "oteryn:item.tibia.i9596";

#[test]
fn seclib1_isle_scythe_uses_selected_tool_and_exact_permission() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("seclib1-scythe", || {
        let callback: Callback = serde_json::from_value(serde_json::json!({
            "key": "oteryn:quest-callback/authored-seclib1-isle-scythe",
            "quest": "oteryn:quest.the_secret_library_quest",
            "trigger": "use",
            "selector": {"position": MONUMENT, "appearance": 2028},
            "requires_world_admission": true,
            "reversible": false,
            "passages": [],
            "teleport": DESTINATION,
            "storage": [{
                "track":"oteryn:quest-progress/quest/u11_80/the_secret_library/library_permission",
                "minimum":7,"maximum":7,"set":7,"preserve":true
            }],
            "offerings": [],
            "item_on_target": {
                "item":SCYTHE,"alternatives":[ALTERNATE],"count":1,
                "reusable":true,"selected_instance":true
            },
            "return_on_failure":false,
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/lib/register_actions.lua:1035-1059"]
        }))?;
        for (stage, tool, expected) in [(7, SCYTHE, true), (7, ALTERNATE, true), (6, SCYTHE, false), (8, SCYTHE, false)] {
            quest_world_callbacks::literal_uses::exercise_selected_tool(
                callback.clone(), 2028, 1, stage, tool, expected,
            )?;
        }
        Ok(())
    })
}
