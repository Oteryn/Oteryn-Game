use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::{connection::UseCommand, item_view::ItemKey};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, QuestTrack,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use serde_json::json;
use std::{num::NonZeroU64, sync::Arc};

pub(super) const PREFIX: &str = "qespike-machine-deposit/";
const QUEST: &str = "oteryn:quest.elemental_spheres_quest";
const COUNT: &str = "oteryn:quest-progress/elemental_spheres/machine_gem_count";
const MISSION: &str = "oteryn:quest-progress/elemental_spheres/quest_line";
const GEM: &str = "oteryn:item.tibia.i678";
pub(super) const MACHINE: Position = Position {
    x: 33268,
    y: 31830,
    floor: -10,
};

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/qespike/machine-deposit-knight-first",
        "quest":QUEST,"trigger":"use","selector":{"position":MACHINE,"appearance":842},
        "reversible":false,"return_on_failure":false,"requires_world_admission":true,
        "offerings":[],"passages":[],
        "item_on_target":{"item":GEM,"count":1,"selected_instance":true},
        "effects":{"tool_vocation":"knight"},
        "storage":[{"track":MISSION,"minimum":1,"maximum":3,"set":1,"preserve":true},
            {"track":COUNT,"minimum":-1,"maximum":-1,"set":1}],
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/other/enchanting.lua:6-10,47-57"]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    harness: &Harness,
    callback: &Callback,
    name: &str,
) -> TestResult {
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("quest catalogue")?
        .content_revision();
    owner.quest_catalogue = Some(Arc::new(
        QuestStateCatalogue::new(
            revision,
            vec![
                QuestTrack {
                    key: COUNT.into(),
                    quest: QUEST.into(),
                    initial: -1,
                    min: -1,
                    max: 100,
                },
                QuestTrack {
                    key: MISSION.into(),
                    quest: QUEST.into(),
                    initial: 1,
                    min: -1,
                    max: 3,
                },
            ],
            Vec::new(),
        )
        .map_err(debug)?
        .with_world_callback_fixture(callback.clone())?,
    ));
    owner
        .states
        .quest_callbacks
        .fixture
        .set(vec![callback.clone()])
        .map_err(|_| "deposit callback")?;
    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("machine user")?;
    let vocation = name.strip_prefix(PREFIX).ok_or("fixture vocation")?;
    let (facts, build) =
        super::party_xp_fixtures::initialize_build(owner, session, vocation, 149).await?;
    {
        let runtime = owner.runtime.lock().await;
        let mut spells = owner.states.spell_states.lock().await;
        let mut state =
            crate::spell::cast::PlayerSpellState::new(facts, 0, 0).ok_or("machine caster")?;
        state.load_owned_build(&build).map_err(debug)?;
        state.make_playable(owner.owner_now()).map_err(debug)?;
        assert!(spells.replace_owned_player_state_test_fixture(&runtime, actor, session, state));
    }
    owner.refresh_quest_session(session, false).await;
    super::item_look_fixtures::seed_main_backpack(harness, &[(GEM, 2), (GEM, 2)]).await?;
    let map = owner.bundle_map().ok_or("machine map")?;
    let pos = TilePos {
        x: MACHINE.x,
        y: MACHINE.y,
        floor: MACHINE.floor,
    };
    let key = crate::map::view::placement_key(pos, 1).ok_or("machine placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    let use_command = UseCommand {
        game_session_id: session,
        command_id: admitted.continuity.next_command_id,
        item_fence: Some(fence.item_fence()),
    };
    let result = owner
        .tame_mount_impl(
            actor,
            use_command,
            intent,
            Some(ItemKey::Instance(id(72))),
            None,
            true,
        )
        .await
        .map_err(debug)?
        .ok_or("gem USE")?;
    let allowed = vocation == "elite_knight";
    assert_eq!(
        result.disposition,
        if allowed {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        }
    );
    let after: (i64, i64, i64) = sqlx::query_as(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),-1),
         (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($2,'hex')::uuid),
         (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($3,'hex')::uuid)",
    )
    .bind(COUNT)
    .bind(id(71).as_slice())
    .bind(id(72).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(after, if allowed { (1, 2, 1) } else { (-1, 2, 2) });
    Ok(())
}
