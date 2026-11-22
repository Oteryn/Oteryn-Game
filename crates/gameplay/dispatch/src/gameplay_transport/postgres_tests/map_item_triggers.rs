//! Native removal and arrival consequences through the live item-move dispatcher.
use super::fixture::{Harness, debug, id};
use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop, native_scene};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::{
    connection::{FirstEntryOutcome, FreshAdmissionAuthority, UseCommand},
    item_move::{LoopMove, LoopMoveRefusal, LoopMoveReplay, LoopMoveResult, LoopTarget},
    item_view::ItemKey,
};
use crate::map::overlay::TilePos;
use oteryn_gameplay_world::map::view::MapFacts;

#[test]
fn parchment_removal_starts_four_demons_once_and_schedules_the_seal_reset() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_scene(Scene::Parchment)
}

#[test]
fn scarab_coin_basin_atomically_consumes_the_drop_and_routes_the_flame_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_scene(Scene::CoalBasin { occupant: true })
}

#[test]
fn scarab_coin_basin_consumes_the_drop_without_a_flame_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_scene(Scene::CoalBasin { occupant: false })
}

#[derive(Clone, Copy)]
enum Scene {
    Parchment,
    CoalBasin { occupant: bool },
}

fn run_scene(scene: Scene) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |admin| {
        // The seal goes into the backpack, so pickup preflight needs the actual
        // durable level to derive carry capacity. A Ground-only move does not.
        let initialized = matches!(scene, Scene::Parchment);
        let h = Harness::create_with_admission_seed(admin, "map_item_trigger", initialized, false)
            .await?;
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
        let generation = controller.active().ok_or("content generation")?;
        let native = generation.native_gameplay().ok_or("native content")?;
        let (origin, world) = match scene {
            Scene::Parchment => {
                let origin = MovementLocalPosition {
                    x: 33062,
                    y: 31625,
                    floor: 15,
                };
                let world = native_crop(
                    native,
                    NativeCrop {
                        bytes: include_bytes!(concat!(
                            env!("CARGO_MANIFEST_DIR"),
                            "/../../../content/world/placements/region-z15-x129-y123.b3"
                        )),
                        region: (15, 129, 123),
                        bounds: (33059, 31622, 33068, 31629),
                        start: (33062, 31625),
                        expected: vec![ExpectedTile {
                            position: (33063, 31624),
                            ids: vec![353, 2474, 2819],
                            counted_entry: None,
                        }],
                    },
                )?;
                (origin, world)
            }
            Scene::CoalBasin { occupant } => {
                let origin = MovementLocalPosition {
                    x: 33097,
                    y: if occupant { 32816 } else { 32815 },
                    floor: 13,
                };
                let bytes = include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z13-x129-y128.b3"
                ));
                let world = native_scene(
                    native,
                    vec![
                        NativeCrop {
                            bytes,
                            region: (13, 129, 128),
                            bounds: (33097, 32815, 33099, 32817),
                            start: (33097, 32815),
                            expected: vec![
                                ExpectedTile {
                                    position: (33098, 32816),
                                    ids: vec![415, 2114],
                                    counted_entry: None,
                                },
                                ExpectedTile {
                                    position: (33097, 32816),
                                    ids: vec![10145, 1959],
                                    counted_entry: None,
                                },
                            ],
                        },
                        NativeCrop {
                            bytes,
                            region: (13, 129, 128),
                            bounds: (33093, 32824, 33094, 32825),
                            start: (33093, 32824),
                            expected: vec![ExpectedTile {
                                position: (33093, 32824),
                                ids: vec![423],
                                counted_entry: None,
                            }],
                        },
                    ],
                    &[],
                )?;
                (origin, world)
            }
        };
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let frame = world.frame_binding_digest(deps.admitted.world_id);
        let room = activated
            .qualified_room()
            .bundle_view(
                cells.clone(),
                frame,
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: origin.x,
                    y: origin.y,
                    floor: origin.floor,
                },
            )
            .ok_or("tomb bundle room")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);

        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "quest fence")?
            .ok_or("quest fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(frame.as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i2854".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;
        deps.initialize(&owner).await;
        match scene {
            Scene::Parchment => exercise_parchment(&owner, &h, &deps, admitted, &world).await?,
            Scene::CoalBasin { occupant } => {
                owner.quest_catalogue = Some(std::sync::Arc::new(
                    oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                        .map_err(debug)?
                        .catalogue()
                        .clone(),
                ));
                owner.admit_progression_session(&mut admitted).await;
                owner.admit_quest_session(&admitted).await;
                exercise_basin(&owner, &h, &deps, admitted, occupant).await?;
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn exercise_parchment(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    deps: &Dependencies,
    admitted: super::AdmittedSession,
    world: &crate::map::boot::BundleWorld,
) -> TestResult {
    let placement = crate::map::view::placement_key(
        TilePos {
            x: 33063,
            y: 31624,
            floor: -15,
        },
        2,
    )
    .ok_or("source seal placement")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 1,
        item_fence: admitted.item_fence,
    };
    let request = LoopMove {
        item: ItemKey::MapBase {
            bundle_digest: world.base().digest(),
            placement_key: placement,
            reset_epoch: 0,
        },
        target: LoopTarget::MainBackpack,
        count: None,
        slot: None,
    };
    let text = world
        .facts()
        .base_text(placement)
        .ok_or("seal inscription")?;
    assert_eq!(
        owner.move_loop_item(deps.actor, command, request).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        owner.committed_loop_move(deps.actor, command).await,
        Some(LoopMoveReplay::Moved)
    );
    assert_eq!(
        owner
            .move_loop_item(
                deps.actor,
                UseCommand {
                    command_id: 2,
                    ..command
                },
                request
            )
            .await,
        LoopMoveResult::Refused(LoopMoveRefusal::Stale)
    );
    let native = owner.runtime.lock().await;
    let mut positions = native
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .map(|actor| {
            native
                .read_actor_position(actor)
                .map(|snapshot| {
                    let at = snapshot.position();
                    (at.x, at.y, at.floor)
                })
                .map_err(debug)
        })
        .collect::<Result<Vec<_>, _>>()?;
    positions.sort_unstable();
    assert_eq!(
        positions,
        [
            (33060, 31623, 15),
            (33060, 31627, 15),
            (33066, 31623, 15),
            (33066, 31627, 15)
        ]
    );
    drop(native);
    let (has_deadline, inscription): (bool, String) = sqlx::query_as(
            "SELECT p.reset_at BETWEEN r.created_at + interval '2 hours' AND r.created_at + interval '2 hours 1 second', t.text \
             FROM game_parchment_removals p JOIN game_item_loop_move_receipts r ON r.transaction_id=p.move_transaction_id \
             JOIN game_map_item_mint_receipts m ON m.transaction_id=p.mint_transaction_id \
             JOIN game_item_texts t ON t.item_instance_id=m.item_instance_id",
        ).fetch_one(&h.pool).await?;
    assert!(has_deadline);
    assert_eq!(inscription, text);
    // Advance the persisted deadline in this disposable database. The next
    // move still goes through the live reset admission and the normal MINT.
    let mut tx = h.pool.begin().await?;
    sqlx::query(
        "ALTER TABLE game_parchment_removals DISABLE TRIGGER game_parchment_removal_immutable",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE game_parchment_removals SET reset_at=clock_timestamp()-interval '1 second'",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "ALTER TABLE game_parchment_removals ENABLE TRIGGER game_parchment_removal_immutable",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    owner.states.loop_ground.invalidate();
    assert_eq!(
        owner
            .move_loop_item(
                deps.actor,
                UseCommand {
                    command_id: 3,
                    ..command
                },
                request
            )
            .await,
        LoopMoveResult::Committed
    );
    let (mints, linked): (i64, i64) = sqlx::query_as(
            "SELECT count(*), count(parchment_previous) FROM game_map_item_mint_receipts WHERE placement_key=$1::text::numeric",
        ).bind(placement.to_string()).fetch_one(&h.pool).await?;
    assert_eq!((mints, linked), (2, 1));

    Ok(())
}

async fn exercise_basin(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    deps: &Dependencies,
    admitted: super::AdmittedSession,
    occupant: bool,
) -> TestResult {
    use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
    let position = Position {
        x: 33098,
        y: 32816,
        floor: -13,
    };
    let fence = admitted.item_fence.ok_or("item fence")?;
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence,
        &Offering {
            position,
            item: "oteryn:item.tibia.i3042".into(),
            count: 3,
        },
        59,
        true,
    )
    .await?;
    let item: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3042'",
    ).fetch_one(&h.pool).await?;
    let item = <[u8; 16]>::try_from(item).map_err(|_| "item id")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 2,
        item_fence: Some(fence),
    };
    let request = LoopMove {
        item: ItemKey::Instance(item),
        target: LoopTarget::Ground(oteryn_protocol_oteryn::item_view::WorldTilePosition {
            x: i32::from(position.x),
            y: i32::from(position.y),
            floor: i16::from(position.floor),
        }),
        count: Some(2),
        slot: None,
    };
    assert_eq!(
        owner.move_loop_item(deps.actor, command, request).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        owner.committed_loop_move(deps.actor, command).await,
        Some(LoopMoveReplay::Moved)
    );
    // Assert the real route and Ground custody before touching new receipt
    // columns so the same mounted cases demonstrate the behaviour on main.
    let at = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    assert_eq!(
        (at.x, at.y, at.floor),
        if occupant {
            (33093, 32824, 13)
        } else {
            (33097, 32815, 13)
        }
    );
    let ground_coins: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3042'"
    ).fetch_one(&h.pool).await?;
    assert_eq!(ground_coins, 0);
    let (exact_tx, has_player, quantity, remains, ground): (bool, bool, i64, i64, i64) = sqlx::query_as(
        "SELECT o.created_xact_id=r.created_xact_id,o.basin_recipient IS NOT NULL, (o.before_item->>'quantity')::bigint, \
         (SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid), \
         (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id=o.item_instance_id) \
         FROM game_dreamer_ring_offerings o JOIN game_item_loop_move_receipts r ON r.transaction_id=o.move_transaction_id WHERE o.basin_uid=9033",
    ).bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        (exact_tx, has_player, quantity, remains, ground),
        (true, occupant, 2, 1, 0)
    );
    let progress: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE track_key='oteryn:quest-progress/quest/u7_4/the_ancient_tombs/default_start'",
    ).fetch_optional(&h.pool).await?;
    assert_eq!(progress, occupant.then_some(1));
    if occupant {
        {
            let mut runtime = deps.runtime.lock().await;
            let at = runtime.read_actor_position(deps.actor).map_err(debug)?;
            let map = owner.bundle_map().ok_or("coal basin world")?;
            let doors = crate::map::floor::BundleDoors(&map.facts);
            let prepared = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                owner.movement_cells(),
                deps.actor,
                admitted.game_session_id,
                at,
                MovementLocalPosition {
                    x: 33097,
                    y: 32816,
                    floor: 13,
                },
                false,
                &doors,
            )
            .map_err(debug)?;
            runtime.commit_source_step(prepared).map_err(debug)?;
        }
        let request = LoopMove {
            count: None,
            ..request
        };
        sqlx::query("CREATE FUNCTION reject_test_coal_basin() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'fixture rejects debit' USING ERRCODE='23514'; END $$")
            .execute(&h.pool).await?;
        sqlx::query("CREATE TRIGGER reject_test_coal_basin BEFORE INSERT ON game_dreamer_ring_offerings FOR EACH ROW WHEN (NEW.basin_uid=9033) EXECUTE FUNCTION reject_test_coal_basin()")
            .execute(&h.pool).await?;
        assert_eq!(
            owner
                .move_loop_item(
                    deps.actor,
                    UseCommand {
                        command_id: 3,
                        ..command
                    },
                    request
                )
                .await,
            LoopMoveResult::Refused(LoopMoveRefusal::Rejected)
        );
        let (carried, receipts): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT i.quantity FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_dreamer_ring_offerings WHERE basin_uid=9033)",
        ).bind(item.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!((carried, receipts), (1, 1));
        let position = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        assert_eq!((position.x, position.y, position.floor), (33097, 32816, 13));
        sqlx::query("DROP TRIGGER reject_test_coal_basin ON game_dreamer_ring_offerings")
            .execute(&h.pool)
            .await?;
    } else {
        let ordinary = UseCommand {
            command_id: 3,
            ..command
        };
        let ordinary_drop = LoopMove {
            target: LoopTarget::Ground(oteryn_protocol_oteryn::item_view::WorldTilePosition {
                x: 33097,
                y: 32815,
                floor: -13,
            }),
            count: None,
            ..request
        };
        assert_eq!(
            owner.move_loop_item(deps.actor, ordinary, ordinary_drop).await,
            LoopMoveResult::Committed
        );
        let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
        let replay = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            owner.committed_loop_move(deps.actor, ordinary),
        )
        .await?;
        drop(permit);
        assert_eq!(replay, Some(LoopMoveReplay::Moved));
    }
    Ok(())
}
