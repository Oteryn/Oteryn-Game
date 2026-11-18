use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

const ROPE: &str = "oteryn:item.tibia.i3003";
const BARREL: &str = "oteryn:item.tibia.i2523";
const HOLE: TilePos = TilePos {
    x: 32595,
    y: 32199,
    floor: -8,
};
const BELOW: TilePos = TilePos { floor: -9, ..HOLE };
const SOUTH: TilePos = TilePos { y: 32200, ..HOLE };
const EAST: TilePos = TilePos { x: 32596, ..SOUTH };

#[test]
fn outlaw_camp_rope_use_lifts_the_same_ground_barrel_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("outlaw-camp-rope", move || {
        super::fixture::run(async |_| scenario(admin, None).await)
    })
}

#[test]
fn outlaw_camp_moves_empty_map_barrel_to_ground_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("outlaw-camp-barrel", move || {
        super::fixture::run(async |_| scenario(admin, Some((EAST, EAST))).await)
    })
}

#[test]
fn outlaw_camp_moves_barrel_through_open_hole_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("outlaw-camp-barrel-drop", move || {
        super::fixture::run(async |_| scenario(admin, Some((HOLE, BELOW))).await)
    })
}

async fn scenario(admin: String, barrel_move: Option<(TilePos, TilePos)>) -> TestResult {
    let harness =
        Harness::create_with_admission_seed(admin, "outlaw_camp_rope", true, false).await?;
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
    let index = native.item_index().ok_or("item index")?;
    let rope = index
        .definition_ref("Item", ROPE, "definition-r1")
        .ok_or("rope definition")?;
    let barrel = if barrel_move.is_some() {
        Some(
            index
                .definition_ref("Item", BARREL, "definition-r1")
                .ok_or("barrel definition")?,
        )
    } else {
        None
    };
    let map = rope_world(deps.admitted.world_id, deps.admitted.channel_id, barrel)?;
    let cells = map
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let world = deps.admitted.world_id;
    let room = super::rust_lottery::bundle_action_room(pin.qualified_room(), &map, world, &cells)?;
    let frame = room.frame_binding().digest();
    let start = MovementLocalPosition {
        x: i32::from(SOUTH.x),
        y: i32::from(SOUTH.y),
        floor: -i16::from(SOUTH.floor),
    };
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        frame,
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    deps = Dependencies::with_content(channel_pin, room.clone(), native.spell_book().clone())?;
    deps.runtime
        .lock()
        .await
        .initialize_pinned_test_position(deps.actor, start)
        .map_err(debug)?;
    let mut owner = deps.owner(&harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
    deps.initialize(&owner).await;

    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(deps.admitted.game_session_id)
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
        .bind(frame.as_slice()).execute(&harness.pool).await?;
    if let Some(route) = barrel_move {
        move_barrel(&owner, &deps, &harness, &map, fence, route).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        return harness.cleanup().await;
    }
    super::item_look::seed_main_backpack(&harness, &[(ROPE, 1)]).await?;
    seed_barrel(
        &harness,
        map.map_revision(),
        pin.identity().server_artifact_digest(),
        frame,
        BELOW,
    )
    .await?;

    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("rope USE capabilities")?;
    let mut placement = map.base().digest().to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(HOLE, 0)
            .ok_or("hole placement")?
            .to_be_bytes(),
    );
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (_, result, _) =
        super::item_craft::exchange(&owner, admitted, 1, &payload, Some(rope)).await?;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(
        deps.runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position(),
        start
    );

    let items: Vec<(Vec<u8>, Vec<u8>, i64)> = sqlx::query_as(
        "SELECT uuid_send(i.item_instance_id), g.spatial_position, i.quantity
         FROM game_item_instances i JOIN game_item_ground_locations g USING (item_instance_id)
         WHERE i.definition_production_key = $1",
    )
    .bind(BARREL)
    .fetch_all(&harness.pool)
    .await?;
    assert_eq!(items, vec![(id(85).to_vec(), cell(SOUTH), 1)]);
    let receipt: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts WHERE command_id=1")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(receipt, 1);
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}

