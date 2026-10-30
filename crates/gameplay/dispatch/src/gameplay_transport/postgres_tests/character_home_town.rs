//! Synthetic citizenship tile and two temples through the real bundle loader.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::ability::creature_bite::CreatureBiteVitals;
use crate::foundation::{CharacterLease, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

fn towns() -> oteryn_world_bundle::subarea::Table {
    use oteryn_world_bundle::subarea::{Area, Mask, Run, Table, Temple};
    let hash = "a".repeat(64);
    let area = |id, town, x| Area {
        id,
        key: format!("oteryn:content.area.fixture{id}"),
        name: format!("Town {id}"),
        parent: (id != 1).then_some(1),
        source_record_sha256: hash.clone(),
        town_id: town,
        hometown: town.is_some(),
        temple: town.map(|_| Temple {
            x,
            y: 11,
            floor: -7,
        }),
    };
    Table {
        default_town: Some(8),
        client_version: "15.30".into(),
        candidate_sha256: hash.clone(),
        areas: vec![area(1, None, 0), area(2, Some(8), 2), area(3, Some(6), 8)],
        masks: vec![Mask {
            subarea: 2,
            asset: format!("content/assets/files/subarea-0002-{hash}.bmp.lzma"),
            compressed_sha256: hash.clone(),
            bmp_sha256: hash.clone(),
            source_record_sha256: hash,
            source_tiles: 1,
            admitted_tiles: 1,
        }],
        runs: vec![Run {
            floor: -7,
            y: 11,
            begin: 2,
            end: 3,
            subarea: 2,
        }],
    }
}

#[test]
fn citizen_tile_persists_town_death_uses_temple_and_reconnect_keeps_it() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "home town PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "citizen_temple", false, false).await?;
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
        assert_eq!(
            revisions(&h).await?,
            (1, 1),
            "materializing the default is not a semantic change"
        );
        let (outcome, _) = owner
            .ordinary_paced_step(actor, session, StepDirection::East, None, None)
            .await;
        assert_eq!(outcome.moved_to.ok_or("citizen step")?.actor_position.x, 8);
        let stored: i32 = sqlx::query_scalar("SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(stored, 6);
        assert_eq!(
            revisions(&h).await?,
            (2, 2),
            "citizenship advances the root and progression revision together"
        );
        assert_eq!(owner.home_town(session, Some((actor, 6))).await, Some(6));
        assert_eq!(
            revisions(&h).await?,
            (2, 2),
            "the same citizenship is idempotent"
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_home_town_receipts WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(receipts, 1, "one receipt for the actual town change");
        assert!(sqlx::query(
            "UPDATE game_character_home_towns SET town_id=8 WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).execute(&h.pool).await.is_err(), "town writes require their receipt");
        assert!(sqlx::query(
            "UPDATE game_character_home_town_receipts SET town_after=8 WHERE character_id=encode($1,'hex')::uuid",
        ).bind(id(41).as_slice()).execute(&h.pool).await.is_err(), "receipts are immutable");
        let temple = MovementLocalPosition {
            x: 8,
            y: 11,
            floor: 7,
        };
        assert_eq!(owner.home_temple(session).await, Some(temple));
        let channel = owner.channel_id;
        owner.channel_id = crate::foundation::ChannelId::decode(&id(44)).map_err(debug)?;
        assert_eq!(owner.home_town(session, Some((actor, 8))).await, None);
        owner.channel_id = channel;
        // A stale generation cannot overwrite citizenship.
        let mut stale = fence(2)?;
        stale.character_lease_generation = 1;
        assert!(
            h.root
                .character_home_town(&authority, &h.node, stale, 8, Some(8))
                .await
                .is_err()
        );
        let death = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .place_respawned_player(actor, session, MovementLocalPosition { x: 9, ..temple })
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            states
                .apply_creature_damage(
                    &mut runtime,
                    actor,
                    session,
                    100_000,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                        owner.owner_now().get(),
                    ),
                )
                .ok_or("damage")?;
            states
                .player_death(&runtime, actor, session)
                .ok_or("death")?
        };
        owner
            .respawn_after_death(actor, session, death)
            .await
            .ok_or("temple respawn")?;
        assert_eq!(
            deps.runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            temple
        );
        // Recreate the dispatch owner, as on reconnect; citizenship is loaded from PostgreSQL.
        drop(owner);
        let mut reconnected = deps.owner(&h, &authority);
        reconnected.active_generation = Some(active);
        reconnected.movement_cells = &cells;
        assert_eq!(reconnected.home_temple(session).await, Some(temple));
        let mut current = fence(3)?;
        current.character_lease_generation = 2;
        let saved = h
            .root
            .read_character_last_position(&authority, &h.node, current)
            .await
            .map_err(debug)?
            .ok_or("respawn saved position")?;
        assert_eq!(saved.record.cell.x, 8);
        assert_eq!(saved.record.cell.y, 11);
        assert_eq!(saved.record.cell.floor, 7);
        eprintln!(
            "C20d-W23_POSTGRES_EXECUTED: citizenship, temple death respawn, reconnect reload; PostgreSQL 18.6; migration 0534"
        );
        drop(reconnected);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn revisions(h: &Harness) -> TestResult<(u64, u64)> {
    let (root, progression): (String, String) = sqlx::query_as(
        "SELECT r.character_revision::text,p.character_revision::text \
         FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) \
         WHERE r.character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    Ok((root.parse()?, progression.parse()?))
}
fn world(world: crate::foundation::WorldId) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
        TerrainKind,
    };
    let manifest = Manifest {
        subareas: towns(),
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
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
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
    let tiles = [(2, 0), (3, 0), (8, 0), (9, 0)]
        .into_iter()
        .map(|(x, flags)| Tile {
            x,
            y: 11,
            flags,
            house: 0,
            zones: vec![],
            items: vec![Item {
                palette: 0,
                depth: 0,
                attrs: Attrs {
                    unique: (x == 3).then_some(9056),
                    ..Attrs::default()
                },
            }],
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
