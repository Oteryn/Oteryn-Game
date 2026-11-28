//! S03 live guild journey and client replay: a real guild foundation (Found), the own-guild summary, the
//! roster and the invitation the founder sends to a peer. The consumer is `request`,
//! `apply_result` and `panel_text` in `crates/client-ui/src/features/0040-guild`. The MOTD, bank,
//! war and guild-chat parts of the full row have no player-protocol writer and are not captured.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::guild::guild as wire;
use oteryn_protocol_oteryn::CommandStatus;

const GUILD_NAME: &str = "Qualification Guard";

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?
        .write_all(bytes)?;
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("S03 logout refused: {reply:?}").into());
    }
    Ok(())
}

fn own_name(client: &DevClientSession) -> TestResult<String> {
    let entities = client.world_entities().ok_or("world entities missing")?;
    entities
        .iter()
        .find(|entity| entity.entity.identity == *entities.own_identity())
        .and_then(|entity| entity.display_name.as_ref())
        .ok_or("own display name missing")?
        .as_str()
        .map(str::to_owned)
        .map_err(|e| format!("invalid display name: {e:?}").into())
}

async fn raw(client: &mut DevClientSession, kind: u32, payload: Vec<u8>) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_GUILD_V1, kind, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("guild command status {:?}", reply.status).into());
    }
    evidence(&format!(
        "S03_CLIENT_FRAME session={} command={} sequence={} kind={kind}",
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        reply.command_id,
        reply.server_sequence,
    ));
    Ok(reply.payload)
}

async fn intent(
    client: &mut DevClientSession,
    value: &wire::GuildIntent,
) -> TestResult<(Vec<u8>, wire::GuildIntentResult)> {
    let payload = wire::encode_guild_intent(value).map_err(|e| format!("encode intent: {e:?}"))?;
    let bytes = raw(client, wire::COMMAND_TYPE_GUILD_INTENT, payload).await?;
    let decoded = wire::decode_guild_intent_result(&bytes)
        .map_err(|e| format!("intent reply undecodable: {e:?}"))?;
    Ok((bytes, decoded))
}

