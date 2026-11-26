//! Nomads Land's native gestures through the Channel and the PostgreSQL owners.
use super::fixture::{Harness, debug, id, run};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_use::fluid::{ItemFluidChange, ItemFluidRequest};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, UseCommand, test_use_frame,
};
use crate::gameplay_transport::item_move::{ItemMoveStep, item_move};
use crate::gameplay_transport::item_view::{ItemKey, SessionItemView};
use crate::gameplay_transport::session_ingress::{CommandAction, SessionCommandStream};
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{
    Offering, Position,
    nomads_ritual::{CRACK, ENTRANCE, SCIMITAR, WATER, WATER_SUBTYPE},
};
use oteryn_protocol_oteryn::item_view::{
    ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection, WorldTilePosition,
    decode_item_move_result_for, encode_item_move_intent_for,
};
use oteryn_protocol_oteryn::world_object::{self as use_wire, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget};
use std::sync::Arc;

const FLASK: &str = "oteryn:item.tibia.i2874";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    PourReach,
    ExactOffering,
    InterruptedDrop,
    InterruptedPour,
    PrimedDrop,
    OpenPour,
    ExpiredCrack,
    NoReferral,
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

#[test]
fn nomads_ritual_second_offering_stays_on_ground_and_crack_resets() -> TestResult {
    scenario(Case::PrimedDrop)
}

#[test]
fn nomads_ritual_second_pour_cannot_extend_the_open_entrance() -> TestResult {
    scenario(Case::OpenPour)
}

#[test]
fn nomads_ritual_pour_rolls_back_after_crack_expiry() -> TestResult {
    scenario(Case::ExpiredCrack)
}

#[test]
fn nomads_ritual_ineligible_drop_retains_its_move_result() -> TestResult {
    scenario(Case::NoReferral)
}

