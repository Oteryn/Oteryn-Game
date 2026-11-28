//! S02 client replay capture: the real party shield/skull icon replies as the inviter sees the
//! invited peer, then as each member sees the other after the accepted join. The consumer is
//! `apply` in `apps/client/src/features/0134-party-skull/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::party as runtime;
use oteryn_dev_client::{DevClientSession, EntityRef};
use oteryn_protocol_oteryn::{CommandStatus, party_skull};
use runtime::party::PartyIntent;

const CAPTURES: [&str; 4] = [
    "s02-invited.bin",
    "s02-member.bin",
    "s02-leader.bin",
    "s02-expected.txt",
];
const REPLAY: &str =
    "play::feature_0134_party_skull::protocol::e2e_s02::e2e_s02_replays_party_shields_and_skulls";

struct CaptureDirectory(std::path::PathBuf);

impl CaptureDirectory {
    fn new() -> TestResult<Self> {
        let output = std::process::Command::new("mktemp").arg("-d").output()?;
        if !output.status.success() {
            return Err("S02 could not create a private capture directory".into());
        }
        Ok(Self(std::str::from_utf8(&output.stdout)?.trim_end().into()))
    }
}

impl Drop for CaptureDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn replay(captures: CaptureDirectory) -> TestResult {
    let budget = Duration::from_secs(480);
    verdict::Harness::global().bounded_wait(tokio::time::Instant::now() + budget, "client replay");
    let output = tokio::task::spawn_blocking(
        move || -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            let output = std::process::Command::new("timeout")
                .args([
                    "--kill-after=5s",
                    "470s",
                    "cargo",
                    "+1.100.0",
                    "test",
                    "--locked",
                    "-j1",
                ])
                .args(["-p", "oteryn-client", "--lib", REPLAY, "--", "--exact"])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .env("E2E_ALL_CAPTURE_DIR", &captures.0)
                .output()?;
            let stdout = std::str::from_utf8(&output.stdout)?;
            let test_line = format!("test {REPLAY} ... ok");
            if !output.status.success()
                || !stdout.lines().any(|line| line == test_line)
                || !stdout
                    .lines()
                    .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"))
            {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let last_error = stderr.lines().last().unwrap_or_default();
                return Err(
                    format!("S02 replay failed: {} {stdout} {last_error}", output.status).into(),
                );
            }
            if let Some(export) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                for name in CAPTURES {
                    std::fs::copy(captures.0.join(name), Path::new(&export).join(name))?;
                }
            }
            Ok(test_line)
        },
    )
    .await?;
    let line = output.map_err(|error| error.to_string())?;
    evidence(&format!("S02_CLIENT_REPLAY {line}"));
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

fn require_fresh_party_reply(status: CommandStatus) -> TestResult {
    if status != CommandStatus::Accepted {
        return Err(format!("party command was not freshly accepted: {status:?}").into());
    }
    Ok(())
}

async fn party(
    client: &mut DevClientSession,
    intent: Option<PartyIntent>,
) -> TestResult<runtime::party::PartyResult> {
    let command = match intent {
        Some(intent) => runtime::PartyCommand::intent(intent)
            .map_err(|e| format!("party input: {e:?}"))?,
        None => runtime::PartyCommand::query(),
    };
    let reply = client.session_mut().gated_command(
        runtime::party::CAPABILITY_PARTY_V1, command.kind, &command.payload,
    ).await?;
    require_fresh_party_reply(reply.status)?;
    let operation = runtime::party::decode_party_result(&reply.payload)
        .map_err(|e| format!("party operation: {e:?}"))?;
    if operation.disposition != runtime::party::PartyDisposition::Ok {
        return Err(format!("party refused: {:?}", operation.disposition).into());
    }
    Ok(operation)
}

fn check_icons(
    icons: &[party_skull::PartyIcon],
    expected: [(EntityRef, u8); 2],
) -> TestResult {
    if icons.len() != expected.len() || !expected.into_iter().all(|(entity, shield)| {
        icons.contains(&party_skull::PartyIcon { entity, shield, skull: 0 })
    }) {
        return Err(format!("S02 icon set mismatch expected={expected:?} skulls=0 actual={icons:?}").into());
    }
    Ok(())
}

fn require_unpartied(state: &runtime::party::PartyResult) -> TestResult {
    if state.shared_xp_enabled || !state.members.is_empty() || !state.invitations.is_empty() {
        return Err(format!("S02 expected no party or invitations: {state:?}").into());
    }
    Ok(())
}

