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

async fn capture_reply(
    client: &mut DevClientSession,
    command: &runtime::PartyCommand,
    capability: u32,
) -> TestResult<(String, Vec<u8>)> {
    let session = session_label(client);
    let before = client.last_server_sequence();
    let reply = client
        .session_mut()
        .gated_command(
            capability,
            command.kind,
            &command.payload,
        )
        .await?;
    let name = record_reply(&session, command, &reply)?;
    if reply.server_sequence <= before {
        return Err("S01 reply did not advance the sequence".into());
    }
    Ok((name, reply.payload))
}

fn session_label(client: &DevClientSession) -> String {
    let session = client.join_snapshot().game_session_id;

    let encoded = session
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &encoded[..8],
        &encoded[8..12],
        &encoded[12..16],
        &encoded[16..20],
        &encoded[20..]
    )
}

fn record_reply(
    session: &str,
    command: &runtime::PartyCommand,
    reply: &oteryn_dev_client::GatedCommandResult,
) -> TestResult<String> {
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
    if reply.status != runtime::CommandStatus::Accepted {
        return Err(format!(
            "party command {} refused: {:?}",
            reply.command_id, reply.status
        )
        .into());
    }
    if command.kind == runtime::party::COMMAND_TYPE_PARTY_INTENT
        && matches!(
            runtime::party::decode_party_intent(&command.payload),
            Ok(PartyIntent::PassLeadership(_))
        )
    {
        capture(&format!("{name}.request"), &command.payload)?;
    }
    Ok(name)
}

async fn check_peer_shield(
    observer: &mut DevClientSession,
    target: EntityRef,
    expected: u8,
    watermark: u64,
    mut events: Vec<oteryn_dev_client::SessionEvent>,
) -> TestResult {
    use oteryn_dev_client::SessionEvent;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    loop {
        events.extend(observer.take_events());
        let mut matched = None;
        for event in events {
            match event {
                SessionEvent::PartyIcons(delta) if delta.server_sequence > watermark => {
                    matched = Some((delta.server_sequence, delta.value.iter()
                        .find(|icon| icon.entity == target).map(|icon| icon.shield)));
                }
                SessionEvent::PartyMembershipNotice(delta) => {
                    return Err(format!("S01 unexpected Party notice during shield observation: {delta:?}").into());
                }
                _ => {}
            }
        }
        if let Some((sequence, Some(shield))) = matched
            && shield == expected
        {
            evidence(&format!("S01_SHIELD sequence={sequence} watermark={watermark} shield={shield} target={target:?}"));
            return Ok(());
        }
        events = observer
            .wait_for_events(deadline.saturating_duration_since(tokio::time::Instant::now()))
            .await
            .map_err(|error| format!("S01 pushed Party shield missing: target={target:?} expected={expected} watermark={watermark} error={error}"))?;
    }
}

async fn send(client: &mut DevClientSession, intent: Option<PartyIntent>) -> TestResult<Sent> {
    let query = runtime::PartyCommand::query();
    let mutation = intent.is_some();
    let command = match intent {
        Some(intent) => runtime::PartyCommand::intent(intent)
            .map_err(|error| format!("party input: {error:?}"))?,
        None => query.clone(),
    };
    let session = session_label(client);
    let mut sequence = client.last_server_sequence();
    let first_command = client.next_command_id();
    let mut frames = Vec::new();
    let (_, response) =
        runtime::send_observed(client.session_mut(), &command, |reply| {
            frames.push(reply.clone());
        })
        .await?;
    if frames.len() != 1 + usize::from(mutation) {
        return Err(format!("S01 Party exchange returned {} frames: {:?}", frames.len(), response.status).into());
    }
    let mut names = Vec::new();
    for (index, frame) in frames.iter().enumerate() {
        if frame.server_sequence <= sequence
            || first_command.checked_add(index as u64) != Some(frame.command_id)
        {
            return Err("S01 Party exchange frames are out of order".into());
        }
        sequence = frame.server_sequence;
        names.push(record_reply(&session, if index == 0 { &command } else { &query }, frame)?);
    }
    let decode = |payload: &[u8]| -> TestResult<runtime::party::PartyResult> {
        let result = runtime::party::decode_party_result(payload)
            .map_err(|error| format!("party reply: {error:?}"))?;
        if result.disposition != PartyDisposition::Ok {
            return Err(format!("party refused: {:?}", result.disposition).into());
        }
        Ok(result)
    };
    if mutation {
        let receipt = decode(&response.operation)?;
        if receipt.shared_xp_enabled || !receipt.members.is_empty() || !receipt.invitations.is_empty() {
            return Err(format!("S01 party mutation returned an invalid receipt: {receipt:?}").into());
        }
    }
    let result = decode(response.view.as_deref().ok_or("S01 Party exchange returned no view")?)?;
    Ok(Sent { operation: names[0].clone(), view: names[usize::from(mutation)].clone(), result })
}