async fn query(
    client: &mut DevClientSession,
    value: &wire::GuildQuery,
) -> TestResult<(Vec<u8>, wire::GuildQueryResult)> {
    let payload = wire::encode_guild_query(value).map_err(|e| format!("encode query: {e:?}"))?;
    let bytes = raw(client, wire::COMMAND_TYPE_GUILD_QUERY, payload).await?;
    let decoded = wire::decode_guild_query_result(&bytes)
        .map_err(|e| format!("query reply undecodable: {e:?}"))?;
    Ok((bytes, decoded))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_S03_ACCOUNT").await?;
    let founder_scope = identity.clients(clients);
    let mut founder = connect_character(&founder_scope, 0x4f, 0).await?;
    character_prep::declare_seeded(
        "S03.client",
        "q_s03,level-8,Premium",
        "guild founding prerequisites",
    );
    request_fixture_control(
        uuid_text(founder.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "s03-sorcerer".into(),
    )
    .await?;
    logout(&mut founder).await?;
    founder = connect_character(&founder_scope, 0x51, 0).await?;
    if full_e2ef7::player_level(&mut founder).await? != 8 {
        return Err("S03 founder did not reload at level 8".into());
    }
    character_prep::grant_premium(&founder).await?;
    full_e2ef7::require_current_light_premium(&mut founder).await?;
    let founder_session = founder.join_snapshot().game_session_id;
    let capture_root = std::env::temp_dir().join(format!(
        "s03-capture-{}",
        uuid_text(founder_session.as_bytes())
    ));
    std::fs::create_dir(&capture_root)?;
    let mut invitee = connect_character(clients, 0x50, 1).await?;
    let founder_name = own_name(&founder)?;
    let invitee_name = own_name(&invitee)?;

    if query(&mut founder, &wire::GuildQuery::Own).await?.1 != wire::GuildQueryResult::Own(None) {
        return Err("S03 founder already has a guild".into());
    }
    if query(&mut invitee, &wire::GuildQuery::Invitations).await?.1
        != wire::GuildQueryResult::Invitations(Vec::new())
    {
        return Err("S03 invitee already has invitations".into());
    }

    let (found_bytes, found) = intent(
        &mut founder,
        &wire::GuildIntent::Found {
            name: GUILD_NAME.into(),
        },
    )
    .await?;
    if found.code != 1 {
        return Err(format!("Found refused: {found:?}").into());
    }
    capture(&capture_root, "s03-found.bin", &found_bytes)?;
    let (own_bytes, own) = query(&mut founder, &wire::GuildQuery::Own).await?;
    let wire::GuildQueryResult::Own(Some(summary)) = own else {
        return Err("own guild summary absent after Found".into());
    };
    if summary.name != GUILD_NAME
        || summary.guild != found.guild
        || summary.own_rank != 1
        || summary.ranks.len() != 3
    {
        return Err(format!("own summary mismatch: {summary:?}").into());
    }
    capture(&capture_root, "s03-own.bin", &own_bytes)?;
    let (roster_bytes, roster) = query(
        &mut founder,
        &wire::GuildQuery::Roster {
            guild: found.guild,
            after: None,
        },
    )
    .await?;
    let wire::GuildQueryResult::Roster {
        guild,
        revision,
        members,
        next,
    } = &roster
    else {
        return Err("roster reply has the wrong shape".into());
    };
    if *guild != found.guild || *revision != summary.revision || next.is_some() {
        return Err("S03 roster identity, revision or cursor mismatch".into());
    }
    let [member] = members.as_slice() else {
        return Err(format!("roster mismatch: {members:?}").into());
    };
    if member.name != founder_name
        || member.character != founder_scope.characters[0]
        || member.level != 1
    {
        return Err(format!("roster member {} is not the founder", member.name).into());
    }
    capture(&capture_root, "s03-roster.bin", &roster_bytes)?;
    let (_, invite) = intent(
        &mut founder,
        &wire::GuildIntent::Invite {
            guild: found.guild,
            name: invitee_name.clone(),
        },
    )
    .await?;
    if invite.code != 1 {
        return Err(format!("Invite refused: {invite:?}").into());
    }
    let (invitations_bytes, invitations) =
        query(&mut invitee, &wire::GuildQuery::Invitations).await?;
    let wire::GuildQueryResult::Invitations(list) = invitations else {
        return Err("invitation reply has the wrong shape".into());
    };
    let [invitation] = list.as_slice() else {
        return Err(format!("invitation mismatch: {list:?}").into());
    };
    if invitation.name != GUILD_NAME || invitation.guild != found.guild {
        return Err(format!("invitation names {}", invitation.name).into());
    }
    capture(&capture_root, "s03-invitations.bin", &invitations_bytes)?;
    let expected = format!(
        "guild_name={GUILD_NAME}\nfounder={founder_name}\nrank_count={}\nown_rank={}\nroster_level={}\ninvitee={invitee_name}\n",
        summary.ranks.len(),
        summary.own_rank,
        member.level
    );
    capture(&capture_root, "s03-expected.txt", expected.as_bytes())?;
    let (_, own_after_invite) = query(&mut founder, &wire::GuildQuery::Own).await?;
    if !matches!(&own_after_invite, wire::GuildQueryResult::Own(Some(current))
        if current.guild == summary.guild && current.name == summary.name
            && current.own_rank == summary.own_rank && current.ranks == summary.ranks)
    {
        return Err("S03 founding state changed when sending the invitation".into());
    }
    let invitee_session = invitee.join_snapshot().game_session_id;
    logout(&mut founder).await?;
    logout(&mut invitee).await?;
    let mut founder = connect_character(&founder_scope, 0x52, 0).await?;
    let mut invitee = connect_character(clients, 0x53, 1).await?;
    if founder.join_snapshot().game_session_id == founder_session
        || invitee.join_snapshot().game_session_id == invitee_session
        || query(&mut founder, &wire::GuildQuery::Own).await?.1 != own_after_invite
        || query(&mut invitee, &wire::GuildQuery::Invitations).await?.1
            != wire::GuildQueryResult::Invitations(vec![invitation.clone()])
    {
        return Err("S03 guild or invitation did not persist across fresh login".into());
    }
    let (_, persisted_roster) = query(
        &mut founder,
        &wire::GuildQuery::Roster {
            guild: found.guild,
            after: None,
        },
    )
    .await?;
    let wire::GuildQueryResult::Roster {
        guild,
        members,
        next,
        ..
    } = persisted_roster
    else {
        return Err("S03 relogged roster has the wrong shape".into());
    };
    if guild != found.guild || next.is_some() || members.as_slice() != [member.clone()] {
        return Err("S03 exact founder roster did not persist across fresh login".into());
    }
    logout(&mut founder).await?;
    logout(&mut invitee).await?;

    let replay = required("NODE_BOOT_S03_REPLAY_BIN")?;
    let replay_root = capture_root.clone();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(replay)
            .env("E2E_ALL_CAPTURE_DIR", replay_root)
            .args([
                "guild::e2e_s03::guild_replay_found_summary_roster_and_invitation",
                "--exact",
            ])
            .output()
    })
    .await??;
    std::fs::remove_dir_all(&capture_root)?;
    let replay_log = String::from_utf8(output.stdout)?;
    if !output.status.success() || !replay_log.contains("test result: ok. 1 passed; 0 failed;") {
        return Err(format!("S03 client replay failed: {replay_log}").into());
    }
    evidence("S03_CLIENT_REPLAY assertions=passed tests=1");
    Ok(expected.replace('\n', ";").replace(' ', "_"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=S03.client status=PASS evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S03.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=S03.client status=FAIL evidence=timeout"),
    }
}
