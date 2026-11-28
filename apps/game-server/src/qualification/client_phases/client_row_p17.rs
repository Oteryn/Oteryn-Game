//! P17 client replay capture: the real Exiva List payloads after the target denies everyone, adds
//! a named player and removes it again, plus the rejection of a malformed SetModes. The
//! consumer is `ExivaPanel::apply` and `panel_text` in
//! `crates/client-ui/src/features/0147-exiva-settings`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::exiva::{
    CAPABILITY_EXIVA_SETTINGS_V1, COMMAND_TYPE_EXIVA_SETTINGS, PlayerList,
};

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;

    std::fs::File::create_new(root.join(name))?.write_all(bytes)?;
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P17 client logout refused".into());
    }
    Ok(())
}

async fn settings(
    client: &mut DevClientSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>, u64)> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_EXIVA_SETTINGS_V1,
            COMMAND_TYPE_EXIVA_SETTINGS,
            payload,
        )
        .await?;
    Ok((reply.status, reply.payload, reply.server_sequence))
}

async fn accepted(client: &mut DevClientSession, payload: &[u8]) -> TestResult<(Vec<u8>, u64)> {
    let (status, reply, sequence) = settings(client, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("settings {payload:?} refused: {status:?}").into());
    }
    Ok((reply, sequence))
}

async fn list(client: &mut DevClientSession) -> TestResult<(Vec<u8>, PlayerList, u64)> {
    let (payload, sequence) = accepted(client, &[5]).await?;
    let decoded = PlayerList::decode(&payload).ok_or("server List undecodable")?;
    Ok((payload, decoded, sequence))
}

fn own_name(client: &DevClientSession) -> TestResult<String> {
    let entities = client.world_entities().ok_or("entity snapshot absent")?;
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
    let capture_root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("P17 capture directory absent")?;
    let capture_root = Path::new(&capture_root);
    let mut caster = connect_character(clients, 0x43, 1).await?;
    let caster_name = own_name(&caster)?;
    logout(&mut caster).await?;
    drop(caster);
    let mut target = connect_character(clients, 0x42, 0).await?;
    let (_, initial, _) = list(&mut target).await?;
    if matches!(initial.modes, 0 | 2) || !initial.players.is_empty() {
        return Err(format!(
            "expected a distinct enabled mode and no explicit entries: {initial:?}"
        )
        .into());
    }
    let capture_session = uuid_text(target.join_snapshot().game_session_id.as_bytes());

    if accepted(&mut target, &[0, 0]).await?.0 != [1] {
        return Err("SetModes(0) lacked the accepted receipt".into());
    }
    let (denied_bytes, denied, denied_sequence) = list(&mut target).await?;
    if denied.modes != 0 || !denied.players.is_empty() {
        return Err(format!("deny-all List mismatch: {denied:?}").into());
    }
    capture(capture_root, "p17-denied.bin", &denied_bytes)?;

    if accepted(&mut target, &[0, 2]).await?.0 != [1]
        || list(&mut target).await?.1
            != (PlayerList {
                modes: 2,
                players: Vec::new(),
            })
    {
        return Err("SetModes(2) did not enable the empty explicit list".into());
    }
    let mut add = vec![6];
    add.extend(caster_name.as_bytes());
    if accepted(&mut target, &add).await?.0 != [1] {
        return Err("AddName lacked the accepted receipt".into());
    }
    let (allowed_bytes, allowed, allowed_sequence) = list(&mut target).await?;
    let [(caster_id, listed_name)] = allowed.players.as_slice() else {
        return Err(format!("allow List mismatch: {allowed:?}").into());
    };
    if allowed.modes != 2 || *caster_id != clients.characters[1] || listed_name != &caster_name {
        return Err(format!("allow List mismatch: {allowed:?}").into());
    }
    capture(capture_root, "p17-allowed.bin", &allowed_bytes)?;

    let (status, refused, refused_sequence) = settings(&mut target, &[0, 32]).await?;
    if status != CommandStatus::Rejected || refused != [0] {
        return Err(format!("malformed SetModes: status={status:?} payload={refused:?}").into());
    }
    capture(capture_root, "p17-refused.bin", &refused)?;
    let (after_bytes, _, _) = list(&mut target).await?;
    if after_bytes != allowed_bytes {
        return Err("malformed SetModes changed the allow list".into());
    }

    let mut remove = vec![2];
    remove.extend(caster_id);
    if accepted(&mut target, &remove).await?.0 != [1] {
        return Err("RemovePlayer lacked the accepted receipt".into());
    }
    let (removed_bytes, removed, removed_sequence) = list(&mut target).await?;
    if removed.modes != 2 || !removed.players.is_empty() {
        return Err(format!("removal List mismatch: {removed:?}").into());
    }
    capture(capture_root, "p17-removed.bin", &removed_bytes)?;
    let previous_session = target.join_snapshot().game_session_id;
    logout(&mut target).await?;
    drop(target);
    let mut target = connect_character(clients, 0x44, 0).await?;
    if target.join_snapshot().game_session_id == previous_session
        || list(&mut target).await?.1 != removed
    {
        return Err("fresh admission did not retain the removed list".into());
    }
    // Leave the target's permissions as found.
    if accepted(&mut target, &[0, initial.modes]).await?.0 != [1]
        || list(&mut target).await?.1 != initial
    {
        return Err("restoring the target's initial settings did not take".into());
    }
    logout(&mut target).await?;
    release_row_characters(clients).await?;
    let id_hex: String = caster_id.iter().map(|byte| format!("{byte:02x}")).collect();
    if !(0 < denied_sequence
        && denied_sequence < allowed_sequence
        && allowed_sequence < refused_sequence
        && refused_sequence < removed_sequence)
    {
        return Err("P17 capture results are out of order".into());
    }
    let binding = format!(
        "{capture_session}:{denied_sequence}:{allowed_sequence}:{refused_sequence}:{removed_sequence}"
    );
    let expected = format!(
        "binding={binding}\ndenied_modes={}\nallowed_modes={}\nplayer={id_hex} {caster_name}\nremoved_modes={}\nrefused_status=Rejected\n",
        denied.modes, allowed.modes, removed.modes
    );
    capture(capture_root, "p17-expected.txt", expected.as_bytes())?;
    Ok(binding)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P17.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P17.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P17.client status=FAIL evidence=timeout"),
    }
}
