//! W10 client capture: native Hazard state before and after a Primal Menace kill, selection,
//! locked-level refusal, and state restored by a fresh admission.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{attack, hazard, logout::LogoutResult};

const PRIMAL: &str = "oteryn:creature.the_primal_menace";

async fn probe(
    client: &mut DevClientSession,
    root: Option<&Path>,
    key: &str,
    request: &[u8],
    expected_status: CommandStatus,
    expected_payload: &[u8],
) -> TestResult<Vec<u8>> {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let reply = client
        .session_mut()
        .gated_command(
            hazard::CAPABILITY_HAZARD_V1,
            hazard::COMMAND_TYPE_HAZARD,
            request,
        )
        .await?;
    if reply.status != expected_status || reply.payload != expected_payload {
        return Err(format!(
            "Hazard {key}: {reply:?}; expected {expected_status:?} {expected_payload:?}"
        )
        .into());
    }
    if let Some(root) = root {
        let prefix = format!("w10-{key}");
        let status = reply.status as u32;
        std::fs::write(root.join(format!("{prefix}-request.bin")), request)?;
        std::fs::write(
            root.join(format!("{prefix}-status.bin")),
            status.to_be_bytes(),
        )?;
        std::fs::write(root.join(format!("{prefix}.bin")), &reply.payload)?;
        std::fs::write(
            root.join(format!("{prefix}.status.json")),
            serde_json::to_vec(&serde_json::json!({
                "session_id": session,
                "command_id": reply.command_id,
                "server_sequence": reply.server_sequence,
                "status": status,
            }))?,
        )?;
    }
    Ok(reply.payload)
}

async fn defeat_primal(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("no reachable combat tile for the Primal Menace")?;
    walk(client, world, &path).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, PRIMAL.into()).await?;
    let target = parse_spawned_entity(&answer)?;
    let visible = tokio::time::Instant::now() + Duration::from_secs(10);
    while !client
        .world_entities()
        .is_some_and(|entities| entities.get(&target).is_some())
    {
        if tokio::time::Instant::now() >= visible {
            return Err("spawned Primal Menace never became visible".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    let modes = attack::FightModes {
        chase: attack::ChaseMode::Chase,
        ..attack::FightModes::DEFAULT
    };
    let modes_reply = client.session_mut().set_fight_modes(&modes).await?;
    if modes_reply.disposition != attack::AttackIntentDisposition::Ok {
        return Err(format!("Primal Menace chase mode refused: {modes_reply:?}").into());
    }
    let reply = client.session_mut().attack_target(Some(&target)).await?;
    if reply.disposition != attack::AttackIntentDisposition::Ok {
        return Err(format!("Primal Menace attack refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(deadline, "W10-Primal-death");
    committed_hunt_death(
        client,
        clients,
        Some(clients.characters[1]),
        PRIMAL,
        target,
        deadline,
    )
    .await?;
    client.session_mut().attack_target(None).await?;
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    let root = root.as_deref().map(Path::new);
    let bundle = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bundle,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let client = connect_world(clients, 0x05).await?;
    let mut client = provisioning::provision_fixture(client, clients, 1, 100).await?;
    let session = client.join_snapshot().game_session_id;
    use CommandStatus::{Accepted, Rejected};
    probe(&mut client, root, "initial", &[], Accepted, &[1, 1]).await?;
    defeat_primal(&mut client, clients, world).await?;
    let query = probe(&mut client, root, "query", &[], Accepted, &[2, 1]).await?;
    let select = probe(&mut client, root, "select", &[2], Accepted, &[2, 2]).await?;
    probe(&mut client, root, "locked", &[3], Rejected, &[]).await?;
    probe(&mut client, root, "unchanged", &[], Accepted, &[2, 2]).await?;

    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| is_logout_rest_tile(world, at))
        .ok_or("no reachable logout rest tile after the Primal Menace fight")?;
    walk(&mut client, world, &path).await?;
    let (result, observed, _, _) = logout_retrying_busy(&mut client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("Hazard logout refused: {observed}").into());
    }
    drop(client);
    let mut client = connect_world(clients, 0x06).await?;
    if client.join_snapshot().game_session_id == session {
        return Err("Hazard readmission reused the original session".into());
    }
    probe(&mut client, root, "relog", &[], Accepted, &[2, 2]).await?;
    let expected = format!(
        "query_unlocked={}\nquery_selected={}\nselect_unlocked={}\nselect_selected={}\n",
        query[0], query[1], select[0], select[1],
    );
    if let Some(root) = root {
        std::fs::write(root.join("w10-expected.txt"), &expected)?;
    }
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W10.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W10.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W10.client status=FAIL evidence=timeout"),
    }
}
