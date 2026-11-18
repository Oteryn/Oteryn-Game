//! Crystal 201189f8 Hidden Seal wall switches and trapdoor, through production USE and StepIn.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, id, run},
    prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::{
    boot::{self, BundleCollisionIndexExt},
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_world_bundle::{bundle, sector};

#[derive(Clone, Copy)]
struct Circuit {
    switch: TilePos,
    passage: TilePos,
    unique: u16,
    plate: Option<TilePos>,
}

impl Circuit {
    fn start(self) -> TilePos {
        TilePos {
            x: self.switch.x - 1,
            ..self.switch
        }
    }
}

const WALL: Circuit = Circuit {
    switch: TilePos {
        x: 32315,
        y: 31910,
        floor: -12,
    },
    passage: TilePos {
        x: 32259,
        y: 31891,
        floor: -10,
    },
    unique: 30010,
    plate: None,
};
const TRAPDOOR: Circuit = Circuit {
    switch: TilePos {
        x: 32266,
        y: 31861,
        floor: -11,
    },
    passage: TilePos {
        x: 32266,
        y: 31860,
        floor: -11,
    },
    unique: 30012,
    plate: Some(TilePos {
        x: 32266,
        y: 31860,
        floor: -12,
    }),
};
const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";

#[test]
fn banshee_hidden_wall_use_resets_remotely_and_manual_close_reopens_safely_postgres() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(WALL)
}

#[test]
fn banshee_hidden_trapdoor_use_and_pressure_step_restore_parity_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(TRAPDOOR)
}

fn run_case(circuit: Circuit) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(circuit).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Hidden Seal scenario panicked")?
        .map_err(Into::into)
}

pub(super) fn local(at: TilePos) -> MovementLocalPosition {
    MovementLocalPosition {
        x: at.x.into(),
        y: at.y.into(),
        floor: -i16::from(at.floor),
    }
}

pub(super) fn position(at: TilePos) -> serde_json::Value {
    serde_json::json!({"x":at.x,"y":at.y,"floor":at.floor})
}

fn scenario(circuit: Circuit) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "banshee_circuit", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut lever = serde_json::json!({
            "key":"oteryn:quest-callback/banshee/hidden-switch",
            "quest": QUEST,
            "trigger":"use", "selector":{"unique":circuit.unique},
            "lever":[2772,2773], "reversible":true,
            "storage":[], "offerings":[], "return_on_failure":false,
            "passages":[{"position":position(circuit.passage),"appearance":2129}],
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/action-1-first_seal_lever.lua:3-45"]
        });
        if circuit.plate.is_some() {
            lever["passages"][0]["replacement"] = 369.into();
        } else {
            lever["effects"] = serde_json::json!({"passages_reset_after_ms":100000});
        }
        let mut callbacks = vec![serde_json::from_value::<Callback>(lever)?];
        if let Some(plate) = circuit.plate {
            callbacks.push(serde_json::from_value(serde_json::json!({
                "key":"oteryn:quest-callback/banshee/trapdoor-pressure-plate",
                "quest": QUEST,
                "trigger":"step_in", "selector":{"position":position(plate), "ordinal":0},
                "reversible":false, "storage":[], "offerings":[], "return_on_failure":false,
                "passages":[
                    {"position":position(circuit.passage),"appearance":2129,"replacement":369},
                    {"position":position(circuit.switch),"appearance":27260,"replacement":2773,"if_present":true}
                ],
                "effects":{"passages_close_after_ms":3000},
                "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/movement-1-first_seal_close_mw.lua:46-68"]
            }))?);
        }

        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let world = world(circuit)?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let origin = local(circuit.start());
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            world.frame_binding_digest(deps.admitted.world_id),
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
            2,
            pin,
        )
        .map_err(debug)?;
        let session = deps.admitted.game_session_id;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner
            .states
            .quest_callbacks
            .fixture
            .set(callbacks)
            .map_err(|_| "duplicate callback installation")?;
        deps.initialize(&owner).await;

        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            return Err("bundle collision index".into());
        };
        let map = index.map();
        let switch = placement_key(circuit.switch, 1).ok_or("switch placement")?;
        let wall = placement_key(circuit.passage, 1).ok_or("wall placement")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&switch.to_be_bytes());
        let actor = deps.actor;
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: None,
        };
        let target = |expected_revision| WorldObjectTarget {
            placement: placement.clone(),
            expected_revision,
        };
        let uses = if circuit.plate.is_some() { 1 } else { 4 };
        for revision in 0..uses {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(1))
                .ok_or("advance action clock")?;
            let outcome = owner
                .use_quest_callback(actor, command(revision + 1), map, &target(revision))
                .await
                .ok_or("wall callback did not dispatch")?;
            assert_eq!(
                outcome.disposition,
                UseDisposition::Committed,
                "switch revision {revision}"
            );
            if circuit.plate.is_some() {
                break;
            }
            match revision {
                0 => assert!(map.facts.base_hidden(wall)),
                1 => {
                    assert!(!map.facts.base_hidden(wall));
                    assert_eq!(map.facts.next_movement_decay_time(), None);
                }
                2 => {
                    assert!(map.facts.base_hidden(wall));
                    let due = map
                        .facts
                        .next_movement_decay_time()
                        .ok_or("second wall reset")?;
                    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
                    assert_eq!(map.facts.expire_movement_items(due), 1);
                    assert!(!map.facts.base_hidden(wall));
                    assert_eq!(map.facts.object_revision(switch), 3, "only the wall resets");
                }
                _ => assert!(!map.facts.base_hidden(wall)),
            }
        }

        if let Some(plate) = circuit.plate {
            let appearance = |at: TilePos| -> TestResult<u16> {
                let tile = map
                    .overlay
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .ok_or("source tile")?;
                let id = *tile.ids().get(1).ok_or("source item")?;
                Ok(map
                    .facts
                    .base_entry(at, 1, id)
                    .ok_or("source facts")?
                    .appearance_id)
            };
            assert_eq!(appearance(circuit.passage)?, 369);
            assert_eq!(appearance(circuit.switch)?, 2773);
            assert_eq!(map.facts.next_movement_decay_time(), None);

            let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
            let (origin, entered) = {
                let mut runtime = owner.runtime.lock().await;
                let origin = runtime.read_actor_position(actor).map_err(debug)?;
                let closed = crate::map::floor::BundleDoors(&map.facts);
                let proof = crate::map::floor::prepare_quest_callback_teleport(
                    &runtime,
                    owner.movement_cells(),
                    actor,
                    session,
                    origin,
                    local(plate),
                    false,
                    &closed,
                )
                .map_err(debug)?;
                let entered = runtime.commit_source_step(proof).map_err(debug)?;
                (origin, entered)
            };
            assert_eq!(
                owner
                    .step_quest_callback(&permit, actor, session, origin, entered)
                    .await,
                entered
            );
            let due = map
                .facts
                .next_movement_decay_time()
                .ok_or("pressure plate reset")?;
            assert_eq!(map.facts.expire_movement_items(due - 1), 0);
            assert_eq!(appearance(circuit.passage)?, 369);
            assert_eq!(map.facts.expire_movement_items(due), 2);
            assert_eq!(appearance(circuit.passage)?, 2129);
            assert_eq!(appearance(circuit.switch)?, 27260);
        }

        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key = $1",
        )
        .bind(QUEST)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, 0);
        Ok(())
    })
}

