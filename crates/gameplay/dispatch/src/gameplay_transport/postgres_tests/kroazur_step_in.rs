use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelContentPin, MovementLocalPosition};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::writer::*;

#[test]
fn kroazur_step_in_requires_access_and_uses_shared_boss_timer() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("kroazur-step-in", move || {
        run(async move |_| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "kroazur_step_in", true, false).await?;
            let result = async {
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let world = super::fixture::fence(1)?.runtime_scope.world_id();
                let channel =
                    crate::foundation::ChannelId::decode(&super::fixture::id(43)).map_err(debug)?;
                let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
                let (bytes, pins) = source_tiles()?;
                let base = crate::map::load(&bytes, &pins).map_err(debug)?;
                let map = boot(
                    &bytes,
                    &BootPins {
                        map_revision: crate::map::overlay::map_revision(&base),
                        bundle: pins,
                        start: TilePos {
                            x: 1,
                            y: 0,
                            floor: -9,
                        },
                    },
                    world,
                    channel,
                    |_| None,
                )
                .map_err(debug)?;
                let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
                let pin = ChannelContentPin::from_activation(
                    world,
                    1,
                    room.compiled().server_digest(),
                    room.compiled().client_digest(),
                    map.frame_binding_digest(world),
                    map.map_revision_digest(),
                    (1, 0, 9),
                );
                let view = room
                    .bundle_view(
                        cells.clone(),
                        map.frame_binding_digest(world),
                        map.map_revision_digest(),
                        crate::content::NativeEntryStart {
                            x: 1,
                            y: 0,
                            floor: 9,
                        },
                    )
                    .ok_or("Kroazur source map view")?;
                let deps = Dependencies::with_content(
                    pin,
                    room,
                    crate::spell::cast::v1_spell_book().map_err(debug)?,
                )?;
                let mut owner = deps.owner(&h, &authority);
                owner.movement_cells = (&cells).into();
                owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&view));
                owner
                    .runtime
                    .lock()
                    .await
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                deps.initialize(&owner).await;
                let (step, _) = owner
                    .paced_step(
                        deps.actor,
                        deps.admitted.game_session_id,
                        StepDirection::East,
                    )
                    .await;
                assert_eq!(step.disposition, StepDisposition::Moved);
                owner
                    .committed_paced_movement(deps.admitted, step.moved_to)
                    .await;
                assert_eq!(
                    owner
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    MovementLocalPosition {
                        x: 1,
                        y: 0,
                        floor: 9
                    },
                    "without the access track, the step must return to the previous tile"
                );
                Ok(())
            }
            .await;
            h.cleanup().await?;
            result
        })
    })
}

fn source_tiles() -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    let manifest = Manifest {
        build_class: BuildClass::Production,
        format: FORMAT.into(),
        min_reader_version: VERSION,
        projection_class: "server".into(),
        compiler_version: "kroazur-step-test".into(),
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "content-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-9],
        },
        palette: vec![PaletteEntry {
            key: "oteryn:terrain.tibia.i100".into(),
            family: Family::Terrain,
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
                blocks_projectile: None,
            }),
        }],
        subareas: Default::default(),
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let tile = |x, action| Tile {
        x,
        y: 0,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![Item {
            palette: 0,
            depth: 0,
            attrs: Attrs {
                action,
                ..Attrs::default()
            },
        }],
    };
    let bytes = write(
        &manifest,
        &[Sector {
            floor: -9,
            sx: 0,
            sy: 0,
            tiles: vec![tile(1, None), tile(2, Some(25029))],
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "content-1".into(),
            production: false,
        },
    ))
}
