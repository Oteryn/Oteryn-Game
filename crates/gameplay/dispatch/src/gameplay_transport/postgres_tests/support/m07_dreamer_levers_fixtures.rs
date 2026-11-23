use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness, debug}};

use crate::content::native_gameplay::NativeGameplayState;

use crate::map::view::{MapFacts, placement_key};

use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const NAME: &str = "m07/dreamer-lever-north";

pub(super) const CONTROL: Position = Position { x:32761, y:32289, floor:-14 };

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{NativeCrop, ExpectedTile, native_scene_with_appearances};
    native_scene_with_appearances(native, vec![NativeCrop {
        bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content/world/placements/region-z14-x127-y126.b3")),
        region:(14,127,126), bounds:(32759,32287,32765,32293), start:(32762,32289),
        expected: vec![
            ExpectedTile { position:(32761,32289),ids:vec![410,2772],counted_entry:None },
            ExpectedTile { position:(32763,32289),ids:vec![410,2772],counted_entry:None },
            ExpectedTile { position:(32761,32291),ids:vec![410,2772],counted_entry:None },
            ExpectedTile { position:(32763,32291),ids:vec![410,2772],counted_entry:None },
            ExpectedTile { position:(32760,32288),ids:vec![410,1272],counted_entry:None },
            ExpectedTile { position:(32762,32288),ids:vec![410,1271],counted_entry:None },
        ],
    }], &[], &[2773])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    harness: &Harness, callback: &Callback,
) -> TestResult {
    let version: String=sqlx::query_scalar("SELECT current_setting('server_version')").fetch_one(&harness.pool).await?;
    eprintln!("POSTGRES_DATABASE_READY {NAME} version={version}");
    let initial = counts(harness).await?;
    let source = placement_key(crate::map::overlay::TilePos { x:CONTROL.x,y:CONTROL.y,floor:CONTROL.floor },1).ok_or("native lever key")?;
    let mut placement=owner.bundle_map().ok_or("map")?.overlay.base().digest().to_vec();
    placement.extend_from_slice(&source.to_be_bytes());
    let target=|expected_revision| WorldObjectTarget { placement:placement.clone(),expected_revision };
    let use_source=|expected_revision| oteryn_protocol_oteryn::world_object::encode_use_intent(&target(expected_revision)).map_err(debug);
    let wall=crate::map::overlay::TilePos { x:32762,y:32288,floor:-14 };
    assert!(owner.bundle_map().ok_or("map")?.facts.quest_callback_blocked(owner.bundle_map().ok_or("map")?.overlay.base(),wall,true));
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("clock")?;
    let response=super::quest_world_callbacks_fixtures::wire(owner,admitted,1,2,use_source(0)?).await?;
    assert_eq!(oteryn_protocol_oteryn::world_object::decode_use_result(&response).map_err(debug)?,UseDisposition::Committed);
    let due={
        let map=owner.bundle_map().ok_or("map")?;
        assert_eq!(callback.passages.len(),6,"retain the conditional1027 source entry");
        for passage in &callback.passages {
            if passage.appearance==1027 { continue; }
            let pos=crate::map::overlay::TilePos { x:passage.position.x,y:passage.position.y,floor:passage.position.floor };
            let key=placement_key(pos,1).ok_or("wall key")?;
            assert!(map.facts.base_hidden(key));
        }
        assert!(!map.facts.quest_callback_blocked(map.overlay.base(),wall,true));
        map.facts.next_movement_decay_time().ok_or("remote deadline")?
    };
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("clock")?;
    let response=super::quest_world_callbacks_fixtures::wire(owner,admitted,2,2,use_source(1)?).await?;
    assert_eq!(oteryn_protocol_oteryn::world_object::decode_use_result(&response).map_err(debug)?,UseDisposition::NothingToUse);
    {
        let map=owner.bundle_map().ok_or("map")?;
        assert_eq!(map.facts.next_movement_decay_time(),Some(due),"active lever cannot extend timers");
        assert_eq!(map.facts.expire_movement_items(due-1),0);
        assert_eq!(map.facts.expire_movement_items(due),5,"only five actually present source walls return");
        assert!(map.facts.quest_callback_blocked(map.overlay.base(),wall,true));
        assert_eq!(map.facts.object_revision(source),1,"lever stays active after walls return");
        let source_due=map.facts.next_movement_decay_time().ok_or("lever deadline")?;
        assert!((900_000..=1_100_000).contains(&(source_due-due)));
        assert_eq!(map.facts.expire_movement_items(source_due-1),0);
        assert_eq!(map.facts.expire_movement_items(source_due),1);
        assert_eq!(map.facts.object_revision(source),2);
        assert_eq!(map.facts.next_movement_decay_time(),None);
        let tile=map.overlay.base().tile(CONTROL.x,CONTROL.y,CONTROL.floor).ok_or("control tile")?;
        let id=*tile.ids().get(1).ok_or("lever palette")?;
        assert_eq!(map.facts.base_entry(crate::map::overlay::TilePos { x:CONTROL.x,y:CONTROL.y,floor:CONTROL.floor },1,id).ok_or("lever facts")?.appearance_id,2772);
        let corner=map.overlay.base().tile(32760,32288,-14).ok_or("corner")?;
        assert_eq!(corner.ids().len(),2,"the absent1027 is never created");
    }
    owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("clock")?;
    let response=super::quest_world_callbacks_fixtures::wire(owner,admitted,3,2,use_source(0)?).await?;
    assert_eq!(oteryn_protocol_oteryn::world_object::decode_use_result(&response).map_err(debug)?,UseDisposition::StaleState);
    assert_eq!(owner.bundle_map().ok_or("map")?.facts.next_movement_decay_time(),None);
    assert_eq!(counts(harness).await?,initial,"a world switch grants no item or quest receipt");
    Ok(())
}