#[test]
fn nomads_ritual_migration_keeps_the_reviewed_sculpture_checksum() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "nomads_ritual_schema", false, false).await?;
        // SQLx SHA-384 of the already reviewed ad43ca652 migration.
        let checksum: String = sqlx::query_scalar(
            "SELECT encode(checksum,'hex') FROM _sqlx_migrations WHERE version=3260 AND success",
        ).fetch_one(&h.pool).await?;
        assert_eq!(checksum, "59469ac59f6ac6f4335b44d5154abf197243c59e9bb5faa64d1ce71c10e2b8847c6f6af6e6dde6e5e252b01f0e6f34d1");
        let schema: (bool, bool, bool) = sqlx::query_as(
            "SELECT has_table_privilege('oteryn_game_runtime','game_nomads_ritual_gestures','SELECT'),
                    has_table_privilege('oteryn_game_runtime','game_nomads_ritual_gestures','INSERT'),
                    has_table_privilege('oteryn_game_runtime','game_nomads_ritual_gestures','UPDATE')",
        ).fetch_one(&h.pool).await?;
        assert_eq!(schema, (true, true, false));
        h.cleanup().await
    })
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
            assert_eq!((primed, opened, mountain, place.selector.appearance), (5858, 7181, 1085, Some(6298)));
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    owner.offered_capabilities(), &[4, 6, 11, 15, 18],
                ).ok_or("Nomads USE and ITEM_MOVE capabilities")?;
            if case != Case::NoReferral {
                super::rottin_rewards_fixtures::seed_tracks(
                    &h,
                    &owner,
                    &admitted,
                    &[(&place.storage[0].track, place.storage[0].minimum)],
                ).await?;
            }
            let session = admitted.game_session_id;
            let command = |command_id| UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence.item_fence()),
            };
            let command_id = admitted.continuity.next_command_id;
            if matches!(case, Case::ExactOffering | Case::InterruptedDrop | Case::PrimedDrop | Case::NoReferral) {
                let [offering] = place.offerings.as_slice() else {
                    return Err("one source offering".into());
                };
                let older = if case == Case::ExactOffering {
                    super::quest_world_callbacks_fixtures::input_item(&owner, fence.item_fence(), offering, 61, false).await?;
                    let older = live_item(&h, SCIMITAR, None).await?;
                    let map = owner.bundle_map().ok_or("map")?;
                    for position in [CRACK, ENTRANCE] {
                        let placement_key = key(position, 1)?;
                        let target = WorldObjectTarget {
                            placement: [map.overlay.base().digest().as_slice(), placement_key.to_be_bytes().as_slice()].concat(),
                            expected_revision: map.facts.object_revision(placement_key),
                        };
                        assert!(owner.use_quest_callback(deps.actor, command(900), map, &target).await.is_none());
                    }
                    assert!(on_ground(&h, older).await?);
                    Some(older)
                } else {
                    None
                };
                super::quest_world_callbacks_fixtures::input_item(&owner, fence.item_fence(), offering, 62, true).await?;
                let dropped = live_item(&h, SCIMITAR, older.as_ref()).await?;
                if let Some(older) = older {
                    assert!(older < dropped, "fixture must put the older scimitar first");
                }
                let drop_command = command(command_id);
                let stream = if case == Case::InterruptedDrop {
                    owner.fixture_interrupt_nomads_ritual(drop_command)?;
                    Some(reserve(&owner, &admitted)?)
                } else if case == Case::NoReferral {
                    Some(reserve(&owner, &admitted)?)
                } else {
                    None
                };
                ready(&owner).await?;
                if case == Case::ExactOffering {
                    super::journal_inventory::runtime_role(owner.root).await?;
                }
                let result = route_drop(&owner, &admitted, drop_command, dropped).await?;
                let original = place.selector.appearance.ok_or("crack")?;
                if let Some(stream) = stream {
                    assert_eq!(appearance(&owner, CRACK, 1)?, original);
                    if case == Case::NoReferral {
                        assert!(matches!(result, ItemMoveStep::Result(ItemMoveOutcome::Moved, _)));
                        assert!(on_ground(&h, dropped).await?);
                    } else {
                        assert!(matches!(result, ItemMoveStep::Disconnect));
                        assert_eq!(consumed(&h, &place.key).await?, [dropped.to_vec()]);
                        assert_eq!(progress(&h, &place.storage[1].track).await?, 1);
                    }
                    // Lose transport before the command result reaches the wire.
                    drop(stream);
                    let payload = recover(&owner, &mut admitted, drop_command).await?;
                    assert_eq!(decode_item_move_result_for(&payload, move_selection()).map_err(debug)?, ItemMoveOutcome::Moved);
                } else {
                    assert!(matches!(result, ItemMoveStep::Result(ItemMoveOutcome::Moved, _)));
                }
                if case == Case::NoReferral {
                    assert!(on_ground(&h, dropped).await?);
                    assert_eq!(appearance(&owner, CRACK, 1)?, original);
                    assert!(consumed(&h, &place.key).await?.is_empty());
                } else {
                    assert_eq!(appearance(&owner, CRACK, 1)?, primed);
                    assert_eq!(consumed(&h, &place.key).await?, [dropped.to_vec()]);
                }
                if let Some(older) = older {
                    assert_eq!(live_item(&h, SCIMITAR, None).await?, older);
                    assert!(on_ground(&h, older).await?);
                }
                if case == Case::InterruptedDrop {
                    let map = owner.bundle_map().ok_or("map")?;
                    let rev = map.facts.object_revision(key(CRACK, 1)?);
                    assert!(matches!(route_drop(&owner, &admitted, drop_command, dropped).await?,
                        ItemMoveStep::Result(ItemMoveOutcome::Moved, _)));
                    assert_eq!(map.facts.object_revision(key(CRACK, 1)?), rev);
                }
                if case == Case::PrimedDrop {
                    let map = owner.bundle_map().ok_or("map")?;
                    let crack_key = key(CRACK, 1)?;
                    let revision = map.facts.object_revision(crack_key);
                    let due = map.facts.next_movement_decay_time().ok_or("crack timer")?;
                    let peer_position = (CRACK.x - 1..=CRACK.x + 1).flat_map(|x| {
                        (CRACK.y - 1..=CRACK.y + 1).map(move |y| TilePos { x, y, floor: CRACK.floor })
                    }).find(|pos| (pos.x, pos.y) != start && world.enterable(*pos)).ok_or("second player tile")?;
                    let peer = admit_peer(&h, &owner, admitted, peer_position).await?;
                    let peer_fence = peer.item_fence.ok_or("second player's item fence")?;
                    assert_ne!(peer_fence.character_id, fence.character_id);
                    super::rottin_rewards_fixtures::seed_tracks(
                        &h, &owner, &peer, &[(&place.storage[0].track, place.storage[0].minimum)],
                    ).await?;
                    super::quest_world_callbacks_fixtures::input_item(&owner, peer_fence, offering, 64, true).await?;
                    let second = live_item(&h, SCIMITAR, None).await?;
                    let second_command = UseCommand { game_session_id: peer.game_session_id,
                        command_id: peer.continuity.next_command_id, item_fence: peer.item_fence };
                    assert!(matches!(route_drop(&owner, &peer, second_command, second).await?,
                        ItemMoveStep::Result(ItemMoveOutcome::Moved, _)));
                    assert!(on_ground(&h, second).await?);
                    assert_eq!(consumed(&h, &place.key).await?, [dropped.to_vec()]);
                    assert_eq!(map.facts.object_revision(crack_key), revision);
                    assert_eq!(map.facts.next_movement_decay_time(), Some(due));
                    assert_eq!(map.facts.expire_movement_items(due), 1);
                    assert_eq!(appearance(&owner, CRACK, 1)?, original);
                }
            } else {
                let [offering] = place.offerings.as_slice() else { return Err("one source offering".into()); };
                super::quest_world_callbacks_fixtures::input_item(&owner, fence.item_fence(), offering, 62, true).await?;
                let scimitar = live_item(&h, SCIMITAR, None).await?;
                assert!(matches!(route_drop(&owner, &admitted, command(10_000), scimitar).await?,
                    ItemMoveStep::Result(ItemMoveOutcome::Moved, _)));
                super::quest_world_callbacks_fixtures::input_item(
                    &owner, fence.item_fence(), &Offering { position: CRACK, item: FLASK.into(), count: 1 }, 63, true,
                ).await?;
                let flask = live_item(&h, FLASK, None).await?;
                let map = owner.bundle_map().ok_or("map")?;
                assert_eq!(appearance(&owner, CRACK, 1)?, primed);
                let mut neighbours = Vec::new();
                for x in WATER.x - 1..=WATER.x + 1 {
                    for y in WATER.y - 1..=WATER.y + 1 {
                        let pos = TilePos { x, y, floor: WATER.floor };
                        if (x, y) != (WATER.x, WATER.y)
                            && map.overlay.base().tile(x, y, WATER.floor).is_some_and(|tile| tile.walkable())
                            && world.enterable(pos)
                        {
                            neighbours.push(pos);
                        }
                    }
                }
                assert!(neighbours.iter().any(|pos| pos.x > WATER.x), "far side of the water is walkable");
                if case != Case::PourReach {
                    neighbours.truncate(1);
                }
                if case == Case::OpenPour {
                    neighbours.push(neighbours[0]);
                }
                for (i, pos) in neighbours.into_iter().enumerate() {
                    let destination = MovementLocalPosition { x: i32::from(pos.x), y: i32::from(pos.y), floor: -i16::from(pos.floor) };
                    let previous = owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position();
                    if previous != destination {
                        super::qengine_kill_stories_fixtures::move_actor(&owner, deps.actor, destination).await?;
                    }
                    let pour = command(command_id + u64::try_from(i)?);
                    ready(&owner).await?;
                    owner.root.commit_item_fluid(
                        owner.character, owner.holder, fence.item_fence(),
                        ItemFluidRequest {
                            command_id: 20_000 + u64::try_from(i)?,
                            transaction_id: crate::operator_files::uuid_v7()?,
                            change: ItemFluidChange {
                                item_instance_id: flask,
                                definition: TypedDefinitionRef { family: "Item".into(), production_key: FLASK.into(), revision_ref: "definition-r1".into() },
                                fluid_before: 0,
                                fluid_after: u8::try_from(WATER_SUBTYPE)?,
                            },
                            other: None,
                        },
                    ).await.map_err(debug)?;
                    let water_key = key(WATER, 0)?;
                    let target = WorldObjectTarget {
                        placement: [map.overlay.base().digest().as_slice(), water_key.to_be_bytes().as_slice()].concat(),
                        expected_revision: map.facts.object_revision(water_key),
                    };
                    let mountain_key = key(ENTRANCE, 1)?;
                    let mountain_revision = map.facts.object_revision(mountain_key);
                    let stream = if case == Case::InterruptedPour {
                        owner.fixture_interrupt_nomads_ritual(pour)?;
                        Some(reserve(&owner, &admitted)?)
                    } else {
                        None
                    };
                    ready(&owner).await?;
                    let (mut items, handle) = item_handle(flask)?;
                    let body = use_wire::encode_item_use_intent(&UseIntent {
                        target: UseTarget::Item(handle), use_with: None,
                        use_with_map_item: Some(MapUseTarget::Object(target)), use_with_position: None,
                    }).map_err(debug)?;
                    let frame = oteryn_protocol_oteryn::encode_client_command(admitted.continuity.connection_generation,
                        &oteryn_protocol_oteryn::ClientCommandValue { command_id: pour.command_id,
                            command_type: use_wire::COMMAND_TYPE_USE_INTENT, payload: &body })?;
                    let result = if case == Case::ExpiredCrack {
                        pour_after_crack_expired(&h, &owner, &mut admitted, &frame, &mut items, flask).await?
                    } else {
                        test_use_frame(&owner, &mut admitted, &frame, &mut items).await
                    };
                    if let Some(stream) = stream {
                        assert!(result.is_none(), "the interrupted pour has no terminal result");
                        assert_eq!(appearance(&owner, ENTRANCE, 1)?, mountain);
                        assert_eq!(fluid_transitions(&h, pour).await?, 1);
                        assert_eq!(progress(&h, &open.storage[1].track).await?, 1);
                        // The recovery result also survives disappearance of the optional splash.
                        sqlx::query("DELETE FROM game_item_fluid_splashes").execute(&h.pool).await?;
                        drop(stream);
                        let payload = recover(&owner, &mut admitted, pour).await?;
                        assert_eq!(use_wire::decode_use_result(&payload).map_err(debug)?, UseDisposition::Committed);
                        items.as_mut().ok_or("USE item view")?.map_view(&[], |_| Ok(())).map_err(debug)?;
                        assert_eq!(test_use_frame(&owner, &mut admitted, &frame, &mut items).await.ok_or("durable USE replay")?.disposition,
                            UseDisposition::Committed);
                    } else if case == Case::ExpiredCrack {
                        assert_eq!(result.ok_or("expiry USE result")?.disposition, UseDisposition::NothingToUse);
                        assert_eq!(fluid_transitions(&h, pour).await?, 0);
                        assert_eq!(progress(&h, &open.storage[1].track).await?, 0);
                        assert_eq!(appearance(&owner, ENTRANCE, 1)?, mountain);
                        continue;
                    } else {
                        assert_eq!(result.ok_or("routed USE result")?.disposition, UseDisposition::Committed);
                    }
                    assert_eq!(appearance(&owner, ENTRANCE, 1)?, opened, "pour from {pos:?}");
                    assert_eq!(map.facts.object_revision(mountain_key), mountain_revision + u64::from(case != Case::OpenPour || i == 0));
                    assert_eq!(fluid_transitions(&h, pour).await?, 1);
                    if case == Case::PourReach {
                        sqlx::query("UPDATE game_nomads_ritual_gestures SET expires_at=clock_timestamp() WHERE fluid_transaction_id IS NOT NULL")
                            .execute(&h.pool).await?;
                        assert!(map.facts.transform_movement_item(mountain_key, mountain));
                    }
                    if case == Case::OpenPour && i == 1 {
                        map.facts.expire_movement_items(owner.owner_now().get() + 121_000_000);
                        assert_eq!(appearance(&owner, ENTRANCE, 1)?, mountain);
                    }
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

async fn admit_peer(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    position: TilePos,
) -> TestResult<AdmittedSession> {
    let source = super::house_transfer_fixtures::RecoverySource::start(h).await?;
    let character = super::desert_party_rite_fixtures::create_peer(
        h, owner.character, &source.evidence, 0, "Knight",
    ).await?;
    let (session, operation) = super::activity_deadline_paging_fixtures::admit_created_member_with_operation(
        h, &source.evidence, 0, character,
    ).await?;
    let fence = owner.current_quest_fence(session).await.map_err(|()| "peer fence")?.ok_or("peer fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_pinned_test_position(actor, MovementLocalPosition {
            x: i32::from(position.x), y: i32::from(position.y), floor: -i16::from(position.floor),
        }).map_err(debug)?;
        runtime.bind_attacker_lease(actor, session, crate::foundation::CharacterLease::new(
            crate::foundation::CharacterId::decode(&character).map_err(debug)?,
            fence.character_lease_generation,
        ).map_err(debug)?).map_err(debug)?;
        actor
    };
    let native = owner.active_generation.and_then(|generation| generation.native_gameplay()).ok_or("peer item generation")?;
    h.root.initialize_admission_equipment(
        owner.character, owner.holder, &fence.item_fence(), native.source_digest(), &operation,
    ).await.map_err(debug)?;
    h.root.grant_starter_items(
        owner.character, owner.holder, &fence.item_fence(),
        &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
        &crate::durability::starter_grant::StarterTemplate {
            definition: TypedDefinitionRef { family: "Item".into(), production_key: "oteryn:item.tibia.i2854".into(), revision_ref: "definition-r1".into() },
            ..crate::durability::starter_grant::StarterTemplate::main()
        },
    ).await.map_err(debug)?;
    admitted.game_session_id = session;
    admitted.runtime_actor = Some(actor);
    admitted.item_fence = Some(fence.item_fence());
    owner.admit_quest_session(&admitted).await;
    Ok(admitted)
}