/// Keep the raw reply and record every decoded icon in its original order.
async fn icon(
    client: &mut DevClientSession,
    target: EntityRef,
    view: &str,
    expected: [(EntityRef, u8); 2],
) -> TestResult<(Vec<u8>, u8, u8, String, String)> {
    use std::fmt::Write as _;
    const SHIELDS: [&str; 12] = [
        "None",
        "Invitation blue",
        "Invitation yellow",
        "Member",
        "Leader",
        "Shared member",
        "Shared leader",
        "Inactive blinking member",
        "Inactive blinking leader",
        "Inactive member",
        "Inactive leader",
        "Outsider gray",
    ];
    const SKULLS: [&str; 7] = ["None", "Yellow", "Green", "White", "Red", "Black", "Orange"];
    let reply = client
        .session_mut()
        .gated_command(
            party_skull::CAPABILITY_PARTY_SKULL_V1,
            party_skull::COMMAND_TYPE_PARTY_SKULL_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "S02 icon query refused view={view} session={} command_id={} status={:?} server_sequence={} payload_hex={}",
            hex(client.join_snapshot().game_session_id.as_bytes()),
            reply.command_id,
            reply.status,
            reply.server_sequence,
            hex(&reply.payload),
        ).into());
    }
    let icons = party_skull::decode_party_icons(&reply.payload)
        .map_err(|e| format!("icon reply undecodable: {e:?}"))?;
    check_icons(&icons, expected)?;
    let found = icons
        .iter()
        .find(|entry| entry.entity == target)
        .ok_or("target absent from the icon reply")?;
    let mut rows = String::new();
    for entry in &icons {
        let identity = entry.entity.identity;
        let generation = entry.entity.generation;
        let shield = entry.shield;
        let skull = entry.skull;
        let shield_label = SHIELDS
            .get(usize::from(shield))
            .ok_or("unknown party shield")?;
        let skull_label = SKULLS.get(usize::from(skull)).ok_or("unknown skull")?;
        writeln!(
            rows,
            "{view}\t{}\t{generation}\t{shield}\t{skull}\tEntity {identity:02x?} / {generation} | Shield: {shield_label} | Skull: {skull_label}",
            hex(&identity),
        )?;
    }
    let receipt = format!(
        "{view}_session={}\n{view}_command={}\n{view}_sequence={}\n",
        hex(client.join_snapshot().game_session_id.as_bytes()),
        reply.command_id, reply.server_sequence,
    );
    Ok((reply.payload, found.shield, found.skull, rows, receipt))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sees(client: &DevClientSession, target: EntityRef) -> bool {
    client
        .world_entities()
        .is_some_and(|view| view.iter().any(|entry| entry.entity == target))
}

