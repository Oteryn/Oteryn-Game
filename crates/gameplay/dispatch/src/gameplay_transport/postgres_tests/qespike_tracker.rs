//! Crystal 201189f8 data-global/scripts/quests/spike_tasks/actions_ghost_detector.lua:74-103.
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

pub(super) const NAME: &str = "qespike-tracker";
const KEY: &str = "oteryn:quest-reward/qespike/ghost-detector-0";
const QUEST: &str = "oteryn:quest.spike_task";
const TRACK: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_upper_track_main";
// Native custody fixture for the generic detector binding.
const TOOL: &str = "oteryn:item.tibia.i3003";

#[test]
fn spike_detector_reuses_personal_target_and_advances_without_spending_first_use() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, discovery_edge()?, false)
    })
}

fn callback() -> TestResult<RewardCallback> {
    Ok(serde_json::from_value(json!({
        "key":KEY,"quest":QUEST, "npc":"oteryn:npc.gnomilly", "keywords":["track"],
        "topic":0,"next_topic":0,"carried_item":TOOL,
        "storage":[{"track":TRACK,"minimum":0,"maximum":0,"set":1}],
        "carried_tracker":{"area":{"minimum":[32171,32512,8],"maximum":[32352,32671,10]},
            "radius":4,"heading_prefix":"The detector points ",
            "ground_search":{"attempts":50,"appearances":[[19271,19302],[19550,19557]]}},
        "take":[],
        "reply":["You found a malignant presence, the glowing detector signals another presence nearby."],
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/spike_tasks/actions_ghost_detector.lua:74-103",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/libs/functions/position.lua:148-161"]
    }))?)
}

fn discovery_edge() -> TestResult<Position> {
    let tracker = callback()?.carried_tracker.ok_or("tracker")?;
    // The fixture terrain has no grey sand. Crystal returns the final draw.
    let [x, y, z] = tracker.target_at(KEY, &id(42), &id(41), 0, None, 49);
    let other = tracker.target_at(KEY, &id(42), &id(43), 0, None, 49);
    assert_ne!([x, y, z], other, "players have independent targets");
    Ok(Position {
        x: (x - 4).try_into()?,
        y: y.try_into()?,
        floor: (-z).try_into()?,
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
                max: 3,
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
        .map_err(|()| "tracker fence")?
        .ok_or("tracker fence")?;
    let edge = discovery_edge()?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: edge.x - 1,
                ..edge
            },
            item: TOOL.into(),
            count: 1,
        },
        26422,
        true,
    )
    .await?;
    let tool = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", TOOL, "definition-r1"))
        .ok_or("fixture tool")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(tool),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    async fn snapshot(harness: &Harness) -> TestResult<(i64, i64)> {
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
    assert_eq!(snapshot(harness).await?, (0, 0));
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
    assert_eq!(snapshot(harness).await?, (1, 0));
    Ok(())
}
