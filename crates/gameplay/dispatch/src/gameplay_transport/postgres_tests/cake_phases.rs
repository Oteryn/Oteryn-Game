//! Actual Channel USE of a source Cake tile, with durable world and native map proofs.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn w05d_cake_bites_use_native_map_and_commit_exact_world_receipts() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{
        run(async|admin|{
            let started=std::time::Instant::now();
            let h=Harness::create_with_admission_seed(admin,"cake_stage_two",true,false).await?;
            {
                prepare_current_session(&h).await?;
                let seal=h.recovery.seal_current().map_err(debug)?;
                let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut deps=Dependencies::new()?;
                let(controller,pin)=crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id,None);
                let active=controller.active().ok_or("active content")?;let native=active.native_gameplay().ok_or("native")?;
                let start=MovementLocalPosition{x:33302,y:32070,floor:7};
                let first=MovementLocalPosition{x:33303,y:32070,floor:7};
                let second=MovementLocalPosition{x:33303,y:32071,floor:7};
                let world=cake_bundle(native,deps.admitted.world_id,deps.admitted.channel_id,start,&[(first,12085),(second,12086)])?;
                let cells=world.movement_cells(pin.qualified_room().movement_cells()).map_err(debug)?;
                let room=super::rust_lottery::bundle_action_room(pin.qualified_room(),&world,deps.admitted.world_id,&cells)?;
                let content_pin=ChannelContentPin::from_activation(deps.admitted.world_id,pin.activation_sequence(),pin.identity().server_artifact_digest(),pin.identity().client_artifact_digest(),room.frame_binding().digest(),room.map_revision_digest(),(start.x,start.y,start.floor));
                let mut runtime=ChannelRuntimeV1::from_committed_assignment(deps.admitted.world_id,deps.admitted.channel_id,crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,1,1,1,"runtime-scope-assignment:1",2,content_pin).map_err(debug)?;
                let reserved=runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
                deps.actor=runtime.commit_fresh_session(reserved).map_err(debug)?;
                runtime.initialize_pinned_test_position(deps.actor,start).map_err(debug)?;
                runtime.bind_attacker_lease(deps.actor,deps.admitted.game_session_id,crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,2).map_err(debug)?).map_err(debug)?;
                deps.runtime=tokio::sync::Mutex::new(runtime);
                let mut owner=deps.owner(&h,&authority);owner.active_generation=Some(active);owner.movement_cells=crate::gameplay_transport::activity_runtime::SharedOwner::Borrowed(&cells);
                owner.rooms=crate::gameplay_transport::SeamRooms::new(None,Some(&room));
                deps.initialize(&owner).await;
                let current=crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone()).current_session_at(deps.admitted.game_session_id).await?.0;
                let fence=crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
                let equipment_before:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(equipment_before,0);
                let ground_key=crate::map::view::placement_key(world.start(),0).ok_or("unrelated ground placement")?;
                let unrelated=WorldObjectTarget{placement:world.base().digest().into_iter().chain(ground_key.to_be_bytes()).collect(),expected_revision:0};
                let unrelated_command=UseCommand{game_session_id:deps.admitted.game_session_id,command_id:900,item_fence:Some(fence)};
                assert!(owner.use_cake_world_object(deps.actor,unrelated_command,&unrelated).await.is_none(),"unrelated base source never enters Cake's equipment owner");
                assert_eq!(owner.use_world_object(deps.actor,unrelated_command,unrelated).await.disposition,UseDisposition::NothingToUse);
                // Separate the ordinary200ms action clock from Cake's90s bite rule.
                tokio::time::sleep(std::time::Duration::from_millis(210)).await;

                sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
                sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
                sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
                owner.reset_epoch=1;
                let map=owner.bundle_map().ok_or("bundle map")?;let bundle=map.overlay.base().digest();
                // Historical completed phase-one fixture. Every tested bite uses
                // the production fence, map owner, command receipt and DB clock.
                let mut tx=h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2026-02-21 05:00:00+00','2026-02')")
                    .bind(id(42).as_slice()).bind(bundle.as_slice()).bind(world.map_revision()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)").bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_cake_world_progress VALUES(encode($1,'hex')::uuid,2026,200,200)").bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_cake_phases(world_id,event_year,stage,layer) VALUES(encode($1,'hex')::uuid,2026,2,1)").bind(id(42).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                owner.refresh_terrain_tools().await;
                let target=|position:MovementLocalPosition|->TestResult<WorldObjectTarget>{
                    let key=crate::map::view::placement_key(crate::map::overlay::TilePos{x:u16::try_from(position.x)?,y:u16::try_from(position.y)?,floor:i8::try_from(-position.floor)?},1).ok_or("cake placement")?;
                    Ok(WorldObjectTarget{placement:bundle.into_iter().chain(key.to_be_bytes()).collect(),expected_revision:map.facts.object_revision(key)})
                };
                let one=target(first)?;let two=target(second)?;
                let command=UseCommand{game_session_id:deps.admitted.game_session_id,command_id:1,item_fence:Some(fence)};
                assert!(!world.enterable(crate::map::overlay::TilePos{x:33303,y:32070,floor:-7}));
                assert_eq!(owner.use_world_object(deps.actor,command,one.clone()).await.disposition,UseDisposition::Committed);
                assert_eq!(owner.use_world_object(deps.actor,command,one).await.disposition,UseDisposition::Committed,"same command replays before resolving the now-hidden source");
                tokio::time::sleep(std::time::Duration::from_millis(210)).await;
                assert_eq!(owner.use_world_object(deps.actor,UseCommand{command_id:2,..command},two).await.disposition,UseDisposition::NotPossible,"90-second cooldown is world-owner time");
                let progress:i16=sqlx::query_scalar("SELECT layer_progress FROM game_cake_phases WHERE world_id=encode($1,'hex')::uuid AND event_year=2026").bind(id(42).as_slice()).fetch_one(&h.pool).await?;
                let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_cake_bite_receipts WHERE world_id=encode($1,'hex')::uuid").bind(id(42).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!((progress,receipts),(1,1));
                assert!(world.enterable(crate::map::overlay::TilePos{x:33303,y:32070,floor:-7}),"eaten solid source disappears from actual movement collision");
                let mut view=crate::gameplay_transport::world_map::SessionMapView::default();
                let mut items=crate::gameplay_transport::item_view::SessionItemView::resume(Default::default()).with_map_view();
                let update=owner.observe_world_map(deps.actor,&mut view,&mut items).await.ok_or("native map observer")?.map_err(debug)?.ok_or("native map snapshot")?;
                let crate::gameplay_transport::world_map::MapUpdate::Snapshot(snapshot)=update else{return Err("fresh map snapshot".into())};
                let decoded=oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload).map_err(debug)?;
                let eaten=decoded.tiles.iter().find(|tile|tile.position.x==33303&&tile.position.y==32070).ok_or("eaten tile")?;
                assert_eq!(eaten.items.len(),1,"native client sees ground after the source cake is eaten");
            }
            h.cleanup().await?;
            eprintln!("W05D_CAKE_PHASE_POSTGRES_COMPLETED native_bite_world_receipt_replay_cooldown_collision_projection elapsed={:?}",started.elapsed());
            Ok(())
        }).map_err(|e|e.to_string())
    })?.join().map_err(|_|"Cake phase thread panicked")??;
    Ok(())
}
/// Test-only open terrain at real donor coordinates. Item identities and compact
/// references come from the actual active generation; production geometry is untouched.
fn cake_bundle(
    native: &crate::content::native_gameplay::NativeGameplayState,
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    start: crate::foundation::MovementLocalPosition,
    tops: &[(crate::foundation::MovementLocalPosition, u32)],
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self as bundle, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
        Terrain, TerrainKind,
    };
    let index = native.item_index().ok_or("native Item definition index")?;
    let mut keys = vec!["terrain:grass".to_owned()];
    for (_, id) in tops {
        let key = format!("oteryn:item.tibia.i{id}");
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    let palette = keys
        .iter()
        .enumerate()
        .map(|(n, key)| {
            Ok(PaletteEntry {
                key: key.clone(),
                family: if n == 0 {
                    Family::Terrain
                } else {
                    Family::Item
                },
                id: if n == 0 {
                    0
                } else {
                    index
                        .definition_ref("Item", key, "definition-r1")
                        .ok_or_else(|| format!("missing canonical fixture reference {key}"))?
                        .get()
                        - 1
                },
                terrain: (n == 0).then_some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(true),
                    ground_speed: Some(150),
                    blocks_projectile: None,
                }),
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    let min_x = u16::try_from(
        tops.iter()
            .map(|(p, _)| p.x)
            .chain([start.x])
            .min()
            .ok_or("fixture x")?
            - 1,
    )?;
    let max_x = u16::try_from(
        tops.iter()
            .map(|(p, _)| p.x)
            .chain([start.x])
            .max()
            .ok_or("fixture x")?
            + 1,
    )?;
    let min_y = u16::try_from(
        tops.iter()
            .map(|(p, _)| p.y)
            .chain([start.y])
            .min()
            .ok_or("fixture y")?
            - 1,
    )?;
    let max_y = u16::try_from(
        tops.iter()
            .map(|(p, _)| p.y)
            .chain([start.y])
            .max()
            .ok_or("fixture y")?
            + 1,
    )?;
    let floors = tops
        .iter()
        .map(|(p, _)| p.floor)
        .chain([start.floor])
        .map(|floor| i8::try_from(-floor))
        .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
    let mut sectors = std::collections::BTreeMap::new();
    for floor in &floors {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let mut items = vec![Item {
                    palette: 0,
                    depth: 0,
                    attrs: Attrs::default(),
                }];
                for (pos, id) in tops.iter().filter(|(pos, _)| {
                    pos.x == i32::from(x)
                        && pos.y == i32::from(y)
                        && pos.floor == -i16::from(*floor)
                }) {
                    let key = format!("oteryn:item.tibia.i{id}");
                    let _ = pos;
                    items.push(Item {
                        palette: u32::try_from(
                            keys.iter()
                                .position(|known| known == &key)
                                .ok_or("fixture palette")?,
                        )?,
                        depth: 0,
                        attrs: Attrs::default(),
                    });
                }
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
        |key| {
            Some(crate::map::facts::ItemDefinition {
                reference: index.definition_ref("Item", key, "definition-r1")?,
                solid: native
                    .item_policy(key, "definition-r1")?
                    .record()
                    .attributes
                    .blocks_movement,
                blocks_projectile: native
                    .item_policy(key, "definition-r1")?
                    .record()
                    .attributes
                    .blocks_projectile?,
                pickupable: false,
                door: false,
            })
        },
    )
    .map_err(debug)
}
