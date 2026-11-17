//! Source route population through real NPC_TALK quotes, payment and arrival consumption.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NativeEntryStart, NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn s10_travel_postgres_source_keywords_and_free_arrival() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "S10 travel PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 source_travel duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "s10_source_travel", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let bundle = world_fixture(world)?;
        let base = pin.qualified_room().clone();
        let cells = bundle
            .movement_cells(base.movement_cells())
            .map_err(debug)?;
        let room = base
            .bundle_view(
                cells,
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                NativeEntryStart {
                    x: 2,
                    y: 0,
                    floor: 7,
                },
            )
            .ok_or("bundle room")?;
        let deps = Dependencies::with_content_and_capacity(
            ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                (2, 0, 7),
            ),
            base,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            4,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = room.movement_cells().into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        for key in ["oteryn:npc.rapanaio_boat", "oteryn:npc.dalbrect"] {
            assert!(
                book.get(key)
                    .is_none_or(|entry| entry.travel_routes.is_empty()),
                "quest-owned route escaped: {key}"
            );
        }
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let keys = ["a_ghostly_sage", "rapanaio_isle_of_evil", "scrutinon"];
        let actors = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let npcs = keys
                .iter()
                .enumerate()
                .map(|(index, key)| NpcPlacements {
                    key: format!("oteryn:npc.{key}"),
                    placements: vec![NpcPlacement {
                        x: 3,
                        y: index as u16,
                        floor: -7,
                        direction: MovementFacing::South,
                    }],
                })
                .collect();
            runtime
                .place_npcs(&NpcPlacementTable { npcs }, |_| true, true)
                .map_err(debug)?
                .actors
        };
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 100)]).await?;
        assert!(owner.ensure_source_map_initialized().await);
        let routes = [
            (0, "help"),
            (1, "kazordoon"),
            (2, "edron"),
            (2, "venore"),
            (2, "darashia"),
            (2, "ab'dendriel"),
        ];
        for (ordinal, (index, keyword)) in routes.into_iter().enumerate() {
            owner.clock_origin -= std::time::Duration::from_secs(1);
            let npc = EntityRef {
                identity: actors[index].placement_identity(),
                generation: actors[index].actor_local_generation(),
            };
            let command = u64::try_from(ordinal * 3 + 1)?;
            super::npc_paid_travel::talk(&owner, &deps, npc, command, "hi").await?;
            let quote =
                super::npc_paid_travel::talk(&owner, &deps, npc, command + 1, keyword).await?;
            let text = quote
                .conversation
                .as_ref()
                .ok_or("travel quote")?
                .lines
                .join(" ");
            assert!(
                text.contains("0 gold") && text.contains(keyword),
                "source keyword {keyword}: {text}"
            );
            if ordinal == 5 {
                let arrived =
                    super::npc_paid_travel::talk(&owner, &deps, npc, command + 2, "yes").await?;
                assert!(!arrived.unknown);
                let position = deps
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position();
                assert_eq!((position.x, position.y, position.floor), (32734, 31668, 6));
            } else {
                super::npc_paid_travel::talk(&owner, &deps, npc, command + 2, "bye").await?;
            }
        }
        let receipt: (i64,i64) = sqlx::query_as("SELECT count(*),COALESCE(sum(fare_gold_units),0)::bigint FROM game_newhaven_travel_receipts WHERE route='oteryn:service.travel.scrutinon/ab_dendriel'")
            .fetch_one(&h.pool).await?;
        assert_eq!(receipt, (1, 0));
        let pending: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_pending_arrivals")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(pending, 0);
        let gold: i64 = sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i3031'")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(gold, 100);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn world_fixture(world: crate::foundation::WorldId) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
        TerrainKind,
    };
    let manifest = Manifest {
        subareas: Default::default(),
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "engineering".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 65535,
            max_y: 65535,
            floors: vec![-15, -9, -7, -6],
        },
        palette: vec![PaletteEntry {
            key: "oteryn:item.tibia.i103".into(),
            family: Family::Terrain,
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
                blocks_projectile: None,
            }),
        }],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let mut grouped = std::collections::BTreeMap::<_, Vec<Tile>>::new();
    let points = [
        (2, 0, -7),
        (3, 0, -7),
        (3, 1, -7),
        (3, 2, -7),
        (32734, 31668, -6),
        (33175, 31764, -6),
        (32954, 32022, -6),
        (33289, 32480, -6),
        (32700, 31990, -15),
        (32834, 32275, -9),
    ];
    for (x, y, floor) in points {
        grouped
            .entry((floor, y / 32, x / 32))
            .or_default()
            .push(Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items: vec![Item {
                    palette: 0,
                    depth: 0,
                    attrs: Attrs::default(),
                }],
            });
    }
    let sectors = grouped
        .into_iter()
        .map(|((floor, sy, sx), tiles)| Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    let bytes = writer::write(&manifest, &sectors, &Default::default()).map_err(debug)?;
    let digest = writer::read(&bytes).map_err(debug)?.digest;
    let pins = crate::map::boot::BootPins {
        bundle: crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: false,
        },
        map_revision: format!(
            "sha256:{}",
            digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        start: crate::map::overlay::TilePos {
            x: 2,
            y: 0,
            floor: -7,
        },
    };
    crate::map::boot::boot(
        &bytes,
        &pins,
        world,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