fn move_selection() -> ItemMoveSelection {
    ItemMoveSelection { world_interaction: true, ..ItemMoveSelection::default() }
}

fn item_handle(item: [u8; 16]) -> TestResult<(Option<SessionItemView>, std::num::NonZeroU64)> {
    let mut view = SessionItemView::resume(Default::default()).with_map_view();
    let item = ItemKey::Instance(item);
    let handle = view.map_view(&[item], |table| Ok(table.handle(&item))).map_err(debug)?
        .ok_or("selected item handle")?;
    Ok((Some(view), handle))
}

async fn route_drop(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    command: UseCommand,
    item: [u8; 16],
) -> TestResult<ItemMoveStep> {
    let (mut items, handle) = item_handle(item)?;
    let selection = move_selection();
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: handle,
            destination: ItemMoveDestination::Ground(WorldTilePosition {
                x: i32::from(CRACK.x), y: i32::from(CRACK.y), floor: -i16::from(CRACK.floor),
            }),
        },
        selection,
    ).map_err(debug)?;
    ready(owner).await?;
    Ok(item_move(owner, admitted.runtime_actor.ok_or("actor")?, command, &payload, selection, false, items.as_mut().ok_or("item move view")?).await)
}

fn reserve(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession) -> TestResult<SessionCommandStream> {
    let mut stream = SessionCommandStream::open(Some(&owner.states.command_ingress), admitted).ok_or("ingress stream")?;
    assert!(matches!(stream.begin(admitted.continuity.next_command_id, admitted.continuity.connection_generation,
        admitted.continuity.server_sequence, false).map_err(debug)?, CommandAction::Execute));
    Ok(stream)
}