async fn wait_for_pair_visibility(
    leader: &mut DevClientSession,
    peer: &mut DevClientSession,
    leader_ref: EntityRef,
    peer_ref: EntityRef,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !sees(leader, peer_ref) || !sees(peer, leader_ref) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "party peers missing from served visibility at {:?} and {:?}",
                leader.world_spatial().actor_position,
                peer.world_spatial().actor_position,
            )
            .into());
        }
        let (leader_live, peer_live) = tokio::join!(
            leader.service_liveness(Duration::from_millis(50)),
            peer.service_liveness(Duration::from_millis(50)),
        );
        leader_live?;
        peer_live?;
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>, captures: &Path) -> TestResult {
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
            && u32::from(at.x.abs_diff(other.x)) + u32::from(at.y.abs_diff(other.y)) == 1
    })
    .ok_or("peer approach route unavailable")?;
    full_row_s02::walk_s02(&mut leader, world, &path).await?;
    let (leader_ref, peer_ref) = (own(&leader)?, own(&peer)?);
    wait_for_pair_visibility(&mut leader, &mut peer, leader_ref, peer_ref).await?;

    require_unpartied(&party(&mut leader, None).await?)?;
    require_unpartied(&party(&mut peer, None).await?)?;
    icon(&mut leader, peer_ref, "before", [(leader_ref, 0), (peer_ref, 0)]).await?;
    party(&mut leader, Some(PartyIntent::Invite(peer_ref))).await?;
    icon(&mut peer, leader_ref, "invitee", [(leader_ref, 1), (peer_ref, 0)]).await?;
    let (invited_bytes, invited_shield, invited_skull, invited_rows, invited_receipt) =
        icon(&mut leader, peer_ref, "invited", [(leader_ref, 4), (peer_ref, 2)]).await?;
    if (invited_shield, invited_skull) != (2, 0) {
        return Err(format!("inviter icons observed={invited_shield}/{invited_skull}").into());
    }
    std::fs::write(captures.join(CAPTURES[0]), invited_bytes)?;

    let state = party(&mut peer, None).await?;
    if state.invitations.len() != 1 || !state.members.is_empty() {
        return Err(format!("S02 expected one pending invitation: {state:?}").into());
    }
    let handle = state
        .invitations
        .first()
        .ok_or("peer lacks the invitation")?
        .handle;
    party(&mut peer, Some(PartyIntent::Accept(handle))).await?;
    let (member_bytes, peer_shield, peer_skull, member_rows, member_receipt) =
        icon(&mut leader, peer_ref, "member", [(leader_ref, 4), (peer_ref, 3)]).await?;
    let (leader_bytes, leader_shield, leader_skull, leader_rows, leader_receipt) =
        icon(&mut peer, leader_ref, "leader", [(leader_ref, 4), (peer_ref, 3)]).await?;
    if (peer_shield, peer_skull) != (3, 0) || (leader_shield, leader_skull) != (4, 0) {
        return Err(format!(
            "joined icons observed={peer_shield}/{peer_skull} {leader_shield}/{leader_skull}"
        )
        .into());
    }
    std::fs::write(captures.join(CAPTURES[1]), member_bytes)?;
    std::fs::write(captures.join(CAPTURES[2]), leader_bytes)?;
    let expected = format!(
        "peer_identity={}\npeer_generation={}\nleader_identity={}\nleader_generation={}\ninvited_shield={invited_shield}\ninvited_skull={invited_skull}\nmember_shield={peer_shield}\nmember_skull={peer_skull}\nleader_shield={leader_shield}\nleader_skull={leader_skull}\n{invited_receipt}{member_receipt}{leader_receipt}\n{invited_rows}{member_rows}{leader_rows}",
        hex(&peer_ref.identity),
        peer_ref.generation,
        hex(&leader_ref.identity),
        leader_ref.generation,
    );
    std::fs::write(captures.join(CAPTURES[3]), expected)?;
    let previous_sessions = [
        leader.join_snapshot().game_session_id,
        peer.join_snapshot().game_session_id,
    ];
    for client in [&mut leader, &mut peer] {
        let reply = client.session_mut().logout().await?;
        if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!(
                "S02 client logout session={} reply={reply:?}",
                hex(client.join_snapshot().game_session_id.as_bytes()),
            ).into());
        }
    }
    drop((leader, peer));
    let mut leader = connect_character(clients, 0x4b, 0).await?;
    let mut peer = connect_character(clients, 0x4c, 1).await?;
    if leader.join_snapshot().game_session_id == previous_sessions[0]
        || peer.join_snapshot().game_session_id == previous_sessions[1]
    {
        return Err("S02 logout verification reused a session".into());
    }
    require_unpartied(&party(&mut leader, None).await?)?;
    require_unpartied(&party(&mut peer, None).await?)?;
    let (leader_ref, peer_ref) = (own(&leader)?, own(&peer)?);
    wait_for_pair_visibility(&mut leader, &mut peer, leader_ref, peer_ref).await?;
    icon(&mut leader, peer_ref, "relog", [(leader_ref, 0), (peer_ref, 0)]).await?;
    icon(&mut peer, leader_ref, "relog", [(leader_ref, 0), (peer_ref, 0)]).await?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let start = std::time::Instant::now();
    let result = verdict::observe(async {
        let captures = CaptureDirectory::new()?;
        verdict::indexed_timeout(Duration::from_secs(120), journey(clients, &captures.0))
            .await
            .map_err(|_| "S02 live capture timed out")??;
        replay(captures).await
    })
    .await;
    let (status, detail) = match result {
        Ok(()) => ("PASS", "live-party-icons-and-client-reducer".to_owned()),
        Err(error) => ("FAIL", error.to_string().replace('\n', " ")),
    };
    evidence(&format!(
        "e2e_all scenario=S02.client status={status} seconds={:.3} evidence={detail}",
        start.elapsed().as_secs_f64()
    ));
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_and_rejected_party_replies_are_not_fresh() {
        assert!(require_fresh_party_reply(CommandStatus::Accepted).is_ok());
        assert!(require_fresh_party_reply(CommandStatus::DuplicateReplay).is_err());
        assert!(require_fresh_party_reply(CommandStatus::Rejected).is_err());
    }

    #[test]
    fn relog_check_rejects_retained_membership_or_invitation() {
        let mut state = runtime::party::PartyResult::refused(runtime::party::PartyDisposition::Ok);
        assert!(require_unpartied(&state).is_ok());
        state.members.push(runtime::party::PartyEntry {
            list: vec![],
            handle: 1,
            name: "Party leader".into(),
            leader: true,
            expires_at: 0,
            party_spell_remaining_ms: None,
            channel: None,
        });
        assert!(require_unpartied(&state).is_err());
        state.invitations = std::mem::take(&mut state.members);
        assert!(require_unpartied(&state).is_err());
    }

    #[test]
    fn icon_check_rejects_extra_duplicate_and_incorrect_icons() {
        let leader = EntityRef { identity: [1; 16], generation: 2 };
        let peer = EntityRef { identity: [2; 16], generation: 3 };
        let expected = [(leader, 4), (peer, 3)];
        let icons = [
            party_skull::PartyIcon { entity: leader, shield: 4, skull: 0 },
            party_skull::PartyIcon { entity: peer, shield: 3, skull: 0 },
        ];
        assert!(check_icons(&icons, expected).is_ok());
        assert!(check_icons(&[icons[0], icons[1], icons[1]], expected).is_err());
        assert!(check_icons(&[icons[0], icons[0]], expected).is_err());
        assert!(check_icons(&[party_skull::PartyIcon { shield: 3, ..icons[0] }, icons[1]], expected).is_err());
        assert!(check_icons(&[party_skull::PartyIcon { skull: 3, ..icons[0] }, icons[1]], expected).is_err());
    }
}
