use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{ReferenceItemField, ReferenceItemType};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::{
    connection::{FreshAdmissionAuthority, UseCommand},
    item_move::{LoopMove, LoopMoveResult, LoopTarget},
    item_view::ItemKey,
};
use crate::map::{boot::BootPins, door::DoorUse, facts::ItemDefinition, overlay::TilePos};
use oteryn_protocol_oteryn::{
    item_view::WorldTilePosition,
    world_spatial::{StepDirection, StepDisposition},
};

const DOOR: &str = "oteryn:item.tibia.i5113";
const COIN: &str = "oteryn:item.tibia.i3031";
const DOORWAY: TilePos = TilePos {
    x: 6,
    y: 0,
    floor: -7,
};
const WEST: TilePos = TilePos { x: 5, ..DOORWAY };
const EAST: TilePos = TilePos { x: 7, ..DOORWAY };
const SOUTH: TilePos = TilePos { y: 1, ..DOORWAY };

#[test]
fn m06_quest_door_step_out_moves_item_south_when_east_is_blocked() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, StepDirection::West, false)
}

#[test]
fn m06_quest_door_step_out_closes_when_player_steps_south() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, StepDirection::South, false)
}

#[test]
fn m06_quest_door_step_out_keeps_custody_when_both_exits_are_blocked() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, StepDirection::West, true)
}

