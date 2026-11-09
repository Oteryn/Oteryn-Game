//! Actual Channel USE of a source Cake tile, with durable world and native map proofs.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn m57_cake_bridge_real_step_final_bite_five_machines_and_brutus_rewards_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{
        run(async|admin|{
            let started=std::time::Instant::now();
            let h=Harness::create_with_admission_seed(admin,"m57_cake_complete",true,false).await?;
            {
                prepare_current_session(&h).await?;
                let seal=h.recovery.seal_current().map_err(debug)?;
                let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut deps=Dependencies::new()?;
                let(controller,pin)=crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id,None);
                let active=controller.active().ok_or("active content")?;let native=active.native_gameplay().ok_or("native")?;
                let start=MovementLocalPosition{x:33303,y:32051,floor:7};
                let first=MovementLocalPosition{x:33303,y:32070,floor:7};
                let base=MovementLocalPosition{x:33303,y:32071,floor:7};
                let machines=[(32346,31822),(32350,32275),(33251,31802),(33191,32397),(32377,32801)];
                let mut tops=vec![(first,12085),(base,12090)];
                for x in 33300..=33306 {for y in 32052..=32059 {
                    tops.push((MovementLocalPosition{x,y,floor:7},12085));
                }}
                for &(x,y) in &machines {tops.push((MovementLocalPosition{x,y,floor:7},8886));}
                let historical:Vec<_>=(32065..=32103).flat_map(|y|(33287..=33330).map(move|x|MovementLocalPosition{x,y,floor:7}))
                    .filter(|p|*p!=first && *p!=base).take(1115).collect();
                tops.extend(historical.iter().map(|p|(*p,12085)));
                let world=cake_bundle(native,deps.admitted.world_id,deps.admitted.channel_id,start,&tops)?;
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
                sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
                sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
                sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
                owner.reset_epoch=1;
                let bundle=world.overlay().base().digest();
                // Historical completed phase-one fixture. Every tested bite uses
                // the production fence, map owner, command receipt and DB clock.
                let mut tx=h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2026-02-21 05:00:00+00','2026-02')")
                    .bind(id(42).as_slice()).bind(bundle.as_slice()).bind(world.map_revision()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)").bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_cake_world_progress VALUES(encode($1,'hex')::uuid,2026,200,200)").bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_cake_phases(world_id,event_year,stage,layer,window_generation,layer_progress,window_opened_at,window_ends_at) VALUES(encode($1,'hex')::uuid,2026,2,4,1,1115,statement_timestamp()-interval '20 minutes',statement_timestamp()+interval '10 minutes')").bind(id(42).as_slice()).execute(&mut *tx).await?;
                // Historical quorum: 93 distinct source eaters, at most12 bites
                // each, every source tile distinct, spaced90s within the30min window.
                let mut eaters=Vec::new();
                // Copy writable source fields only: PostgreSQL owns generated
                // name_key, and each historical contributor has a distinct name.
                for ordinal in 0_u8..93 {
                    let who:String=sqlx::query_scalar("SELECT game_character_uuid_v7()::text").fetch_one(&mut *tx).await?;
                    let name=format!("Cake Eater {}{}",char::from(b'A'+ordinal/26),char::from(b'a'+ordinal%26));
                    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex) SELECT $1::uuid,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,$3,sex FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid")
                        .bind(&who).bind(id(41).as_slice()).bind(name).execute(&mut *tx).await?;
                    eaters.push(who);
                }
                for (i,p) in historical.iter().enumerate() {
                    let key=crate::map::view::placement_key(crate::map::overlay::TilePos{x:p.x as u16,y:p.y as u16,floor:-7},1).ok_or("historical placement")?;
                    let placement:Vec<_>=bundle.into_iter().chain(key.to_be_bytes()).collect();
                    let target=WorldObjectTarget{placement:placement.clone(),expected_revision:1};
                    let intent=oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
                    sqlx::query("INSERT INTO game_cake_bite_receipts(game_session_id,command_id,character_id,world_id,event_year,window_generation,layer,placement,source_item,progress_after,before_revision,intent,contributed_at) VALUES($1::uuid,$2,$1::uuid,encode($3,'hex')::uuid,2026,1,4,$4,12085,$5,1,$6,statement_timestamp()-interval '20 minutes'+$7*interval '90 seconds')")
                        .bind(&eaters[i/12]).bind(i64::try_from(i%12+1)?).bind(id(42).as_slice()).bind(placement)
                        .bind(i16::try_from(i+1)?).bind(intent).bind(i32::try_from(i%12)?).execute(&mut *tx).await?;
                }
                tx.commit().await?;
                owner.refresh_terrain_tools().await;
                use oteryn_protocol_oteryn::world_spatial::{StepDirection,StepDisposition};
                let at=|p:MovementLocalPosition|crate::map::overlay::TilePos{x:p.x as u16,y:p.y as u16,floor:-(p.floor as i8)};
                let bridge=MovementLocalPosition{x:33303,y:32052,floor:7};
                assert!(world.enterable(at(bridge)),"48227 replaces the original blocked cake stack");
                let projected:(i64,i64)=sqlx::query_as("SELECT count(*) FILTER(WHERE state_item_id=48227),count(*) FILTER(WHERE state_item_id=12113) FROM game_terrain_tool_transforms WHERE cake_event_year=2026").fetch_one(&h.pool).await?;
                assert_eq!(projected,(40,16));
                assert_eq!(owner.ordinary_paced_step(deps.actor,deps.admitted.game_session_id,StepDirection::South,None,None).await.0.disposition,StepDisposition::Moved);
                let tile=crate::map::view::compose_tile(world.overlay(),world.facts(),at(bridge)).map_err(debug)?.ok_or("bridge tile")?;
                assert_eq!(tile.entries.len(),1);
                assert_eq!(tile.entries[0].facts.appearance_id,48227);
                // Travel fixture relocates between distant donor sites; every
                // tested final tile entry still goes through the real paced STEP.
                stage_fixture_position(&owner,&world,deps.actor,deps.admitted.game_session_id,MovementLocalPosition{x:first.x-1,..first}).await?;
                let key=crate::map::view::placement_key(at(first),1).ok_or("last cake placement")?;
                let target=WorldObjectTarget{placement:bundle.into_iter().chain(key.to_be_bytes()).collect(),expected_revision:world.facts().object_revision(key)};
                let command=UseCommand{game_session_id:deps.admitted.game_session_id,command_id:1,item_fence:Some(fence)};
                assert_eq!(owner.use_world_object(deps.actor,command,target.clone()).await.disposition,UseDisposition::Committed);
                assert_eq!(owner.use_world_object(deps.actor,command,target).await.disposition,UseDisposition::Committed);
                assert!(owner.refresh_cake_world_map().await);
                let completed:(i16,bool)=sqlx::query_as("SELECT stage,pending_bonus FROM game_cake_phases").fetch_one(&h.pool).await?;
                assert_eq!(completed,(3,true));
                let map_receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_terrain_tool_receipts WHERE tool_item_instance_id IS NULL AND tool_definition_family IS NULL AND transformed_item_id=0").fetch_one(&h.pool).await?;
                assert_eq!(map_receipts,1,"real Cake USE has a map source, not a carried tool");
                let mut refused_tx=h.pool.begin().await?;
                let refusal=sqlx::query("INSERT INTO game_terrain_tool_receipts(game_session_id,command_id,character_id,tool_item_instance_id,intent,world_id,channel_id,placement,reset_epoch,transformed_item_id) SELECT game_session_id,999,character_id,NULL,intent,world_id,channel_id,placement,reset_epoch,0 FROM game_terrain_tool_receipts WHERE command_id=1")
                    .execute(&mut *refused_tx).await.err().ok_or("unrelated tool-less receipt must be refused")?;
                assert!(refusal.as_database_error().is_some_and(|error|error.message().contains("terrain tool receipt has no source in its world")));
                refused_tx.rollback().await?;
                // A later immutable server-save epoch activates the earned bonus.
                let mut tx=h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_reset_epochs SELECT (jsonb_populate_record(NULL::game_world_reset_epochs,to_jsonb(r)||jsonb_build_object('reset_epoch',2,'activated_at','2026-02-22 05:00:00+00'))).* FROM game_world_reset_epochs r WHERE reset_epoch=1").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_world_reset_active SET reset_epoch=2").execute(&mut *tx).await?;
                tx.commit().await?;
                owner.reset_epoch=2;
                assert!(owner.refresh_cake_world_map().await);
                assert!(h.root.cake_brutus_present(owner.world_id).await?);
                for &(x,y) in &machines {
                    for target in [base,MovementLocalPosition{x,y,floor:7}] {
                        let origin=MovementLocalPosition{x:target.x-1,..target};
                        stage_fixture_position(&owner,&world,deps.actor,deps.admitted.game_session_id,origin).await?;
                        // Let the movement owner's existing pacing clock advance.
                        let before=owner.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?;
                        let (step,duration)=owner.ordinary_paced_step(deps.actor,deps.admitted.game_session_id,StepDirection::East,Some(before),None).await;
                        assert_eq!(step.disposition,StepDisposition::Moved,"donor errand source at {target:?}");
                        if let Some(duration)=duration {tokio::time::sleep(duration).await;}
                    }
                }
                let progress:(bool,i16)=sqlx::query_as("SELECT armed,cities FROM game_cake_errands WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(progress,(false,31));
                super::item_look::seed_main_backpack(&h,&[]).await?;
                brutus_rewards(&h,&owner,&world,deps.actor,deps.admitted.game_session_id,fence).await?;
                // End-of-event cleanup restores the original two-entry stack
                // and its blocking geometry, with no canonical bridge left over.
                let mut tx=h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_world_reset_epochs SET activated_at='2026-02-27 05:00:00+00' WHERE reset_epoch=2").execute(&mut *tx).await?;
                tx.commit().await?;
                assert!(owner.refresh_cake_world_map().await);
                assert!(!world.enterable(at(bridge)));
                let restored=crate::map::view::compose_tile(world.overlay(),world.facts(),at(bridge)).map_err(debug)?.ok_or("restored tile")?;
                assert_eq!(restored.entries.len(),2);
                assert_eq!(restored.entries[1].facts.appearance_id,12085);
            }
            h.cleanup().await?;
            eprintln!("M57_CAKE_BRIDGE_PG_PASS floor40 rails16 real_step final_bite stage3 five_machines item_cupcake replay cleanup elapsed={:?}",started.elapsed());
            Ok(())
        }).map_err(|e|e.to_string())
    })?.join().map_err(|_|"M57 Cake thread panicked")??;
    Ok(())
}
/// Stage the fixture between geographically separate source sites through the
/// existing fenced runtime relocation owner. Only the following ordinary STEP
/// or NPC call can advance the quest; staging itself records no Cake progress.
async fn stage_fixture_position(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    world: &crate::map::boot::BundleWorld,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    at: MovementLocalPosition,
) -> TestResult {
    let tile = crate::map::overlay::TilePos {
        x: u16::try_from(at.x)?,
        y: u16::try_from(at.y)?,
        floor: i8::try_from(-at.floor)?,
    };
    assert!(
        world.enterable(tile),
        "fixture staging destination must be walkable"
    );
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    runtime
        .relocate_arena_player(actor, session, before, at)
        .map_err(debug)?;
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
            let centres: Vec<_> = tops
                .iter()
                .map(|(p, _)| *p)
                .chain([start])
                .filter(|p| (-p.floor) == i16::from(*floor) && (p.y - i32::from(y)).abs() <= 1)
                .collect();
            if centres.is_empty() {
                continue;
            }
            for x in min_x..=max_x {
                if !centres.iter().any(|p| (p.x - i32::from(x)).abs() <= 1) {
                    continue;
                }

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
                        attrs: Attrs {
                            action: if *id == 8886 {
                                Some(match (pos.x, pos.y) {
                                    (32346, 31822 | 31823) => 43910,
                                    (32350, 32275 | 32276) => 43911,
                                    (33251, 31802 | 31803) => 43912,
                                    (33191, 32397 | 32398) => 43913,
                                    (32377, 32801 | 32802) => 43914,
                                    _ => return Err("fixture machine lacks donor action".into()),
                                })
                            } else {
                                None
                            },
                            ..Attrs::default()
                        },
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

async fn brutus_rewards(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    world: &crate::map::boot::BundleWorld,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> TestResult {
    use crate::foundation::{MovementFacing, NpcPlacement};
    use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    stage_fixture_position(
        owner,
        world,
        actor,
        session,
        MovementLocalPosition {
            x: 33302,
            y: 32064,
            floor: 7,
        },
    )
    .await?;
    let book = NpcTalkBook::default().with(
        "oteryn:npc.brutus_the_baker",
        NpcTalkEntry::new("Brutus The Baker", Some("Hello!")),
    );
    owner
        .states
        .chat
        .install_npc_talk(std::sync::Arc::new(book));
    owner
        .states
        .chat
        .admit(session, "Cake Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc_actor = {
        let mut runtime = owner.runtime.lock().await;
        let npc = runtime
            .place_dynamic_npc(
                "oteryn:npc.brutus_the_baker",
                NpcPlacement {
                    x: 33303,
                    y: 32064,
                    floor: -7,
                    direction: MovementFacing::South,
                },
            )
            .map_err(debug)?;
        let npc = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|visible| visible.actor == npc)
            .ok_or("Brutus source actor")?;
        EntityRef {
            identity: npc.actor.placement_identity(),
            generation: npc.generation,
        }
    };
    let command = |command_id| UseCommand {
        game_session_id: session,
        command_id,
        item_fence: Some(fence),
    };
    let intent = |text: &str| NpcTalkIntent {
        npc_actor,
        text: text.into(),
    };
    assert_eq!(
        owner
            .npc_talk(actor, command(100), intent("hi"))
            .await
            .disposition,
        NpcIntentDisposition::Ok
    );
    for (id, text) in [(101, "abyes backpack"), (102, "blueberry")] {
        // Respect NPC0-RL-04's actual four-intents-per-second ingress clock.
        tokio::time::sleep(std::time::Duration::from_millis(260)).await;
        let reply = owner.npc_talk(actor, command(id), intent(text)).await;
        assert!(
            reply.inventory_changed,
            "Brutus {text}: disposition={:?}, unknown={}",
            reply.disposition, reply.unknown
        );
        tokio::time::sleep(std::time::Duration::from_millis(260)).await;
        let replay = owner.npc_talk(actor, command(id), intent(text)).await;
        assert!(
            replay.inventory_changed,
            "same command replays {text}: disposition={:?}, unknown={}",
            replay.disposition, replay.unknown
        );
    }
    for (id, text) in [(103, "cabinet"), (104, "lemon")] {
        tokio::time::sleep(std::time::Duration::from_millis(260)).await;
        let refused = owner.npc_talk(actor, command(id), intent(text)).await;
        assert_eq!(
            refused.disposition,
            NpcIntentDisposition::Ok,
            "category refusal must reach the NPC owner, not an ingress limit"
        );
        assert!(
            !refused.inventory_changed,
            "annual category already claimed: {text}"
        );
    }
    let claims:(bool,bool,i64,i64)=sqlx::query_as("SELECT e.item_claimed,e.cupcake_claimed,(SELECT count(*) FROM game_cake_reward_claims),(SELECT count(*) FROM game_reward_claim_mint_receipts) FROM game_cake_errands e WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(claims, (true, true, 2, 2));
    let items:Vec<String>=sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE definition_production_key IN('oteryn:item.tibia.i20347','oteryn:item.tibia.i28484') ORDER BY definition_production_key").fetch_all(&h.pool).await?;
    assert_eq!(
        items,
        ["oteryn:item.tibia.i20347", "oteryn:item.tibia.i28484"]
    );
    Ok(())
}