async fn move_barrel(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    harness: &Harness,
    map: &crate::map::boot::BundleWorld,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    (destination, landing): (TilePos, TilePos),
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{ItemMoveStep, item_move};
    use crate::gameplay_transport::item_view::{ItemKey, ItemViewContinuity, SessionItemView};
    use crate::gameplay_transport::world_map::{MapUpdate, SessionMapView};
    use oteryn_protocol_oteryn::item_view::{
        ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection, WorldTilePosition,
        encode_item_move_intent_for,
    };

    let mut observer = SessionMapView::default();
    let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    let Some(MapUpdate::Snapshot(_)) = owner
        .observe_world_map(deps.actor, &mut observer, &mut items)
        .await
        .ok_or("barrel map observation")?
        .map_err(debug)?
    else {
        return Err("missing barrel map snapshot".into());
    };
    let source = items
        .table()
        .handle(&ItemKey::MapBase {
            bundle_digest: map.base().digest(),
            placement_key: crate::map::view::placement_key(SOUTH, 1).ok_or("barrel placement")?,
            reset_epoch: 0,
        })
        .ok_or("movable barrel handle")?;
    let selection = ItemMoveSelection {
        equip_drop: false,
        container_tree: false,
        world_interaction: true,
    };
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source,
            destination: ItemMoveDestination::Ground(WorldTilePosition {
                x: destination.x.into(),
                y: destination.y.into(),
                floor: destination.floor.into(),
            }),
        },
        selection,
    )
    .map_err(debug)?;
    assert!(matches!(
        item_move(
            owner,
            deps.actor,
            UseCommand {
                game_session_id: deps.admitted.game_session_id,
                command_id: 1,
                item_fence: Some(fence),
            },
            &payload,
            selection,
            false,
            &mut items
        )
        .await,
        ItemMoveStep::Result(ItemMoveOutcome::Moved, _)
    ));
    let result: (Vec<u8>, i64) = sqlx::query_as(
        "SELECT g.spatial_position, i.quantity FROM game_item_ground_locations g
         JOIN game_item_instances i USING (item_instance_id) WHERE i.definition_production_key = $1",
    ).bind(BARREL).fetch_one(&harness.pool).await?;
    assert_eq!(result, (cell(landing), 1));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_map_item_mint_receipts")
        .fetch_one(&harness.pool)
        .await?;
    assert_eq!(count, 1);
    Ok(())
}

fn rope_world(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    barrel: Option<std::num::NonZeroU32>,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector, writer};
    let (template, mut pins) = crate::map::boot::tests::bundle();
    let mut manifest = bundle::read(&template)?.manifest;
    manifest.world = bundle::Extent {
        min_x: HOLE.x,
        max_x: if barrel.is_some() {
            EAST.x + 1
        } else {
            HOLE.x + 1
        },
        min_y: HOLE.y,
        max_y: SOUTH.y + 1,
        floors: vec![BELOW.floor, HOLE.floor],
    };
    // Ground appearances and speeds from the generated Outlaw Camp regions.
    manifest.palette = [(351, 120), (355, 200), (385, 130)]
        .into_iter()
        .enumerate()
        .map(|(index, (appearance, speed))| bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: index as u32,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(speed),
                blocks_projectile: None,
            }),
        })
        .collect();
    let tile = |pos: TilePos, palette| sector::Tile {
        x: pos.x,
        y: pos.y,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![sector::Item {
            palette,
            depth: 0,
            attrs: sector::Attrs::default(),
        }],
    };
    let mut sectors = [
        bundle::Sector {
            floor: HOLE.floor,
            sx: HOLE.x / 32,
            sy: HOLE.y / 32,
            tiles: vec![tile(HOLE, 2), tile(SOUTH, 0)],
        },
        bundle::Sector {
            floor: BELOW.floor,
            sx: BELOW.x / 32,
            sy: BELOW.y / 32,
            tiles: vec![tile(BELOW, 1)],
        },
    ];
    if let Some(reference) = barrel {
        manifest.palette.push(bundle::PaletteEntry {
            key: BARREL.into(),
            family: bundle::Family::Item,
            id: reference.get() - 1,
            terrain: None,
        });
        sectors[0].tiles[1].items.push(sector::Item {
            palette: 3,
            depth: 0,
            attrs: sector::Attrs::default(),
        });
        sectors[0].tiles.push(tile(EAST, 0));
    }
    sectors.sort_by_key(bundle::order);
    let bytes = writer::write(&manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: SOUTH,
        },
        world,
        channel,
        |key| {
            (key == BARREL)
                .then_some(barrel)
                .flatten()
                .map(|reference| crate::map::facts::ItemDefinition {
                    reference,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
        },
    )
    .map_err(|error| debug(error).into())
}

fn cell(pos: TilePos) -> Vec<u8> {
    [
        i32::from(pos.x).to_be_bytes().as_slice(),
        i32::from(pos.y).to_be_bytes().as_slice(),
        i16::from(pos.floor).to_be_bytes().as_slice(),
    ]
    .concat()
}

pub(super) async fn seed_barrel(
    h: &Harness,
    map: &str,
    content: [u8; 32],
    frame: [u8; 32],
    position: TilePos,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
          definition_revision_ref,quantity,lifecycle,minted_transaction_id)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)",
    ).bind(id(85).as_slice()).bind(id(42).as_slice()).bind(BARREL).bind(id(86).as_slice()).execute(&mut *tx).await?;
    let reference = [
        crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
        id(86).as_slice(),
    ]
    .concat();
    sqlx::query(
        "INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,
          spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,
          'sha256:'||encode($7,'hex'),$8,nextval('game_item_ground_stack_ordinal_seq'))",
    ).bind(id(85).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(cell(position))
        .bind(reference).bind(map).bind(content.as_slice()).bind(frame.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
