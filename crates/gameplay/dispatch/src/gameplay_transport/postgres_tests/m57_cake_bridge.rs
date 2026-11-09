//! Cake's source bridge through the live Channel map, movement and PostgreSQL phase.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn m57_cake_bridge_creation_cleanup_and_real_traversal() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        run(async |admin| {
            let started = std::time::Instant::now();
            let h = Harness::create_with_admission_seed(admin, "m57_bridge", true, false).await?;
            {
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut deps = Dependencies::new()?;
                let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id, None);
                let active = controller.active().ok_or("active content")?;
                let native = active.native_gameplay().ok_or("native content")?;
                let start = MovementLocalPosition { x: 33303, y: 32052, floor: 7 };
                let world = bridge_bundle(deps.admitted.world_id, deps.admitted.channel_id, start)?;
                let cells = world.movement_cells(pin.qualified_room().movement_cells()).map_err(debug)?;
                let room = super::rust_lottery::bundle_action_room(pin.qualified_room(), &world, deps.admitted.world_id, &cells)?;
                let content_pin = ChannelContentPin::from_activation(
                    deps.admitted.world_id, pin.activation_sequence(),
                    pin.identity().server_artifact_digest(), pin.identity().client_artifact_digest(),
                    room.frame_binding().digest(), room.map_revision_digest(),
                    (start.x, start.y, start.floor),
                );
                let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                    deps.admitted.world_id, deps.admitted.channel_id,
                    crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                    1, 1, 1, "runtime-scope-assignment:1", 2, content_pin,
                ).map_err(debug)?;
                let reserved = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
                deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
                runtime.initialize_pinned_test_position(deps.actor, start).map_err(debug)?;
                runtime.bind_attacker_lease(deps.actor, deps.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?, 2).map_err(debug)?).map_err(debug)?;
                deps.runtime = tokio::sync::Mutex::new(runtime);
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(active);
                owner.movement_cells = crate::gameplay_transport::activity_runtime::SharedOwner::Borrowed(&cells);
                owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
                owner.reset_epoch = 1;
                deps.initialize(&owner).await;
                let bundle = world.base().digest();
                sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                    .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
                sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                    .bind(id(42).as_slice()).bind(id(43).as_slice())
                    .bind(pin.identity().server_artifact_digest().as_slice())
                    .bind(pin.identity().client_artifact_digest().as_slice())
                    .bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
                // Historical world contributions are fixture input. The tested
                // presence and every crossing use the production current owner.
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2026-02-21 05:00:00+00','2026-02')")
                    .bind(id(42).as_slice()).bind(bundle.as_slice()).bind(world.map_revision())
                    .bind(pin.identity().server_artifact_digest().as_slice())
                    .bind(pin.identity().client_artifact_digest().as_slice())
                    .bind(room.frame_binding().digest().as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
                    .bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_cake_world_progress VALUES(encode($1,'hex')::uuid,2026,200,200)")
                    .bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_cake_phases(world_id,event_year,stage,layer) VALUES(encode($1,'hex')::uuid,2026,2,1)")
                    .bind(id(42).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                let water = TilePos { x: 33303, y: 32053, floor: -7 };
                assert!(!world.enterable(water));
                // This is the same Channel refresh used by real terrain actions.
                owner.refresh_terrain_tools().await;
                assert!(world.enterable(water), "stage two must replace the water with the admitted Cake floor");
                let entries = world.facts().movement_item_entries();
                assert_eq!(entries.iter().filter(|(_, entry)| entry.facts.appearance_id == 48227).count(), 40);
                assert_eq!(entries.iter().filter(|(_, entry)| entry.facts.appearance_id == 12113).count(), 16);
                let index = native.item_index().ok_or("Item index")?;
                let reference = index.definition_ref("Item", "oteryn:item.tibia.i48227", "definition-r1").ok_or("bridge Item")?;
                assert!(entries.iter().filter(|(_, entry)| entry.facts.appearance_id == 48227)
                    .all(|(_, entry)| entry.facts.definition == oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference)));
                owner.refresh_terrain_tools().await;
                assert_eq!(world.facts().movement_item_entries(), entries, "repeated current projection creates no duplicate floor");
                for y in 32053..=32060 {
                    let (outcome, wait) = owner.ordinary_paced_step(deps.actor, deps.admitted.game_session_id, StepDirection::South, None, None).await;
                    assert_eq!(outcome.moved_to.ok_or("real Cake bridge crossing")?.actor_position.y, y);
                    if y != 32060 && let Some(wait) = wait { tokio::time::sleep(wait).await; }
                }
                sqlx::query("UPDATE game_cake_phases SET stage=3,layer=4 WHERE world_id=encode($1,'hex')::uuid AND event_year=2026")
                    .bind(id(42).as_slice()).execute(&h.pool).await?;
                owner.refresh_terrain_tools().await;
                assert!(world.enterable(water), "completion leaves the bridge until event cleanup");
                sqlx::query("UPDATE game_cake_phases SET event_end_cleaned_up=true WHERE world_id=encode($1,'hex')::uuid AND event_year=2026")
                    .bind(id(42).as_slice()).execute(&h.pool).await?;
                owner.refresh_terrain_tools().await;
                assert!(!world.enterable(water), "cleanup restores the original impassable water");
                assert!(world.facts().movement_item_entries().is_empty());
                owner.refresh_terrain_tools().await;
                assert!(!world.enterable(water), "cleanup replay does not rebuild the bridge");
            }
            h.cleanup().await?;
            assert!(started.elapsed() > std::time::Duration::from_millis(10));
            eprintln!("M57_CAKE_BRIDGE_POSTGRES_COMPLETED elapsed={:?}", started.elapsed());
            Ok(())
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "M57 bridge thread panicked")??;
    Ok(())
}

