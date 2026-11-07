//! W23: all Crystal citizen registrations and Svargrond roof access through paced movement
//! and PostgreSQL. test_citizen_bindings.py separately qualifies the shipped World stamps.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{CharacterLease, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

// Crystal 201189f8 data-global/scripts/movements/teleport/citizen.lua:1-19;
// data-global/lib/tables/town.lua supplies the town ids.
const CITIZENS: [(u16, u32); 17] = [
    (9056, 6),
    (9057, 8),
    (9058, 9),
    (9059, 5),
    (9060, 7),
    (9061, 13),
    (9062, 10),
    (9063, 11),
    (9064, 14),
    (9065, 15),
    (9066, 16),
    (9067, 17),
    (9068, 12),
    (9240, 18),
    (9500, 21),
    (9510, 20),
    (9515, 22),
];

#[test]
fn w23_all_citizen_uniques_and_svargrond_quest_boundary() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "citizen qualification thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "citizen_qualification", false, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = fence(1)?.runtime_scope.world_id();
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_source(
            world_id,
            "content-1",
            None,
        )?;
        let active = controller.active().ok_or("active generation")?;
        let bundle = world(world_id)?;
        let cells = bundle
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = pin
            .qualified_room()
            .bundle_view(
                cells.clone(),
                bundle.frame_binding_digest(world_id),
                bundle.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 2,
                    y: 11,
                    floor: 7,
                },
            )
            .ok_or("bundle room")?;
        let channel_pin = crate::foundation::ChannelContentPin::from_activation(
            world_id,
            1,
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (2, 11, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            active
                .native_gameplay()
                .ok_or("native")?
                .spell_book()
                .clone(),
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        assert!(owner.progression_binding(session).is_some());
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
        }
        assert_eq!(owner.home_town(session, Some((actor, 8))).await, None);
        {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        deps.initialize(&owner).await;
        assert_eq!(owner.home_town(session, None).await, Some(8));
        owner
            .states
            .chat
            .admit(session, "Citizen", owner.owner_now());
        let mut current_town = 8;
        let mut expected_receipts = 0_i64;
        for (index, (unique, town)) in CITIZENS.into_iter().enumerate() {
            let progress_cases: &[Option<i64>] = if town == 16 {
                &[None, Some(7), Some(8)]
            } else {
                &[None]
            };
            for &progress in progress_cases {
                let started = std::time::Instant::now();
                if let Some(progress) = progress {
                    // Administrator seeds only the predecessor quest fact, as in npc_paid_travel.
                    // The movement and citizenship writer below retain all runtime SQL guards.
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL session_replication_role=replica")
                        .execute(&mut *tx)
                        .await?;
                    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,'oteryn:quest-progress/storage/41174','oteryn:quest.barbarian_test',$2,2) ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value")
                        .bind(id(41).as_slice()).bind(progress).execute(&mut *tx).await?;
                    tx.commit().await?;
                }
                // Set up the predecessor position only; the tested step uses the real movement lane.
                owner
                    .runtime
                    .lock()
                    .await
                    .place_respawned_player(
                        actor,
                        session,
                        MovementLocalPosition {
                            x: 2,
                            y: 11 + i32::try_from(index)?,
                            floor: 7,
                        },
                    )
                    .map_err(debug)?;
                let accepted = town != 16 || progress.is_some_and(|value| value >= 8);
                if accepted && current_town != town {
                    current_town = town;
                    expected_receipts += 1;
                }
                let (outcome, _) = owner
                    .ordinary_paced_step(actor, session, StepDirection::East, None, None)
                    .await;
                let landed = outcome.moved_to.ok_or("citizen step")?.actor_position;
                assert_eq!(
                    (landed.x, landed.y, landed.floor),
                    (8, 11, 7),
                    "unique {unique}"
                );
                assert_eq!(
                    owner.home_town(session, None).await,
                    Some(current_town),
                    "unique {unique}, progress {progress:?}"
                );
                let messages = owner.states.chat.drain(session);
                let expected = if accepted {
                    format!("You are now a citizen of Town {town}.")
                } else {
                    "You first need to absolve the Barbarian Test Quest to become citizen!"
                        .to_owned()
                };
                assert!(messages.iter().any(|line| matches!(line,
                    oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == &expected)),
                    "unique {unique}, progress {progress:?}: {messages:?}");
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_home_town_receipts WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(receipts, expected_receipts);
                eprintln!(
                    "W23_POSTGRES_EXECUTED unique={unique} town={town} progress={progress:?} accepted={accepted} duration={:.3}s",
                    started.elapsed().as_secs_f64()
                );
            }
        }
        // Crystal citizen_svargrond.lua:9-16: roof access is exactly 8, whereas the
        // citizenship predicate above permits >=8. The gate never changes citizenship.
        for progress in [-1_i64, 7, 8, 9] {
            let started = std::time::Instant::now();
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_character_quest_tracks SET value=$2 WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/storage/41174'")
                .bind(id(41).as_slice()).bind(progress).execute(&mut *tx).await?;
            tx.commit().await?;
            owner
                .runtime
                .lock()
                .await
                .place_respawned_player(
                    actor,
                    session,
                    MovementLocalPosition {
                        x: 32209,
                        y: 31134,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            let (outcome, _) = owner
                .ordinary_paced_step(actor, session, StepDirection::West, None, None)
                .await;
            let landed = outcome.moved_to.ok_or("roof gate step")?.actor_position;
            let expected = if progress == 8 {
                (32212, 31131, 5)
            } else {
                (32210, 31134, 7)
            };
            assert_eq!(
                (landed.x, landed.y, landed.floor),
                expected,
                "roof progress {progress}"
            );
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .facing(),
                Some(crate::foundation::MovementFacing::East)
            );
            assert_eq!(owner.home_town(session, None).await, Some(current_town));
            let messages = owner.states.chat.drain(session);
            if progress != 8 {
                assert!(messages.iter().any(|line| matches!(line,
                    oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. }
                        if text == "You have to be a honorary barbarian to access the roof. Talk to the Jarl about it.")));
            }
            let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_home_town_receipts WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(receipts, expected_receipts);
            eprintln!(
                "W23_ROOF_POSTGRES_EXECUTED progress={progress} duration={:.3}s",
                started.elapsed().as_secs_f64()
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn world(world: crate::foundation::WorldId) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::subarea::{Area, Temple};
    use oteryn_world_bundle::writer::{
        self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
        TerrainKind,
    };
    let mut subareas = super::character_home_town::towns();
    subareas.areas.truncate(1);
    for (index, (_, town)) in CITIZENS.into_iter().enumerate() {
        subareas.areas.push(Area {
            id: u32::try_from(index)? + 2,
            key: format!("oteryn:content.area.citizen{town}"),
            name: format!("Town {town}"),
            parent: Some(1),
            source_record_sha256: "a".repeat(64),
            town_id: Some(town),
            hometown: true,
            temple: Some(Temple {
                x: 8,
                y: 11,
                floor: -7,
            }),
        });
    }
    subareas.masks[0].subarea = 3;
    subareas.masks[0].asset = subareas.masks[0]
        .asset
        .replace("subarea-0002-", "subarea-0003-");
    subareas.runs[0].subarea = 3;
    let manifest = Manifest {
        subareas,
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "pvp-live-test".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Default::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 34000,
            max_y: 33000,
            floors: vec![-7, -5],
        },
        palette: vec![PaletteEntry {
            family: Family::Terrain,
            key: "oteryn:item.tibia.i100".into(),
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
            }),
        }],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let mut tiles = vec![];
    for (index, (unique, _)) in CITIZENS.into_iter().enumerate() {
        for x in [2, 3] {
            tiles.push(Tile {
                x,
                y: 11 + u16::try_from(index)?,
                flags: 0,
                house: 0,
                zones: vec![],
                items: vec![Item {
                    palette: 0,
                    depth: 0,
                    attrs: Attrs {
                        unique: (x == 3).then_some(unique),
                        ..Attrs::default()
                    },
                }],
            });
        }
    }
    tiles.push(Tile {
        x: 8,
        y: 11,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![Item {
            palette: 0,
            depth: 0,
            attrs: Attrs::default(),
        }],
    });
    tiles.sort_by_key(|tile| (tile.y, tile.x));
    let mut sectors = vec![Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles,
    }];
    for (floor, coords) in [
        (-7, vec![(32208, 31134), (32209, 31134), (32210, 31134)]),
        (-5, vec![(32212, 31131)]),
    ] {
        sectors.push(Sector {
            floor,
            sx: 32208 / 32,
            sy: 31134 / 32,
            tiles: coords
                .into_iter()
                .map(|(x, y)| Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items: vec![Item {
                        palette: 0,
                        depth: 0,
                        attrs: Attrs {
                            action: (x == 32208).then_some(30032),
                            ..Attrs::default()
                        },
                    }],
                })
                .collect(),
        });
    }
    let bytes = writer::write(&manifest, &sectors, &Default::default()).map_err(debug)?;
    let pins = crate::map::BundlePins {
        digest: oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 2,
                y: 11,
                floor: -7,
            },
        },
        world,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