fn run_case(admin: String, direction: StepDirection, south_blocked: bool) -> TestResult {
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(Box::pin(scenario(admin, direction, south_blocked)))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "quest door step-out case panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED door_step_out {direction:?} south_blocked={south_blocked} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn scenario(admin: String, direction: StepDirection, south_blocked: bool) -> TestResult {
    let harness = Harness::create_with_admission_seed(admin, "door_step_out", true, false).await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let world = deps.admitted.world_id;
    let channel = deps.admitted.channel_id;
    let session = deps.admitted.game_session_id;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    deps.room = pin.qualified_room().clone();
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let index = native.item_index().ok_or("item index")?;
    let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
    let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
        (DOOR, reference(DOOR).ok_or("door definition")?.get() - 1),
        (COIN, reference(COIN).ok_or("coin definition")?.get() - 1),
    );
    let mut data = oteryn_world_bundle::bundle::read(&bytes)?;
    let tiles = &mut data.sectors.first_mut().ok_or("door sector")?.tiles;
    let west = tiles
        .iter_mut()
        .find(|tile| tile.x == WEST.x)
        .ok_or("west tile")?;
    let wall = west.items.pop().ok_or("wall")?;
    let mut south = west.clone();
    south.x = SOUTH.x;
    south.y = SOUTH.y;
    if south_blocked {
        south.items.push(wall.clone());
    }
    let east = tiles
        .iter_mut()
        .find(|tile| tile.x == EAST.x)
        .ok_or("east tile")?;
    east.items.truncate(1);
    east.items.push(wall);
    // Queen of the Banshees quest door, the same action as the placed M06Q fixture.
    tiles
        .iter_mut()
        .find(|tile| tile.x == DOORWAY.x)
        .and_then(|tile| tile.items.get_mut(1))
        .ok_or("quest door")?
        .attrs
        .action = Some(10089);
    tiles.push(south);
    let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &data.manifest,
        &data.sectors,
        &data.spawns,
        &data.npcs,
    )?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let definition = |key: &str| {
        let policy = native.item_policy(key, "definition-r1")?.record();
        let pickupable = matches!(
            &policy.semantics.physical,
            ReferenceItemField::Known(physical)
                if physical.pickupable == ReferenceItemField::Known(true)
        );
        let door = matches!(
            &policy.semantics.classification,
            ReferenceItemField::Known(classification)
                if classification.item_type == ReferenceItemField::Known(ReferenceItemType::Door)
        );
        Some(ItemDefinition {
            reference: reference(key)?,
            solid: policy.attributes.blocks_movement,
            blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
            pickupable,
            door,
        })
    };
    let map = crate::map::boot::boot(
        &bytes,
        &BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: WEST,
        },
        world,
        channel,
        definition,
    )
    .map_err(debug)?;
    let door = crate::map::view::placement_key(DOORWAY, 1).ok_or("door placement")?;
    assert_eq!(
        map.facts().doors().toggle(door, 0, false, 100, true),
        Some(DoorUse::Toggled {
            revision: 1,
            open: true
        })
    );
    assert!(map.enterable(DOORWAY));
    assert!(!map.enterable(EAST));
    assert_eq!(map.enterable(SOUTH), !south_blocked);
    let cells = map
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let frame = map.frame_binding_digest(world);
    let revision = map.map_revision_digest();
    let start = crate::content::NativeEntryStart {
        x: i32::from(WEST.x),
        y: i32::from(WEST.y),
        floor: 7,
    };
    deps.combat_room = Some(
        deps.room
            .bundle_view(cells.clone(), frame, revision, start)
            .ok_or("bundle combat view")?,
    );
    let content_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        frame,
        revision,
        (i32::from(WEST.x), i32::from(WEST.y), 7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        content_pin,
    )
    .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    // Begin on the open doorway so the only STEP exercises its departure handler.
    runtime
        .initialize_pinned_test_position(deps.actor, local(DOORWAY))
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    let mut owner = deps.owner(&harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "read fence")?
        .ok_or("current character fence")?
        .item_fence();
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice()).bind(frame.as_slice())
        .execute(&harness.pool).await?;
    super::item_look_fixtures::seed_main_backpack(&harness, &[(COIN, 1)]).await?;
    let backpack = owner
        .root
        .read_character_backpack(&authority, fence.character_id)
        .await?
        .ok_or("backpack")?;
    let item = backpack
        .entries
        .first()
        .ok_or("coin")?
        .item
        .item_instance_id;
    assert_eq!(
        owner
            .move_loop_item(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 100,
                    item_fence: Some(fence),
                },
                LoopMove {
                    item: ItemKey::Instance(item),
                    count: None,
                    slot: None,
                    target: LoopTarget::Ground(WorldTilePosition {
                        x: i32::from(DOORWAY.x),
                        y: i32::from(DOORWAY.y),
                        floor: i16::from(DOORWAY.floor)
                    }),
                }
            )
            .await,
        LoopMoveResult::Committed
    );
    let before = owner.root.read_item_loop_ground(world, channel).await?;
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].item.item_instance_id, item);
    assert_eq!(before[0].item.ground.spatial_position, cell(DOORWAY));

    let (step, _) = owner
        .ordinary_paced_step(deps.actor, session, direction, None, None)
        .await;
    assert_eq!(step.disposition, StepDisposition::Moved);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        local(if direction == StepDirection::South {
            SOUTH
        } else {
            WEST
        })
    );
    let after = owner.root.read_item_loop_ground(world, channel).await?;
    let moves: (i64, i64) = sqlx::query_as("SELECT count(*), COALESCE(sum(quantity_after),0)::bigint FROM game_item_loop_move_lines WHERE item_instance_id=encode($1,'hex')::uuid AND from_kind=3 AND to_kind=3")
        .bind(item.as_slice()).fetch_one(&harness.pool).await?;
    if south_blocked {
        assert_eq!(
            after, before,
            "a blocked relocation must preserve durable custody"
        );
        assert_eq!(moves, (0, 0));
    } else {
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].item.item_instance_id, item);
        assert_eq!(after[0].item.quantity, 1);
        assert_eq!(after[0].item.ground.spatial_position, cell(SOUTH));
        assert_eq!(moves, (1, 1));
    }
    assert_eq!(map.facts().doors().revision(door), 2);
    assert!(!map.enterable(DOORWAY));
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}

fn local(pos: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(pos.x),
        y: i32::from(pos.y),
        floor: -i16::from(pos.floor),
    }
}

fn cell(pos: TilePos) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(10);
    bytes.extend_from_slice(&i32::from(pos.x).to_be_bytes());
    bytes.extend_from_slice(&i32::from(pos.y).to_be_bytes());
    bytes.extend_from_slice(&i16::from(pos.floor).to_be_bytes());
    bytes
}
