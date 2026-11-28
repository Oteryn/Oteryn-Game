use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness,debug}};

use crate::content::native_gameplay::NativeGameplayState;

use crate::durability::character_progression::CurrentCharacterGameplayFence;

use crate::foundation::MovementLocalPosition;

use crate::map::view::{MapFacts,placement_key};

use oteryn_gameplay_quests::quest::world_callbacks::{Callback,Position,Offering};

use oteryn_protocol_oteryn::world_object::{UseDisposition,WorldObjectTarget};

pub(super) const NAME:&str="m07/realm-trial";

const BEGIN:Position=Position{x:33080,y:32583,floor:-3};

pub(super) const CONTROL:Position=Position{x:33080,y:32582,floor:-3};

pub(super) fn start()->Position {Position{x:33079,y:32583,floor:-3}}

pub(super) fn world(native:&NativeGameplayState)->TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{NativeCrop,ExpectedTile,native_scene_with_appearances};
    native_scene_with_appearances(native,vec![
        NativeCrop{
            bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z03-x129-y127.b3")),
            region:(3,129,127),bounds:(33073,32580,33084,32593),start:(33079,32583),
            expected:vec![
                ExpectedTile{position:(33080,32582),ids:vec![104,1772,1823,2772],counted_entry:None},
                ExpectedTile{position:(33080,32583),ids:vec![430,2124],counted_entry:None},
                ExpectedTile{position:(33079,32583),ids:vec![104],counted_entry:None},
                ExpectedTile{position:(33075,32591),ids:vec![104,2129],counted_entry:None},
            ],
        },
        NativeCrop{
            bytes:include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z05-x129-y127.b3")),
            region:(5,129,127),bounds:(33087,32582,33089,32584),start:(33088,32583),
            expected:vec![ExpectedTile{position:(33088,32583),ids:vec![103],counted_entry:None}],
        },
    ],&[],&[2773])
}

pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,
    harness:&Harness,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult {
    let version:String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&harness.pool).await?;
    eprintln!("POSTGRES_DATABASE_READY {NAME} version={version}");
    let key=Offering{position:BEGIN,item:"oteryn:item.tibia.i14682".into(),count:1};
    assert_eq!(Box::pin(use_source(owner,admitted,1,BEGIN,1,0)).await?,UseDisposition::Rejected);
    assert_eq!(receipts(harness).await?,0);
    super::quest_world_callbacks_fixtures::input_item(owner,fence.item_fence(),&key,81,true).await?;
    assert_eq!(key_count(harness).await?,1);
    assert_eq!(Box::pin(use_source(owner,admitted,2,CONTROL,3,0)).await?,UseDisposition::Rejected,"a key is not a started trial");
    assert_eq!(receipts(harness).await?,0);
    let rune = Offering { position: BEGIN, item: "oteryn:item.tibia.i3148".into(), count: 1 };
    super::quest_world_callbacks_fixtures::input_item(owner, fence.item_fence(), &rune, 82, true).await?;
    assert_eq!(super::tibia_tales_fixtures::use_tool_at(owner, admitted, BEGIN, 1, 3148).await?, UseDisposition::Committed);
    let field = placement_key(crate::map::overlay::TilePos { x: BEGIN.x, y: BEGIN.y, floor: BEGIN.floor }, 1).ok_or("field")?;
    let stone = placement_key(crate::map::overlay::TilePos { x: CONTROL.x, y: CONTROL.y, floor: CONTROL.floor }, 1).ok_or("stone")?;
    assert!(owner.bundle_map().ok_or("map")?.facts.base_hidden(field));
    assert!(!owner.bundle_map().ok_or("map")?.facts.base_hidden(stone));
    let (step, _) = owner.paced_step(admitted.runtime_actor.ok_or("actor")?, admitted.game_session_id,
        oteryn_protocol_oteryn::world_spatial::StepDirection::East).await;
    assert_eq!(step.disposition, oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved);
    assert!(owner.bundle_map().ok_or("map")?.facts.base_hidden(stone));
    assert_eq!(phase(owner,fence,callback).await?,Some(1));
    let blocker={
        let mut runtime=owner.runtime.lock().await;
        owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native policies")?
            .install_companion_policies(&mut runtime).map_err(debug)?;
        let creature="oteryn:creature.dragon";
        let health=runtime.companion_policy(creature).map_err(debug)?.maximum_health;
        let area=callback.effects.clear_area.as_ref().ok_or("Realm clearance area")?;
        let outside_x=area.minimum.x.checked_sub(1).ok_or("outside Realm area")?;
        let outside=MovementLocalPosition{x:i32::from(outside_x),y:i32::from(area.minimum.y),floor:-i16::from(area.minimum.floor)};
        assert!(!area.contains(Position{x:outside_x,y:area.minimum.y,floor:area.minimum.floor}));
        for _ in 0..4_097 {
            let unrelated=runtime.admit_encounter_actor(creature,health,outside).map_err(debug)?;
            runtime.install_creature_policy(unrelated,creature).map_err(debug)?;
        }
        let actor=runtime.admit_encounter_actor(creature,health,MovementLocalPosition{x:33088,y:32583,floor:5}).map_err(debug)?;
        runtime.install_creature_policy(actor,creature).map_err(debug)?;actor
    };
    assert_eq!(Box::pin(use_source(owner,admitted,4,CONTROL,3,0)).await?,UseDisposition::Rejected,"live guardian on another opted-in floor blocks clearance");
    assert_eq!(phase(owner,fence,callback).await?,Some(1));
    assert_eq!(receipts(harness).await?,1);assert_eq!(key_count(harness).await?,1);
    assert!(owner.runtime.lock().await.kill_creature_for_test(blocker));
    assert_eq!(Box::pin(use_source(owner,admitted,5,CONTROL,3,0)).await?,UseDisposition::Committed,"cleared trial commits despite >4096 live creatures outside its area");
    assert_eq!(phase(owner,fence,callback).await?,Some(2));
    assert_eq!(receipts(harness).await?,2);assert_eq!(key_count(harness).await?,1,"using the rune and lever leaves the portal key intact");
    assert_eq!(Box::pin(use_source(owner,admitted,5,CONTROL,3,0)).await?,UseDisposition::Committed,"historical command replays without another write");
    assert_eq!(receipts(harness).await?,2);
    let control = placement_key(
        crate::map::overlay::TilePos { x: CONTROL.x, y: CONTROL.y, floor: CONTROL.floor },
        3,
    ).ok_or("control key")?;
    let wall = placement_key(
        crate::map::overlay::TilePos { x: 33075, y: 32591, floor: -3 },
        1,
    ).ok_or("wall key")?;
    let map = owner.bundle_map().ok_or("map")?;
    let wall_due = map.facts.next_movement_decay_time().ok_or("wall reset")?;
    assert_eq!(map.facts.expire_movement_items(wall_due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(wall_due), 1);
    assert_eq!(map.facts.object_revision(wall), 2);
    assert_eq!(map.facts.object_revision(control), 1);
    let control_due = wall_due;
    assert_eq!(map.facts.next_movement_decay_time(), None);
    let remaining = control_due.checked_sub(owner.owner_now().get()).ok_or("live cooldown")?;
    owner.clock_origin = owner.clock_origin
        .checked_sub(std::time::Duration::from_micros(remaining))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(map.facts.expire_movement_items(control_due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(control_due), 0, "cooldown expiry never moves the switch");
    assert_eq!(map.facts.object_revision(control), 1);
    assert_eq!(
        Box::pin(use_source(owner, admitted, 6, CONTROL, 3, 1)).await?,
        UseDisposition::Committed,
        "the first use after expiry returns the switch manually",
    );
    assert_eq!(receipts(harness).await?, 2);
    assert!(!owner.bundle_map().ok_or("map")?.facts.base_hidden(wall));
    assert_eq!(
        Box::pin(use_source(owner, admitted, 7, CONTROL, 3, 2)).await?,
        UseDisposition::Committed,
        "a separate use opens the passage again with the retained key",
    );
    assert_eq!(key_count(harness).await?, 1);
    assert_eq!(receipts(harness).await?, 3);
    let map = owner.bundle_map().ok_or("map")?;
    let due = map.facts.next_movement_decay_time().ok_or("second wall reset")?;
    assert_eq!(
        Box::pin(use_source(owner, admitted, 8, CONTROL, 3, 3)).await?,
        UseDisposition::Committed,
        "the active lever returns independently of the passage",
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert!(map.facts.base_hidden(wall));
    assert_eq!(map.facts.next_movement_decay_time(), Some(due));
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert!(!map.facts.base_hidden(wall));
    assert_eq!(
        Box::pin(use_source(owner, admitted, 9, CONTROL, 3, 4)).await?,
        UseDisposition::NothingToUse,
        "returning the switch leaves the opening cooldown in force",
    );
    assert_eq!(receipts(harness).await?, 3);
    Ok(())
}

async fn use_source(owner:&mut ComposedFreshAdmission<'_, '_, '_>,admitted:&mut AdmittedSession,id:u64,position:Position,ordinal:u8,revision:u64)->TestResult<UseDisposition> {
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("clock")?;
    let map=owner.bundle_map().ok_or("map")?;
    let key=placement_key(crate::map::overlay::TilePos{x:position.x,y:position.y,floor:position.floor},ordinal).ok_or("source key")?;
    let mut placement=map.overlay.base().digest().to_vec();placement.extend_from_slice(&key.to_be_bytes());
    let payload=oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget{placement,expected_revision:revision}).map_err(debug)?;
    let reply=Box::pin(super::quest_world_callbacks_fixtures::wire(owner,admitted,id,2,payload)).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

async fn key_count(harness:&Harness)->TestResult<i64>{
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i14682' AND i.lifecycle=1").fetch_one(&harness.pool).await?)
}

async fn receipts(harness:&Harness)->TestResult<i64>{
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts").fetch_one(&harness.pool).await?)
}

async fn phase(owner:&ComposedFreshAdmission<'_, '_, '_>,fence:CurrentCharacterGameplayFence,callback:&Callback)->TestResult<Option<i64>>{
    Ok(owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?.tracks().get(&callback.storage[0].track).copied())
}
