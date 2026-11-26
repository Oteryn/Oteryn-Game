//! Nomads Land's native gestures through the Channel and the PostgreSQL owners.
use super::fixture::{Harness, debug, id, run};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_use::fluid::{ItemFluidChange, ItemFluidRequest};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveReplay, LoopMoveResult, LoopTarget};
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{
    Offering, Position,
    nomads_ritual::{CRACK, ENTRANCE, SCIMITAR, WATER, WATER_SUBTYPE},
};
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const FLASK: &str = "oteryn:item.tibia.i2874";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    PourReach,
    ExactOffering,
    InterruptedDrop,
    InterruptedPour,
}

#[test]
fn nomads_ritual_pours_from_each_walkable_water_neighbour() -> TestResult {
    scenario(Case::PourReach)
}

#[test]
fn nomads_ritual_runtime_consumes_only_the_moved_scimitar() -> TestResult {
    scenario(Case::ExactOffering)
}

#[test]
fn nomads_ritual_resumes_an_interrupted_drop() -> TestResult {
    scenario(Case::InterruptedDrop)
}

#[test]
fn nomads_ritual_resumes_an_interrupted_pour() -> TestResult {
    scenario(Case::InterruptedPour)
}

fn scenario(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("nomads-ritual", move || {
        run(async |admin| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "nomads_ritual", false, false).await?;
            prepare_current_session(&h).await?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native profiles")?;
            let start = (CRACK.x + 1, CRACK.y);
            let world = super::l6_world_pickups_fixtures::native_scene_with_appearances(
                native,
                vec![super::l6_world_pickups_fixtures::NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z06-x129-y127.b3"
                    )),
                    region: (6, 129, 127),
                    bounds: (CRACK.x - 1, WATER.y - 2, WATER.x + 2, WATER.y + 2),
                    start,
                    expected: Vec::new(),
                }],
                &[],
                &[5858, 7181],
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
                        floor: -i16::from(CRACK.floor),
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
                (
                    i32::from(start.0),
                    i32::from(start.1),
                    -i16::from(CRACK.floor),
                ),
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
                        floor: -i16::from(CRACK.floor),
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

            let place =
                super::quest_world_callbacks_fixtures::callback("nomads-land-place-scimitar")?;
            let open =
                super::quest_world_callbacks_fixtures::callback("nomads-land-open-entrance")?;
            let primed = place.effects.transform.ok_or("transformed crack")?;
            let opened = open.effects.transform.ok_or("transformed mountain")?;
            let mountain = open.selector.appearance.ok_or("mountain appearance")?;
            super::rottin_rewards_fixtures::seed_tracks(
                &h,
                &owner,
                &admitted,
                &[(&place.storage[0].track, place.storage[0].minimum)],
            )
            .await?;
            let command = |command_id| UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            };
            let command_id = admitted.continuity.next_command_id;
            if matches!(case, Case::ExactOffering | Case::InterruptedDrop) {
                let [offering] = place.offerings.as_slice() else {
                    return Err("one source offering".into());
                };
                let older = if case == Case::ExactOffering {
                    super::quest_world_callbacks_fixtures::input_item(
                        &owner,
                        fence.item_fence(),
                        offering,
                        61,
                        false,
                    )
                    .await?;
                    Some(live_item(&h, SCIMITAR, None).await?)
                } else {
                    None
                };
                super::quest_world_callbacks_fixtures::input_item(
                    &owner,
                    fence.item_fence(),
                    offering,
                    62,
                    true,
                )
                .await?;
                let dropped = live_item(&h, SCIMITAR, older.as_ref()).await?;
                if let Some(older) = older {
                    assert!(older < dropped, "fixture must put the older scimitar first");
                }
                if case == Case::InterruptedDrop {
                    owner.fixture_interrupt_nomads_ritual(command(command_id))?;
                }
                ready(&owner).await?;
                if case == Case::ExactOffering {
                    super::journal_inventory::runtime_role(owner.root).await?;
                }
                let result = owner
                    .move_loop_item(
                        deps.actor,
                        command(command_id),
                        LoopMove {
                            slot: None,
                            item: ItemKey::Instance(dropped),
                            count: None,
                            target: LoopTarget::Ground(WorldTilePosition {
                                x: i32::from(CRACK.x),
                                y: i32::from(CRACK.y),
                                floor: i16::from(CRACK.floor),
                            }),
                        },
                    )
                    .await;
                if case == Case::InterruptedDrop {
                    assert_eq!(result, LoopMoveResult::Unknown);
                    assert_eq!(
                        appearance(&owner, CRACK, 1)?,
                        place.selector.appearance.ok_or("crack")?
                    );
                    let dropped_on_ground: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid)",
                    ).bind(dropped.as_slice()).fetch_one(&h.pool).await?;
                    assert!(dropped_on_ground, "item commit precedes the interruption");
                    ready(&owner).await?;
                    assert_eq!(
                        owner
                            .committed_loop_move(deps.actor, command(command_id))
                            .await,
                        Some(LoopMoveReplay::Moved)
                    );
                } else {
                    assert_eq!(result, LoopMoveResult::Committed);
                }
                assert_eq!(appearance(&owner, CRACK, 1)?, primed);
                let consumed: Vec<Vec<u8>> = sqlx::query_scalar(
                    "SELECT uuid_send(item_instance_id) FROM game_quest_callback_consumptions WHERE transition_key=$1",
                ).bind(&place.key).fetch_all(&h.pool).await?;
                assert_eq!(consumed, [dropped.to_vec()]);
                if let Some(older) = older {
                    assert_eq!(live_item(&h, SCIMITAR, None).await?, older);
                }
                if case == Case::InterruptedDrop {
                    let rev = owner
                        .bundle_map()
                        .ok_or("map")?
                        .facts
                        .object_revision(key(CRACK, 1)?);
                    ready(&owner).await?;
                    assert_eq!(
                        owner
                            .committed_loop_move(deps.actor, command(command_id))
                            .await,
                        Some(LoopMoveReplay::Moved)
                    );
                    assert_eq!(
                        owner
                            .bundle_map()
                            .ok_or("map")?
                            .facts
                            .object_revision(key(CRACK, 1)?),
                        rev
                    );
                }
            } else {
                super::quest_world_callbacks_fixtures::input_item(
                    &owner,
                    fence.item_fence(),
                    &Offering {
                        position: CRACK,
                        item: FLASK.into(),
                        count: 1,
                    },
                    63,
                    true,
                )
                .await?;
                let flask = live_item(&h, FLASK, None).await?;
                let map = owner.bundle_map().ok_or("map")?;
                assert!(map.facts.transform_movement_item(key(CRACK, 1)?, primed));
                let mut neighbours = Vec::new();
                for x in WATER.x - 1..=WATER.x + 1 {
                    for y in WATER.y - 1..=WATER.y + 1 {
                        let pos = TilePos {
                            x,
                            y,
                            floor: WATER.floor,
                        };
                        if (x, y) != (WATER.x, WATER.y)
                            && map
                                .overlay
                                .base()
                                .tile(x, y, WATER.floor)
                                .is_some_and(|tile| tile.walkable())
                            && world.enterable(pos)
                        {
                            neighbours.push(pos);
                        }
                    }
                }
                assert!(
                    neighbours.iter().any(|pos| pos.x > WATER.x),
                    "far side of the water is walkable"
                );
                if case == Case::InterruptedPour {
                    neighbours.truncate(1);
                }
                for (i, pos) in neighbours.into_iter().enumerate() {
                    super::qengine_kill_stories_fixtures::move_actor(
                        &owner,
                        deps.actor,
                        MovementLocalPosition {
                            x: i32::from(pos.x),
                            y: i32::from(pos.y),
                            floor: -i16::from(pos.floor),
                        },
                    )
                    .await?;
                    *owner.states.world_gate.lock().map_err(|_| "world gate")? = Default::default();
                    let pour = command(command_id + 2 * u64::try_from(i)? + 1);
                    ready(&owner).await?;
                    owner
                        .root
                        .commit_item_fluid(
                            owner.character,
                            owner.holder,
                            fence.item_fence(),
                            ItemFluidRequest {
                                command_id: pour.command_id - 1,
                                transaction_id: crate::operator_files::uuid_v7()?,
                                change: ItemFluidChange {
                                    item_instance_id: flask,
                                    definition: TypedDefinitionRef {
                                        family: "Item".into(),
                                        production_key: FLASK.into(),
                                        revision_ref: "definition-r1".into(),
                                    },
                                    fluid_before: 0,
                                    fluid_after: u8::try_from(WATER_SUBTYPE)?,
                                },
                                other: None,
                            },
                        )
                        .await
                        .map_err(debug)?;
                    let water_key = key(WATER, 0)?;
                    let target = WorldObjectTarget {
                        placement: [
                            map.overlay.base().digest().as_slice(),
                            water_key.to_be_bytes().as_slice(),
                        ]
                        .concat(),
                        expected_revision: map.facts.object_revision(water_key),
                    };
                    let mountain_key = key(ENTRANCE, 1)?;
                    let mountain_revision = map.facts.object_revision(mountain_key);
                    if case == Case::InterruptedPour {
                        owner.fixture_interrupt_nomads_ritual(pour)?;
                    }
                    ready(&owner).await?;
                    let result = owner
                        .use_fluid_map(deps.actor, pour, ItemKey::Instance(flask), target.clone())
                        .await
                        .ok_or("water pour outcome")?;
                    if case == Case::InterruptedPour {
                        assert!(
                            result.is_err(),
                            "the interrupted pour has no terminal result"
                        );
                        assert_eq!(appearance(&owner, ENTRANCE, 1)?, mountain);
                        assert_eq!(
                            fluid_transitions(&h, pour).await?,
                            1,
                            "fluid commit precedes interruption"
                        );
                        ready(&owner).await?;
                        assert_eq!(
                            owner
                                .use_fluid_map(
                                    deps.actor,
                                    pour,
                                    ItemKey::Instance(flask),
                                    target.clone()
                                )
                                .await
                                .ok_or("replayed pour")?
                                .map_err(|()| "pending replay")?
                                .disposition,
                            UseDisposition::Committed,
                        );
                    } else {
                        assert_eq!(
                            result.map(|outcome| outcome.disposition),
                            Ok(UseDisposition::Committed)
                        );
                    }
                    assert_eq!(
                        appearance(&owner, ENTRANCE, 1)?,
                        opened,
                        "pour from {pos:?}"
                    );
                    assert_eq!(
                        map.facts.object_revision(mountain_key),
                        mountain_revision + 1
                    );
                    if case == Case::InterruptedPour {
                        ready(&owner).await?;
                        let replay = owner
                            .use_fluid_map(deps.actor, pour, ItemKey::Instance(flask), target)
                            .await
                            .ok_or("duplicate pour replay")?;
                        assert_eq!(
                            replay.map(|outcome| outcome.disposition),
                            Ok(UseDisposition::Committed)
                        );
                        assert_eq!(
                            map.facts.object_revision(mountain_key),
                            mountain_revision + 1
                        );
                    }
                    assert_eq!(fluid_transitions(&h, pour).await?, 1);
                    assert!(
                        map.facts
                            .transform_movement_item(key(ENTRANCE, 1)?, mountain)
                    );
                }
            }
            eprintln!("NOMADS_RITUAL_PG_EXECUTED");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

