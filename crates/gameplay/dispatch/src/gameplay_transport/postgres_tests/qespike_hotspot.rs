//! Crystal 201189f8 data-global/scripts/quests/spike_tasks/actions_thermometer.lua:1-36.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    reward_callbacks::RewardCallback,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::{
    world_object::{UseDisposition, UseIntent, UseTarget, encode_item_use_intent},
    world_spatial::{COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent},
};
use serde_json::json;
use std::{sync::Arc, time::Duration};

pub(super) const NAME: &str = "qespike-hotspot";
const KEY: &str = "oteryn:quest-reward/qespike/lava-temperature";
const QUEST: &str = "oteryn:quest.spike_task";
const TRACK: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_lower_lava_main";
// The carried binding is generic. Use a native rope in the fixture until
// the Content session admits Crystal's thermometer (19206).
const CARRIED_TOOL: &str = "oteryn:item.tibia.i3003";
const POOLS: [([i32; 3], [i32; 3]); 9] = [
    ([32263, 32481, 13], [32274, 32490, 13]),
    ([32163, 32558, 13], [32173, 32567, 13]),
    ([32201, 32667, 13], [32211, 32672, 13]),
    ([32135, 32606, 14], [32143, 32614, 14]),
    ([32330, 32519, 14], [32339, 32521, 14]),
    ([32260, 32697, 14], [32272, 32705, 14]),
    ([32176, 32493, 15], [32186, 32502, 15]),
    ([32341, 32577, 15], [32347, 32586, 15]),
    ([32220, 32643, 15], [32223, 32652, 15]),
];

#[test]
fn spike_thermometer_only_consumes_at_the_shared_server_save_hotspot() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, selected_pool()?, false)
    })
}

fn callback() -> TestResult<RewardCallback> {
    Ok(serde_json::from_value(json!({
        "key":KEY,"quest":QUEST, "npc":"oteryn:npc.gnomargery",
        "keywords":["temperature"],"topic":0,"next_topic":0,"carried_item":CARRIED_TOOL,
        "server_save_hotspot":{
            "areas": POOLS.map(|(minimum,maximum)| json!({"minimum": minimum, "maximum": maximum})),
            "cold_reply":"This is not the hot spot!", "hot_effect":16
        },
        "storage":[{"track":TRACK,"minimum":0,"maximum":0,"set":1}],
        "take":[{"item":CARRIED_TOOL,"count":1}],
        "reply":["Whew! That was that hot, it melted the thermometer! At least you've found the hot spot!"],
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/spike_tasks/actions_thermometer.lua:1-36"]
    }))?)
}

fn selected_pool() -> TestResult<Position> {
    let source = callback()?;
    let hotspot = source.server_save_hotspot.ok_or("thermometer hotspot")?;
    let ([x, y, floor], _) = POOLS
        .into_iter()
        .find(|(point, _)| hotspot.contains(KEY, &id(42), 0, *point))
        .ok_or("world reset pool")?;
    Ok(Position {
        x: x.try_into()?,
        y: y.try_into()?,
        floor: (-floor).try_into()?,
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision();
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            revision,
            vec![QuestTrack {
                key: TRACK.into(),
                quest: QUEST.into(),
                initial: 0,
                min: -1,
                max: 1,
            }],
            Vec::new(),
        )
        .map_err(debug)?
        .with_reward_callback_fixture(callback()?)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "thermometer fence")?
        .ok_or("thermometer fence")?;
    let hot = selected_pool()?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: hot.x - 1,
                ..hot
            },
            item: CARRIED_TOOL.into(),
            count: 1,
        },
        26421,
        true,
    )
    .await?;
    let tool = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", CARRIED_TOOL, "definition-r1"))
        .ok_or("native thermometer definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(tool),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    async fn progress(harness: &Harness) -> TestResult<(i64, i64)> {
        Ok(sqlx::query_as(
            "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0),
            (SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$2)",
        )
        .bind(TRACK)
        .bind(KEY)
        .fetch_one(&harness.pool)
        .await?)
    }
    let (next, result, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(tool),
    )
    .await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(progress(harness).await?, (0, 0));

    owner.clock_origin -= Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    let (next, result, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(tool),
    )
    .await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(progress(harness).await?, (1, 1));
    Ok(())
}
