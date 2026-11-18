//! Crystal 201189f8 data-global/scripts/actions/tools/skinning.lua:259-268.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack, reward_callbacks::RewardCallback, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};
use serde_json::json;
use std::{num::NonZeroU64, sync::Arc};

pub(super) const NAME: &str = "qespike-ashes";
const QUEST: &str = "oteryn:quest.elemental_spheres_quest";
const TRACK: &str = "oteryn:quest-progress/elemental_spheres/ashes-sampling";
const KEY: &str = "oteryn:quest-reward/qespike/ashes-sample";
const TOOL: &str = "oteryn:item.tibia.i5908";
const SAMPLE: &str = "oteryn:item.tibia.i5878";
const CREATURE: &str = "oteryn:creature.minotaur";

#[test]
fn ashes_click_and_hotkey_share_the_age_gate_and_preserve_knife_and_corpse() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 7,
                y: 0,
                floor: -7,
            },
            false,
        )
    })
}

fn catalogue(revision: &str, minimum_age_ms: u32) -> TestResult<Arc<QuestStateCatalogue>> {
    // The fixture uses a native corpse and MINT output with established item profiles.
    // The admitted production knife and complete receipt and timing owners are the same.
    let callback: RewardCallback = serde_json::from_value(json!({
        "key":KEY,"quest":QUEST,"npc":"oteryn:npc.arkulius","keywords":["mission"],
        "topic":0,"next_topic":0,"carried_item":TOOL,"creature_target":CREATURE,
        "corpse_target":{"appearance":5969,"minimum_age_ms":minimum_age_ms,
            "maximum_age_ms":60000,"reusable_tool":true,"speech":"You got Neutral matter."},
        "take":[],"per_occurrence_mint":true,
        "storage":[{"track":TRACK,"minimum":-1,"maximum":1,"set":1}],
        "give":{"item":SAMPLE,"count":1},
        "reply":["You got Neutral matter."],
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/tools/skinning.lua:259-268"]
    }))?;
    Ok(Arc::new(
        QuestStateCatalogue::new(
            revision,
            vec![QuestTrack {
                key: TRACK.into(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: 2,
            }],
            Vec::new(),
        )
        .map_err(debug)?
        .with_reward_callback_fixture(callback)?,
    ))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let content = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision()
        .to_owned();
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let race = native
        .reward_table()
        .row(CREATURE)
        .map_err(debug)?
        .race
        .clone()
        .ok_or("source race")?;
    let knife_definition = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", TOOL, "definition-r1"))
        .ok_or("native obsidian knife definition")?;
    super::item_look::seed_main_backpack(harness, &[(TOOL, 1)]).await?;
    let corpse =
        super::srx2_scavenge::mint_corpse(owner, harness, native, 5969, Some(race), 9401).await?;
    let actor = admitted.runtime_actor.ok_or("player")?;
    let session = admitted.game_session_id;
    let first_command_id = admitted.continuity.next_command_id;
    let intent = UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(
            NonZeroU64::new(2).ok_or("corpse handle")?,
        )),
        use_with_position: None,
    };
    let mut last = None;
    for (offset, earliest, hotkey, expected) in [
        (0, 30_000, false, UseDisposition::Rejected),
        (1, 30_000, true, UseDisposition::Rejected),
        (2, 0, false, UseDisposition::Committed),
        (3, 0, true, UseDisposition::Committed),
    ] {
        owner.quest_catalogue = Some(catalogue(&content, earliest)?);
        owner.refresh_quest_session(session, false).await;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("quest fence")?;
        let command = UseCommand {
            game_session_id: session,
            command_id: first_command_id + offset,
            item_fence: Some(fence.item_fence()),
        };
        let mut intent = intent.clone();
        if hotkey {
            intent.target = UseTarget::ItemByDefinition(knife_definition);
        }
        let result = owner
            .use_quest_tool(
                actor,
                command,
                intent.clone(),
                (!hotkey).then_some(ItemKey::Instance(id(71))),
                Some(ItemKey::Instance(corpse)),
            )
            .await
            .map_err(debug)?
            .ok_or("quest USE")?;
        assert_eq!(result.disposition, expected);
        last = Some((command, intent));
    }
    let (command, intent) = last.ok_or("committed command")?;
    let replay = owner
        .use_quest_tool(actor, command, intent.clone(), None, None)
        .await
        .map_err(debug)?
        .ok_or("reusable replay")?;
    assert_eq!(replay.disposition, UseDisposition::Committed);
    let changed_target = UseIntent {
        use_with_map_item: Some(MapUseTarget::Item(NonZeroU64::MAX)),
        ..intent
    };
    let conflicting_replay = owner
        .use_quest_tool(actor, command, changed_target, None, None)
        .await
        .map_err(debug)?
        .ok_or("conflicting hotkey replay")?;
    assert_eq!(conflicting_replay.disposition, UseDisposition::Rejected);
    let snapshot: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_quest_corpse_samples WHERE transition_key=$1),
         (SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1),
         (SELECT state_revision::bigint FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid),
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$3),
         (SELECT i.quantity FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id)
          WHERE i.item_instance_id=encode($4,'hex')::uuid AND i.lifecycle=1 AND e.character_id=encode($5,'hex')::uuid),
         (SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id)
          WHERE i.definition_production_key=$6 AND i.lifecycle=1 AND e.character_id=encode($5,'hex')::uuid)",
    ).bind(KEY).bind(corpse.as_slice()).bind(TRACK).bind(id(71).as_slice()).bind(id(41).as_slice()).bind(SAMPLE)
    .fetch_one(&harness.pool).await?;
    assert_eq!(snapshot, (2, 0, 1, 1, 1, 2));
    Ok(())
}
