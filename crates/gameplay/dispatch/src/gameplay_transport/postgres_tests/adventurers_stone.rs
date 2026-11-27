//! The real carried USE route, its floor-change publication, and the Guild exit.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::adventurers_stone::STONE_KEY;
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, MovementLocalPosition, WorldId,
};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::character_home_town::ADVENTURERS_GUILD_RETURN_ACTION;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, UseCommand,
    serve_admitted,
};
use crate::gameplay_transport::{ComposedFreshAdmission, item_view::ItemKey};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::{ActorPosition, StepDirection};
use oteryn_protocol_oteryn::world_spatial_entities::{
    WorldSpatialDeltaView, decode_world_spatial_delta_view,
};
use oteryn_protocol_oteryn::{
    self as wire, MessageType, world_interaction, world_map, world_object,
};
use tokio::io::AsyncWriteExt;

const BAG: u8 = 170;
const STONE: u8 = 171;
const ARRIVAL: MovementLocalPosition = MovementLocalPosition {
    x: 32210,
    y: 32300,
    floor: 6,
};
const REFUSAL: &str =
    "Try to move more to the center of a temple to use the spiritual energy for a teleport.";
// City temple tiles from content/world/areas/cities/areas-00000-00021.json.
const CARLIN: TilePos = TilePos {
    x: 32360,
    y: 31782,
    floor: -7,
};
const THAIS: TilePos = TilePos {
    x: 32369,
    y: 32241,
    floor: -7,
};

#[derive(Clone, Copy)]
enum Case {
    Wire,
    WireWithRuneUse,
    BattleSign,
    Return,
}

#[test]
fn adventurers_stone_use_writes_position_and_floor_map_before_result() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_stack(admin, "guild_stone_wire", Case::Wire)
}

#[test]
fn adventurers_stone_handle_reaches_teleport_owner_with_rune_use_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_stack(admin, "guild_stone_rune_use", Case::WireWithRuneUse)
}

#[test]
fn adventurers_stone_use_refuses_battle_sign_with_donor_text() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_stack(admin, "guild_stone_fight", Case::BattleSign)
}

#[test]
fn adventurers_stone_persists_landing_and_guild_exit_consumes_origin_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    on_stack(admin, "guild_stone_return", Case::Return)
}

