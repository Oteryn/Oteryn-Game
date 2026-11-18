//! Knife USE couples source progress and beard custody to a public native spawn.
use super::quest_world_callbacks::{self, tool_fixture};
use super::{TestResult, fixture::debug};
use oteryn_gameplay_quests::quest::{
    QuestTrack,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::UseDisposition;

const QUEST: &str = "oteryn:quest.what_a_foolish_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u8_1/what_afoolish_quest/emperor_beard_shave";
const KNIFE: &str = "oteryn:item.tibia.i3469";
const GUARD: &str = "oteryn:creature.dwarf_guard";

#[test]
fn shaving_the_emperor_with_the_knife_grants_the_beard_and_spawns_one_dwarf_guard() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("foolish-dwarf-guard", move || scenario(admin))
}

fn scenario(admin: String) -> TestResult {
    let target = Position {
        x: 32659,
        y: 31852,
        floor: -13,
    };
    let guard_at = Position {
        x: 32656,
        y: 31853,
        floor: -13,
    };
    // Crystal 201189f8 data-global/scripts/lib/register_actions.lua:1158-1167.
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/emperor-beard-shave",
        "quest": QUEST,
        "trigger": "use",
        "selector": {
            "appearance": 114,
            "native": { "position": target, "appearance": 114, "ordinal": 1 }
        },
        "requires_world_admission": true,
        "reversible": false,
        "passages": [],
        "offerings": [],
        "return_on_failure": false,
        "storage": [{ "track": TRACK, "minimum": -1, "maximum": 0, "set": 1 }],
        "item_on_target": { "item": KNIFE, "count": 1, "reusable": true, "selected_instance": true },
        "item_reward": { "item": "oteryn:item.tibia.i113", "count": 1 },
        "effects": { "creature_spawn": { "creature": GUARD, "position": guard_at } },
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/lib/register_actions.lua:1158-1167"]
    }))?;
    let world = quest_world_callbacks::world_with_cells(&callback, target, &[guard_at])?;
    tool_fixture::run_world(
        admin,
        "foolish_dwarf_guard",
        world,
        callback,
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: 1,
        }],
        async |owner, admitted, h, fence| {
            let (next, disposition) =
                tool_fixture::use_tool(owner, admitted, fence, KNIFE, target, 1).await?;
            assert_eq!(disposition, UseDisposition::Committed);
            let guards = async || -> TestResult<Vec<crate::foundation::ExactActorRef>> {
                let runtime = owner.runtime.lock().await;
                Ok(runtime
                    .current_live_creature_census()
                    .map_err(debug)?
                    .into_iter()
                    .filter(|actor| {
                        runtime.matches_live_creature_identity(*actor, GUARD.as_bytes())
                    })
                    .collect())
            };
            let summoned = guards().await?;
            let [guard] = summoned.as_slice() else {
                return Err("one public Dwarf Guard".into());
            };
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(*guard)
                    .map_err(debug)?
                    .position(),
                quest_world_callbacks::local(guard_at),
            );
            let quantities: Vec<(String, i64)> = sqlx::query_as(
                "SELECT definition_production_key, quantity FROM game_item_instances \
                 WHERE lifecycle=1 AND definition_production_key IN ('oteryn:item.tibia.i113','oteryn:item.tibia.i3469') \
                 ORDER BY definition_production_key",
            ).fetch_all(&h.pool).await?;
            assert_eq!(
                quantities,
                vec![("oteryn:item.tibia.i113".into(), 1), (KNIFE.into(), 1)]
            );
            let snapshot = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(snapshot.tracks().get(TRACK), Some(&1));
            assert!(next.continuity.next_command_id > admitted.continuity.next_command_id);
            Ok(())
        },
    )
}
