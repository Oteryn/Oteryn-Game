//! S01 client replay capture: the real party replies for an invitation seen by the peer, the
//! accepted join and the leadership pass. The consumer is `request`/`apply_result` in
//! `crates/client-ui/src/features/0042-party`.
use super::*;
use oteryn_dev_client::client_runtime::party as runtime;
use oteryn_dev_client::{DevClientSession, EntityRef};
use runtime::party::{PartyDisposition, PartyIntent};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    own_from(client.world_entities())
}

fn own_from(input: Option<&oteryn_dev_client::WorldEntities>) -> TestResult<EntityRef> {
    let entities = input.ok_or("entity snapshot absent")?;
    entities
        .iter()
        .find(|entry| &entry.entity.identity == entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "own actor absent".into())
}

struct Sent {
    operation: Vec<u8>,
    view: Vec<u8>,
    result: runtime::party::PartyResult,
}

async fn send(client: &mut DevClientSession, intent: Option<PartyIntent>) -> TestResult<Sent> {
    let command = match intent {
        Some(intent) => {
            runtime::PartyCommand::intent(intent).map_err(|e| format!("party input: {e:?}"))?
        }
        None => runtime::PartyCommand::query(),
    };
    let reply = runtime::send(client.session_mut(), &command).await?;
    let operation = runtime::party::decode_party_result(&reply.operation)
        .map_err(|e| format!("party operation: {e:?}"))?;
    if operation.disposition != PartyDisposition::Ok {
        return Err(format!("party refused: {:?}", operation.disposition).into());
    }
    let view = reply.view.ok_or("party current view absent")?;
    let result =
        runtime::party::decode_party_result(&view).map_err(|e| format!("party view: {e:?}"))?;
    Ok(Sent {
        operation: reply.operation,
        view,
        result,
    })
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut leader = connect_character(clients, 0x47, 0).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let peer = connect_character(clients, 0x48, 1).await;
    let mut peer = peer?;
    let other = tile(peer.world_spatial().actor_position)?;
    let path = route(world, tile(leader.world_spatial().actor_position)?, |at| {
        at != other
            && at.floor == other.floor
            && at.x.abs_diff(other.x).max(at.y.abs_diff(other.y)) <= 1
    })
    .ok_or("peer approach route unavailable")?;
    walk(&mut leader, world, &path).await?;

    send(&mut leader, Some(PartyIntent::Invite(own(&peer)?))).await?;
    let invited = send(&mut peer, None).await?;
    let [invitation] = invited.result.invitations.as_slice() else {
        return Err(format!("invitation mismatch: {:?}", invited.result).into());
    };
    let inviter = invitation.name.clone();
    capture("s01-invitations.bin", &invited.view)?;

    let joined = send(&mut peer, Some(PartyIntent::Accept(invitation.handle))).await?;
    if joined.result.members.len() != 2 || !joined.result.invitations.is_empty() {
        return Err(format!("join mismatch: {:?}", joined.result).into());
    }
    let Some(joiner) = joined.result.members.iter().find(|m| !m.leader) else {
        return Err("join has no non-leader member".into());
    };
    let joiner_name = joiner.name.clone();
    capture("s01-joined-op.bin", &joined.operation)?;
    capture("s01-joined-view.bin", &joined.view)?;

    let leader_view = send(&mut leader, None).await?;
    let Some(member) = leader_view.result.members.iter().find(|m| !m.leader) else {
        return Err("leader view has no member".into());
    };
    let passed = send(&mut leader, Some(PartyIntent::PassLeadership(member.handle))).await?;
    if !passed
        .result
        .members
        .iter()
        .any(|m| m.leader && m.name == joiner_name)
    {
        return Err("leadership did not pass".into());
    }
    capture("s01-passed-op.bin", &passed.operation)?;
    capture("s01-passed-view.bin", &passed.view)?;
    send(&mut leader, Some(PartyIntent::Leave)).await?;

    row_logout(&mut peer).await?;
    let expected = format!(
        "inviter={inviter}\njoiner={joiner_name}\nmembers={}\nshared_xp={}\nnew_leader={joiner_name}\n",
        joined.result.members.len(),
        joined.result.shared_xp_enabled
    );
    capture("s01-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";").replace(' ', "_"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut leader).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=S01.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S01.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=S01.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_projection_cannot_supply_evidence() {
        assert!(own_from(None).is_err());
    }
}