fn bridge_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    start: MovementLocalPosition,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Item, Tile};
    use oteryn_world_bundle::writer::{self as bundle, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain, TerrainKind};
    // The source's 4519 shore is walkable (150); 4597 water is not (speed 0).
    // This combat-only scene never grants full Source map qualification.
    let palette = [(4519, true, 150), (4597, false, 0)].into_iter().enumerate().map(|(index, (id, walkable, speed))| PaletteEntry {
        key: format!("oteryn:terrain.tibia.i{id}"), family: Family::Terrain, id: index as u32,
        terrain: Some(Terrain { kind: TerrainKind::Ground, walkable: Some(walkable), ground_speed: Some(speed), blocks_projectile: Some(false) }),
    }).collect();
    let mut tiles = Vec::new();
    for y in 32052..=32060 {
        for x in 33300..=33306 {
            tiles.push(Tile { x, y, flags: 0, house: 0, zones: vec![], items: vec![Item {
                palette: if (x == 33303 && y == 32052) || y == 32060 { 0 } else { 1 },
                depth: 0, attrs: Default::default(),
            }] });
        }
    }
    let manifest = Manifest {
        subareas: Default::default(), format: bundle::FORMAT.into(), min_reader_version: bundle::VERSION,
        projection_class: "server".into(), compiler_version: "m57-source-bridge-scene".into(), build_class: BuildClass::Production,
        identity: Identity { project_format_version: "OTERYN_WORLD_PROJECT/v2".into(), world_schema_version: "world-schema-1".into(), content_revision: "rev-1".into(), ..Default::default() },
        world: Extent { min_x: 33300, min_y: 32052, max_x: 33307, max_y: 32061, floors: vec![-7] },
        palette, draft_areas: vec![], skipped_provisional_keys: vec![], dropped_teleports: vec![], spawns: Default::default(), npcs: Default::default(),
    };
    let sectors = [Sector { floor: -7, sx: 33300 / 32, sy: 32052 / 32, tiles }];
    let bytes = bundle::write(&manifest, &sectors, &Default::default()).map_err(debug)?;
    let digest = bundle::read(&bytes).map_err(debug)?.digest;
    let pins = crate::map::BundlePins { digest, project_format_version: "OTERYN_WORLD_PROJECT/v2".into(), world_schema_version: "world-schema-1".into(), content_revision: "rev-1".into(), production: false };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(&bytes, &crate::map::boot::BootPins {
        map_revision: crate::map::overlay::map_revision(&base), bundle: pins,
        start: TilePos { x: u16::try_from(start.x)?, y: u16::try_from(start.y)?, floor: i8::try_from(-start.floor)? },
    }, world, channel, |_| None).map_err(debug)
}
