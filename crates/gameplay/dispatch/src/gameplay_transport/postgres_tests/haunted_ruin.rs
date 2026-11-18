use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::{num::NonZeroU64, sync::Arc};

pub(super) const NAME: &str = "qengine-rewards-haunted-ruin";
const TARGET: Position = Position { x: 101, y: 100, floor: -7 };
pub(super) const DOOR: Position = Position { x: 102, y: 100, floor: -7 };
const QUEST: &str = "oteryn:quest.the_inquisition_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission04";
const WATER: &str = "oteryn:item.tibia.i133";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_holy_water.lua:65-80";

#[test]
fn haunted_ruin_holy_water_sets_mission_spawns_ghost_and_opens_timed_door() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, TARGET, false)
    })
}

pub(super) fn callback(at: Position) -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/haunted-ruin-fixture",
        "quest":QUEST, "trigger":"use",
        "selector":{"position":at,"ordinal":1,"action":2003},
        "requires_world_admission":true,
        "reversible":false, "passages":[], "offerings":[], "return_on_failure":false,
        "storage":[
            {"track":TRACK,"minimum":12,"maximum":12,"set":13},
            {"track":MISSION,"minimum":0,"maximum":3,"set":2}
        ],
        "item_on_target":{"item":WATER,"count":1,"reusable":false,"selected_instance":true},
        "effects":{
            "use_summon":{"creature":"oteryn:creature.pirate_ghost"},
            "target_transform":{"position":DOOR,"from":7869,"to":7868,"reset_after_ms":10000,"if_present":true}
        },
        "sources":[SOURCE]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    {
        use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
        let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native")?;
        let mut runtime = owner.runtime.lock().await;
        native.install_companion_policies(&mut runtime).map_err(debug)?;
    }
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let digest = catalogue.definition_hash(QUEST).ok_or("prior quest definition")?;
    owner.quest_catalogue = Some(Arc::new(catalogue.with_world_callback_fixtures(vec![callback(TARGET)?])?));
    sqlx::query(
        "INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision)
         SELECT character_id,$2,'content-1',$3,character_revision,character_revision
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice()).bind(QUEST).bind(digest.as_slice())
    .execute(&h.pool).await?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,$2,quest_key,12,committed_character_revision FROM game_character_quest_states
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$3",
    )
    .bind(id(41).as_slice()).bind(TRACK).bind(QUEST).execute(&h.pool).await?;

    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner, fence.item_fence(), &Offering { position:TARGET, item:WATER.into(), count:1 }, 954, true
    ).await?;
    let reference = owner.active_generation.and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", WATER, "definition-r1"))
        .ok_or("holy water reference")?;
    let target = oteryn_world_bundle::bundle::placement_key(-7, TARGET.x, TARGET.y, 1).ok_or("target")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&target.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::new(3).ok_or("handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement, expected_revision:map.facts.object_revision(target),
        })),
        use_with_position:None,
    }).map_err(debug)?;
    let (next, disposition, _) = super::item_craft::exchange(
        owner, *admitted, admitted.continuity.next_command_id, &payload, Some(reference)
    ).await?;
    *admitted = next;
    assert_eq!(disposition, UseDisposition::Committed);
    let stage: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(stage, 13);
    let door_appearance = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<u16> {
        let map = owner.bundle_map().ok_or("map")?;
        let door = crate::map::overlay::TilePos { x:DOOR.x,y:DOOR.y,floor:DOOR.floor };
        let tile = map.overlay.base().tile(door.x,door.y,door.floor).ok_or("door tile")?;
        let id = *tile.ids().get(1).ok_or("door placement")?;
        Ok(map.facts.base_entry(door,1,id).ok_or("door appearance")?.appearance_id)
    };
    assert_eq!(door_appearance(owner)?, 7868);
    let runtime = owner.runtime.lock().await;
    assert!(runtime.positioned_melee_census().map_err(debug)?.iter().any(|(actor, position, session)| {
        session.is_none() && position.position() == super::quest_world_callbacks::local(TARGET)
            && runtime.matches_live_creature_identity(*actor, b"oteryn:creature.pirate_ghost")
    }));
    drop(runtime);
    let consumed: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
           (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$2)",
    ).bind(MISSION).bind(WATER).fetch_one(&h.pool).await?;
    assert_eq!(consumed, (2,0));
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(11)).ok_or("clock")?;
    owner.tick_timed_items().await;
    assert_eq!(door_appearance(owner)?, 7869);
    Ok(())
}
