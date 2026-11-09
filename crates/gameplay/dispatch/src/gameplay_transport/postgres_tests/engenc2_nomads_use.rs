//! Actual native window USE must refuse before a quest receipt when crates are absent.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;
#[test]
fn engenc2_nomads_native_use_refuses_missing_ground_before_quest_receipt() -> TestResult {
    scenario(Scenario::Window)
}
#[test]
fn engenc2_nomads_flat_source_crate_materializes_once_postgres() -> TestResult {
    scenario(Scenario::Crate)
}
#[test]
fn engenc2_nomads_amphora_ground_relocation_refuses_inventory_postgres() -> TestResult {
    scenario(Scenario::Amphora)
}
#[test]
fn engenc2_nomads_native_amphora_push_dispatch_postgres() -> TestResult {
    scenario(Scenario::SourceAmphora)
}
#[derive(Clone, Copy)]
enum Scenario {
    Window,
    Crate,
    Amphora,
    SourceAmphora,
}
fn scenario(mode: Scenario) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("engenc2-nomads-use", move || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "engenc2_nomads_use", true, false)
                .await?;
            prepare_current_session(&h).await?;
            let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
                .into_iter()
                .find(|c| c.key == "oteryn:quest-callback/engenc2/nomads-land-window")
                .ok_or("production window")?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native profiles")?;
            let start = if matches!(mode, Scenario::SourceAmphora) {
                (33217, 32854)
            } else {
                (33216, 32856)
            };
            let world = super::l6_world_pickups::native_crop(
                native,
                super::l6_world_pickups::NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z09-x129-y128.b3"
                    )),
                    region: (9, 129, 128),
                    bounds: (33213, 32852, 33220, 32859),
                    start,
                    expected: vec![super::l6_world_pickups::ExpectedTile {
                        position: (33216, 32855),
                        ids: vec![7358, 1744],
                        counted_entry: None,
                    }],
                },
            )?;
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
                        x: i32::from(start.0),
                        y: i32::from(start.1),
                        floor: 9,
                    },
                )
                .ok_or("bundle room")?;
            let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                world.map_revision_digest(),
                (i32::from(start.0), i32::from(start.1), 9),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                64,
                pin,
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
                        x: i32::from(start.0),
                        y: i32::from(start.1),
                        floor: 9,
                    },
                )
                .map_err(debug)?;
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback.clone()])
                    .is_ok()
            );
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            admitted.item_fence = Some(fence.item_fence());
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
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
            // Replacing the fixture runtime does not transfer the admitted Character lease.
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
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            owner.admit_quest_session(&admitted).await;

            if matches!(mode, Scenario::SourceAmphora) {
                use crate::gameplay_transport::item_move::{
                    LoopMove, LoopMoveRefusal, LoopMoveResult, LoopTarget,
                };
                use crate::gameplay_transport::item_view::ItemKey;
                let key = oteryn_world_bundle::bundle::placement_key(-9, 33218, 32853, 1)
                    .ok_or("amphora key")?;
                let source = ItemKey::MapBase {
                    bundle_digest: world.base().digest(),
                    placement_key: key,
                    reset_epoch: 0,
                };
                let command = |id| -> TestResult<UseCommand> {
                    Ok(UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: id,
                        item_fence: Some(fence.item_fence()),
                    })
                };
                let result = owner
                    .move_loop_item(
                        deps.actor,
                        command(900)?,
                        LoopMove {
                            item: source,
                            count: None,
                            target: LoopTarget::Ground(
                                oteryn_protocol_oteryn::item_view::WorldTilePosition {
                                    x: 33216,
                                    y: 32854,
                                    floor: -9,
                                },
                            ),
                        },
                    )
                    .await;
                assert_eq!(
                    result,
                    LoopMoveResult::Committed,
                    "native empty amphora must push through the real dispatcher"
                );
                let (item, cell): (Vec<u8>, Vec<u8>) = sqlx::query_as(
                    "SELECT uuid_send(g.item_instance_id),g.spatial_position FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i2904' LIMIT 2"
                ).fetch_one(&h.pool).await?;
                let item: [u8; 16] = item.try_into().map_err(|_| "amphora GUID")?;
                assert_eq!(
                    cell,
                    [
                        33216_i32.to_be_bytes().as_slice(),
                        32854_i32.to_be_bytes().as_slice(),
                        (-9_i16).to_be_bytes().as_slice()
                    ]
                    .concat()
                );
                let repeated = owner
                    .move_loop_item(
                        deps.actor,
                        command(901)?,
                        LoopMove {
                            item: source,
                            count: None,
                            target: LoopTarget::MainBackpack,
                        },
                    )
                    .await;
                assert_eq!(repeated, LoopMoveResult::Refused(LoopMoveRefusal::Stale));
                let refused = owner
                    .move_loop_item(
                        deps.actor,
                        command(902)?,
                        LoopMove {
                            item: ItemKey::Instance(item),
                            count: None,
                            target: LoopTarget::MainBackpack,
                        },
                    )
                    .await;
                assert_eq!(
                    refused,
                    LoopMoveResult::Refused(LoopMoveRefusal::NotSupported)
                );
                let receipts: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_map_item_mint_receipts")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(receipts, 1);
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }

            if !matches!(mode, Scenario::Window) {
                let amphora = matches!(mode, Scenario::Amphora);
                let source_y = if amphora { 32853 } else { 32854 };
                use crate::durability::map_item_mint::{
                    FlatMapCrateProof, MapItemMintOutcome, MapItemMintRequest,
                };
                let definition = crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: if amphora {
                        "oteryn:item.tibia.i2904"
                    } else {
                        "oteryn:item.tibia.i2471"
                    }
                    .into(),
                    revision_ref: "definition-r1".into(),
                };
                let item = if amphora {
                    oteryn_gameplay_items::item_ref_admission::generation_ground_item_facts(
                        native,
                        &definition,
                    )
                } else {
                    crate::gameplay_transport::item_ref_admission::generation_item_facts(
                        native,
                        &definition,
                    )
                }
                .map_err(debug)?;
                let placement_key =
                    oteryn_world_bundle::bundle::placement_key(-9, 33218, source_y, 1)
                        .ok_or("source crate key")?;
                let tile = world
                    .base()
                    .tile(33218, source_y, -9)
                    .ok_or("source tile")?;
                let proof = if amphora {
                    None
                } else {
                    Some(
                        FlatMapCrateProof::qualify(
                            &item,
                            world.base().digest(),
                            placement_key,
                            tile.depths(),
                        )
                        .ok_or("native empty source crate proof")?,
                    )
                };
                let request = MapItemMintRequest {
                    flat_crate: proof,
                    command: crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(900).map_err(debug)?,
                    ),
                    base_bundle_digest: world.base().digest(),
                    placement_key,
                    reset_epoch: 0,
                    item,
                    quantity: 1,
                    content_revision: format!(
                        "sha256:{}",
                        native
                            .source_digest()
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    ),
                    ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION
                        .into(),
                    sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
                };
                let content =
                    crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
                        .with_wheel(owner.premium_coordinator);
                let destination = if amphora {
                    crate::durability::item_loop_move::LoopDestination::Ground(
                        crate::durability::item_loop_move::LoopGround {
                            spatial_position: [
                                33217_i32.to_be_bytes().as_slice(),
                                32854_i32.to_be_bytes().as_slice(),
                                (-9_i16).to_be_bytes().as_slice(),
                            ]
                            .concat(),
                            map_revision: crate::map::overlay::map_revision(world.base()),
                            content_revision: request.content_revision.clone(),
                            placement_context: frame.to_vec(),
                        },
                    )
                } else {
                    crate::durability::item_loop_move::LoopDestination::MainBackpack
                };
                assert_eq!(
                    owner
                        .root
                        .preflight_map_item_loop_pickup(
                            owner.character,
                            owner.holder,
                            &fence.item_fence(),
                            &content,
                            &request,
                            &destination,
                        )
                        .await
                        .map_err(debug)?,
                    Ok(()),
                    "the production preflight must admit the qualified crate"
                );
                let mut unqualified = request.clone();
                unqualified.flat_crate = None;
                if amphora {
                    unqualified.quantity = 2;
                }
                let mut frozen = owner
                    .root
                    .freeze_map_item_mint(
                        owner.character,
                        owner.holder,
                        fence.item_fence(),
                        request,
                    )
                    .await
                    .map_err(debug)?;
                let permit = owner.spell_lane_permit().await.ok_or("crate lane")?;
                let first = owner
                    .root
                    .commit_map_item_mint(
                        &permit,
                        owner.character,
                        owner.holder,
                        fence.item_fence(),
                        &mut frozen,
                    )
                    .await
                    .map_err(debug)?;
                let MapItemMintOutcome::Committed(first) = first else {
                    return Err("empty crate must materialize".into());
                };
                let conflict = owner
                    .root
                    .freeze_map_item_mint(
                        owner.character,
                        owner.holder,
                        fence.item_fence(),
                        unqualified,
                    )
                    .await;
                assert!(
                    matches!(
                        conflict,
                        Err(crate::durability::map_item_mint::MapItemMintError::ConflictingCause)
                    ),
                    "the empty-source witness must be part of the frozen intent"
                );
                let replay = owner
                    .root
                    .commit_map_item_mint(
                        &permit,
                        owner.character,
                        owner.holder,
                        fence.item_fence(),
                        &mut frozen,
                    )
                    .await
                    .map_err(debug)?;
                let MapItemMintOutcome::AlreadyCommitted(replay) = replay else {
                    return Err("map crate replay must reuse receipt".into());
                };
                assert_eq!(first.item_instance_id, replay.item_instance_id);
                let receipts: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_map_item_mint_receipts")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(receipts, 1);
                let moved = owner
                    .root
                    .commit_item_loop_move(
                        &permit,
                        owner.character,
                        owner.holder,
                        &fence.item_fence(),
                        frozen.request().command,
                        &content,
                        crate::durability::item_loop_move::LoopMoveRequest {
                            item: first.item_instance_id,
                            count: None,
                            pickup_cell: Some(
                                [
                                    33218_i32.to_be_bytes().as_slice(),
                                    i32::from(source_y).to_be_bytes().as_slice(),
                                    (-9_i16).to_be_bytes().as_slice(),
                                ]
                                .concat(),
                            ),
                            destination: destination.clone(),
                        },
                    )
                    .await
                    .map_err(debug)?;
                assert!(
                    moved.is_ok(),
                    "qualified empty crate must enter backpack: {moved:?}"
                );
                let remaining_ground: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(remaining_ground, i64::from(amphora));
                let carried: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid)"
                ).bind(first.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(carried, !amphora);
                if let crate::durability::item_loop_move::LoopDestination::Ground(to) = destination
                {
                    let refused = owner.root.commit_item_loop_move(
                        &permit, owner.character, owner.holder, &fence.item_fence(),
                        crate::foundation::CommandRef::new(admitted.game_session_id, crate::foundation::CommandId::new(901).map_err(debug)?),
                        &content,
                        crate::durability::item_loop_move::LoopMoveRequest {
                            item: first.item_instance_id, count: None, pickup_cell: Some(to.spatial_position),
                            destination: crate::durability::item_loop_move::LoopDestination::MainBackpack,
                        },
                    ).await.map_err(debug)?;
                    assert!(
                        matches!(
                            refused,
                            Err(crate::durability::item_loop_move::LoopMoveRefusal::NotSupported)
                        ),
                        "Ground-only amphora cannot enter inventory: {refused:?}"
                    );
                }
                drop(permit);
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let key = oteryn_world_bundle::bundle::placement_key(-9, 33216, 32855, 1)
                .ok_or("window placement")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let outcome = owner
                .use_object(
                    deps.actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 1,
                        item_fence: admitted.item_fence,
                    },
                    WorldObjectTarget {
                        placement,
                        expected_revision: 0,
                    },
                )
                .await;
            assert_eq!(
                outcome.disposition,
                UseDisposition::Rejected,
                "the actual window must reach its physical Ground guard"
            );
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                receipts, 0,
                "a refused arrangement must create no quest receipt"
            );

            // Initial physical fixtures follow the existing item harness convention.
            // Re-enable all guards before observing the actual USE/quest transaction.
            let mut seed = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *seed)
                .await?;
            let mut number = 150_u8;
            let content = format!(
                "sha256:{}",
                generation
                    .identity()
                    .server_artifact_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            for requirement in &callback.world_requirements {
                let oteryn_gameplay_quests::quest::world_callbacks::WorldRequirement::Ground {
                    position,
                    ground,
                } = requirement
                else {
                    return Err("Ground pattern".into());
                };
                let mut spatial = i32::from(position.x).to_be_bytes().to_vec();
                spatial.extend_from_slice(&i32::from(position.y).to_be_bytes());
                spatial.extend_from_slice(&i16::from(position.floor).to_be_bytes());
                for item in &ground.stacks {
                    number += 1;
                    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
    .bind(id(number).as_slice()).bind(id(42).as_slice()).bind(&item.item).bind(&item.revision).bind(id(160).as_slice()).execute(&mut *seed).await?;
                    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,$9)")
    .bind(id(number).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&spatial).bind(vec![1_u8;16]).bind(world.map_revision()).bind(&content).bind(frame.as_slice()).bind(i64::from(number)).execute(&mut *seed).await?;
                }
            }
            seed.commit().await?;
            // A refused USE still starts the normal owner-clock cooldown.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(3))
                .ok_or("owner clock")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let accepted = owner
                .use_object(
                    deps.actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 2,
                        item_fence: admitted.item_fence,
                    },
                    WorldObjectTarget {
                        placement,
                        expected_revision: 0,
                    },
                )
                .await;
            assert_eq!(
                accepted.disposition,
                UseDisposition::Committed,
                "the same real USE must commit once physical Ground matches"
            );
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
            )
            .bind(&callback.key)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(count, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "ENGENC2_NOMADS_USE_PG_EXECUTED elapsed_s={:.6} success={}",
        started.elapsed().as_secs_f64(),
        result.is_ok()
    );
    result
}
