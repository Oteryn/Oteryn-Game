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
    retire: bool,
    rebuild: bool,
) -> TestResult {
    let [offering] = callback.offerings.as_slice() else { return Err("diamond offering".into()) };
    let [rank, _claim] = callback.storage.as_slice() else { return Err("rank and claim guards".into()) };
    let [resting_lever, active_lever] = callback.lever.ok_or("source lever")?;
    let count_outputs = |map: &crate::map::boot::BundleMap| {
        map.facts.movement_item_entries().iter().filter(|(position, entry)| {
            (position.x, position.y, position.floor) ==
                (offering.position.x, offering.position.y, offering.position.floor)
                && entry.facts.appearance_id == 7845
                && !entry.facts.pickupable
        }).count()
    };
    let ground_outputs = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<usize> {
        Ok(count_outputs(owner.bundle_map().ok_or("map")?))
    };
    let before_mints: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources")
        .fetch_one(&harness.pool).await?;
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
    assert_eq!(ground_outputs(owner)?, 0);

    // Both returning players have the authored prerequisite in their saved progress.
    let prerequisite = [(rank.track.clone(), rank.minimum)];
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        harness, owner, admitted, &prerequisite, 152,
    )
    .await
    .map_err(|error| format!("first Dread Lord saved rank: {error}"))?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;

    let mut peer = dread_lord_peer(owner, admitted, harness)
        .await
        .map_err(|error| format!("second Dread Lord admission: {error}"))?;
    owner.admit_progression_session(&mut peer).await;
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        harness, owner, &peer, &prerequisite, 153,
    )
    .await
    .map_err(|error| format!("second Dread Lord saved rank: {error}"))?;
    owner.refresh_quest_session(peer.game_session_id, false).await;

    owner
        .states
        .quest_callbacks
        .interrupt_after_commit
        .store(true, std::sync::atomic::Ordering::Release);
    assert!(pull(owner, admitted, resting_lever).await.is_err());
    assert!(owner.has_pending_quest_callback(admitted.game_session_id));
    assert_eq!(counts(harness).await?.1, before.1 + 1);
    assert_eq!(ground_outputs(owner)?, 0);

    if rebuild {
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        use crate::map::boot::BundleCollisionIndexExt;
        let native = owner.active_generation.and_then(|active| active.native_gameplay())
            .ok_or("native gameplay")?;
        let rebuilt = dread_lord_world(native, callback)?;
        let cells = rebuilt.movement_cells(owner.movement_cells()).map_err(debug)?;
        let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
            return Err("rebuilt Dread Lord map".into());
        };
        let map = index.map();
        assert_eq!(count_outputs(map), 0);
        owner.restore_paid_quest_scene(map).await.ok_or("paid scene recovery")?;
        assert_eq!(count_outputs(map), 1, "the claim's scene survives a rebuilt map");
        assert_eq!(counts(harness).await?.1, before.1 + 1);
        assert_eq!(ground_outputs(owner)?, 0, "the old map never published the scroll");
        return Ok(());
    }

    // Supply a new diamond so the second Dread Lord would otherwise be eligible.
    super::quest_world_callbacks_fixtures::offering(owner, fence, offering, 90004).await?;
    if retire {
        use crate::gameplay_transport::connection::GraceExpiryResult;

        let transport = admitted.controller.ok_or("Dread Lord controller")?.transport;
        assert_eq!(
            owner
                .release_terminal(
                    *admitted,
                    crate::gameplay_transport::TerminalRelease::StaffKick(transport),
                )
                .await,
            GraceExpiryResult::Released
        );
        assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
        assert_eq!(ground_outputs(owner)?, 1, "the paid effect survives retirement");
        assert_eq!(
            pull(owner, &mut peer, active_lever).await?,
            UseDisposition::NothingToUse
        );
        assert_eq!(counts(harness).await?.1, before.1 + 1);
        assert_eq!(ground_outputs(owner)?, 1);
        return Ok(());
    }
    let competing_pull = pull(owner, &mut peer, resting_lever).await?;
    let resumed = owner.reconcile_quest_callbacks(admitted.game_session_id).await;
    // Without ITEM_USE_V1 the wire encodes exhaustion as a generic refusal.
    assert_eq!((competing_pull, resumed), (UseDisposition::Rejected, true));
    assert!(!owner.has_pending_quest_callback(admitted.game_session_id));

    let map = owner.bundle_map().ok_or("map")?;
    let due = map.facts.next_movement_decay_time().ok_or("lever deadline")?;
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(ground_outputs(owner)?, 1);
    let after = counts(harness).await?;
    assert_eq!(after.1, before.1 + 1);
    assert_eq!(pull(owner, admitted, resting_lever).await?, UseDisposition::Rejected);
    assert_eq!(counts(harness).await?, after);
    assert_eq!(ground_outputs(owner)?, 1);

    assert_eq!(pull(owner, &mut peer, resting_lever).await?, UseDisposition::Committed);
    assert_eq!(counts(harness).await?, (after.0 + 1, after.1 + 1));
    assert_eq!(ground_outputs(owner)?, 1, "the second claim reuses the scene item");
    let map = owner.bundle_map().ok_or("map")?;
    let second_due = map.facts.next_movement_decay_time().ok_or("second lever deadline")?;
    assert_eq!(map.facts.expire_movement_items(second_due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(second_due), 1);
    assert_eq!(ground_outputs(owner)?, 1);

    use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
    use crate::map::boot::BundleCollisionIndexExt;
    let native = owner.active_generation.and_then(|active| active.native_gameplay())
        .ok_or("native gameplay")?;
    let rebuilt = dread_lord_world(native, callback)?;
    let cells = rebuilt.movement_cells(owner.movement_cells()).map_err(debug)?;
    let NativeMovementCollisionIndex::Bundle(index) = cells.index() else {
        return Err("rebuilt Dread Lord map".into());
    };
    let recovered = index.map();
    owner.restore_paid_quest_scene(recovered).await.ok_or("paid scene recovery")?;
    assert_eq!(count_outputs(recovered), 1, "both receipts restore the same scene item");
    owner.restore_paid_quest_scene(recovered).await.ok_or("paid scene replay")?;
    assert_eq!(count_outputs(recovered), 1, "replay preserves the shared scene item");
    assert_eq!(counts(harness).await?.1, before.1 + 2);

    let after_mints: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources")
        .fetch_one(&harness.pool).await?;
    assert_eq!(after_mints, before_mints);
    eprintln!("POSTGRES_CASE_COMPLETED dread-lord-lever");
    Ok(())
}

async fn dread_lord_peer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    harness: &Harness,
) -> TestResult<AdmittedSession> {
    use super::fixture::id;
    use crate::gameplay_transport::connection::{ControllerBinding, SessionContinuity};
    use crate::world_party_runtime_postgres_cases::{
        FreshPlayerOwners, committed_fresh_player_from_source_owned,
    };

    let _source = super::house_transfer_fixtures::RecoverySource::start(harness).await?;
    let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
        .fetch_one(&harness.pool).await?;
    let (current, _) = committed_fresh_player_from_source_owned(
        &FreshPlayerOwners {
            root: harness.root.clone(),
            node: harness.node.clone(),
            recovery: &harness.recovery,
        },
        60,
        63,
        171,
        now,
        Some("platform-test"),
    )
    .await?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("peer fence")?;
    let session = fence.game_session_id;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                super::quest_world_callbacks_fixtures::local(DREAD_LORD_START),
            )
            .map_err(debug)?;
        actor
    };
    Ok(AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("peer transport")?,
            account_id: id(63),
        }),
        continuity: SessionContinuity {
            selected_capabilities: admitted.continuity.selected_capabilities,
            ..SessionContinuity::FRESH
        },
        item_fence: Some(fence),
        ..*admitted
    })
}
