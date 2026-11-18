//! Source-qualified dormitory admission and Servant death use the ordinary STEP and loot owners.
use super::quest_world_callbacks::{self, tool_fixture};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use oteryn_gameplay_quests::quest::{
    QuestTrack,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

const QUEST: &str = "oteryn:quest.the_gravedigger_of_drefia_quest";
const MISSION_55: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission55";
const MISSION_56: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission56";
const MISSION_57: &str = "oteryn:quest-progress/quest/u10_10/the_gravedigger_of_drefia/mission57";
const SERVANT: &str = "oteryn:creature.necromancer_servant";
const FRAGMENT: &str = "oteryn:item.tibia.i18933";

#[test]
fn dormitory_entry_commits_once_and_redirects_an_ineligible_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("gravedigger-room", move || {
        dormitory_gate(admin.clone(), 0)?;
        dormitory_gate(admin, 1)
    })
}

#[test]
fn dormitory_step_spawns_the_servant_and_its_death_keeps_the_fragment_in_the_corpse() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("gravedigger-servant", move || servant(admin))
}

fn dormitory_gate(admin: String, entered: i64) -> TestResult {
    let pad = Position {
        x: 33018,
        y: 32438,
        floor: -10,
    };
    let room = Position {
        x: 33015,
        y: 32440,
        floor: -10,
    };
    let outside = Position {
        x: 33018,
        y: 32437,
        floor: -10,
    };
    // Crystal 201189f8 movements_dormitory_teleport.lua:9-17 and startup/tables/item.lua:634-644.
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/gravedigger-dormitory-entry",
        "quest": QUEST, "trigger": "step_in",
        "selector": { "appearance": 1949, "native": { "position": pad, "appearance": 1949, "ordinal": 2 } },
        "requires_world_admission": true, "reversible": false, "passages": [], "offerings": [],
        "storage": [
            { "track": MISSION_55, "minimum": 1, "maximum": 1, "set": 1, "preserve": true },
            { "track": MISSION_56, "minimum": 0, "maximum": 0, "set": 1 }
        ],
        "teleport": room, "failure_teleport": outside, "return_on_failure": true,
        "success_message": "You should hurry, try not to dwell here any longer than a few minutes.",
        "failure_message": "The teleport is accessible only once.",
        "effects": { "teleport_effect": 11 },
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_gravedigger_of_drefia/movements_dormitory_teleport.lua:9-17"]
    }))?;
    let world = quest_world_callbacks::world_with_cells(&callback, pad, &[outside])?;
    tool_fixture::run_world(
        admin,
        "gravedigger_dormitory_gate",
        world,
        callback,
        tracks([(MISSION_55, 1), (MISSION_56, entered)]),
        async |owner, mut admitted, _, fence| {
            step_east(owner, &mut admitted).await?;
            let landed = owner
                .runtime
                .lock()
                .await
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .map_err(debug)?
                .position();
            assert_eq!(
                landed,
                quest_world_callbacks::local(if entered == 0 { room } else { outside })
            );
            let snapshot = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(
                snapshot
                    .tracks()
                    .get(MISSION_56)
                    .copied()
                    .unwrap_or(entered),
                1
            );
            Ok(())
        },
    )
}

fn servant(admin: String) -> TestResult {
    let pad = Position {
        x: 33015,
        y: 32438,
        floor: -11,
    };
    let birth = Position {
        x: 33011,
        y: 32437,
        floor: -11,
    };
    // Crystal 201189f8 movements_necromancer_servant.lua:9-11 and startup/tables/item.lua:646-650.
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/fixture/gravedigger-servant-step",
        "quest": QUEST, "trigger": "step_in",
        "selector": { "appearance": 19147, "native": { "position": pad, "appearance": 19147, "ordinal": 0 } },
        "requires_world_admission": true, "reversible": false, "passages": [], "offerings": [],
        "storage": [
            { "track": MISSION_56, "minimum": 1, "maximum": 1, "set": 1, "preserve": true },
            { "track": MISSION_57, "minimum": 0, "maximum": 0, "set": 0, "preserve": true }
        ],
        "return_on_failure": false,
        "effects": { "creature_spawn": { "creature": SERVANT, "position": birth } },
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_gravedigger_of_drefia/movements_necromancer_servant.lua:9-11"]
    }))?;
    let world = quest_world_callbacks::world_with_cells(&callback, pad, &[birth])?;
    tool_fixture::run_world(
        admin,
        "gravedigger_servant",
        world,
        callback,
        tracks([(MISSION_56, 1), (MISSION_57, 0)]),
        async |owner, mut admitted, h, _| {
            step_east(owner, &mut admitted).await?;
            let servant = {
                let runtime = owner.runtime.lock().await;
                let matches = runtime
                    .current_live_creature_census()
                    .map_err(debug)?
                    .into_iter()
                    .filter(|actor| {
                        runtime.matches_live_creature_identity(*actor, SERVANT.as_bytes())
                    })
                    .collect::<Vec<_>>();
                let [servant] = matches.as_slice() else {
                    return Err("one living Necromancer Servant".into());
                };
                assert_eq!(
                    runtime
                        .read_actor_position(*servant)
                        .map_err(debug)?
                        .position(),
                    quest_world_callbacks::local(birth)
                );
                *servant
            };
            super::engworldchg_journals::kill_actor(
                owner,
                &admitted,
                servant,
                admitted.continuity.next_command_id,
            )
            .await?;
            // Crystal 201189f8 data-global/monster/humans/necromancer_servant.lua:66-72.
            let rows: Vec<(i64, String)> = sqlx::query_as("SELECT i.quantity, parent.definition_production_key FROM game_item_instances i JOIN game_item_corpse_container_entries e USING(item_instance_id) JOIN game_item_instances parent ON parent.item_instance_id=e.parent_item_instance_id WHERE i.definition_production_key=$1 AND i.lifecycle=1")
                .bind(FRAGMENT).fetch_all(&h.pool).await?;
            assert_eq!(rows, [(1, "oteryn:item.tibia.i18174".into())]);
            Ok(())
        },
    )
}

fn tracks<const N: usize>(inputs: [(&str, i64); N]) -> Vec<QuestTrack> {
    inputs
        .into_iter()
        .map(|(key, initial)| QuestTrack {
            key: key.into(),
            quest: QUEST.into(),
            initial,
            min: 0,
            max: 1,
        })
        .collect()
}

async fn step_east(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let command = admitted.continuity.next_command_id;
    let result = quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&result).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
