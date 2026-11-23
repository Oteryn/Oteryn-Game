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
    run_case(admin, StepDirection::West, false, false)
}

#[test]
fn m06_quest_door_step_out_closes_when_player_steps_south() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, StepDirection::South, false, false)
}

#[test]
fn m06_quest_door_step_out_retires_item_when_both_exits_are_blocked() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, StepDirection::West, true, false)
}

#[test]
fn m06_quest_door_step_out_skips_a_walkable_east_tile_with_full_ground() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin.clone(), StepDirection::West, false, true)?;
    run_case(admin, StepDirection::West, true, true)
}

#[test]
fn m06_runtime_role_has_no_unfenced_door_retirement_privilege() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::fixture::run(async move |_| {
        let h = Harness::create_with_admission_seed(admin, "door_retirement_privileges", false, false).await?;
        let (insert, update, legacy): (bool, bool, bool) = sqlx::query_as(
            "SELECT has_table_privilege('oteryn_game_runtime','game_door_authored_retirements','INSERT'),
                    has_table_privilege('oteryn_game_runtime','game_door_authored_retirements','UPDATE'),
                    COALESCE(has_function_privilege('oteryn_game_runtime',to_regprocedure(
                       'game_retire_door_ground(uuid,uuid,uuid,uuid,uuid,numeric,bytea,text,text,bytea,text)'
                    ),'EXECUTE'),false)",
        )
        .fetch_one(&h.pool).await?;
        assert_eq!((insert, update, legacy), (false, false, false));
        h.cleanup().await
    })
}

fn run_case(admin: String, direction: StepDirection, south_blocked: bool, east_full: bool) -> TestResult {
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(Box::pin(scenario(admin, direction, south_blocked, east_full)))
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

async fn scenario(admin: String, direction: StepDirection, south_blocked: bool, east_full: bool) -> TestResult {
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
    let authored_coin = east.items.pop().ok_or("authored coin")?;
    east.items.truncate(1);
    if !east_full {
        east.items.push(wall);
    }
    // Queen of the Banshees quest door, the same action as the placed M06Q fixture.
    let doorway = tiles
        .iter_mut()
        .find(|tile| tile.x == DOORWAY.x)
        .ok_or("doorway")?;
    doorway.items.get_mut(1).ok_or("quest door")?.attrs.action = Some(10089);
    if south_blocked {
        let mut bound_coin = authored_coin.clone();
        bound_coin.attrs.description = Some("Bound doorway marker".into());
        doorway.items.push(authored_coin);
        doorway.items.push(bound_coin);
    }
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
    assert_eq!(map.enterable(EAST), east_full);
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
    let content_digest = content_pin.server_artifact_digest();
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
    if east_full {
        seed_full_east_ground(&harness.pool, item).await?;
        owner.states.loop_ground.invalidate();
    }
    let origin = owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?;

    let (step, _) = owner
        .ordinary_paced_step(deps.actor, session, direction, None, None)
        .await;
    assert_eq!(step.disposition, StepDisposition::Moved);
    let departure = owner
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?;
    assert_eq!(
        departure.position(),
        local(if direction == StepDirection::South {
            SOUTH
        } else {
            WEST
        })
    );
    let mut after = owner.root.read_item_loop_ground(world, channel).await?;
    if east_full {
        let east = cell(EAST);
        let before_filter = after.len();
        after.retain(|entry| entry.item.ground.spatial_position != east);
        assert_eq!(before_filter - after.len(), crate::durability::ground_cell_limit::GROUND_CELL_ROOT_LIMIT);
    }
    let moves: (i64, i64) = sqlx::query_as("SELECT count(*), COALESCE(sum(quantity_after),0)::bigint FROM game_item_loop_move_lines WHERE item_instance_id=encode($1,'hex')::uuid AND from_kind=3 AND to_kind=3")
        .bind(item.as_slice()).fetch_one(&harness.pool).await?;
    if south_blocked {
        assert!(
            after.is_empty(),
            "the stranded coin must leave Ground custody"
        );
        let authored = crate::map::view::placement_key(DOORWAY, 2).ok_or("authored coin key")?;
        {
            use crate::durability::item_decay_retire::{DoorGroundItem, DoorGroundRetirement};
            let permit = owner.spell_lane_permit().await.ok_or("Ground owner lane")?;
            // Replay the same cleanup as if its first commit reply had been lost.
            owner
                .root
                .retire_door_ground(
                    &permit,
                    &authority,
                    owner.holder,
                    fence,
                    DoorGroundRetirement {
                        door_key: door,
                        bundle_digest: map.base().digest(),
                        content_digest,
                        reset_epoch: 0,
                        origin: origin.position(),
                        destination: departure.position(),
                        origin_revision: origin.revision(),
                        departure_revision: departure.revision(),
                        items: vec![
                            DoorGroundItem::Instance(item),
                            DoorGroundItem::Authored(authored),
                        ],
                    },
                )
                .await?;
        }
        let retired: (i16, i64, i64) = sqlx::query_as(
            "SELECT i.lifecycle, i.quantity, (SELECT count(*) FROM game_door_ground_retirements r
             WHERE r.item_instance_id=i.item_instance_id AND r.transaction_id=i.last_transaction_id)
             FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(item.as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(retired, (2, 0, 1));
        assert_eq!(
            owner
                .root
                .read_map_item_mint_placements(
                    world,
                    channel,
                    map.base().digest(),
                    0,
                    map.base().entry_count()
                )
                .await?,
            [authored],
            "a cold Ground projection must hide the ordinary authored coin and keep the bound coin"
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

async fn seed_full_east_ground(pool: &sqlx::PgPool, template: [u8; 16]) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query(
        "WITH template AS (
            SELECT * FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid
         ), copies AS (
            INSERT INTO game_item_instances (item_instance_id, world_id, definition_family,
                definition_production_key, definition_revision_ref, quantity, lifecycle, minted_transaction_id)
            SELECT game_character_uuid_v7(),world_id,definition_family,definition_production_key,
                definition_revision_ref,quantity,lifecycle,game_character_uuid_v7()
            FROM template CROSS JOIN generate_series(1,$3::int)
            RETURNING item_instance_id, minted_transaction_id
         )
         INSERT INTO game_item_ground_locations
            (item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,
             spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,scope_kind,
             stack_ordinal)
         SELECT c.item_instance_id,g.world_id,g.channel_id,g.runtime_scope_ownership_generation,
                $2,substring(g.corpse_ref FROM 1 FOR octet_length(g.corpse_ref)-16)||uuid_send(c.minted_transaction_id),
                g.map_revision,g.content_revision,g.native_room_placement_context,g.scope_kind,
                nextval('game_item_ground_stack_ordinal_seq')
         FROM copies c CROSS JOIN game_item_ground_locations g
         WHERE g.item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(template.as_slice())
    .bind(cell(EAST))
    .bind(i32::try_from(crate::durability::ground_cell_limit::GROUND_CELL_ROOT_LIMIT)?)
    .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
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