fn on_stack(admin: String, name: &'static str, case: Case) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async move |_| scenario(admin, name, case).await).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Stone fixture thread panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String, name: &str, case: Case) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, name, false, false).await?;
    prepare_current_session(&h).await?;
    seed_stone(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = fence(1)?.runtime_scope.world_id();
    let (controller, pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native content")?;
    let reference = native
        .item_index()
        .ok_or("item index")?
        .definition_ref("Item", STONE_KEY, "definition-r1")
        .ok_or("Stone definition reference")?;
    let bundle = test_world(world)?;
    let cells = bundle
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let start = crate::content::NativeEntryStart {
        x: i32::from(CARLIN.x),
        y: i32::from(CARLIN.y),
        floor: 7,
    };
    let room = pin
        .qualified_room()
        .bundle_view(
            cells.clone(),
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            start,
        )
        .ok_or("bundle room")?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (i32::from(CARLIN.x), i32::from(CARLIN.y), 7),
    );
    let deps = Dependencies::with_content(channel_pin, room, native.spell_book().clone())?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    deps.initialize(&owner).await;
    let actor = deps.actor;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let mut supported = vec![
        4,
        6,
        7,
        world_object::CAPABILITY_ITEM_USE_V1,
        world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
    ];
    let mut offered = owner.offered_capabilities().to_vec();
    if matches!(case, Case::BattleSign) {
        supported.push(world_interaction::CAPABILITY_WORLD_INTERACTION_V1);
        if !offered
            .iter()
            .any(|capability| capability.id == world_interaction::CAPABILITY_WORLD_INTERACTION_V1)
        {
            offered.push(OfferedCapability {
                id: world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
                requires: &world_interaction::CAPABILITY_WORLD_INTERACTION_V1_REQUIRES,
            });
        }
    }
    if matches!(case, Case::WireWithRuneUse) {
        supported.push(world_object::CAPABILITY_RUNE_USE_V1);
        offered.push(OfferedCapability {
            id: world_object::CAPABILITY_RUNE_USE_V1,
            requires: world_object::CAPABILITY_RUNE_USE_V1_REQUIRES,
        });
    }
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(&offered, &supported)
        .ok_or("Stone USE and map capabilities")?;
    if matches!(case, Case::BattleSign) {
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(world_interaction::CAPABILITY_WORLD_INTERACTION_V1)
        );
    }
    assert_eq!(
        admitted
            .continuity
            .selected_capabilities
            .contains(world_object::CAPABILITY_RUNE_USE_V1),
        matches!(case, Case::WireWithRuneUse),
    );
    let session = admitted.game_session_id;
    let current = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "item fence")?
        .ok_or("active item fence")?;
    admitted.item_fence = Some(current.item_fence());
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            session,
            CharacterLease::new(
                CharacterId::decode(current.character_id.as_bytes()).map_err(debug)?,
                current.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    owner.admit_progression_session(&mut admitted).await;
    assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);

    if matches!(case, Case::BattleSign) {
        let runtime = owner.runtime.lock().await;
        owner.states.attack.lock().await.record_hit_taken(
            &runtime,
            actor,
            session,
            owner.owner_now(),
        );
    }
    if matches!(case, Case::Return) {
        assert_eq!(owner.home_town(session, None).await, Some(8));
        let use_outcome = owner
            .use_backpack_item(
                actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 1,
                    item_fence: admitted.item_fence,
                },
                ItemKey::Instance(id(STONE)),
            )
            .await;
        assert_eq!(use_outcome.disposition, UseDisposition::Committed);
        assert_eq!(saved_position(&owner, current).await?, ARRIVAL);
        assert_eq!(
            h.root
                .read_guild_stone_origin(&authority, &h.node, current.item_fence())
                .await?,
            Some(6)
        );

        // The next real STEP enters the action-4253 tile. The home town is Thais,
        // so getting to Carlin proves the handler read the durable Stone origin.
        let (returned, _) = owner
            .ordinary_paced_step(actor, session, StepDirection::East, None, None)
            .await;
        let expected = MovementLocalPosition {
            x: i32::from(CARLIN.x),
            y: i32::from(CARLIN.y),
            floor: 7,
        };
        assert_eq!(
            returned
                .moved_to
                .ok_or("Guild return positioning")?
                .actor_position,
            wire_position(expected)
        );
        assert_eq!(saved_position(&owner, current).await?, expected);
        assert_eq!(
            h.root
                .read_guild_stone_origin(&authority, &h.node, current.item_fence())
                .await?,
            None
        );
        assert_eq!(owner.home_town(session, None).await, Some(8));
    } else {
        let refused = matches!(case, Case::BattleSign);
        use_on_wire(&owner, admitted, reference, refused).await?;
        if refused {
            let position = owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position();
            assert_eq!(
                wire_position(position),
                ActorPosition {
                    x: i32::from(CARLIN.x),
                    y: i32::from(CARLIN.y),
                    floor: 7
                }
            );
            assert_eq!(
                h.root
                    .read_guild_stone_origin(&authority, &h.node, current.item_fence())
                    .await?,
                None
            );
            // The feedback may have reached the peer during the command update;
            // the wire fixture checks its text before closing the peer.
        } else {
            assert_eq!(saved_position(&owner, current).await?, ARRIVAL);
        }
    }
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(STONE).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(quantity, 1);
    eprintln!("C12_GUILD_STONE_POSTGRES_CASE_COMPLETED {name}");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

fn wire_position(p: MovementLocalPosition) -> ActorPosition {
    ActorPosition {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}

async fn saved_position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    current: CurrentCharacterGameplayFence,
) -> TestResult<MovementLocalPosition> {
    let saved = owner
        .root
        .read_character_last_position(owner.character, owner.holder, current)
        .await
        .map_err(debug)?
        .ok_or("durable Stone position")?;
    let cell = saved.record.cell;
    Ok(MovementLocalPosition {
        x: cell.x,
        y: cell.y,
        floor: cell.floor,
    })
}

