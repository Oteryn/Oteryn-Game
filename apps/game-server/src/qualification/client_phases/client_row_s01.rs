//! S01 client replay capture: the real party replies for an invitation seen by the peer, the
//! accepted join and the leadership pass. The consumer is `request`/`apply_result` in
//! `crates/client-ui/src/features/0042-party`.
use super::*;
use oteryn_dev_client::client_runtime::party as runtime;
use oteryn_dev_client::{DevClientSession, EntityRef};
use runtime::party::{PartyDisposition, PartyIntent};
use std::io::Write as _;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    if let Some(root) = std::env::var_os("NODE_BOOT_S01_CAPTURE_DIR") {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Path::new(&root).join(name))?
            .write_all(bytes)?;
    }
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "S01_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
    Ok(())
}

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client.world_entities().ok_or("entity snapshot absent")?;
    entities
        .iter()
        .find(|entry| &entry.entity.identity == entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "own actor absent".into())
}

struct Sent {
    status: runtime::CommandStatus,
    command_id: u64,
    sequence: u64,
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
    let before = client.last_server_sequence();
    let (command_id, reply) = runtime::send_with_receipt(client.session_mut(), &command).await?;
    if reply.status != runtime::CommandStatus::Accepted || client.last_server_sequence() <= before {
        return Err(format!(
            "party command {command_id} did not succeed on first attempt: {:?}",
            reply.status
        )
        .into());
    }
    let operation = runtime::party::decode_party_result(&reply.operation)
        .map_err(|e| format!("party operation: {e:?}"))?;
    if operation.disposition != PartyDisposition::Ok {
        return Err(format!("party refused: {:?}", operation.disposition).into());
    }
    let view = reply.view.ok_or("party current view absent")?;
    let result =
        runtime::party::decode_party_result(&view).map_err(|e| format!("party view: {e:?}"))?;
    Ok(Sent {
        status: reply.status,
        command_id,
        sequence: client.last_server_sequence(),
        operation: reply.operation,
        view,
        result,
    })
}

fn capture_receipt(label: &str, client: &DevClientSession, sent: &Sent) -> TestResult {
    let receipt = format!(
        "{:?} {} {} {:?}",
        sent.status,
        sent.command_id,
        sent.sequence,
        client.join_snapshot().game_session_id
    );
    capture(&format!("s01-{label}-receipt.txt"), receipt.as_bytes())
}