async fn counts(harness:&Harness)->TestResult<(i64,i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions)").fetch_one(&harness.pool).await?)
}

pub(super) const DREAD_LORD_LEVER: Position = Position { x: 32781, y: 32225, floor: -14 };
pub(super) const DREAD_LORD_START: Position = Position { x: 32782, y: 32226, floor: -14 };

pub(super) fn dread_lord_world(
    native: &NativeGameplayState,
    callback: &Callback,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{NativeCrop, native_scene_with_appearances};
    let [_, active] = callback.lever.ok_or("Dread Lord lever appearances")?;
    native_scene_with_appearances(native, vec![NativeCrop {
        bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content/world/placements/region-z14-x128-y125.b3")),
        region: (14, 128, 125),
        bounds: (32781, 32225, 32783, 32227),
        start: (DREAD_LORD_START.x, DREAD_LORD_START.y),
        expected: vec![],
    }], &[], &[active])
}

pub(super) async fn exercise_dread_lord(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    callback: &Callback,
) -> TestResult {
    use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestTransition};
    use std::sync::Arc;

    let [offering] = callback.offerings.as_slice() else { return Err("diamond offering".into()) };
    let [rank, _claim] = callback.storage.as_slice() else { return Err("rank and claim guards".into()) };
    let [resting_lever, _active_lever] = callback.lever.ok_or("source lever")?;
    let lava = callback.effects.item_presence.as_ref().ok_or("lava")?;
    let lava_tile = crate::map::overlay::TilePos {
        x: offering.position.x,
        y: offering.position.y,
        floor: offering.position.floor,
    };
    let lava_count = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<usize> {
        let map = owner.bundle_map().ok_or("map")?;
        let items = map.facts.movement_items(lava_tile);
        let mut count = 0;
        for entry in items.iter().filter(|entry| entry.facts.appearance_id == lava.appearance) {
            assert!(!entry.facts.pickupable);
            assert!(!entry.facts.blocks_projectile);
            count += 1;
        }
        Ok(count)
    };
    let carried = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON c.item=i.item_instance_id",
        )
        .bind(fence.character_id.as_bytes().to_vec())
        .bind(super::fixture::id(42).to_vec())
        .fetch_one(&harness.pool)
    };
    let before_carried = carried().await?;
    let before = counts(harness).await?;
    super::quest_world_callbacks_fixtures::offering(owner, fence, offering, 90001).await?;
    async fn pull(
        owner: &mut ComposedFreshAdmission<'_, '_, '_>,
        admitted: &mut AdmittedSession,
        appearance: u16,
    ) -> TestResult<UseDisposition> {
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let target = super::questpc7_hoist_story_fixtures::target(owner, DREAD_LORD_LEVER, appearance)?;
        let command = admitted.continuity.next_command_id;
        let reply = super::quest_world_callbacks_fixtures::wire(owner, admitted, command, 2, target).await?;
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
    }
    assert_eq!(pull(owner, admitted, resting_lever).await?, UseDisposition::Rejected);
    assert_eq!(counts(harness).await?, before);
    assert_eq!(lava_count(owner)?, 0);

    // Exercise the stored rank through the fenced Quest writer and the native USE path.
    let transition = "oteryn:quest-transition/fixture/dread-lord-rank";
    let catalogue = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?.clone()
        .with_fixture_transition(QuestTransition {
            key: transition.into(),
            quest: callback.quest.clone(),
            effects: vec![QuestEffect {
                track: rank.track.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(rank.minimum),
            }],
            completes: false,
            experience: None,
            world_change: None,
        }).map_err(debug)?;
    owner.quest_catalogue = Some(Arc::new(catalogue));
    super::qengine_outfit_stories_fixtures::commit_fixture(owner, admitted, transition, 90002).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    assert_eq!(pull(owner, admitted, resting_lever).await?, UseDisposition::Committed);
    assert_eq!(lava_count(owner)?, 1);

    let map = owner.bundle_map().ok_or("map")?;
    let due = map.facts.next_movement_decay_time().ok_or("lever deadline")?;
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(carried().await?, before_carried);
    assert_eq!(lava_count(owner)?, 1);
    let after = counts(harness).await?;
    assert_eq!(after.1, before.1 + 1);
    assert_eq!(pull(owner, admitted, resting_lever).await?, UseDisposition::Rejected);
    assert_eq!(counts(harness).await?, after);
    assert_eq!(carried().await?, before_carried);
    assert_eq!(lava_count(owner)?, 1);
    eprintln!("POSTGRES_CASE_COMPLETED dread-lord-lever");
    Ok(())
}
