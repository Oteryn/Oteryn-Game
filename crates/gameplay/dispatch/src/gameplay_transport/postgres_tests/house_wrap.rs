//! HOUSE-ITEM-WIRE-1: framed commands, real composed House owner and runtime-role PG.
use super::fixture::{Harness, debug, id, run};
use super::house_transfer::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::house_scope_handoff::{
    HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{ControlActor, OperationKey};
use crate::foundation::{
    ChannelContentPin, CharacterId, CharacterLease, MessageType, MovementLocalPosition,
    decode_wire_envelope,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
    house_wrap as wire,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
const POSITION: [i32; 3] = [32310, 32804, 7];

#[test]
fn house_wrap_dispatch_postgres_roundtrip_replay_and_refusal() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            run(async |admin| {
                let started = std::time::Instant::now();
                let h =
                    Harness::create_with_admission_seed(admin, "house_wrap", true, false).await?;
                let result = scenario(&h,false).await;
                h.cleanup().await?;
                result?;
                println!(
                    "PG_MARKER house-wrap roundtrip replay refusals elapsed={:?}",
                    started.elapsed()
                );
                Ok(())
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "house wrap thread panicked")??;
    Ok(())
}

#[test]
fn srx1_house_cask_charge_checkpoint_exhaustion_and_replay_postgres()->TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none(){return Ok(());}
    std::thread::Builder::new().stack_size(256<<20).spawn(||{
        run(async |admin|{
            let started=std::time::Instant::now();
            let h=Harness::create_with_admission_seed(admin,"srx1_house_cask",true,false).await?;
            let result=scenario(&h,true).await;
            h.cleanup().await?;result?;
            eprintln!("SRX1_PG House cask checkpoint/exhaustion/replay elapsed={:?}",started.elapsed());
            Ok(())
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"House cask PG thread panicked")??;
    Ok(())
}

async fn scenario(h: &Harness,cask:bool) -> TestResult {
    super::reset();
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let base = Dependencies::new()?;
    let world = wrap_bundle(base.admitted.world_id, base.admitted.channel_id)?;
    let cells = world
        .movement_cells(base.room.movement_cells())
        .map_err(debug)?;
    let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
        base.admitted.world_id,
        None,
    );
    let active = controller.active().ok_or("active content")?;
    let pin = ChannelContentPin::from_activation(
        base.admitted.world_id,
        1,
        active.identity().server_artifact_digest(),
        [2; 32],
        [3; 32],
        [4; 32],
        (32309, 32805, 7),
    );
    let deps = Dependencies::with_content(pin, base.room, base.spells)?;
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = (&cells).into();
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    // This fixture boots the bounded command set it will carry through the real handoff.
    const OFFERS: &[crate::gameplay_transport::capabilities::OfferedCapability] = &[
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 4,
            requires: &[6],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 6,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {id:15,requires:&[4]},
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 49,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 86,
            requires: &[],
        },
        crate::gameplay_transport::capabilities::OfferedCapability {
            id: 207,
            requires: &[49],
        },
    ];
    owner.required_capabilities = crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
        &[],crate::foundation::RuntimeScopeRefV1::channel(deps.admitted.world_id,deps.admitted.channel_id),1,"policy-1",OFFERS,
    ).map_err(debug)?;

    deps.initialize(&owner).await;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 32309,
                    y: 32805,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    sqlx::query(include_str!(
        "../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user,encode($1,'hex')::uuid,$2,1")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&h.pool).await?;
    h.root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes([82; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            actor_role: "oteryn_test_admin".into(),
            command: HouseScopeAssignmentCommand::Assign {
                house: HouseId::new(deps.admitted.world_id, HOUSE).ok_or("house")?,
                target: h.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(deps.admitted.game_session_id)
        .await?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), if cask {&[4,6,15,49,86,207]} else {&[4, 6, 49, 86, 207]})
            .ok_or("wrap offer")?;
    let request = wire::Request {
        action: wire::Action::Wrap,
        item: id(71),
        expected_revision: 1,
        acl_revision: 1,
        position: POSITION,
    };
    assert_eq!(
        exchange(&owner, &mut admitted, 1, request).await?,
        (
            CommandStatus::Rejected,
            wire::Reply::Refused(wire::Refusal::NotInHouse)
        )
    );
    let mut body = vec![8, 1, 18, HOUSE.len() as u8];
    body.extend(HOUSE.as_bytes());
    let (inside, _) = owner
        .transfer_house(admitted, 2, &body)
        .await
        .ok_or("house transfer outcome")?;
    let mut admitted = inside.ok_or("house transfer admission")?;

    // NX-1 refresh must use the exact named-House owner, including on ordinary players.
    let house_fence = admitted.item_fence.ok_or("House item fence")?;
    let status = owner.root.staff_live_status(owner.character, owner.holder, house_fence).await.map_err(debug)?;
    assert_eq!(status, crate::durability::staff::StaffLiveStatus::default());
    let mut stale = house_fence;
    stale.connection_generation = crate::foundation::ConnectionGeneration::new(
        house_fence.connection_generation.get() + 1,
    ).map_err(debug)?;
    assert!(owner.root.staff_live_status(owner.character, owner.holder, stale).await.is_err());

    if cask {return house_cask(&owner,h,admitted).await;}

    // Seed a real placed oven and a backpack, then restore every constraint before commands.
    super::item_look::seed_main_backpack(h, &[]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i6355','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(id(73).as_slice()).execute(&mut *tx).await?;
    let position: Vec<u8> = POSITION.iter().flat_map(|v| v.to_be_bytes()).collect();
    sqlx::query(
        "INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING",
    )
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(&position)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(&position).bind(id(74).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(id(71).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
    // This donor wrap target has no XML name. The panel must still advertise
    // the real festive table using its current, source-qualified display name.
    let mut festive_position=POSITION;festive_position[0]+=1;
    let festive_position:Vec<u8>=festive_position.iter().flat_map(|v|v.to_be_bytes()).collect();
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i30231','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(75).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
        .bind(id(42).as_slice()).bind(HOUSE).bind(&festive_position).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
        .bind(id(75).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(&festive_position).bind(id(77).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(id(75).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(id(77).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    admitted.continuity.next_command_id = 1;
    assert_eq!(
        exchange_checked(&owner, &mut admitted, 1, request, true).await?,
        (CommandStatus::Accepted, wire::Reply::Committed(2))
    );
    assert_custody(h, "oteryn:item.tibia.i6371", false).await?;
    let query_command = admitted.continuity.next_command_id;
    let (status, catalogue) = furniture_query(&owner, &mut admitted, query_command, 0).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(catalogue.first(), Some(&7), "native furniture catalogue must be served by the real House owner");
    let catalogue=wire::Catalogue::decode(&catalogue).ok_or("native catalogue codec")?;
    assert_eq!(catalogue.acl_revision,1);
    assert_eq!(catalogue.entries.len(),2);
    assert!(catalogue.entries.iter().any(|entry|entry.item==id(75) && entry.action==wire::Action::Wrap && entry.name=="festive table"));
    assert_eq!((catalogue.entries[0].item,catalogue.entries[0].revision,catalogue.entries[0].action),(id(71),2,wire::Action::Unwrap));
    assert!(catalogue.entries[0].name.contains("oven"));
    // New composed owner / command directory forces durable replay, bypassing cached ingress.
    drop(owner);
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = (&cells).into();
    owner.evidence = &evidence.evidence;
    owner.active_generation = Some(active);
    admitted.continuity.next_command_id = 1;
    assert_eq!(
        exchange(&owner, &mut admitted, 1, request).await?.1,
        wire::Reply::Committed(2)
    );
    assert_custody(h, "oteryn:item.tibia.i6371", false).await?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        exchange(
            &owner,
            &mut admitted,
            command,
            wire::Request {
                item: id(70),
                ..request
            }
        )
        .await?
        .1,
        wire::Reply::Refused(wire::Refusal::NotWrapable)
    );
    let access_body: String = sqlx::query_scalar(
        "SELECT pg_get_functiondef('game_house_access(uuid,text,uuid)'::regprocedure)",
    )
    .fetch_one(&h.pool)
    .await?;
    sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_house_access(p_world_id UUID,p_house_key TEXT,p_character_id UUID) RETURNS TABLE(role TEXT,content_fenced BOOLEAN,acl_revision NUMERIC,guild_revisions JSONB) LANGUAGE sql AS $$ SELECT 'GUEST'::text,false,1::numeric,NULL::jsonb $$").execute(&h.pool).await?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        exchange(
            &owner,
            &mut admitted,
            command,
            wire::Request {
                expected_revision: 2,
                action: wire::Action::Unwrap,
                ..request
            }
        )
        .await?
        .1,
        wire::Reply::Refused(wire::Refusal::NoAccess)
    );
    // Restores only the function definition read from this isolated test database.
    sqlx::raw_sql(sqlx::AssertSqlSafe(access_body))
        .execute(&h.pool)
        .await?;
    let unwrap = wire::Request {
        action: wire::Action::Unwrap,
        expected_revision: 2,
        ..request
    };
    for (bad, refusal) in [
        (
            wire::Request {
                acl_revision: 2,
                ..unwrap
            },
            wire::Refusal::Stale,
        ),
        (
            wire::Request {
                position: [0, 0, 7],
                ..unwrap
            },
            wire::Refusal::Blocked,
        ),
    ] {
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            exchange(&owner, &mut admitted, command, bad).await?.1,
            wire::Reply::Refused(refusal)
        );
    }
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        exchange_checked(&owner, &mut admitted, command, unwrap, true)
            .await?
            .1,
        wire::Reply::Committed(3)
    );
    assert_custody(h, "oteryn:item.tibia.i6355", true).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_furniture_wrap_receipts")
            .fetch_one(&h.pool)
            .await?,
        2
    );
    Ok(())
}
async fn assert_custody(h: &Harness, key: &str, house: bool) -> TestResult {
    let (stored,placed,carried):(String,bool,bool)=sqlx::query_as("SELECT definition_production_key,EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id),EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((stored.as_str(), placed, carried), (key, house, !house));
    let mint: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(minted_transaction_id) FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(mint, id(73));

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_instances")
            .fetch_one(&h.pool)
            .await?,
        3
    );
    Ok(())
}
async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    request: wire::Request,
) -> TestResult<(CommandStatus, wire::Reply)> {
    exchange_checked(owner, admitted, command, request, false).await
}
async fn exchange_checked(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    request: wire::Request,
    expect_delta: bool,
) -> TestResult<(CommandStatus, wire::Reply)> {
    let body = request.encode();
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: command,
            command_type: wire::COMMAND_WRAP,
            payload: &body,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&(frame.len() as u32).to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &end
    {
        *admitted = *current;
    }
    let mut cursor = 0;
    let mut reply = None;
    let mut inventory_delta = false;
    while cursor < bytes.len() {
        let n = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into()?) as usize;
        let envelope = decode_wire_envelope(&bytes[cursor + 4..cursor + 4 + n]).map_err(debug)?;
        cursor += 4 + n;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            reply = Some((
                result.status,
                wire::Reply::decode(result.payload).ok_or("wrap result")?,
            ));
        } else if envelope.message_type() == MessageType::StateDelta {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            inventory_delta |= delta.domain_id
                == oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY;
        }
    }
    if expect_delta {
        assert!(
            inventory_delta,
            "committed House conversion must publish inventory delta"
        );
    }
    reply.ok_or_else(|| format!("wrap command {command}: {end:?}").into())
}

