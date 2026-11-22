//! Command-2 rope USE through the composed movement and durable item owners.
use super::fixture::{Harness, debug, id, run};
use super::item_craft_fixtures::exchange;
use super::quest_callbacks_fixtures::source_world_spatial;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
use crate::foundation::{
    ChannelContentPin, ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition, NodeId,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::map::overlay::{TilePos, map_revision};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[derive(Clone, Copy)]
enum Case {
    Creature,
    Item,
    Empty,
}

#[test]
fn rope_pull_moves_the_creature_before_the_item_below_the_hole() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_scenario_thread(admin, Case::Creature)
}

#[test]
fn rope_pull_moves_the_top_ground_stack_one_floor_above_the_stairs() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_scenario_thread(admin, Case::Item)
}

#[test]
fn rope_pull_refuses_an_empty_hole() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_scenario_thread(admin, Case::Empty)
}

fn on_scenario_thread(admin: String, case: Case) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(&admin, case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "rope pull PostgreSQL scenario panicked")??;
    Ok(())
}

fn ground_bytes(x: i32, y: i32, floor: i16) -> Vec<u8> {
    [
        x.to_be_bytes().as_slice(),
        y.to_be_bytes().as_slice(),
        floor.to_be_bytes().as_slice(),
    ]
    .concat()
}

fn scenario(admin: &str, case: Case) -> TestResult {
    // Crystal 201189f8 register_actions.lua holeId includes 394 (open hole) and 435 (stairs).
    const HOLE: TilePos = TilePos {
        x: 101,
        y: 100,
        floor: -7,
    };
    const BELOW: MovementLocalPosition = MovementLocalPosition {
        x: 101,
        y: 100,
        floor: 8,
    };
    const LANDING: MovementLocalPosition = MovementLocalPosition {
        x: 101,
        y: 101,
        floor: 7,
    };
    const ISSUER: MovementLocalPosition = MovementLocalPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let (tool, opening) = match case {
        Case::Creature => ("oteryn:item.tibia.i3003", 394),
        Case::Item => ("oteryn:item.tibia.i9598", 435),
        Case::Empty => ("oteryn:item.tibia.i646", 394),
    };
    run(async |_| {
        super::reset();
        let h = Harness::create_with_admission_seed(admin, "rope_pull", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        // The fixture uses Crystal appearances and real bundle geometry, with ordinary
        // ground on both floors and the donor's south-first landing open.
        let world = source_world_spatial(
            (100, 100, -7),
            &[
                (HOLE.x, HOLE.y, HOLE.floor, 0, opening),
                (102, 101, -8, 0, 386),
            ],
        )?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (ISSUER.x, ISSUER.y, ISSUER.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, ISSUER)
            .map_err(debug)?;
        let creature = if matches!(case, Case::Creature) {
            Some(
                runtime
                    .admit_pinned_monster_lab_creature(BELOW, "oteryn:creature.tibia.rat", 20)
                    .map_err(debug)?,
            )
        } else {
            None
        };
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        deps.initialize(&owner).await;

        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
                .ok_or("USE capabilities")?;

        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        super::item_look_fixtures::seed_main_backpack(
            &h,
            &[
                ("oteryn:item.tibia.i3031", 5),
                ("oteryn:item.tibia.i3031", 9),
                (tool, 1),
            ],
        )
        .await?;
        let map = owner.bundle_map().ok_or("rope test map")?;
        let lower = ground_bytes(BELOW.x, BELOW.y, i16::from(HOLE.floor - 1));
        let upper = ground_bytes(LANDING.x, LANDING.y, i16::from(HOLE.floor));
        if !matches!(case, Case::Empty) {
            let permit = owner.spell_lane_permit().await.ok_or("Ground seed lane")?;
            let content_revision = format!(
                "sha256:{}",
                generation
                    .identity()
                    .server_artifact_digest()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            for (item, command) in [(71, 101), (72, 102)] {
                h.root
                    .commit_item_loop_move(
                        &permit,
                        &authority,
                        &h.node,
                        &fence,
                        CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(command).map_err(debug)?,
                        ),
                        &NativeItemWeights(native),
                        LoopMoveRequest {
                            item: id(item),
                            count: None,
                            pickup_cell: None,
                            destination: LoopDestination::Ground(LoopGround {
                                spatial_position: lower.clone(),
                                map_revision: map_revision(world.base()),
                                content_revision: content_revision.clone(),
                                placement_context: activated.frame_binding().digest().to_vec(),
                            }),
                        },
                    )
                    .await?
                    .map_err(debug)?;
            }
        }
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(
            &crate::map::view::placement_key(HOLE, 1)
                .ok_or("opening")?
                .to_be_bytes(),
        );
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(
                index
                    .definition_ref("Item", tool, "definition-r1")
                    .ok_or("rope tool")?,
            ),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision: 0,
            })),
            use_with_position: None,
        })
        .map_err(debug)?;
        let (_, result, _) = exchange(
            &owner,
            admitted,
            admitted.continuity.next_command_id,
            &payload,
            None,
        )
        .await?;
        assert_eq!(
            result,
            if matches!(case, Case::Empty) {
                UseDisposition::NotPossible
            } else {
                UseDisposition::Committed
            }
        );
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            ISSUER
        );
        if let Some(creature) = creature {
            assert_eq!(
                runtime
                    .read_actor_position(creature)
                    .map_err(debug)?
                    .position(),
                LANDING
            );
        }
        drop(runtime);
        let mut ground = h
            .root
            .read_item_loop_ground_for_scope(fence.runtime_scope)
            .await?;
        ground.sort_by_key(|row| row.item.item_instance_id);
        if matches!(case, Case::Empty) {
            assert!(ground.is_empty());
        } else {
            assert_eq!(ground.len(), 2);
            assert_eq!(ground[0].item.ground.spatial_position, lower);
            assert_eq!(
                ground[1].item.ground.spatial_position,
                if matches!(case, Case::Item) {
                    upper
                } else {
                    lower
                }
            );
            assert_eq!(ground[1].item.quantity, 9);
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