async fn recover(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, command: UseCommand) -> TestResult<Vec<u8>> {
    let ingress = &owner.states.command_ingress;
    assert!(!ingress.resumable(admitted));
    ready(owner).await?;
    assert!(owner.recover_nomads_ingress(admitted).await);
    ingress.recover_trade_continuity(admitted).ok_or("continuity after interrupted ritual")?;
    assert!(ingress.resumable(admitted));
    assert_eq!(admitted.continuity.next_command_id, command.command_id + 1);
    let mut stream = SessionCommandStream::open(Some(ingress), admitted).ok_or("recovered stream")?;
    let CommandAction::Replay(frame) = stream.begin(command.command_id, admitted.continuity.connection_generation,
        admitted.continuity.server_sequence, false).map_err(debug)? else { return Err("original typed replay".into()); };
    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame).map_err(debug)?;
    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
    assert_eq!(result.status, oteryn_protocol_oteryn::CommandStatus::DuplicateReplay);
    Ok(result.payload.to_vec())
}

async fn consumed(h: &Harness, key: &str) -> TestResult<Vec<Vec<u8>>> {
    Ok(sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_quest_callback_consumptions WHERE transition_key=$1")
        .bind(key).fetch_all(&h.pool).await?)
}

async fn on_ground(h: &Harness, item: [u8; 16]) -> TestResult<bool> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid)")
        .bind(item.as_slice()).fetch_one(&h.pool).await?)
}

async fn progress(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)")
        .bind(track).fetch_one(&h.pool).await?)
}

async fn pour_after_crack_expired(
    h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    frame: &[u8], items: &mut Option<SessionItemView>, flask: [u8; 16],
) -> TestResult<Option<crate::gameplay_transport::connection::UseOutcome>> {
    // Block the fluid writer on its item row after the route has admitted the map target.
    let mut blocker = h.pool.begin().await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *blocker).await?;
    sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(flask.as_slice()).execute(&mut *blocker).await?;
    let expire = async {
        tokio::time::timeout(std::time::Duration::from_millis(1500), async {
            loop {
                let waiting: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))")
                    .bind(pid).fetch_one(&h.pool).await?;
                if waiting { break Ok::<_, sqlx::Error>(()); }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        }).await??;
        sqlx::query("UPDATE game_nomads_ritual_gestures SET expires_at=clock_timestamp() WHERE move_transaction_id IS NOT NULL")
            .execute(&mut *blocker).await?;
        blocker.commit().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (result, expired) = tokio::join!(test_use_frame(owner, admitted, frame, items), expire);
    expired?;
    Ok(result)
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
