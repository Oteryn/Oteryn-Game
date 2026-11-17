//! S10: budgeted production Channel presence moves a source NPC through the real Item owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NativeEntryStart, NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::npc_voice::NpcVoiceCycle;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn s10_wandering_postgres_moves_within_source_radius_and_pauses_for_customers() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "S10 wandering PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 wandering duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "s10_wandering", false, false).await?;
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
        let base_room = pin.qualified_room().clone();
        let cells = bundle
            .movement_cells(base_room.movement_cells())
            .map_err(debug)?;
        let room = base_room
            .bundle_view(
                cells,
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                NativeEntryStart {
                    x: 9,
                    y: 10,
                    floor: 7,
                },
            )
            .ok_or("qualified bundle room")?;
        let deps = Dependencies::with_content(
            ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                bundle.frame_binding_digest(world),
                bundle.map_revision_digest(),
                (9, 10, 7),
            ),
            base_room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
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
        let mut talk = NpcTalkBook::from_model(&model);
        talk.bind_source_voices(&catalogue, &model, |text| {
            crate::chat::ChatText::parse(text)
                .ok()
                .map(|text| text.as_str().to_owned())
        })
        .map_err(debug)?;
        assert!(
            talk.get("oteryn:npc.norf").is_some(),
            "native customer conversation"
        );
        let walks = talk.source_walks().ok_or("source walk book")?;
        assert!(walks.profile("oteryn:npc.norf").is_some());
        assert!(
            walks.terrain_qualified("oteryn:npc.norf"),
            "source height binding"
        );
        owner.states.chat.install_npc_talk(Arc::new(talk));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let actor = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.norf".into(),
                            placements: vec![NpcPlacement {
                                x: 10,
                                y: 10,
                                floor: -7,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?
                .actors[0]
        };
        assert!(owner.ensure_source_map_initialized().await);
        let mut cycle = {
            let runtime = deps.runtime.lock().await;
            NpcVoiceCycle::new(&runtime, owner.states.chat.npc_talk().ok_or("talk book")?)
        };
        for tick in 0..12_u64 {
            owner.emit_npc_voices(&mut cycle, tick * 1_000_000).await;
            owner.emit_npc_voices(&mut cycle, tick * 1_000_000).await;
            let at = deps
                .runtime
                .lock()
                .await
                .borrow_movement_position()
                .read(actor)
                .map_err(debug)?
                .position();
            assert!(at.x.abs_diff(10) <= 2 && at.y.abs_diff(10) <= 2 && at.floor == 7);
            assert_ne!((at.x, at.y), (9, 10), "player occupancy is authoritative");
        }
        let before = deps
            .runtime
            .lock()
            .await
            .borrow_movement_position()
            .read(actor)
            .map_err(debug)?;
        assert!(
            before.revision() > 1,
            "production NPC cadence never committed a step"
        );
        let npc = EntityRef {
            identity: actor.placement_identity(),
            generation: actor.actor_local_generation(),
        };
        super::npc_paid_travel::talk(&owner, &deps, npc, 1, "hi").await?;
        for tick in 12..20_u64 {
            owner.emit_npc_voices(&mut cycle, tick * 1_000_000).await;
        }
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .borrow_movement_position()
                .read(actor)
                .map_err(debug)?,
            before
        );
        super::npc_paid_travel::talk(&owner, &deps, npc, 2, "bye").await?;
        for tick in 20..28_u64 {
            owner.emit_npc_voices(&mut cycle, tick * 1_000_000).await;
        }
        assert!(
            deps.runtime
                .lock()
                .await
                .borrow_movement_position()
                .read(actor)
                .map_err(debug)?
                .revision()
                > before.revision()
        );
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
            max_x: 31,
            max_y: 31,
            floors: vec![-7],
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
    let tiles = (8..=12)
        .flat_map(|y| {
            (8..=12).map(move |x| Tile {
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
            })
        })
        .collect();
    let bytes = writer::write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles,
        }],
        &Default::default(),
    )
    .map_err(debug)?;
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
            x: 9,
            y: 10,
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