pub(super) fn wrap_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    wrap_bundle_with_static_decorations(world, channel, &[])
}

pub(super) fn static_decoration_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    static_decoration_bundle_with(world,channel,&[("oteryn:item.tibia.i5302",7,0)])
}

pub(super) fn static_timed_decoration_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    static_decoration_bundle_with(world,channel,&[("oteryn:item.tibia.i35949",7,0)])
}

pub(super) fn static_bed_decoration_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    static_decoration_bundle_with(world,channel,&[("oteryn:item.tibia.i2491",7,0),("oteryn:item.tibia.i2492",7,1)])
}

struct StaticDecoration {
    key: &'static str,
    x: u16,
    y: u16,
    definition: crate::map::facts::ItemDefinition,
}

fn static_decoration_bundle_with(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    declarations: &[(&'static str,u16,u16)],
) -> TestResult<crate::map::boot::BundleWorld> {
    let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(world,None);
    let native=controller.active().and_then(|active|active.native_gameplay()).ok_or("native generation")?;
    let index=native.item_index().ok_or("Item index")?;
    let mut extras=Vec::with_capacity(declarations.len());
    for &(key,x,y) in declarations {
        let reference=index.definition_ref("Item",key,"definition-r1").ok_or("decoration reference")?;
        let policy=native.item_policy(key,"definition-r1").ok_or("decoration policy")?.record();
        extras.push(StaticDecoration {key,x,y,definition:crate::map::facts::ItemDefinition {
            reference,solid:policy.attributes.blocks_movement,
            blocks_projectile:policy.attributes.blocks_projectile.ok_or("decoration projectile fact")?,
            pickupable:false,door:false }});
    }
    wrap_bundle_with_static_decorations(world,channel,&extras)
}

fn wrap_bundle_with_static_decorations(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    extras: &[StaticDecoration],
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::boot::{self, tests as fixture};
    use oteryn_world_bundle::bundle;
    let (bytes, mut pins) =
        fixture::bundle_with_items("oteryn:item.tibia.i1629", "oteryn:item.tibia.i1000");
    let mut data = bundle::read(&bytes).map_err(debug)?;
    let sector = data.sectors.first_mut().ok_or("sector")?;
    let mut entrance = sector
        .tiles
        .iter()
        .find(|t| t.x == 3)
        .ok_or("grass tile")?
        .clone();
    let seed=sector.tiles.iter().find(|tile|tile.x==7).ok_or("House tile")?;
    let ground=seed.items.first().ok_or("House ground")?.palette;
    let flags=seed.flags;
    // Clear only the fixture seed before placing actual native decorations.
    for tile in &mut sector.tiles {
        if tile.x == 7 { tile.items.truncate(1); }
    }
    for extra in extras {
        let palette=if let Some(at)=data.manifest.palette.iter().position(|entry|entry.key==extra.key) {
            u32::try_from(at)?
        } else {
            let at=u32::try_from(data.manifest.palette.len())?;
            data.manifest.palette.push(bundle::PaletteEntry {key:extra.key.into(),family:bundle::Family::Item,
                id:extra.definition.reference.get()-1,terrain:None});
            at
        };
        let item=oteryn_world_bundle::sector::Item {palette,depth:0,attrs:Default::default()};
        if let Some(tile)=sector.tiles.iter_mut().find(|tile|tile.x==extra.x&&tile.y==extra.y) {
            tile.house=3437;
            tile.items.push(item);
        } else {
            sector.tiles.push(oteryn_world_bundle::sector::Tile {x:extra.x,y:extra.y,flags,house:3437,zones:Vec::new(),
                items:vec![oteryn_world_bundle::sector::Item {palette:ground,depth:0,attrs:Default::default()},item]});
        }
    }
    for tile in &mut sector.tiles {
        if tile.x == 7 {
            tile.house = 3437;
        }
        if tile.x == 6 {
            tile.house = 3437;
            tile.items.last_mut().ok_or("door")?.attrs.door = Some(42);
        }
        tile.x += 32303;
        tile.y += 32804;
    }
    entrance.x = 32309;
    entrance.y = 32805;
    sector.tiles.push(entrance);
    // Native sector encoding orders by (y, x), including the second bed half.
    sector.tiles.sort_by_key(|tile| (tile.y, tile.x));
    sector.sx = 1009;
    sector.sy = 1025;
    data.manifest.world.min_x = 32288;
    data.manifest.world.min_y = 32800;
    data.manifest.world.max_x = 32320;
    data.manifest.world.max_y = 32832;
    let bytes = oteryn_world_bundle_compiler::bundle::write_with_npcs(
        &data.manifest,
        &data.sectors,
        &data.spawns,
        &data.npcs,
    )
    .map_err(debug)?;
    pins.digest = bundle::read(&bytes).map_err(debug)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot::boot(
        &bytes,
        &boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 32309,
                y: 32805,
                floor: -7,
            },
        },
        world,
        channel,
        |key| extras.iter().find(|extra|extra.key==key).map(|extra|extra.definition).or_else(||fixture::door_items(key)),
    )
    .map_err(debug)
}

