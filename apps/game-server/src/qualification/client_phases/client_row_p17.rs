//! P17 client replay capture: the real Exiva List payloads after the target denies everyone, adds
//! a named player and removes it again, plus the empty rejection of a malformed SetModes. The
//! consumer is `ExivaPanel::apply` and `panel_text` in
//! `crates/client-ui/src/features/0147-exiva-settings`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::exiva::{
    CAPABILITY_EXIVA_SETTINGS_V1, COMMAND_TYPE_EXIVA_SETTINGS, PlayerList,
};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn settings(
    client: &mut DevClientSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_EXIVA_SETTINGS_V1,
            COMMAND_TYPE_EXIVA_SETTINGS,
            payload,
        )
        .await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(client: &mut DevClientSession, payload: &[u8]) -> TestResult<Vec<u8>> {
    let (status, reply) = settings(client, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("settings {payload:?} refused: {status:?}").into());
    }
    Ok(reply)
}

async fn list(client: &mut DevClientSession) -> TestResult<(Vec<u8>, PlayerList)> {
    let payload = accepted(client, &[5]).await?;
    let decoded = PlayerList::decode(&payload).ok_or("server List undecodable")?;
    Ok((payload, decoded))
}

fn own_name(client: &DevClientSession) -> TestResult<String> {
    own_name_from(client.world_entities())
}

fn own_name_from(input: Option<&oteryn_dev_client::WorldEntities>) -> TestResult<String> {
    let entities = input.ok_or("entity snapshot absent")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .and_then(|entry| entry.display_name)
        .ok_or("own display name absent")?
        .as_str()
        .map(str::to_owned)
        .map_err(|error| format!("own name invalid UTF-8: {error:?}").into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut caster = connect_character(clients, 0x43, 1).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
    let caster_name = own_name(&caster)?;
    row_logout(&mut caster).await?; row_needs_logout_0 = false;
    if row_needs_logout_0 { row_logout(&mut caster).await?; row_needs_logout_0 = false; }
    let mut target = connect_character(clients, 0x42, 0).await?;
    let mut row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let (_, initial) = list(&mut target).await?;

    if accepted(&mut target, &[0, 0]).await? != [1] {
        return Err("SetModes(0) lacked the accepted receipt".into());
    }
    let (denied_bytes, denied) = list(&mut target).await?;
    if denied.modes != 0 || !denied.players.is_empty() {
        return Err(format!("deny-all List mismatch: {denied:?}").into());
    }
    capture("p17-denied.bin", &denied_bytes)?;

    let mut add = vec![6];
    add.extend(caster_name.as_bytes());
    if accepted(&mut target, &add).await? != [1] {
        return Err("AddName lacked the accepted receipt".into());
    }
    let (allowed_bytes, allowed) = list(&mut target).await?;
    let [(caster_id, listed_name)] = allowed.players.as_slice() else {
        return Err(format!("allow List mismatch: {allowed:?}").into());
    };
    if allowed.modes != 2 || listed_name != &caster_name {
        return Err(format!("allow List mismatch: {allowed:?}").into());
    }
    capture("p17-allowed.bin", &allowed_bytes)?;

    let (status, refused) = settings(&mut target, &[0, 32]).await?;
    if status != CommandStatus::Rejected || !refused.is_empty() {
        return Err(format!("malformed SetModes: status={status:?} payload={refused:?}").into());
    }
    let (after_bytes, _) = list(&mut target).await?;
    if after_bytes != allowed_bytes {
        return Err("malformed SetModes changed the allow list".into());
    }

    let mut remove = vec![2];
    remove.extend(caster_id);
    if accepted(&mut target, &remove).await? != [1] {
        return Err("RemovePlayer lacked the accepted receipt".into());
    }
    let (removed_bytes, removed) = list(&mut target).await?;
    if removed.modes != 2 || !removed.players.is_empty() {
        return Err(format!("removal List mismatch: {removed:?}").into());
    }
    capture("p17-removed.bin", &removed_bytes)?;
    // Leave the target's permissions as found.
    accepted(&mut target, &[0, initial.modes]).await?;
    let id_hex: String = caster_id.iter().map(|byte| format!("{byte:02x}")).collect();
    let expected = format!(
        "denied_modes={}\nallowed_modes={}\nplayer={id_hex} {caster_name}\nremoved_modes={}\nrefused_status=Rejected\n",
        denied.modes, allowed.modes, removed.modes
    );
    capture("p17-expected.txt", expected.as_bytes())?;
    row_logout(&mut target).await?; row_needs_logout_1 = false;
    Ok(expected.replace('\n', ";").replace(' ', "_"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { row_logout(&mut target).await?; }
    row_result

    }.await;
    if row_needs_logout_0 {
        row_logout(&mut caster).await?;
    }
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P17.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P17.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P17.client status=FAIL evidence=timeout"),
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
