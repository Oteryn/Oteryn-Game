//! S01 client replay capture: the real party replies for an invitation seen by the peer, the
//! accepted join and the leadership pass. The consumer is `request`/`apply_result` in
//! `crates/client-ui/src/features/0042-party`.
use super::*;
use oteryn_dev_client::client_runtime::party as runtime;
use oteryn_dev_client::{DevClientSession, EntityRef};
use runtime::party::{PartyDisposition, PartyIntent};
use std::io::Write as _;

// Keep setup selection separate from the indexed reporter scanned by node boot.
pub(super) fn selected() -> bool {
    e2e_row_selected(179)
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .or_else(|| std::env::var_os("NODE_BOOT_S01_CAPTURE_DIR"))
    else {
        return Err("S01 fresh capture directory absent".into());
    };
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&root).join(name))?
        .write_all(bytes)?;
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
    operation: String,
    view: String,
    result: runtime::party::PartyResult,
}

async fn exchange(
    client: &mut DevClientSession,
    command: &runtime::PartyCommand,
) -> TestResult<(String, runtime::party::PartyResult)> {
    let session = client.join_snapshot().game_session_id;
    let encoded = session
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let session = format!(
        "{}-{}-{}-{}-{}",
        &encoded[..8],
        &encoded[8..12],
        &encoded[12..16],
        &encoded[16..20],
        &encoded[20..]
    );
    let before = client.last_server_sequence();
    let reply = client
        .session_mut()
        .gated_command(
            runtime::party::CAPABILITY_PARTY_V1,
            command.kind,
            &command.payload,
        )
        .await?;
    let name = format!("s01-{session}-{}.json", reply.command_id);
    capture(
        &name,
        &serde_json::to_vec(&serde_json::json!({
            "command_id": reply.command_id,
            "game_session_id": session,
            "payload": reply.payload,
            "server_sequence": reply.server_sequence,
            "status": reply.status as u32,
        }))?,
    )?;
    if reply.status != runtime::CommandStatus::Accepted || reply.server_sequence <= before {
        return Err(format!(
            "party command {} refused: {:?}",
            reply.command_id, reply.status
        )
        .into());
    }
    let result = runtime::party::decode_party_result(&reply.payload)
        .map_err(|error| format!("party reply: {error:?}"))?;
    if result.disposition != PartyDisposition::Ok {
        return Err(format!("party refused: {:?}", result.disposition).into());
    }
    Ok((name, result))
}

async fn send(client: &mut DevClientSession, intent: Option<PartyIntent>) -> TestResult<Sent> {
    let query = runtime::PartyCommand::query();
    let Some(intent) = intent else {
        let (view, result) = exchange(client, &query).await?;
        return Ok(Sent {
            operation: view.clone(),
            view,
            result,
        });
    };
    let command =
        runtime::PartyCommand::intent(intent).map_err(|error| format!("party input: {error:?}"))?;
    let (operation, _) = exchange(client, &command).await?;
    let (view, result) = exchange(client, &query).await?;
    Ok(Sent {
        operation,
        view,
        result,
    })
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

async fn expect_notices(
    client: &mut DevClientSession,
    expected: &[runtime::party::PartyMembershipChange],
) -> TestResult {
    let mut seen = Vec::new();
    tokio::time::timeout(Duration::from_secs(12), async {
        let mut events = client.take_events();
        loop {
            for event in events {
                if let oteryn_dev_client::SessionEvent::PartyMembershipNotice(delta) = event {
                    seen.push(delta.value.change);
                    evidence(&format!("S01_NOTICE change={:?} sequence={}", delta.value.change, delta.server_sequence));
                }
            }
            if !expected.starts_with(&seen) {
                return Err(format!("S01 party notice mismatch: expected={expected:?} observed={seen:?}").into());
            }
            if seen == expected {
                return Ok(());
            }
            events = client.wait_for_events(Duration::from_secs(12)).await?;
        }
    })
    .await
    .map_err(|_| format!("S01 party notice deadline: expected={expected:?} observed={seen:?}"))?
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    use runtime::party::PartyMembershipChange::{Disbanded, Invited, Joined, Left};

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
    expect_notices(&mut peer, &[Invited]).await?;
    let invited = send(&mut peer, None).await?;
    check_state(&invited, &[], Some("Packet One"))?;
    let invitation = &invited.result.invitations[0];

    let joined = send(&mut peer, Some(PartyIntent::Accept(invitation.handle))).await?;
    expect_notices(&mut leader, &[Joined]).await?;
    expect_notices(&mut peer, &[Joined]).await?;
    let before_pass = [("Packet One", true), ("Packet Two", false)];
    check_state(&joined, &before_pass, None)?;

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

    check_state(
        &send(&mut leader, Some(PartyIntent::Leave)).await?,
        &[],
        None,
    )?;
    expect_notices(&mut leader, &[Left, Disbanded]).await?;
    expect_notices(&mut peer, &[Left, Disbanded]).await?;
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
    let expected = format!(
        "inviter=Packet One\njoiner=Packet Two\nmembers=2\nshared_xp=false\nnew_leader=Packet Two\ninvitation={}\njoin={}\njoin_view={}\nleadership={}\n",
        invited.view, joined.operation, joined.view, peer_view.view,
    );
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
            operation: String::new(),
            view: String::new(),
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