async fn use_on_wire(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    reference: std::num::NonZeroU32,
    refused: bool,
) -> TestResult {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let peer = async move {
        let mut header = None;
        let mut handle = None;
        loop {
            let frame = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in wire::decode_snapshot_chunk(envelope.payload())
                    .map_err(debug)?
                    .1
                {
                    if domain.domain_id == world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
                        header = Some(
                            world_map::decode_world_map_snapshot(domain.payload)
                                .map_err(debug)?
                                .header,
                        );
                    }
                    if domain.domain_id == wire::item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory = wire::item_view::decode_character_inventory(domain.payload)
                            .map_err(debug)?;
                        handle = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == reference)
                            .map(|entry| entry.handle);
                    }
                }
            }
        }
        let payload = world_object::encode_item_use_intent(&world_object::UseIntent {
            target: world_object::UseTarget::Item(handle.ok_or("carried Stone handle")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let command_id = admitted.continuity.next_command_id;
        let frame = wire::encode_client_command(
            admitted.continuity.connection_generation,
            &wire::ClientCommandValue {
                command_id,
                command_type: world_object::COMMAND_TYPE_USE_INTENT,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        let (mut position_seen, mut map_seen, mut result_seen, mut text_seen) =
            (false, false, false, false);
        let mut guild_map_pending = false;
        loop {
            let frame = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = crate::foundation::decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in wire::decode_snapshot_chunk(envelope.payload())
                    .map_err(debug)?
                    .1
                {
                    if domain.domain_id == world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
                        let map =
                            world_map::decode_world_map_snapshot(domain.payload).map_err(debug)?;
                        guild_map_pending = map.header.origin == wire_position(ARRIVAL);
                        header = Some(map.header);
                    }
                }
            } else if envelope.message_type() == MessageType::SnapshotCommit {
                map_seen |= guild_map_pending;
                guild_map_pending = false;
            } else if envelope.message_type() == MessageType::StateDelta {
                let delta = wire::decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == wire::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
                    let position = match decode_world_spatial_delta_view(
                        admitted.continuity.selected_capabilities.as_slice(),
                        delta.delta_type,
                        delta.payload,
                    )
                    .map_err(debug)?
                    {
                        WorldSpatialDeltaView::OwnActor(row) => row.actor_position,
                        WorldSpatialDeltaView::Entities(row) => row.actor_position,
                    };
                    position_seen |= position == wire_position(ARRIVAL);
                } else if delta.domain_id == world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
                    let map = world_map::decode_world_map_delta(
                        &header.ok_or("map snapshot")?,
                        delta.payload,
                    )
                    .map_err(debug)?;
                    map_seen |= map.header.origin == wire_position(ARRIVAL);
                    header = Some(map.header);
                } else if delta.domain_id == wire::chat::STATE_DOMAIN_CHAT && delta.delta_type == 1
                {
                    let line = wire::chat::decode_chat_line(delta.payload).map_err(debug)?;
                    text_seen |= matches!(line, wire::chat::ChatLine::Private { text, .. } if text == REFUSAL);
                }
            } else if envelope.message_type() == MessageType::CommandResult {
                let result = wire::decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                if refused {
                    assert_eq!(
                        world_object::decode_use_result(result.payload).map_err(debug)?,
                        UseDisposition::NotPossible
                    );
                    assert!(!position_seen && !map_seen);
                } else {
                    assert_eq!(
                        world_object::decode_use_result(result.payload).map_err(debug)?,
                        UseDisposition::Committed
                    );
                    assert!(
                        position_seen && map_seen,
                        "both position and the new floor must precede USE result"
                    );
                }
                result_seen = true;
            }
            if result_seen && (!refused || text_seen) {
                break;
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let (_, result) = tokio::join!(serving, peer);
    result
}

async fn seed_stone(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "DELETE FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    for (seed, key) in [(BAG, "oteryn:item.tibia.i2854"), (STONE, STONE_KEY)] {
        let item =
            crate::durability::item_fee_burn::test_support::item(seed, key, "definition-r1", 1);
        sqlx::raw_sql(sqlx::AssertSqlSafe(item))
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(STONE).as_slice()).bind(id(42).as_slice()).bind(id(205).as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

fn test_world(world: WorldId) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, SECTOR_SIZE, Tile};
    use oteryn_world_bundle::subarea::Temple;
    use oteryn_world_bundle::writer::{
        self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
        TerrainKind,
    };
    let mut subareas = super::character_home_town_fixtures::towns();
    subareas.areas.truncate(3);
    for (area, at) in subareas.areas.iter_mut().skip(1).zip([THAIS, CARLIN]) {
        area.temple = Some(Temple {
            x: at.x,
            y: at.y,
            floor: at.floor,
        });
    }
    let run = &mut subareas.runs[0];
    run.y = THAIS.y;
    run.begin = THAIS.x;
    run.end = THAIS.x + 1;
    let manifest = Manifest {
        subareas,
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "guild-stone-test".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Default::default()
        },
        world: Extent {
            min_x: 32192,
            min_y: 31744,
            max_x: 32384,
            max_y: 32320,
            floors: vec![-7, -6],
        },
        palette: vec![PaletteEntry {
            family: Family::Terrain,
            key: "oteryn:item.tibia.i100".into(),
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
    let guild = TilePos {
        x: u16::try_from(ARRIVAL.x)?,
        y: u16::try_from(ARRIVAL.y)?,
        floor: -i8::try_from(ARRIVAL.floor)?,
    };
    let exit = TilePos {
        x: guild.x + 1,
        ..guild
    };
    let mut sectors = std::collections::BTreeMap::<_, Vec<Tile>>::new();
    for at in [CARLIN, THAIS, guild, exit] {
        sectors
            .entry((at.floor, at.y / SECTOR_SIZE, at.x / SECTOR_SIZE))
            .or_default()
            .push(Tile {
                x: at.x,
                y: at.y,
                flags: 1,
                house: 0,
                zones: vec![],
                items: vec![Item {
                    palette: 0,
                    depth: 0,
                    attrs: Attrs {
                        action: (at == exit).then_some(ADVENTURERS_GUILD_RETURN_ACTION),
                        ..Attrs::default()
                    },
                }],
            });
    }
    let sectors: Vec<_> = sectors
        .into_iter()
        .map(|((floor, sy, sx), tiles)| Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect();
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
            start: CARLIN,
        },
        world,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
