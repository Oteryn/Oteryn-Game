//! Capture native Wrap and Unwrap refusals outside a house, then a stale Wrap inside one.
use super::*;
use crate::durability::house_scope_handoff::{
    HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome, HouseScopeAssignmentRequest,
};
use crate::durability::runtime_scope_assignment::{
    ControlActor, NodeRegistrationFact, OperationKey,
};
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::house_transfer;
use oteryn_protocol_oteryn::house_wrap;
use sqlx::Row;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let result = verdict::indexed_timeout(
        Duration::from_secs(420),
        verdict::observe(journey(clients, Path::new(&directory))),
    )
    .await;
    match result.unwrap_or_else(|_| Err("I10 client capture timed out".into())) {
        Ok(()) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=I10.client status=PASS evidence=house-wrap-unwrap-and-stale-refusals",
        ),
        Err(error) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=I10.client status=FAIL evidence={error}"
        )),
    }
}

async fn probe(
    client: &mut DevClientSession,
    directory: &Path,
    name: &str,
    action: house_wrap::Action,
    revision: u64,
    refusal: house_wrap::Refusal,
) -> TestResult<String> {
    let position = client.world_spatial().actor_position;
    let request = house_wrap::Request {
        action,
        item: [0; 16],
        expected_revision: revision,
        acl_revision: 1,
        position: [position.x, position.y, i32::from(position.floor)],
    }
    .encode();
    let result = client
        .session_mut()
        .gated_command(
            house_wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
            house_wrap::COMMAND_WRAP,
            &request,
        )
        .await?;
    let reply = house_wrap::Reply::decode(&result.payload);
    if result.status != CommandStatus::Rejected
        || reply != Some(house_wrap::Reply::Refused(refusal))
    {
        return Err(format!(
            "I10 {name}: expected {refusal:?}, got {:?} {reply:?}",
            result.status
        )
        .into());
    }
    std::fs::write(directory.join(format!("i10-{name}-request.bin")), request)?;
    std::fs::write(
        directory.join(format!("i10-{name}-status.bin")),
        (result.status as u32).to_be_bytes(),
    )?;
    std::fs::write(directory.join(format!("i10-{name}.bin")), &result.payload)?;
    let hex: String = result
        .payload
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(format!("{name}={hex}\n"))
}

// Private Thais house in the town bundle's authored house catalogue.
const HOUSE: &str = "oteryn:content.house.upper_swamp_lane_4";
const ENTRANCE: TilePos = TilePos {
    x: 32360,
    y: 32252,
    floor: -7,
};

// Seed the supported owned-house fixture and assign its scope to the same live
// node as the Channel. The wire transfer itself establishes House custody.
async fn prepare_house(clients: &SeamClients<'_>, key: &str) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let assignment = sqlx::query(
        "SELECT uuid_send(holder_node_id) AS node, holder_registration_revision::text AS revision, session_user::text AS role
         FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid
         AND channel_id=encode($2,'hex')::uuid AND state=1",
    ).bind(clients.world.as_slice()).bind(clients.channel.as_slice()).fetch_one(&mut db).await?;
    let node: Vec<u8> = assignment.try_get("node")?;
    let node = crate::foundation::NodeId::decode(&node)?;
    let revision: String = assignment.try_get("revision")?;
    let role: String = assignment.try_get("role")?;
    sqlx::query(include_str!(
        "../../../tests/support/owned_house_fixture.sql"
    ))
    .bind(clients.world.as_slice())
    .bind(key)
    .bind(clients.characters[1].as_slice())
    .execute(&mut db)
    .await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants SELECT session_user, encode($1,'hex')::uuid, $2, 1")
        .bind(clients.world.as_slice()).bind(key).execute(&mut db).await?;
    let root = crate::durability::DurabilityRoot::connect_test_runtime(clients.url)
        .map_err(|_| "I10 house fixture root unavailable")?;
    root.await_ready()
        .await
        .map_err(|_| "I10 house fixture root not ready")?;
    let house =
        HouseId::new(WorldId::decode(&clients.world)?, key).ok_or("I10 invalid house key")?;
    let mut operation = [0; 32];
    operation[..16].copy_from_slice(&clients.characters[1]);
    operation[16..].copy_from_slice(&clients.world);
    let outcome = root
        .assign_house_scope(HouseScopeAssignmentRequest {
            operation_key: OperationKey::from_bytes(operation),
            actor: ControlActor::new(&role)
                .map_err(|error| format!("I10 control role: {error:?}"))?,
            actor_role: role,
            command: HouseScopeAssignmentCommand::Assign {
                house,
                target: NodeRegistrationFact::new(node, revision.parse()?),
            },
        })
        .await
        .map_err(|error| format!("I10 assign House scope: {error:?}"))?;
    if !matches!(outcome, HouseScopeAssignmentOutcome::Committed(_)) {
        return Err(format!("I10 house scope assignment refused: {outcome:?}").into());
    }
    Ok(())
}

async fn transfer(client: &mut DevClientSession, key: &str, exit: bool) -> TestResult {
    let intent = house_transfer::HouseTransferIntent {
        exit,
        house: key.to_owned(),
    };
    let payload = house_transfer::encode_house_transfer_intent(&intent)
        .map_err(|error| format!("I10 House transfer encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            house_transfer::CAPABILITY_HOUSE_TRANSFER_V1,
            house_transfer::COMMAND_TYPE_HOUSE_TRANSFER,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || !house_transfer::decode_house_transfer_ack(&reply.payload)
            .is_ok_and(|ack| ack.house.as_deref() == (!exit).then_some(key))
    {
        return Err(format!("I10 House transfer refused: {reply:?}").into());
    }
    Ok(())
}

async fn enter_house(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    let mut client = connect_world(clients, 0xb3).await?;
    if tile(client.world_spatial().actor_position)? != ENTRANCE {
        return Err("I10 fresh admission did not retain the House entrance".into());
    }
    transfer(&mut client, HOUSE, false)
        .await
        .map_err(|error| verdict::context(format!("I10 enter House: {error}"), error))?;
    Ok(client)
}

async fn journey(clients: &SeamClients<'_>, directory: &Path) -> TestResult {
    use house_wrap::{Action, Refusal};

    let world = full_e2ef7::qualification_world(clients)?;
    let mut client = connect_world(clients, 0xb2).await?;
    let mut manifest = probe(
        &mut client,
        directory,
        "wrap",
        Action::Wrap,
        0,
        Refusal::NotInHouse,
    )
    .await?;
    manifest.push_str(
        &probe(
            &mut client,
            directory,
            "unwrap",
            Action::Unwrap,
            0,
            Refusal::NotInHouse,
        )
        .await?,
    );
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == ENTRANCE
    })
    .ok_or("I10 no route to the authored house entrance")?;
    prepare_house(clients, HOUSE).await?;
    classa3_observation::walk(&mut client, world, &path).await?;
    classa3_observation::logout(&mut client).await?;
    drop(client);
    // Start the handoff with a fresh session at the saved entrance and the
    // House scope assignment already published.
    let mut client = enter_house(clients).await?;
    manifest.push_str(
        &probe(
            &mut client,
            directory,
            "refusal",
            Action::Wrap,
            u64::MAX,
            Refusal::Stale,
        )
        .await?,
    );
    std::fs::write(directory.join("i10-expected.txt"), manifest)?;
    transfer(&mut client, HOUSE, true).await?;
    classa3_observation::logout(&mut client).await
}
