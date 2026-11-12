//! Real Cake raid placement, physical machine step, durable extraction and replay.
use super::*;
use crate::foundation::MovementLocalPosition;

#[test]
fn w05d_cake_real_raid_step_extracts_without_kill_and_replays_postgres() -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Cake extraction requires PostgreSQL"
        );
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let run = || -> TestResult {
                crate::bestiary_postgres_harness::runtime()?.block_on(async {
                    let h = Harness::create_with_admission_seed(
                        admin,
                        "cake_real_extraction",
                        false,
                        false,
                    )
                    .await?;
                    let result = scenario(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
            };
            run().map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Cake extraction thread")??;
    Ok(())
}
async fn scenario(h: &Harness) -> TestResult {
    let started = std::time::Instant::now();
    super::super::reset();
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, activated) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
    let active = controller.active().ok_or("active native generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let digest = active.identity().server_artifact_digest();
    let at = |x, y| MovementLocalPosition { x, y, floor: 7 };
    let start = at(32345, 31822);
    let player_entry = at(32344, 31821);
    let machine = at(32346, 31822);
    let world = cake_bundle(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        player_entry,
    )?;
    deps.room = activated.qualified_room().clone();
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return Err("Bundle index".into());
    };
    let bundle_digest = index.map().overlay.base().digest();
    let room = deps
        .room
        .bundle_view(
            cells.clone(),
            world.frame_binding_digest(deps.admitted.world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: player_entry.x,
                y: player_entry.y,
                floor: player_entry.floor,
            },
        )
        .ok_or("Bundle room")?;
    let pin = ChannelContentPin::from_activation(
        deps.admitted.world_id,
        1,
        digest,
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (player_entry.x, player_entry.y, player_entry.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        deps.admitted.world_id,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        101,
        pin,
    )
    .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);
    deps.combat_room = Some(room);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = crate::gameplay_transport::activity_runtime::SharedOwner::Borrowed(&cells);
    owner.reset_epoch = 1;
    let key = ProductionKey::new("oteryn:raid.piece_of_cake").map_err(debug)?;
    let firing = id(91);
    let catalogue = RaidCatalogue::embedded().map_err(debug)?;
    let selected = catalogue
        .get(&key)
        .ok_or("actual Cake catalogue")?
        .select(bundle_digest, firing)
        .map_err(debug)?;
    seed_cake(h, &owner, &key, &selected, firing, bundle_digest).await?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(debug)?;
    let now = i64::from(selected.definition.waves[0].delay_ms);
    let wave = h
        .root
        .advance_raid_run(
            &h.node,
            firing,
            owner.channel_id,
            generation,
            selected.definition.clone(),
            now,
        )
        .await?
        .ok_or("real Cake wave")?;
    {
        let probe = at(32381, 31785);
        let permit = owner.spell_lane_permit().await.ok_or("probe lane")?;
        let runtime = deps.runtime.lock().await;
        let objects = deps.door.lock().await;
        let mut tx = h.pool.begin().await?;
        let binding = runtime.binding();
        let authority =
            crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                &mut tx,
                &permit,
                owner.root,
                owner.character,
                owner.holder,
                binding.runtime_scope(),
                binding.scope_generation().get(),
            )
            .await
            .map_err(debug)?;
        let room = owner.rooms.gate_monster_ai().ok_or("probe physical room")?;
        let tile = room.movement_cells().spell_tiles().lookup(
            room.movement_cells().scope(),
            crate::content::LogicalCell {
                x: probe.x,
                y: probe.y,
                z: i32::from(probe.floor),
            },
        );
        eprintln!(
            "W05D_CAKE_PREFLIGHT static_tile={tile:?} room_frame={:?} runtime_frame={:?}",
            room.frame_binding().digest(),
            runtime.content_pin().frame_binding_digest()
        );
        let qualified =
            crate::durability::native_world_read::qualified_creature_source_tile_in_transaction(
                &mut tx, &authority, room, &runtime, &*objects, probe,
            )
            .await;
        match qualified {
            Ok(ref fact) => eprintln!("W05D_CAKE_PREFLIGHT qualified={:?}", fact.qualified_tile()),
            Err(ref error) => eprintln!("W05D_CAKE_PREFLIGHT qualified_error={error:?}"),
        }
        let mut map = crate::movement::monster_map::CurrentMonsterMap::new(
            &mut tx, &authority, &runtime, room, &objects, native,
        );
        let admitted = map.read(&runtime, probe).await;
        assert!(
            map.validate_transaction(&runtime).await,
            "preflight transaction must remain actual/current"
        );
        assert_eq!(
            admitted,
            Some(true),
            "actual qualified Carlin source spawn candidate"
        );
        drop(map);
        tx.rollback().await?;
    }
    let outcomes = owner
        .apply_raid_wave(&wave, &selected, bundle_digest)
        .await
        .map_err(|_| "real physical Cake wave")?;
    assert_eq!(
        outcomes.len(),
        100,
        "all five cities retain twenty source entries"
    );
    assert_eq!(
        h.root
            .acknowledge_raid_effect(&h.node, wave.identity, outcomes, now)
            .await?,
        RaidWrite::Applied
    );
    let creature = {
        let mut runtime = deps.runtime.lock().await;
        let status = runtime
            .raid_creatures(firing)
            .map_err(debug)?
            .into_iter()
            .find(|s| s.alive)
            .ok_or("actual placed Carlin Cake Golem")?;
        let RaidSpawnOutcome::Placed(actor) = status.outcome else {
            return Err("real actor".into());
        };
        let before = runtime.read_actor_position(actor).map_err(debug)?;
        // Arrange adjacency only; the event is granted exclusively by the real next step.
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(actor, before, start)
            .map_err(debug)?;
        assert!(!runtime.actor_spell_reserved(actor));
        actor
    };
    {
        let permit = owner.spell_lane_permit().await.ok_or("current lane")?;
        let mut runtime = deps.runtime.lock().await;
        let objects = deps.door.lock().await;
        let mut tx = h.pool.begin().await?;
        let binding = runtime.binding();
        let authority =
            crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                &mut tx,
                &permit,
                owner.root,
                owner.character,
                owner.holder,
                binding.runtime_scope(),
                binding.scope_generation().get(),
            )
            .await
            .map_err(debug)?;
        let room = owner
            .rooms
            .gate_monster_ai()
            .ok_or("monster physical room")?;
        let mut map = crate::movement::monster_map::CurrentMonsterMap::new(
            &mut tx, &authority, &runtime, room, &objects, native,
        );
        assert_eq!(map.read(&runtime, machine).await, Some(true));
        assert!(map.validate_transaction(&runtime).await);
        let before = runtime.read_actor_position(creature).map_err(debug)?;
        let moved = map
            .view()
            .step(
                &mut runtime,
                &cells,
                creature,
                before,
                crate::movement::CardinalStep::East,
            )
            .map_err(debug)?;
        assert_eq!(moved.position(), machine);
        assert!(runtime.actor_spell_reserved(creature));
        let health = runtime.companion_snapshot(creature).map_err(debug)?.health;
        assert!(health > 0);
        assert!(
            runtime
                .commit_monster_lab_damage(creature, b"cake-pending-lethal", health)
                .is_err()
        );
        assert!(
            runtime
                .borrow_movement_position()
                .commit_creature_relocation(creature, moved, start)
                .is_err()
        );
        drop(map);
        tx.rollback().await?;
    }
    owner.states.chat.admit(
        deps.admitted.game_session_id,
        "Cake witness",
        owner.owner_now(),
    );
    assert_eq!(
        owner
            .extract_cake_machine_subjects(firing)
            .await
            .map_err(|_| "actual extraction")?
            .len(),
        1
    );
    assert!(!deps.runtime.lock().await.contains_live_creature(creature));
    assert!(
        owner
            .extract_cake_machine_subjects(firing)
            .await
            .map_err(|_| "extraction replay")?
            .is_empty()
    );
    let Some(oteryn_protocol_oteryn::chat::ChatLine::Local {
        speaker,
        speaker_name,
        mode,
        text,
        ..
    }) = owner.states.chat.next(deps.admitted.game_session_id)
    else {
        return Err("actual monster speech never reached CHAT".into());
    };
    assert_eq!(speaker.identity, creature.placement_identity());
    assert_eq!(speaker_name, "Cake Golem");
    assert_eq!(
        mode,
        oteryn_protocol_oteryn::chat::ChatSpeechMode::MonsterSay
    );
    assert_eq!(text, "CRUNSH!");
    assert!(
        owner
            .states
            .chat
            .next(deps.admitted.game_session_id)
            .is_none(),
        "replay emits no second speech"
    );
    {
        let runtime = deps.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let (_, events) = states
            .sounds
            .observe(&runtime, deps.actor, deps.admitted.game_session_id, 0, true)
            .ok_or("current native presentation")?;
        assert_eq!(
            events.len(),
            1,
            "only observed extraction emits source effect"
        );
        assert!(matches!(
            events[0].kind,
            oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                effect_id: 32,
                ..
            }
        ));
    }
    let counts: (i16, i16) =
        sqlx::query_as("SELECT substances,golems FROM game_cake_world_progress")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(counts, (0, 1));
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_cake_lure_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 1);
    let deaths: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_character_bestiary_kill_receipts)+(SELECT count(*) FROM game_party_kill_xp_receipts)+(SELECT count(*) FROM game_item_corpse_container_entries)")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        deaths, 0,
        "extraction is no death, XP, corpse or loot grant"
    );
    eprintln!(
        "W05D_CAKE_REAL_EXTRACTION_PG_COMPLETED elapsed={:?} golems=1 lure_receipts=1 deaths=0",
        started.elapsed()
    );
    Ok(())
}

