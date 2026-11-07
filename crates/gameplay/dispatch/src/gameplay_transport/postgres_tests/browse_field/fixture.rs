use super::super::fixture::{Harness, debug, id, run};
use super::super::{Dependencies, item_look, prepare_current_session};
use super::{BAG, COIN, Case, TestResult, wire};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::boot::{BootPins, boot};
use crate::map::facts::ItemDefinition;
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::browse_field::CAPABILITY_BROWSE_FIELD_V1;

pub(super) fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "browse_field", true, false).await?;
        super::super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        deps.room = pin.qualified_room().clone();
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        let index = native.item_index().ok_or("item index")?;
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (BAG, reference(BAG).ok_or("bag ref")?.get() - 1),
            (COIN, reference(COIN).ok_or("coin ref")?.get() - 1),
        );
        let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        let sector = bundle.sectors.first_mut().ok_or("sector")?;
        let tile = sector.tiles.iter_mut().find(|tile| tile.x == 7).ok_or("field")?;
        let coin = tile.items.get(1).ok_or("coin template")?.clone();
        tile.items.truncate(1);
        if case == Case::Paging {
            // Crystal Container::createBrowseField has a thirty-entry page.
            // Thirty-one authored entries also exceed domain 17's ten-entry cut.
            tile.items.extend(std::iter::repeat_n(coin, 31));
        }
        let mut empty = tile.clone();
        empty.items.truncate(1);
        for x in [8, 9] {
            empty.x = x;
            sector.tiles.push(empty.clone());
        }
        let bytes =
            oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
                .map_err(debug)?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?.digest;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: TilePos { x: 6, y: 0, floor: -7 },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = world.movement_cells(deps.room.movement_cells()).map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            world.map_revision_digest(),
            (6, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1, 1, 1, "runtime-scope-assignment:1", 2, content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_pinned_test_position(
            deps.actor, MovementLocalPosition { x: 6, y: 0, floor: 7 },
        ).map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        ).current_session_at(deps.admitted.game_session_id).await?.0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        item_look::seed_main_backpack(&harness, &[(COIN, 7), (BAG, 1)]).await?;
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(), &[4, 6, 12, 14, 18, CAPABILITY_BROWSE_FIELD_V1],
        ).ok_or("browse capability selectable")?;
        if case == Case::Backpack {
            use crate::gameplay_transport::container_view::ContainerObservation;
            use crate::gameplay_transport::item_view::ItemKey;
            let field = ItemKey::BrowseField { x: 7, y: 0, floor: -7 };
            assert!(matches!(
                owner.observe_container(deps.actor, Some(fence), field).await,
                Some(ContainerObservation::BrowseField { .. })
            ));
            assert!(owner.observe_container(deps.actor, None, field).await.is_none());
            // Change one authority component at a time, retaining the real active session
            // and source map. A stale view must not publish entries under an old fence.
            let mut stale_lease = fence;
            stale_lease.character_lease_generation += 1;
            let mut stale_connection = fence;
            stale_connection.connection_generation =
                crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
            for stale in [stale_lease, stale_connection] {
                assert!(matches!(
                    owner.observe_container(deps.actor, Some(stale), field).await,
                    Some(ContainerObservation::Gone)
                ));
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_loop_move_receipts")
                    .fetch_one(&harness.pool).await?,
                0,
            );
        }
        let replay = wire::exercise(&owner, admitted, &harness, case).await?;
        drop(owner);
        if let Some(payload) = replay {
            // Recreate the composed owner and the connection. The old handles cannot resolve,
            // but the existing item-loop receipt still answers without moving anything twice.
            let mut restarted = deps.owner(&harness, &authority);
            restarted.active_generation = Some(generation);
            restarted.movement_cells = &cells;
            deps.initialize(&restarted).await;
            admitted.continuity.item_view.handle_counter = 1000;
            admitted.continuity.next_command_id = 1;
            wire::replay(&restarted, admitted, &harness, &payload).await?;
            drop(restarted);
        }
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
