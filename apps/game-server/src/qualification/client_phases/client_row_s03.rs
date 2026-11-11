//! S03 client replay capture: a real guild foundation (Found), the own-guild summary, the
//! roster and the invitation the founder sends to a peer. The consumer is `request`,
//! `apply_result` and `panel_text` in `crates/client-ui/src/features/0040-guild`. The MOTD, bank,
//! war and guild-chat parts of the full row have no player-protocol writer and are not captured.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::guild::guild as wire;
use oteryn_protocol_oteryn::CommandStatus;

const GUILD_NAME: &str = "Qualification Guard";

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn own_name(client: &DevClientSession) -> TestResult<String> {
    own_name_from(client.world_entities())
}

fn own_name_from(input: Option<&oteryn_dev_client::WorldEntities>) -> TestResult<String> {
    let entities = input.ok_or("world entities missing")?;
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
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut founder = connect_character(clients, 0x4f, 0).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
    let mut invitee = connect_character(clients, 0x50, 1).await?;
    let mut row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let founder_name = own_name(&founder)?;
    let invitee_name = own_name(&invitee)?;

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
    capture("s03-found.bin", &found_bytes)?;
    let (own_bytes, own) = query(&mut founder, &wire::GuildQuery::Own).await?;
    let wire::GuildQueryResult::Own(Some(summary)) = own else {
        return Err("own guild summary absent after Found".into());
    };
    if summary.name != GUILD_NAME || summary.guild != found.guild {
        return Err(format!("own summary mismatch: {summary:?}").into());
    }
    capture("s03-own.bin", &own_bytes)?;
    let (roster_bytes, roster) = query(
        &mut founder,
        &wire::GuildQuery::Roster {
            guild: found.guild,
            after: None,
        },
    )
    .await?;
    let wire::GuildQueryResult::Roster { members, .. } = roster else {
        return Err("roster reply has the wrong shape".into());
    };
    let [member] = members.as_slice() else {
        return Err(format!("roster mismatch: {members:?}").into());
    };
    if member.name != founder_name {
        return Err(format!("roster member {} is not the founder", member.name).into());
    }
    capture("s03-roster.bin", &roster_bytes)?;
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
    let (invitations_bytes, invitations) = query(&mut invitee, &wire::GuildQuery::Invitations).await?;
    let wire::GuildQueryResult::Invitations(list) = invitations else {
        return Err("invitation reply has the wrong shape".into());
    };
    let [invitation] = list.as_slice() else {
        return Err(format!("invitation mismatch: {list:?}").into());
    };
    if invitation.name != GUILD_NAME {
        return Err(format!("invitation names {}", invitation.name).into());
    }
    capture("s03-invitations.bin", &invitations_bytes)?;
    let expected = format!(
        "guild_name={GUILD_NAME}\nfounder={founder_name}\nrank_count={}\nown_rank={}\nroster_level={}\ninvitee={invitee_name}\n",
        summary.ranks.len(),
        summary.own_rank,
        member.level
    );
    capture("s03-expected.txt", expected.as_bytes())?;
    // Best-effort cleanup so the founder does not stay in a guild for later rows.
    let (_, left) = intent(&mut founder, &wire::GuildIntent::Leave { guild: found.guild }).await?;
    if left.code != 1 {
        return Err(format!("guild restoration refused: {left:?}").into());
    }
    row_logout(&mut founder).await?; row_needs_logout_0 = false;
    row_logout(&mut invitee).await?; row_needs_logout_1 = false;
    Ok(expected.replace('\n', ";").replace(' ', "_"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { row_logout(&mut invitee).await?; }
    row_result

    }.await;
    if row_needs_logout_0 {
        row_logout(&mut founder).await?;
    }
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=S03.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S03.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=S03.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_projection_cannot_supply_evidence() {
        assert!(own_name_from(None).is_err());
    }
}