async fn seed_cake(
    harness: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    key: &ProductionKey,
    selected: &SelectedRaid,
    firing: [u8; 16],
    digest: [u8; 32],
) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'raid-test-map',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,TIMESTAMPTZ '2030-02-21 06:00 Europe/Berlin','2030-02')")
        .bind(owner.world_id.as_bytes().as_slice()).bind(digest.as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(owner.world_id.as_bytes().as_slice())
        .execute(&harness.pool)
        .await?;
    sqlx::query("INSERT INTO game_world_raid_schedules VALUES(encode($1,'hex')::uuid,$2,2,1,1,2,60000,0,NULL,false)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_firings VALUES(encode($1,'hex')::uuid,$2,1,encode($3,'hex')::uuid,1,0,$4,$5,1)")
        .bind(owner.world_id.as_bytes().as_slice()).bind(key.as_str()).bind(firing.as_slice())
        .bind(i64::from(selected.definition.waves.iter().map(|wave| wave.delay_ms).max().ok_or("wave")?)
            + i64::from(selected.definition.despawn_after_ms.unwrap_or(0))).bind(digest.as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_raid_targets VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(firing.as_slice()).bind(owner.channel_id.as_bytes().as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_world_event_epochs(world_id,reset_epoch,active,announcement,seasonal_calendar_revision,seasonal_event_keys) VALUES(encode($1,'hex')::uuid,1,true,'Cake source window',1,ARRAY['piece_of_cake'])").bind(owner.world_id.as_bytes().as_slice()).execute(&harness.pool).await?;
    Ok(())
}

/// Test-only open geometry at real donor coordinates, with canonical Terrain
/// identities and facts. Fixture-local Terrain compact ids never mint Item identities.
fn cake_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    start: crate::foundation::MovementLocalPosition,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self as bundle, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
        Terrain, TerrainKind,
    };
    // Canonical Terrain records: terrain-00000-00499.json (walkable ground970),
    // terrain-02000-02499.json (iron floor8886), source appearance Ground/150.
    let palette = vec![
        PaletteEntry {
            key: "oteryn:terrain.tibia.i970".into(),
            family: Family::Terrain,
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
                blocks_projectile: None,
            }),
        },
        PaletteEntry {
            key: "oteryn:terrain.tibia.i8886".into(),
            family: Family::Terrain,
            id: 1,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
                blocks_projectile: None,
            }),
        },
    ];
    // Open test geometry covers both actual Carlin source raid boxes and machine.
    let (min_x, max_x, min_y, max_y) = (32305u16, 32386u16, 31780u16, 31825u16);
    let floors = std::collections::BTreeSet::from([-7i8]);
    let mut sectors = std::collections::BTreeMap::new();
    for floor in &floors {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let machine = x == 32346 && y == 31822;
                let items = vec![Item {
                    palette: u32::from(machine),
                    depth: 0,
                    attrs: Attrs::default(),
                }];
                sectors
                    .entry((*floor, y / 32, x / 32))
                    .or_insert_with(Vec::new)
                    .push(Tile {
                        x,
                        y,
                        flags: 0,
                        house: 0,
                        zones: vec![],
                        items,
                    });
            }
        }
    }
    let sectors = sectors
        .into_iter()
        .map(|((floor, sy, sx), tiles)| Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect::<Vec<_>>();
    let manifest = Manifest {
        subareas: Default::default(),
        format: bundle::FORMAT.to_owned(),
        min_reader_version: bundle::VERSION,
        projection_class: "server".to_owned(),
        compiler_version: "cake-scene-fixture".to_owned(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".to_owned(),
            world_schema_version: "world-schema-1".to_owned(),
            content_revision: "rev-1".to_owned(),
            ..Identity::default()
        },
        world: Extent {
            min_x,
            min_y,
            max_x: max_x.checked_add(1).ok_or("fixture extent")?,
            max_y: max_y.checked_add(1).ok_or("fixture extent")?,
            floors: floors.into_iter().collect(),
        },
        palette,
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let bytes = bundle::write(&manifest, &sectors, &Default::default()).map_err(debug)?;
    let digest = bundle::read(&bytes).map_err(debug)?.digest;
    let pins = crate::map::BundlePins {
        digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".to_owned(),
        world_schema_version: "world-schema-1".to_owned(),
        content_revision: "rev-1".to_owned(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: u16::try_from(start.x)?,
                y: u16::try_from(start.y)?,
                floor: i8::try_from(-start.floor)?,
            },
        },
        world,
        channel,
        |_key| None,
    )
    .map_err(debug)
}
