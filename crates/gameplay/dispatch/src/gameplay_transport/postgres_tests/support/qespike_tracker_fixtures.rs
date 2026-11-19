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
pub(super) const REPEAT_NAME: &str = "qespike-tracker-repeat";
pub(super) const FINAL_NAME: &str = "qespike-tracker-final";
const KEY: &str = "oteryn:quest-reward/qespike/ghost-detector-0";
const FINAL_KEY: &str = "oteryn:quest-reward/qespike/ghost-detector-2";
const QUEST: &str = "oteryn:quest.spike_task";
const TRACK: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_upper_track_main";
const CYCLE: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_upper_track_daily";
const REPORTED: i64 = 72_000;
// Native custody fixture for the generic detector binding.
const TOOL: &str = "oteryn:item.tibia.i3003";

fn callback(name: &str) -> TestResult<RewardCallback> {
    let final_floor = name == FINAL_NAME;
    let progress = if final_floor { 2 } else { 0 };
    Ok(serde_json::from_value(json!({
        "key":if final_floor { FINAL_KEY } else { KEY },"quest":QUEST, "npc":"oteryn:npc.gnomilly", "keywords":["track"],
        "topic":0,"next_topic":0,"carried_item":TOOL,
        "storage":[{"track":TRACK,"minimum":progress,"maximum":progress,"set":progress+1}],
        "carried_tracker":{"area":{"minimum":[32171,32512,if final_floor { 10 } else { 8 }],"maximum":[32352,32671,10]},
            "radius":4,"heading_prefix":"The detector points ",
            "cycle_track":(name != NAME).then_some(CYCLE),
            "ground_search":(!final_floor).then(|| json!({"attempts":50,"appearances":[[19271,19302],[19550,19557]]}))},
        "take":if final_floor { json!([{"item":TOOL,"count":1}]) } else { json!([]) },
        "reply":if final_floor {
            vec!["Report the task to Gnomilly.", "You found a malignant presence, the glowing detector signals that it does not need any further data."]
        } else { vec!["You found a malignant presence, the glowing detector signals another presence nearby."] },
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/spike_tasks/actions_ghost_detector.lua:74-103",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/libs/functions/position.lua:148-161",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomilly.lua:119-121"]
    }))?)
}

pub(super) fn discovery_edge(name: &str) -> TestResult<Position> {
    let callback = callback(name)?;
    let tracker = callback.carried_tracker.ok_or("tracker")?;
    // The fixture terrain has no grey sand. Crystal returns the final draw.
    let cycle = tracker
        .cycle_track
        .as_ref()
        .map(|_| if name == REPEAT_NAME { REPORTED } else { -1 });
    let probe = tracker
        .ground_search
        .as_ref()
        .map_or(0, |search| search.attempts - 1);
    let [x, y, z] = tracker.target_at(&callback.key, &id(42), &id(41), 0, cycle, probe);
    let other = tracker.target_at(&callback.key, &id(42), &id(43), 0, cycle, probe);
    assert_ne!([x, y, z], other, "players have independent targets");
    if name == REPEAT_NAME {
        let previous = tracker.target_at(KEY, &id(42), &id(41), 0, Some(-1), 49);
        assert!(!tracker.reached([x - 4, y, z], previous));
        assert!(!tracker.reached([x - 5, y, z], previous));
    }
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
    name: &str,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision();
    let callback = callback(name)?;
    let key = callback.key.clone();
    let initial = callback.storage[0].minimum;
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            revision,
            vec![
                QuestTrack {
                    key: TRACK.into(),
                    quest: QUEST.into(),
                    initial,
                    min: -1,
                    max: 3,
                },
                QuestTrack {
                    key: CYCLE.into(),
                    quest: QUEST.into(),
                    initial: -1,
                    min: -1,
                    max: i64::MAX,
                },
            ],
            Vec::new(),
        )
        .map_err(debug)?
        .with_reward_callback_fixture(callback)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "tracker fence")?
        .ok_or("tracker fence")?;
    let edge = discovery_edge(name)?;
    super::quest_world_callbacks_fixtures::input_item(
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
    async fn snapshot(harness: &Harness, key: &str, initial: i64) -> TestResult<(i64, i64)> {
        Ok(sqlx::query_as(
            "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),$3),
            (SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$2)",
        )
        .bind(TRACK)
        .bind(key)
        .bind(initial)
        .fetch_one(&harness.pool)
        .await?)
    }
    let (next, result, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(tool),
    )
    .await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(snapshot(harness, &key, initial).await?, (initial, 0));
    owner.clock_origin -= Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    super::quest_world_callbacks_fixtures::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    if name == REPEAT_NAME {
        let (next, result, _) = super::item_craft_fixtures::exchange(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(tool),
        )
        .await?;
        *admitted = next;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(snapshot(harness, &key, initial).await?, (initial, 0));

        // Seed the predecessor written by Gnomilly's completed report. All
        // detector uses and the successor receipt still go through dispatch.
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(CYCLE).bind(QUEST).bind(REPORTED).execute(&mut *tx).await?;
        tx.commit().await?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        owner.clock_origin -= Duration::from_secs(2);
    }
    let (next, result, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        (name != FINAL_NAME).then_some(tool),
    )
    .await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(
        snapshot(harness, &key, initial).await?,
        (initial + 1, i64::from(name == FINAL_NAME))
    );
    if name == REPEAT_NAME {
        let recorded: i64 = sqlx::query_scalar(
            "SELECT (plan #>> '{carried_observation,tracking_cycle}')::bigint
             FROM game_quest_reward_sources WHERE transition_key=$1",
        )
        .bind(KEY)
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(recorded, REPORTED);
    }
    Ok(())
}