fn check_state(sent: &Sent, members: &[(&str, bool)], inviter: Option<&str>) -> TestResult {
    let result = &sent.result;
    if result.disposition != PartyDisposition::Ok
        || result.shared_xp_enabled
        || result.members.len() != members.len()
        || !members.iter().all(|&(name, leader)| {
            result
                .members
                .iter()
                .filter(|m| m.name == name && m.leader == leader)
                .count()
                == 1
        })
        || result.invitations.len() != usize::from(inviter.is_some())
        || inviter.is_some_and(|name| result.invitations[0].name != name)
    {
        return Err(format!(
            "party state mismatch: members={members:?} inviter={inviter:?} actual={result:?}"
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_protocol_oteryn::logout::LogoutResult;

    let mut leader = connect_character(clients, 0x47, 0).await?;
    let mut peer = connect_character(clients, 0x48, 1).await?;
    for client in [&mut leader, &mut peer] {
        if tile(client.world_spatial().actor_position)? != TOWN {
            return Err("S01 fixture did not enter at the temple".into());
        }
        check_state(&send(client, None).await?, &[], None)?;
    }
    let destination = TilePos {
        y: TOWN.y - 1,
        ..TOWN
    };
    let step = leader.session_mut().step(StepDirection::North).await?;
    if step.disposition != StepDisposition::Moved
        || tile(leader.world_spatial().actor_position)? != destination
    {
        return Err(format!("S01 fixed step to {destination:?} failed: {step:?}").into());
    }

    send(&mut leader, Some(PartyIntent::Invite(own(&peer)?))).await?;
    let invited = send(&mut peer, None).await?;
    check_state(&invited, &[], Some("Packet One"))?;
    let invitation = &invited.result.invitations[0];
    capture_receipt("invitation", &peer, &invited)?;
    capture("s01-invitations.bin", &invited.operation)?;
    capture("s01-invitations-view.bin", &invited.view)?;

    let joined = send(&mut peer, Some(PartyIntent::Accept(invitation.handle))).await?;
    let before_pass = [("Packet One", true), ("Packet Two", false)];
    check_state(&joined, &before_pass, None)?;
    capture_receipt("join", &peer, &joined)?;
    capture("s01-joined-op.bin", &joined.operation)?;
    capture("s01-joined-view.bin", &joined.view)?;

    let leader_view = send(&mut leader, None).await?;
    check_state(&leader_view, &before_pass, None)?;
    let member = leader_view
        .result
        .members
        .iter()
        .find(|m| m.name == "Packet Two")
        .ok_or("peer absent from leader view")?;
    let passed = send(
        &mut leader,
        Some(PartyIntent::PassLeadership(member.handle)),
    )
    .await?;
    let after_pass = [("Packet One", false), ("Packet Two", true)];
    check_state(&passed, &after_pass, None)?;
    let peer_view = send(&mut peer, None).await?;
    check_state(&peer_view, &after_pass, None)?;
    capture_receipt("leadership", &peer, &peer_view)?;
    capture("s01-passed-op.bin", &peer_view.operation)?;
    capture("s01-passed-view.bin", &peer_view.view)?;

    check_state(
        &send(&mut leader, Some(PartyIntent::Leave)).await?,
        &[],
        None,
    )?;
    check_state(&send(&mut peer, None).await?, &[], None)?;
    for client in [&mut leader, &mut peer] {
        let logout = client.session_mut().logout().await?;
        if logout.status != runtime::CommandStatus::Accepted
            || logout.result != Some(LogoutResult::Accepted)
        {
            return Err(format!("S01 first logout refused: {logout:?}").into());
        }
    }
    drop((leader, peer));
    for (index, tag) in [(0, 0x49), (1, 0x4a)] {
        let mut client = connect_character(clients, tag, index).await?;
        check_state(&send(&mut client, None).await?, &[], None)?;
        let logout = client.session_mut().logout().await?;
        if logout.status != runtime::CommandStatus::Accepted
            || logout.result != Some(LogoutResult::Accepted)
        {
            return Err(format!("S01 relog cleanup refused: {logout:?}").into());
        }
    }
    let expected = "inviter=Packet One\njoiner=Packet Two\nmembers=2\nshared_xp=false\nnew_leader=Packet Two\n";
    capture("s01-expected.txt", expected.as_bytes())?;
    Ok("invite-join-pass-leave-logout-relog".to_owned())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
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
    fn rejects_no_op_transfer_stale_invites_and_wrong_party_members() {
        let entry = |name: &str, leader| runtime::party::PartyEntry {
            channel: None,
            list: Vec::new(),
            handle: 1,
            name: name.to_owned(),
            leader,
            expires_at: 0,
            party_spell_remaining_ms: None,
        };
        let mut observation = Sent {
            status: runtime::CommandStatus::Accepted,
            command_id: 1,
            sequence: 1,
            operation: Vec::new(),
            view: Vec::new(),
            result: runtime::party::PartyResult {
                disposition: PartyDisposition::Ok,
                shared_xp_enabled: false,
                members: vec![entry("Packet One", true), entry("Packet Two", false)],
                invitations: Vec::new(),
            },
        };
        let transferred = [("Packet One", false), ("Packet Two", true)];
        assert!(check_state(&observation, &transferred, None).is_err());
        assert!(check_state(&observation, &[], None).is_err());
        observation.result.members = vec![entry("Other", false), entry("Packet Two", true)];
        assert!(check_state(&observation, &transferred, None).is_err());
        observation.result.members[0] = entry("Packet One", false);
        observation
            .result
            .invitations
            .push(entry("Packet One", false));
        assert!(check_state(&observation, &transferred, None).is_err());
        observation.result.invitations.clear();
        observation.result.shared_xp_enabled = true;
        assert!(check_state(&observation, &transferred, None).is_err());
        observation.result.shared_xp_enabled = false;
        assert!(check_state(&observation, &transferred, None).is_ok());
    }
}
