//! Crystal 201189f8 Hidden Seal wall switch, through production action USE.
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

const SWITCH: TilePos = TilePos {
    x: 32315,
    y: 31910,
    floor: -12,
};
const START: TilePos = TilePos { x: 32314, ..SWITCH };
const WALL: TilePos = TilePos {
    x: 32259,
    y: 31891,
    floor: -10,
};
const QUEST: &str = "oteryn:quest.the_queen_of_the_banshees_quest";

#[test]
fn banshee_hidden_wall_use_resets_remotely_and_manual_close_reopens_safely_postgres() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Hidden Seal scenario panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "banshee_circuit", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let callback: Callback = serde_json::from_value(serde_json::json!({
            "key":"oteryn:quest-callback/banshee/hidden-wall",
            "quest": QUEST,
            "trigger":"use", "selector":{"unique":30010},
            "lever":[2772,2773], "reversible":true,
            "storage":[], "offerings":[], "return_on_failure":false,
            "passages":[{"position":{"x":WALL.x,"y":WALL.y,"floor":WALL.floor},"appearance":2129}],
            "effects":{"passages_reset_after_ms":100000},
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_queen_of_the_banshees/action-1-first_seal_lever.lua:24"]
        }))?;

        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let world = world()?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let origin = MovementLocalPosition {
            x: START.x.into(),
            y: START.y.into(),
            floor: 12,
        };
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
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
            .set(vec![callback])
            .map_err(|_| "duplicate callback installation")?;
        deps.initialize(&owner).await;

        let map = cells.map();
        let switch = placement_key(SWITCH, 1).ok_or("switch placement")?;
        let wall = placement_key(WALL, 1).ok_or("wall placement")?;
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
        for revision in 0..=3 {
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

fn world() -> TestResult<boot::BundleWorld> {
    let (bytes, mut pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    for (appearance, walkable) in [(2772, true), (2773, true), (2129, false)] {
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
    data.manifest.world = bundle::Extent {
        min_x: WALL.x,
        min_y: WALL.y,
        max_x: SWITCH.x + 1,
        max_y: SWITCH.y + 1,
        floors: vec![-12, -10],
    };
    let tile = |at: TilePos, extra| {
        let mut items = vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: Default::default(),
        }];
        if let Some((palette, unique)) = extra {
            items.push(sector::Item {
                palette,
                depth: 0,
                attrs: sector::Attrs {
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
    let mut sectors = [
        bundle::Sector {
            floor: SWITCH.floor,
            sx: SWITCH.x / 32,
            sy: SWITCH.y / 32,
            tiles: vec![
                tile(START, Some((2, None))),
                tile(SWITCH, Some((1, Some(30010)))),
            ],
        },
        bundle::Sector {
            floor: WALL.floor,
            sx: WALL.x / 32,
            sy: WALL.y / 32,
            tiles: vec![tile(WALL, Some((3, None)))],
        },
    ];
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: START,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
