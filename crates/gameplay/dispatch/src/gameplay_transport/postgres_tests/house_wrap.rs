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
                let result = scenario(&h).await;
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

async fn scenario(h: &Harness) -> TestResult {
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
    owner.movement_cells = &cells;
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
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 49, 86, 207])
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
    tx.commit().await?;
    admitted.continuity.next_command_id = 1;
    assert_eq!(
        exchange_checked(&owner, &mut admitted, 1, request, true).await?,
        (CommandStatus::Accepted, wire::Reply::Committed(2))
    );
    assert_custody(h, "oteryn:item.tibia.i6371", false).await?;
    // New composed owner / command directory forces durable replay, bypassing cached ingress.
    drop(owner);
    let mut owner = deps.owner(h, &authority);
    owner.movement_cells = &cells;
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
        2
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

fn wrap_bundle(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
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
    for tile in &mut sector.tiles {
        if tile.x == 7 {
            tile.house = 3437;
            tile.items.truncate(1);
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
        fixture::door_items,
    )
    .map_err(debug)
}
