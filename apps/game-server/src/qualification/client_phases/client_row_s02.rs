//! S02 client replay capture: the real party shield/skull icon replies as the inviter sees the
//! invited peer, then as each member sees the other after the accepted join. The consumer is
//! `apply` in `apps/client/src/features/0134-party-skull/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::party as runtime;
use oteryn_dev_client::{DevClientSession, EntityRef};
use oteryn_protocol_oteryn::{CommandStatus, party_skull};
use runtime::party::PartyIntent;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("S02_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client.world_entities().ok_or("entity snapshot absent")?;
    entities
        .iter()
        .find(|entry| &entry.entity.identity == entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "own actor absent".into())
}

async fn party(client: &mut DevClientSession, intent: Option<PartyIntent>) -> TestResult<Vec<u8>> {
    let command = match intent {
        Some(intent) => {
            runtime::PartyCommand::intent(intent).map_err(|e| format!("party input: {e:?}"))?
        }
        None => runtime::PartyCommand::query(),
    };
    let reply = runtime::send(client.session_mut(), &command).await?;
    let operation = runtime::party::decode_party_result(&reply.operation)
        .map_err(|e| format!("party operation: {e:?}"))?;
    if operation.disposition != runtime::party::PartyDisposition::Ok {
        return Err(format!("party refused: {:?}", operation.disposition).into());
    }
    reply.view.ok_or_else(|| "party view absent".into())
}

/// The raw icon reply plus the (shield, skull) it carries for `target`.
async fn icon(client: &mut DevClientSession, target: EntityRef) -> TestResult<(Vec<u8>, u8, u8)> {
    let reply = client
        .session_mut()
        .gated_command(
            party_skull::CAPABILITY_PARTY_SKULL_V1,
            party_skull::COMMAND_TYPE_PARTY_SKULL_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("icon query refused: {:?}", reply.status).into());
    }
    let icons = party_skull::decode_party_icons(&reply.payload)
        .map_err(|e| format!("icon reply undecodable: {e:?}"))?;
    let found = icons
        .iter()
        .find(|entry| entry.entity == target)
        .ok_or("target absent from the icon reply")?;
    Ok((reply.payload, found.shield, found.skull))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut leader = connect_character(clients, 0x49, 0).await?;
    let mut peer = connect_character(clients, 0x4a, 1).await?;
    let other = tile(peer.world_spatial().actor_position)?;
    let path = route(world, tile(leader.world_spatial().actor_position)?, |at| {
        at != other
            && at.floor == other.floor
            && at.x.abs_diff(other.x).max(at.y.abs_diff(other.y)) <= 1
    })
    .ok_or("peer approach route unavailable")?;
    walk(&mut leader, world, &path).await?;
    let (leader_ref, peer_ref) = (own(&leader)?, own(&peer)?);

    party(&mut leader, Some(PartyIntent::Invite(peer_ref))).await?;
    let (invited_bytes, invited_shield, invited_skull) = icon(&mut leader, peer_ref).await?;
    if !(1..=2).contains(&invited_shield) {
        return Err(format!("invitation shield {invited_shield} is not blue/yellow").into());
    }
    capture("s02-invited.bin", &invited_bytes);

    let view = party(&mut peer, None).await?;
    let state = runtime::party::decode_party_result(&view)
        .map_err(|e| format!("peer party view: {e:?}"))?;
    let handle = state
        .invitations
        .first()
        .ok_or("peer lacks the invitation")?
        .handle;
    party(&mut peer, Some(PartyIntent::Accept(handle))).await?;
    let (member_bytes, peer_shield, peer_skull) = icon(&mut leader, peer_ref).await?;
    let (leader_bytes, leader_shield, leader_skull) = icon(&mut peer, leader_ref).await?;
    if peer_shield != 3 && peer_shield != 5 {
        return Err(format!("member shield {peer_shield} is not member/shared").into());
    }
    if leader_shield != 4 && leader_shield != 6 {
        return Err(format!("leader shield {leader_shield} is not leader/shared").into());
    }
    capture("s02-member.bin", &member_bytes);
    capture("s02-leader.bin", &leader_bytes);
    let expected = format!(
        "peer_identity={}\npeer_generation={}\nleader_identity={}\nleader_generation={}\ninvited_shield={invited_shield}\ninvited_skull={invited_skull}\nmember_shield={peer_shield}\nmember_skull={peer_skull}\nleader_shield={leader_shield}\nleader_skull={leader_skull}\n",
        hex(&peer_ref.identity),
        peer_ref.generation,
        hex(&leader_ref.identity),
        leader_ref.generation,
    );
    capture("s02-expected.txt", expected.as_bytes());
    party(&mut leader, Some(PartyIntent::Leave)).await?;
    let _ = leader.session_mut().logout().await?;
    let _ = peer.session_mut().logout().await?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=S02.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S02.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=S02.client status=FAIL evidence=timeout"),
    }
}
