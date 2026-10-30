//! TOOLS-2: ordinary USE → durable terrain transform → replay/reload → database deadline.
use super::fixture::{Harness, debug, id, run};
use super::item_craft::exchange;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn terrain_tool_wire_durable_replay_and_deadline_revert() -> TestResult {
    run_case("oteryn:item.tibia.i3457", 867, 868)
}

#[test]
fn m08_machete_cuts_jungle_grass_through_wire_and_recovers_regrowth() -> TestResult {
    run_case("oteryn:item.tibia.i3308", 3696, 3695)
}

#[test]
fn m08_machete_cuts_grass_through_wire_and_recovers_regrowth() -> TestResult {
    run_case("oteryn:item.tibia.i3308", 3702, 3701)
}

fn run_case(tool: &'static str, before: u16, after: u16) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(tool, before, after).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "terrain tool e2e panic")??;
    Ok(())
}

fn scenario(tool: &str, before: u16, after: u16) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "terrain_tool", true, false).await?;
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
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let target_key = format!("oteryn:item.tibia.i{before}");
        let closed_hole = target_key.as_str();
        let bag = "oteryn:item.tibia.i2854";
        let shovel = tool;
        for key in [shovel, closed_hole, &format!("oteryn:item.tibia.i{after}")] {
            assert!(
                native.item_policy(key, "definition-r1").is_some(),
                "native terrain profile {key}"
            );
        }
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (
                closed_hole,
                reference(closed_hole).ok_or("closed_hole")?.get() - 1,
            ),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let boot_pins = crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 6,
                y: 0,
                floor: -7,
            },
        };
        // The production boot policy: an unknown fact blocks and is not pickupable.
        let definitions = |key: &str| {
            let policy = native.item_policy(key, "definition-r1").map(|p| p.record());
            Some(crate::map::facts::ItemDefinition {
                reference: reference(key)?,
                solid: policy.and_then(|p| p.attributes.blocks_movement),
                blocks_projectile: policy.and_then(|p| p.attributes.blocks_projectile).unwrap_or(true),
                pickupable: policy.is_some_and(|p| matches!(&p.semantics.physical, crate::content::ReferenceItemField::Known(physical) if physical.pickupable == crate::content::ReferenceItemField::Known(true))),
                door: false,
            })
        };
        let map = crate::map::boot::boot(
            &bytes,
            &boot_pins,
            deps.admitted.world_id,
            deps.admitted.channel_id,
            definitions,
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (6, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
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
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        super::item_look::seed_main_backpack(
            &harness,
            &[(shovel, 1), ("oteryn:item.tibia.i3577", 1)],
        )
        .await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("craft offer selectable")?;
        let mut placement = map.base().digest().to_vec();
        let target_pos = crate::map::overlay::TilePos {
            x: 7,
            y: 0,
            floor: -7,
        };
        placement.extend_from_slice(
            &crate::map::view::placement_key(target_pos, 1)
                .ok_or("target placement")?
                .to_be_bytes(),
        );
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // The wire helper resolves the current shovel handle from the inventory snapshot.
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("source")?),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(target.clone())),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let (mut next, result, payload) =
            exchange(&owner, admitted, 1, &payload, reference(shovel))
                .await
                .map_err(|e| format!("initial use: {e}"))?;
        assert_eq!(result, UseDisposition::Committed);
        let state: (i64,i64,bool) = sqlx::query_as("SELECT state_item_id,object_revision,decay_at_unix_ms > floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_terrain_tool_transforms").fetch_one(&harness.pool).await?;
        assert_eq!(state, (i64::from(after), 1, true));
        let key = crate::map::view::placement_key(target_pos, 1).ok_or("key")?;
        assert_eq!(map.facts().pinned().object_revision(key), 1);
        // A new owner recovers the committed state and original receipt without current handles.
        drop(owner);
        let restored_map = crate::map::boot::boot(
            &bytes,
            &boot_pins,
            deps.admitted.world_id,
            deps.admitted.channel_id,
            definitions,
        )
        .map_err(debug)?;
        assert_eq!(restored_map.facts().pinned().object_revision(key), 0);
        let restored_cells = restored_map
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let mut restarted = deps.owner(&harness, &authority);
        restarted.active_generation = Some(active);
        restarted.movement_cells = &restored_cells;
        deps.initialize(&restarted).await;
        let (_, result, _) = exchange(&restarted, admitted, 1, &payload, None)
            .await
            .map_err(|e| format!("replay: {e}"))?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT object_revision FROM game_terrain_tool_transforms"
            )
            .fetch_one(&harness.pool)
            .await?,
            1
        );
        // Still before the durable deadline: a server clock drain cannot revert early.
        restarted.refresh_terrain_tools().await;
        assert_eq!(restored_map.facts().pinned().object_revision(key), 1);
        assert_eq!(
            restored_map
                .facts()
                .pinned()
                .base_entry(
                    target_pos,
                    1,
                    reference(closed_hole).ok_or("closed reference")?.get() - 1
                )
                .ok_or("recovered entry")?
                .appearance_id,
            after
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT state_item_id FROM game_terrain_tool_transforms")
                .fetch_one(&harness.pool)
                .await?,
            i64::from(after)
        );
        // Test clock arrangement: advance the persisted deadline; no wall-clock sleep or bypass
        // of the production clock path is needed.
        sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1").execute(&harness.pool).await?;
        restarted.refresh_terrain_tools().await;
        let state: (i64,i64,bool) = sqlx::query_as("SELECT state_item_id,object_revision,decay_at_unix_ms IS NULL FROM game_terrain_tool_transforms").fetch_one(&harness.pool).await?;
        assert_eq!(state, (i64::from(before), 2, true));
        assert_eq!(restored_map.facts().pinned().object_revision(key), 2);
        assert_eq!(
            restored_map
                .facts()
                .pinned()
                .base_entry(
                    target_pos,
                    1,
                    reference(closed_hole).ok_or("closed reference")?.get() - 1
                )
                .ok_or("reverted entry")?
                .appearance_id,
            before
        );
        restarted.refresh_terrain_tools().await;
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT object_revision FROM game_terrain_tool_transforms"
            )
            .fetch_one(&harness.pool)
            .await?,
            2
        );
        // A stale map revision cannot start another transform or create another receipt.
        next.continuity.next_command_id = 2;
        let (_, result, _) = exchange(&restarted, next, 2, &payload, reference(shovel))
            .await
            .map_err(|e| format!("stale target: {e}"))?;
        assert_eq!(result, UseDisposition::StaleState);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        // A changed payload cannot use the historical receipt's CommandRef.
        let mut changed = intent.clone();
        changed.use_with_map_item = Some(MapUseTarget::Object(WorldObjectTarget {
            expected_revision: 2,
            ..target.clone()
        }));
        // Transport closes on altered duplicate CommandRef bytes before item dispatch. Probe
        // the durable boundary directly as well, so its own receipt binding is qualified.
        let permit = restarted.spell_lane_permit().await.ok_or("permit")?;
        let scope = crate::durability::terrain_tool::TerrainScope {
            map_revision: crate::map::overlay::map_revision(restored_map.base()),
            content_generation: native.source_digest(),
            reset_epoch: 0,
        };
        let result = harness
            .root
            .apply_terrain_tool(
                &permit,
                &authority,
                restarted.holder,
                fence,
                crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(1).map_err(debug)?,
                ),
                encode_item_use_intent(&changed).map_err(debug)?,
                scope.clone(),
                None,
            )
            .await?
            .ok_or("changed replay outcome")?;
        assert_eq!(result, UseDisposition::Rejected);
        // Valid map and tool facts, but a stale lease: the durable owner must refuse.
        let mut stale_fence = fence;
        stale_fence.character_lease_generation += 1;
        let command = crate::foundation::CommandRef::new(
            admitted.game_session_id,
            crate::foundation::CommandId::new(3).map_err(debug)?,
        );
        let mutation = crate::durability::terrain_tool::TerrainMutation {
            tool: id(71),
            tool_key: shovel.into(),
            tool_revision: "definition-r1".into(),
            placement: target.placement.clone(),
            expected_revision: 2,
            before: u32::from(before),
            after: u32::from(after),
            decay: vec![(
                u32::from(before),
                if before == 867 { 600_000 } else { 300_000 },
            )],
        };
        let result = harness
            .root
            .apply_terrain_tool(
                &permit,
                &authority,
                restarted.holder,
                stale_fence,
                command,
                encode_item_use_intent(&changed).map_err(debug)?,
                scope.clone(),
                Some(mutation.clone()),
            )
            .await?
            .ok_or("stale fence outcome")?;
        assert_eq!(result, UseDisposition::Rejected);
        let absent_tool = crate::durability::terrain_tool::TerrainMutation {
            tool: id(99),
            ..mutation.clone()
        };
        let result = harness
            .root
            .apply_terrain_tool(
                &permit,
                &authority,
                restarted.holder,
                fence,
                command,
                encode_item_use_intent(&changed).map_err(debug)?,
                scope.clone(),
                Some(absent_tool),
            )
            .await?
            .ok_or("absent tool outcome")?;
        assert_eq!(result, UseDisposition::StaleState);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        // The same request with only current authority and real carried custody succeeds.
        let result = harness
            .root
            .apply_terrain_tool(
                &permit,
                &authority,
                restarted.holder,
                fence,
                command,
                encode_item_use_intent(&changed).map_err(debug)?,
                scope,
                Some(mutation),
            )
            .await?
            .ok_or("current fence outcome")?;
        drop(permit);
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_terrain_tool_receipts")
                .fetch_one(&harness.pool)
                .await?,
            2
        );
        drop(restarted);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