fn check_state(sent: &Sent, members: &[(&str, bool)], inviter: Option<&str>) -> TestResult {
    check_state_with_xp(sent, members, inviter, false)
}

fn check_state_with_xp(sent: &Sent, members: &[(&str, bool)], inviter: Option<&str>, shared: bool) -> TestResult {
    let result = &sent.result;
    if result.disposition != PartyDisposition::Ok
        || result.shared_xp_enabled != shared
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

#[cfg(test)]
mod notice_tests {
    use super::*;
    use runtime::party::PartyMembershipChange::Joined;

    #[test]
    fn joined_notice_must_follow_the_action_watermark() {
        assert!(check_notice(10, 10, Joined, Some(&Joined)).is_err());
        assert!(check_notice(10, 11, Joined, Some(&Joined)).is_ok());
        assert!(check_notice(10, 11, Joined, None).is_err());
    }
}

fn check_notice(
    watermark: u64,
    sequence: u64,
    kind: runtime::party::PartyMembershipChange,
    expected: Option<&runtime::party::PartyMembershipChange>,
) -> TestResult {
    if sequence <= watermark || expected != Some(&kind) {
        return Err(format!("S01 unexpected notice: change={kind:?} sequence={sequence} watermark={watermark} expected={expected:?}").into());
    }
    Ok(())
}

// Fresh admissions start at generation 1. The handle is scoped to the receiving
// session and to the character named by the notice.
fn expected_member_handle(client: &DevClientSession, subject: [u8; 16]) -> u64 {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"oteryn.party.handle.v1");
    hash.update(client.join_snapshot().game_session_id.as_bytes());
    hash.update(1_u64.to_be_bytes());
    hash.update([1]);
    hash.update(subject);
    let digest = hash.finalize();
    u64::from_be_bytes(core::array::from_fn(|index| digest[index]))
}

async fn expect_notices(
    client: &mut DevClientSession,
    actor: &str,
    watermark: u64,
    expected: &[(runtime::party::PartyMembershipChange, [u8; 16])],
) -> TestResult<Vec<oteryn_dev_client::SessionEvent>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let mut remaining = expected;
    let mut events = client.take_events();
    let mut icons = Vec::new();
    let mut domains = std::collections::BTreeSet::new();
    loop {
        for event in events {
            domains.insert(event.domain_id());
            if matches!(event, oteryn_dev_client::SessionEvent::PartyIcons(_)) {
                icons.push(event);
            } else if let oteryn_dev_client::SessionEvent::PartyMembershipNotice(delta) = event {
                check_notice(watermark, delta.server_sequence, delta.value.change, remaining.first().map(|(change, _)| change))?;
                let handle = expected_member_handle(client, remaining[0].1);
                if delta.value.member_handle != handle {
                    return Err(format!(
                        "S01 notice names the wrong character: actor={actor} change={:?} sequence={} expected_handle={handle} actual_handle={}",
                        delta.value.change, delta.server_sequence, delta.value.member_handle
                    ).into());
                }
                remaining = &remaining[1..];
                evidence(&format!(
                    "S01_NOTICE actor={actor} change={:?} sequence={} watermark={watermark}",
                    delta.value.change, delta.server_sequence
                ));
            }
        }
        if remaining.is_empty() {
            return Ok(icons);
        }
        let budget = deadline.saturating_duration_since(tokio::time::Instant::now());
        match client.wait_for_events(budget).await {
            Ok(next) => events = next,
            Err(error) => {
                return Err(format!(
                    "S01 party notice wait failed: actor={actor} expected={remaining:?} watermark={watermark} last_sequence={} decoded_domains={domains:?} error={error}",
                    client.last_server_sequence()
                ).into());
            }
        }
    }
}