async fn furniture_query(
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    command: u64, page: u16,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let body = [3, page.to_be_bytes()[0], page.to_be_bytes()[1]];
    let frame = encode_client_command(admitted.continuity.connection_generation,
        &ClientCommandValue { command_id: command, command_type: wire::COMMAND_WRAP, payload: &body })?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?; client.shutdown().await?;
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    if let ConnectionEnd::AdmittedThenDisconnected(current) | ConnectionEnd::AdmittedThenClosed(current, _) = end { *admitted = current; }
    let mut bytes = Vec::new(); client.read_to_end(&mut bytes).await?;
    let mut at = 0;
    while at < bytes.len() {
        let count = u32::from_be_bytes(bytes[at..at+4].try_into()?) as usize;
        let envelope = decode_wire_envelope(&bytes[at+4..at+4+count])?; at += 4 + count;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            return Ok((result.status, result.payload.to_vec()));
        }
    }
    Err("missing furniture catalogue result".into())
}

async fn house_cask(owner:&ComposedFreshAdmission<'_, '_, '_>,h:&Harness,mut admitted:AdmittedSession)->TestResult {
    use oteryn_protocol_oteryn::world_object::{UseIntent,UseTarget,MapUseTarget,UseDisposition};
    use crate::gameplay_transport::item_view::ItemKey;
    // Historical fixture with real named-House placement/provenance and a partial counter.
    super::item_look::seed_main_backpack(h,&[("oteryn:item.tibia.i283",2),("oteryn:item.tibia.i283",2),("oteryn:item.tibia.i25904",1)]).await?;
    let mut tx=h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i25880','definition-r1',1,1,game_character_uuid_v7())")
        .bind(id(80).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    let position:Vec<u8>=POSITION.iter().flat_map(|value|value.to_be_bytes()).collect();
    sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
        .bind(id(42).as_slice()).bind(HOUSE).bind(&position).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($5,'hex')::uuid)")
        .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(&position).bind(id(81).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(id(81).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,state_revision) VALUES(encode($1,'hex')::uuid,3,1)")
        .bind(id(80).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_timed_definitions VALUES('Item','oteryn:item.tibia.i25904','definition-r1',500,NULL,false) ON CONFLICT DO NOTHING").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,state_revision) VALUES(encode($1,'hex')::uuid,498,1)").bind(id(73).as_slice()).execute(&mut *tx).await?;
    let mut keg_position=POSITION;keg_position[0]+=1;
    let keg_position:Vec<u8>=keg_position.iter().flat_map(|value|value.to_be_bytes()).collect();
    sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING").bind(id(42).as_slice()).bind(HOUSE).bind(&keg_position).execute(&mut *tx).await?;
    tx.commit().await?;
    charged_wrap::roundtrip(owner,h,&mut admitted).await?;
    house_keg_custody(owner,h,admitted,keg_position).await?;
    let intent=UseIntent{target:UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("flask handle")?),use_with:None,
        use_with_map_item:Some(MapUseTarget::Item(std::num::NonZeroU64::new(2).ok_or("cask handle")?)),use_with_position:None};
    let command=|id|crate::gameplay_transport::connection::UseCommand{game_session_id:admitted.game_session_id,command_id:id,item_fence:admitted.item_fence};
    let actor=admitted.runtime_actor.ok_or("House actor")?;
    let first=owner.use_dispenser_action(actor,command(6),intent.clone(),Some(ItemKey::Instance(id(71))),Some(ItemKey::Instance(id(80)))).await.map_err(|_|"House first use unknown")?.ok_or("House dispenser production route")?;
    assert_eq!(first.disposition,UseDisposition::Committed);
    assert_eq!(sqlx::query_scalar::<_,i32>("SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(80).as_slice()).fetch_one(&h.pool).await?,1);
    let actor=admitted.runtime_actor.ok_or("House actor")?;
    let last=owner.use_dispenser_action(actor,command(7),intent.clone(),Some(ItemKey::Instance(id(72))),Some(ItemKey::Instance(id(80)))).await.map_err(|_|"House last use unknown")?.ok_or("House exhaustion route")?;
    assert_eq!(last.disposition,UseDisposition::Committed);
    let item:(i64,i16)=sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(80).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(item,(0,2));
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT (SELECT count(*) FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid)+(SELECT count(*) FROM game_item_house_reclaim_provenance WHERE item_instance_id=encode($1,'hex')::uuid)").bind(id(80).as_slice()).fetch_one(&h.pool).await?,0);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i236' AND lifecycle=1").fetch_one(&h.pool).await?,3);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(&h.pool).await?,1);
    for number in [6,7] {
        let replay=owner.use_dispenser_action(actor,command(number),intent.clone(),None,None).await.map_err(|_|"House replay unknown")?.ok_or("House replay receipt")?;
        assert_eq!(replay.disposition,UseDisposition::Committed);
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:dispenser.fill'").fetch_one(&h.pool).await?,2);
    Ok(())
}