fn world(circuit: Circuit) -> TestResult<boot::BundleWorld> {
    let mut records = vec![
        (circuit.start(), None),
        (circuit.switch, Some((1, Some(circuit.unique)))),
        (circuit.passage, Some((3, None))),
    ];
    if let Some(plate) = circuit.plate {
        records.push((plate, None));
    }
    scene_world(
        circuit.start(),
        &[
            (2772, true),
            (2773, true),
            (2129, false),
            (369, true),
            (27260, true),
        ],
        &records,
        None,
    )
}

type SceneTile = (TilePos, Option<(u32, Option<u16>)>);

pub(super) fn scene_world(
    start: TilePos,
    appearances: &[(u16, bool)],
    records: &[SceneTile],
    portal: Option<(TilePos, TilePos, u16)>,
) -> TestResult<boot::BundleWorld> {
    let (bytes, mut pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    for &(appearance, walkable) in appearances {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(data.manifest.palette.len())?,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Border,
                walkable: Some(walkable),
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }
    let tile = |at: TilePos, extra| {
        let mut items = vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: Default::default(),
        }];
        if let Some((palette, unique)) = extra {
            let portal = portal.filter(|(source, _, _)| *source == at);
            items.push(sector::Item {
                palette,
                depth: 0,
                attrs: sector::Attrs {
                    action: portal.map(|(_, _, action)| action),
                    teleport: portal.map(|(_, destination, _)| {
                        (
                            destination.x,
                            destination.y,
                            destination.floor.to_ne_bytes()[0],
                        )
                    }),
                    unique,
                    ..Default::default()
                },
            });
        }
        sector::Tile {
            x: at.x,
            y: at.y,
            flags: 0,
            house: 0,
            zones: vec![],
            items,
        }
    };
    let mut floors = std::collections::BTreeSet::new();
    let mut by_sector = std::collections::BTreeMap::<_, Vec<_>>::new();
    for &(at, extra) in records {
        floors.insert(at.floor);
        by_sector
            .entry((at.floor, at.x / 32, at.y / 32))
            .or_default()
            .push(tile(at, extra));
    }
    data.manifest.world = bundle::Extent {
        min_x: records
            .iter()
            .map(|(at, _)| at.x)
            .min()
            .ok_or("minimum x")?,
        min_y: records
            .iter()
            .map(|(at, _)| at.y)
            .min()
            .ok_or("minimum y")?,
        max_x: records
            .iter()
            .map(|(at, _)| at.x)
            .max()
            .ok_or("maximum x")?
            + 1,
        max_y: records
            .iter()
            .map(|(at, _)| at.y)
            .max()
            .ok_or("maximum y")?
            + 1,
        floors: floors.into_iter().collect(),
    };
    let mut sectors: Vec<_> = by_sector
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