fn reject_extra_notices(client: &mut DevClientSession) -> TestResult {
    for event in client.take_events() {
        if let oteryn_dev_client::SessionEvent::PartyMembershipNotice(delta) = event {
            return Err(format!(
                "S01 unexpected late notice: change={:?} handle={} sequence={}",
                delta.value.change, delta.value.member_handle, delta.server_sequence
            ).into());
        }
    }
    Ok(())
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

    let peer_before_invite = peer.last_server_sequence();
    let inviter = send(&mut leader, Some(PartyIntent::Invite(own(&peer)?))).await?;
    check_state(&inviter, &[("Packet One", true)], None)?;
    expect_notices(&mut peer, "peer", peer_before_invite, &[(Invited, clients.characters[0])]).await?;
    let invited = send(&mut peer, None).await?;
    check_state(&invited, &[], Some("Packet One"))?;
    let invitation = &invited.result.invitations[0];

    let leader_before_join = leader.last_server_sequence();
    let peer_before_join = peer.last_server_sequence();
    let joined = send(&mut peer, Some(PartyIntent::Accept(invitation.handle))).await?;
    expect_notices(&mut leader, "leader", leader_before_join, &[(Joined, clients.characters[1])]).await?;
    expect_notices(&mut peer, "peer", peer_before_join, &[(Joined, clients.characters[1])]).await?;
    let before_pass = [("Packet One", true), ("Packet Two", false)];
    check_state(&joined, &before_pass, None)?;

    let leader_view = send(&mut leader, None).await?;
    check_state(&leader_view, &before_pass, None)?;
    use oteryn_protocol_oteryn::chat::{CAPABILITY_CHAT_V1, ChatDisposition, decode_chat_intent_result};
    let toggle = runtime::PartyCommand::shared_experience(true).map_err(|error| format!("S01 shared XP wire: {error:?}"))?;
    let leader_before_toggle = leader.last_server_sequence();
    let peer_before_toggle = peer.last_server_sequence();
    let (toggle_receipt, payload) = capture_reply(&mut leader, &toggle, CAPABILITY_CHAT_V1).await?;
    let receipt = decode_chat_intent_result(&payload).map_err(|error| format!("S01 shared XP reply: {error:?}"))?;
    if receipt.disposition != ChatDisposition::Ok || receipt.wait_seconds != 0 {
        return Err(format!("S01 shared XP toggle refused: {receipt:?}").into());
    }
    let leader_shared = send(&mut leader, None).await?;
    check_state_with_xp(&leader_shared, &before_pass, None, true)?;
    check_state_with_xp(&send(&mut peer, None).await?, &before_pass, None, true)?;
    // Neither fixture character has contributed in combat, so both shared-XP shields blink.
    check_peer_shield(&mut leader, own(&peer)?, 7, leader_before_toggle, Vec::new()).await?;
    check_peer_shield(&mut peer, own(&leader)?, 8, peer_before_toggle, Vec::new()).await?;
    let member = leader_shared
        .result
        .members
        .iter()
        .find(|m| m.name == "Packet Two")
        .ok_or("peer absent from leader view")?;
    let leader_before_pass = leader.last_server_sequence();
    let peer_before_pass = peer.last_server_sequence();
    let passed = send(
        &mut leader,
        Some(PartyIntent::PassLeadership(member.handle)),
    )
    .await?;
    let after_pass = [("Packet One", false), ("Packet Two", true)];
    check_state_with_xp(&passed, &after_pass, None, true)?;
    let peer_view = send(&mut peer, None).await?;
    check_state_with_xp(&peer_view, &after_pass, None, true)?;

    let leader_ref = own(&leader)?;
    let peer_ref = own(&peer)?;
    tokio::try_join!(
        check_peer_shield(&mut leader, peer_ref, 8, leader_before_pass, Vec::new()),
        check_peer_shield(&mut peer, leader_ref, 7, peer_before_pass, Vec::new()),
    )?;

    let leader_before_leave = leader.last_server_sequence();
    let peer_before_leave = peer.last_server_sequence();
    check_state(
        &send(&mut leader, Some(PartyIntent::Leave)).await?,
        &[],
        None,
    )?;
    let leader_leave_icons = expect_notices(&mut leader, "leader", leader_before_leave, &[(Left, clients.characters[0]), (Disbanded, clients.characters[1])]).await?;
    let peer_leave_icons = expect_notices(&mut peer, "peer", peer_before_leave, &[(Left, clients.characters[0]), (Disbanded, clients.characters[1])]).await?;
    check_state(&send(&mut peer, None).await?, &[], None)?;
    check_peer_shield(&mut leader, own(&peer)?, 0, leader_before_leave, leader_leave_icons).await?;
    check_peer_shield(&mut peer, own(&leader)?, 0, peer_before_leave, peer_leave_icons).await?;
    for client in [&mut leader, &mut peer] {
        reject_extra_notices(client)?;
        let logout = client.session_mut().logout().await?;
        if logout.status != runtime::CommandStatus::Accepted
            || logout.result != Some(LogoutResult::Accepted)
        {
            return Err(format!("S01 first logout refused: {logout:?}").into());
        }
        reject_extra_notices(client)?;
    }
    drop((leader, peer));
    for (index, tag) in [(0, 0x49), (1, 0x4a)] {
        let mut client = connect_character(clients, tag, index).await?;
        check_state(&send(&mut client, None).await?, &[], None)?;
        reject_extra_notices(&mut client)?;
        let logout = client.session_mut().logout().await?;
        if logout.status != runtime::CommandStatus::Accepted
            || logout.result != Some(LogoutResult::Accepted)
        {
            return Err(format!("S01 relog cleanup refused: {logout:?}").into());
        }
        reject_extra_notices(&mut client)?;
    }
    let expected = format!(
        "inviter=Packet One\njoiner=Packet Two\nmembers=2\nshared_xp=false\nnew_leader=Packet Two\ninvitation={}\njoin={}\njoin_view={}\nleadership={}\nleader_before={}\ntoggle_receipt={}\nleader_shared={}\nleader_operation={}\nleader_after={}\n",
        invited.view, joined.operation, joined.view, peer_view.view,
        leader_view.view, toggle_receipt, leader_shared.view, passed.operation, passed.view,
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