fn key(pos: Position, ordinal: u8) -> TestResult<u64> {
    Ok(
        oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, ordinal)
            .ok_or("placement key")?,
    )
}

fn appearance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pos: Position,
    ordinal: u8,
) -> TestResult<u16> {
    let map = owner.bundle_map().ok_or("map")?;
    let tile = crate::map::view::compose_tile(
        &map.overlay,
        &map.facts,
        TilePos {
            x: pos.x,
            y: pos.y,
            floor: pos.floor,
        },
    )
    .map_err(debug)?
    .ok_or("tile")?;
    let key = key(pos, ordinal)?;
    Ok(tile
        .entries
        .iter()
        .find(|entry| entry.source.key() == crate::map::view::MapEntryKey::Base(key))
        .ok_or("tile item")?
        .facts
        .appearance_id)
}

async fn live_item(
    h: &Harness,
    definition: &str,
    except: Option<&[u8; 16]>,
) -> TestResult<[u8; 16]> {
    let instance: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1
         AND ($2::bytea IS NULL OR uuid_send(item_instance_id) <> $2)",
    ).bind(definition).bind(except.map(<[u8; 16]>::as_slice)).fetch_one(&h.pool).await?;
    instance.try_into().map_err(|_| "instance id".into())
}

async fn fluid_transitions(h: &Harness, command: UseCommand) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_fluid_transitions WHERE command_id=$1::text::numeric AND fluid_before=$2 AND fluid_after=0")
        .bind(command.command_id.to_string()).bind(WATER_SUBTYPE).fetch_one(&h.pool).await?)
}

async fn ready(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    if owner.root.has_ready_demand() {
        owner.root.maintain_ready_once().await.map_err(debug)?;
    }
    Ok(())
}
