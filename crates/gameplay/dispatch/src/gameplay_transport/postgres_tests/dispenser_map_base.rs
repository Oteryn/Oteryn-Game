use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition, NodeId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::boot::{BootPins, boot, tests::bundle_with_item_ids};
use crate::map::facts::ItemDefinition;
use crate::map::overlay::{TilePos, map_revision};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaskCase {
    Fill,
    Capacity,
    Step,
}

#[test]
fn map_cask_capacity_refusal_leaves_the_authored_source_untouched() -> TestResult {
    run_case(CaskCase::Capacity, "map_cask_capacity_refusal_leaves_the_authored_source_untouched")
}

#[test]
fn retired_map_cask_allows_the_next_step_after_a_lost_acknowledgement() -> TestResult {
    run_case(CaskCase::Step, "retired_map_cask_allows_the_next_step_after_a_lost_acknowledgement")
}

#[test]
fn flask_on_map_base_cask_spends_only_available_charges_and_replays() -> TestResult {
    run_case(CaskCase::Fill, "flask_on_map_base_cask_spends_only_available_charges_and_replays")
}

fn run_case(case: CaskCase, test: &'static str) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin, case, test))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "map-base dispenser scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String, case: CaskCase, test: &str) -> TestResult {
    const CASK: &str = "oteryn:item.tibia.i25880";
    const FLASK: &str = "oteryn:item.tibia.i283";
    const BAG: &str = "oteryn:item.tibia.i2854";
    let harness =
        Harness::create_with_admission_seed(admin, "dispenser_map_base", true, false).await?;
    eprintln!("arenapkg-marker: {test} db ready");
    super::reset();
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
    let active = controller.active().ok_or("active generation")?;
    let index = active
        .native_gameplay()
        .and_then(|native| native.item_index())
        .ok_or("item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let (bytes, pins) = bundle_with_item_ids(
        (BAG, reference(BAG).ok_or("backpack definition")?.get() - 1),
        (CASK, reference(CASK).ok_or("cask definition")?.get() - 1),
    );
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let start = TilePos {
        x: 6,
        y: 0,
        floor: -7,
    };
    let map = boot(
        &bytes,
        &BootPins {
            map_revision: map_revision(&base),
            bundle: pins,
            start,
        },
        deps.admitted.world_id,
        deps.admitted.channel_id,
        |key| {
            Some(ItemDefinition {
                reference: reference(key)?,
                solid: Some(key == CASK),
                blocks_projectile: false,
                pickupable: key == BAG,
                door: false,
            })
        },
    )
    .map_err(debug)?;
    let cells = map
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let room = super::rust_lottery_fixtures::bundle_action_room(
        pin.qualified_room(),
        &map,
        deps.admitted.world_id,
        &cells,
    )?;
    let content_pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (6, 0, 7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        content_pin,
    )
    .map_err(debug)?;
    let session = deps.admitted.game_session_id;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(
            deps.actor,
            MovementLocalPosition {
                x: 6,
                y: 0,
                floor: 7,
            },
        )
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(&harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
    deps.initialize(&owner).await;
    let current = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(session)
        .await?
        .0;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice())
        .execute(&harness.pool).await?;
    let mut entries = vec![(FLASK, 2), (FLASK, 3)];
    if case == CaskCase::Capacity {
        entries.resize(19, ("oteryn:item.tibia.i43872", 1));
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("UPDATE game_character_progression_state SET level=1 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    super::item_look_fixtures::seed_main_backpack(&harness, &entries).await?;

    let pos = TilePos { x: 7, ..start };
    assert!(!map.enterable(pos), "the authored cask blocks the tile");
    let placement_key = crate::map::view::placement_key(pos, 1).ok_or("cask placement")?;
    let placement = [base.digest().as_slice(), &placement_key.to_be_bytes()].concat();
    let map_intent = UseIntent {
        target: UseTarget::Item(1_u64.try_into()?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    };
    let command = |command_id| UseCommand {
        game_session_id: session,
        command_id,
        item_fence: Some(fence),
    };
    let first = owner
        .use_craft(
            deps.actor,
            command(1),
            map_intent.clone(),
            Some(ItemKey::Instance(id(71))),
            None,
        )
        .await
        .ok_or("map cask USE-WITH outcome")?;
    if case == CaskCase::Capacity {
        assert_eq!(first.disposition, UseDisposition::NotPossible);
        let effects: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_map_item_mint_receipts),
             (SELECT count(*) FROM game_item_ground_locations),
             (SELECT count(*) FROM game_item_consume_transform_records),
             (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid)"
        ).bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(effects, (0, 0, 0, 2), "the refused fill has no physical or receipt side effects");
        return Ok(());
    }
    assert_eq!(first.disposition, UseDisposition::Committed);

    let (cask, charges): (Vec<u8>, i32) = sqlx::query_as(
        "SELECT uuid_send(m.item_instance_id),t.charges FROM game_map_item_mint_receipts m
         JOIN game_item_timed_states t USING(item_instance_id)",
    )
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(charges, 998);
    let cask_id: [u8; 16] = cask.as_slice().try_into()?;
    if case == CaskCase::Fill {
        use crate::durability::spell_item_transaction as items;
        use crate::durability::spell_items_abi::SpellGroundTarget;

        let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
        let target = SpellGroundTarget::for_native_tile_read(
            &room,
            &*owner.runtime.lock().await,
            MovementLocalPosition {
                x: i32::from(pos.x),
                y: i32::from(pos.y),
                floor: -i16::from(pos.floor),
            },
        )?;
        let mut tx = harness.pool.begin().await?;
        let scope = items::assert_spell_item_scope_in_transaction(
            &mut tx,
            &permit,
            owner.root,
            owner.character,
            owner.holder,
            fence.runtime_scope,
            fence.scope_ownership_generation.get(),
        )
        .await
        .map_err(debug)?;
        let empty = items::read_creature_empty_tiles_in_transaction(
            &mut tx,
            &scope,
            std::slice::from_ref(&target.spatial_position),
        )
        .await
        .map_err(debug)?;
        assert!(
            empty.is_empty(),
            "a monster must read the occupied cask tile"
        );
        let tile = items::read_creature_scope_tile_in_transaction(&mut tx, &scope, &target)
            .await
            .map_err(debug)?;
        assert!(
            tile.items
                .iter()
                .any(|item| item.item_instance_id == cask_id && item.blocks_movement)
        );
        tx.rollback().await?;
    }
    let placement_states = || {
        harness.root.read_map_item_mint_placement_states(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            map.base().digest(),
            0,
            map.base().entry_count(),
        )
    };
    assert_eq!(
        placement_states().await.map_err(debug)?,
        [(placement_key, false)]
    );
    let ground = crate::gameplay_transport::loop_move::loop_ground(&owner, map.base(), Some(index))
        .await
        .ok_or("cask Ground projection")?;
    assert_eq!(
        ground.overlay().ground_entry(&cask_id).map(|(at, _)| at),
        Some(pos)
    );
    assert!(
        ground
            .overlay()
            .tile(pos)
            .is_some_and(|tile| tile.is_hidden(1))
    );

    assert!(!map.enterable(pos), "the live cask still blocks the tile");

    // Seed the state of a cask with one charge left after previous fills.
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_item_timed_states SET charges=1 WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(&cask)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let last_intent = UseIntent {
        target: UseTarget::Item(2_u64.try_into()?),
        use_with_map_item: Some(MapUseTarget::Item(3_u64.try_into()?)),
        ..map_intent.clone()
    };
    let acknowledged = std::sync::Arc::new(tokio::sync::Notify::new());
    {
        let fill = owner.use_craft(
            deps.actor,
            command(2),
            last_intent.clone(),
            Some(ItemKey::Instance(id(72))),
            Some(ItemKey::Instance(cask_id)),
        );
        let fill = crate::gameplay_transport::item_craft::DISPENSER_ACK_BARRIER
            .scope(std::sync::Arc::clone(&acknowledged), fill);
        tokio::pin!(fill);
        tokio::select! {
            result = &mut fill => return Err(format!("last charge returned before cancellation: {result:?}").into()),
            ready = tokio::time::timeout(std::time::Duration::from_secs(5), acknowledged.notified()) => ready?,
        }
    }
    if case == CaskCase::Step {
        let retired: bool = sqlx::query_scalar(
            "SELECT lifecycle=2 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(&cask).fetch_one(&harness.pool).await?;
        assert!(retired, "the last charge committed before cancellation");
        assert!(!map.enterable(pos), "eager reconciliation has not cleared the tile");
        let (step, _) = owner.paced_step(
            deps.actor, session,
            oteryn_protocol_oteryn::world_spatial::StepDirection::East,
        ).await;
        assert!(step.moved_to.is_some(), "ordinary STEP must reconcile the retired cask before collision");
        assert!(map.enterable(pos), "STEP clears the retired cask collision");
        return Ok(());
    }
    let depleted = crate::gameplay_transport::loop_move::loop_ground(&owner, map.base(), Some(index))
        .await
        .ok_or("Ground projection after lost last-charge acknowledgement")?;
    assert!(depleted.overlay().ground_entry(&cask_id).is_none());
    assert!(map.enterable(pos));
    assert_eq!(
        placement_states().await.map_err(debug)?,
        [(placement_key, true)]
    );
    for (number, intent) in [(1, map_intent), (2, last_intent)] {
        let replay = owner
            .use_craft(deps.actor, command(number), intent, None, None)
            .await
            .ok_or("receipt replay without handles")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
    }

    let cask_state: (i16, bool) = sqlx::query_as(
        "SELECT i.lifecycle, EXISTS(SELECT 1 FROM game_item_ground_locations g
         WHERE g.item_instance_id=i.item_instance_id) FROM game_item_instances i
         WHERE i.item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(&cask)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(cask_state, (2, false));
    let flask_quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1",
    ).bind(id(72).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(flask_quantity, 2);
    let potion_quantity: i64 = sqlx::query_scalar(
        "SELECT sum(i.quantity)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id)
         WHERE i.definition_production_key='oteryn:item.tibia.i236' AND i.lifecycle=1 AND e.parent_item_instance_id=encode($1,'hex')::uuid",
    ).bind(id(70).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(potion_quantity, 3);
    let receipts: Vec<(i64, i32, i32)> = sqlx::query_as(
        "SELECT source_count, (effects->'dispenser'->>'before')::integer, (effects->'dispenser'->>'after')::integer
         FROM game_item_consume_transform_records WHERE recipe='oteryn:dispenser.fill' ORDER BY command_id",
    ).fetch_all(&harness.pool).await?;
    assert_eq!(receipts, [(2, 1000, 998), (1, 1, 0)]);
    let ground = crate::gameplay_transport::loop_move::loop_ground(&owner, map.base(), Some(index))
        .await
        .ok_or("depleted cask Ground projection")?;
    assert!(ground.overlay().ground_entry(&cask_id).is_none());
    assert!(
        ground
            .overlay()
            .tile(pos)
            .is_some_and(|tile| tile.is_hidden(1))
    );
    assert!(
        map.enterable(pos),
        "depletion clears the authored collision"
    );
    Ok(())
}