async fn house_keg_custody(owner:&ComposedFreshAdmission<'_, '_, '_>,h:&Harness,admitted:AdmittedSession,position:Vec<u8>)->TestResult {
    use crate::durability::house_interior::{HouseItemRequest,HouseItemAction,HouseCharacterSource,HouseItemOutcome,HouseBackpackFacts};
    let fence=admitted.item_fence.ok_or("House inventory fence")?;
    let before:serde_json::Value=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    let ordinal:String=sqlx::query_scalar("SELECT placement_ordinal::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?;
    let mut request=HouseItemRequest{
        command:crate::foundation::CommandRef::new(fence.game_session_id,crate::foundation::CommandId::new(900).map_err(debug)?),
        house:HouseId::new(admitted.world_id,HOUSE).ok_or("House")?,item_instance_id:id(73),
        action:HouseItemAction::Place{source:HouseCharacterSource::Inventory{parent:id(70),ordinal:ordinal.parse()?},position},
        expected_acl_revision:1,expected_guild_revisions:None,backpack:None,
        content_revision:"content-1".into(),ruleset_revision:"ruleset-1".into(),sim_revision:"sim-1".into()};
    let place=owner.root.freeze_house_item(owner.character,owner.holder,fence,request.clone()).await.map_err(debug)?;
    let receipt=match owner.root.commit_house_item(owner.character,owner.holder,fence,&place).await.map_err(debug)? {
        HouseItemOutcome::Committed(receipt)=>receipt,other=>return Err(format!("fresh House placement: {other:?}").into())};
    assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?,before);
    assert!(matches!(owner.root.commit_house_item(owner.character,owner.holder,fence,&place).await.map_err(debug)?,HouseItemOutcome::AlreadyCommitted(_)));
    request.command=crate::foundation::CommandRef::new(fence.game_session_id,crate::foundation::CommandId::new(901).map_err(debug)?);
    request.action=HouseItemAction::Take{placement:receipt.transaction_id,provenance_revision:1};
    request.backpack=Some(HouseBackpackFacts{definition:crate::durability::item_mint::TypedDefinitionRef{family:"Item".into(),production_key:"oteryn:item.tibia.i2854".into(),revision_ref:"definition-r1".into()},capacity:20});
    let take=owner.root.freeze_house_item(owner.character,owner.holder,fence,request).await.map_err(debug)?;
    assert!(matches!(owner.root.commit_house_item(owner.character,owner.holder,fence,&take).await.map_err(debug)?,HouseItemOutcome::Committed(_)));
    assert!(matches!(owner.root.commit_house_item(owner.character,owner.holder,fence,&take).await.map_err(debug)?,HouseItemOutcome::AlreadyCommitted(_)));
    assert_eq!(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?,before);
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_timed_state_writes WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(73).as_slice()).fetch_one(&h.pool).await?,0);
    eprintln!("SRX1_PG House keg placement/take/replay preserves 498 charges");
    Ok(())
}

#[path = "house_wrap/charged_wrap.rs"]
mod charged_wrap;
