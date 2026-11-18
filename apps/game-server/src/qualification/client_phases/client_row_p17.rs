//! P17 client replay capture: the real Exiva List payloads after the target denies everyone, adds
//! a named player and removes it again, plus the empty rejection of a malformed SetModes. The
//! consumer is `ExivaPanel::apply` and `panel_text` in
//! `crates/client-ui/src/features/0147-exiva-settings`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::exiva::{
    CAPABILITY_EXIVA_SETTINGS_V1, COMMAND_TYPE_EXIVA_SETTINGS, PlayerList,
};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "P17_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
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

fn settings_restored(receipt: &[u8], same_list: bool) -> bool {
    receipt == [1] && same_list
}

#[test]
fn exiva_restoration_requires_receipt_and_original_list() {
    assert!(!settings_restored(&[1], false));
    assert!(!settings_restored(&[], true));
    assert!(settings_restored(&[1], true));
}

fn exiva_membership_changes(current: &[[u8; 16]], desired: &[[u8; 16]]) -> Vec<(u8, [u8; 16])> {
    current
        .iter()
        .filter(|id| !desired.contains(id))
        .map(|id| (2, *id))
        .chain(
            desired
                .iter()
                .filter(|id| !current.contains(id))
                .map(|id| (1, *id)),
        )
        .collect()
}

#[test]
fn exiva_nonempty_fixture_can_be_cleared_and_restored() {
    let initial = [[1; 16], [2; 16]];
    assert_eq!(
        exiva_membership_changes(&initial, &[]),
        [(2, initial[0]), (2, initial[1])]
    );
    assert_eq!(
        exiva_membership_changes(&[[3; 16]], &initial),
        [(2, [3; 16]), (1, initial[0]), (1, initial[1])]
    );
}

async fn replace_members(
    client: &mut DevClientSession,
    current: &PlayerList,
    desired: &PlayerList,
) -> TestResult {
    let ids = |list: &PlayerList| list.players.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    for (operation, id) in exiva_membership_changes(&ids(current), &ids(desired)) {
        let mut payload = vec![operation];
        payload.extend(id);
        if accepted(client, &payload).await? != [1] {
            return Err(
                format!("Exiva membership mutation lacked receipt: {operation} {id:?}").into(),
            );
        }
    }
    Ok(())
}

fn keep_observation_error<T, E>(observed: Result<T, E>, cleanup: Result<(), E>) -> Result<T, E> {
    observed.and_then(|value| cleanup.map(|()| value))
}

#[test]
fn lost_mutation_reply_remains_root_when_recovery_fails() {
    assert_eq!(
        keep_observation_error::<(), _>(Err("lost reply"), Err("restore failed")),
        Err("lost reply")
    );
    assert_eq!(
        keep_observation_error(Ok(()), Err("restore failed")),
        Err("restore failed")
    );
}

async fn restore_settings(target: &mut DevClientSession, initial: &PlayerList) -> TestResult {
    let receipt = accepted(target, &[0, initial.modes]).await?;
    let (_, current) = list(target).await?;
    replace_members(target, &current, initial).await?;
    let (_, restored) = list(target).await?;
    if !settings_restored(&receipt, restored == *initial) {
        return Err(format!("Exiva restoration unconfirmed: initial={initial:?}; restored={restored:?}; receipt={receipt:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut caster = connect_character(clients, 0x43, 1).await?;
    let caster_name = own_name(&caster)?;
    let _ = caster.session_mut().logout().await?;
    drop(caster);
    let mut target = connect_character(clients, 0x42, 0).await?;
    let (_, initial) = list(&mut target).await?;
    let mut probe = (clients.characters[1], caster_name);
    if initial.players.len() == 100 && !initial.players.iter().any(|(id, _)| *id == probe.0) {
        use sqlx::Connection as _;
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        let ids: Vec<Vec<u8>> = initial.players.iter().map(|(id, _)| id.to_vec()).collect();
        let active: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(character_id) FROM game_character_roots WHERE world_id=encode($1,'hex')::uuid AND lifecycle=1 AND uuid_send(character_id)=ANY($2::bytea[]) ORDER BY character_id LIMIT 1")
            .bind(clients.world.as_slice()).bind(&ids).fetch_one(&mut connection).await?;
        probe = initial
            .players
            .iter()
            .find(|(id, _)| id.as_slice() == active)
            .cloned()
            .ok_or("active whitelist member absent")?;
    }
    let mut baseline = initial.clone();
    baseline.players.retain(|(id, _)| *id != probe.0);
    let observed = tokio::time::timeout(
        Duration::from_secs(90),
        observe_settings(&mut target, &initial, &baseline, &probe),
    )
    .await
    .map_err(Into::into)
    .and_then(|result| result);
    let first_restore = tokio::time::timeout(
        Duration::from_secs(10),
        restore_settings(&mut target, &initial),
    )
    .await;
    let (target_session, restored) = match first_restore {
        Ok(Ok(())) => (Some(target), Ok(())),
        failed => {
            evidence(&format!(
                "P17_CHECK step=restore-reconnect observed={failed:?}"
            ));
            drop(target);
            let recovered: TestResult<DevClientSession> = async {
                wait_character_released(clients, 0).await?;
                let mut recovered = connect_character(clients, 0x44, 0).await?;
                restore_settings(&mut recovered, &initial).await?;
                Ok(recovered)
            }
            .await;
            match recovered {
                Ok(recovered) => (Some(recovered), Ok(())),
                Err(error) => {
                    evidence(&format!(
                        "P17_CHECK step=restore-reconnect-failed error={error}"
                    ));
                    (None, Err(error))
                }
            }
        }
    };
    let expected = keep_observation_error(observed, restored)?;
    let mut target = target_session.ok_or("restored Exiva session absent")?;
    capture("p17-expected.txt", expected.as_bytes());
    let _ = target.session_mut().logout().await?;
    Ok(expected.replace('\n', ";").replace(' ', "_"))
}

async fn observe_settings(
    target: &mut DevClientSession,
    initial: &PlayerList,
    baseline: &PlayerList,
    (probe_id, probe_name): &([u8; 16], String),
) -> TestResult<String> {
    replace_members(target, initial, baseline).await?;

    if accepted(target, &[0, 0]).await? != [1] {
        return Err("SetModes(0) lacked the accepted receipt".into());
    }
    let (denied_bytes, denied) = list(target).await?;
    if denied.modes != 0 || denied.players != baseline.players {
        return Err(format!("deny-all List mismatch: {denied:?}").into());
    }
    capture("p17-denied.bin", &denied_bytes);

    let mut add = vec![6];
    add.extend(probe_name.as_bytes());
    if accepted(target, &add).await? != [1] {
        return Err("AddName lacked the accepted receipt".into());
    }
    if accepted(target, &[0, 2]).await? != [1] {
        return Err("SetModes(player whitelist) lacked the accepted receipt".into());
    }
    let (allowed_bytes, allowed) = list(target).await?;
    if allowed.modes != 2
        || !allowed.players.contains(&(*probe_id, probe_name.clone()))
        || allowed
            .players
            .iter()
            .filter(|(id, _)| id != probe_id)
            .ne(baseline.players.iter())
    {
        return Err(format!("allow List mismatch: {allowed:?}").into());
    }
    capture("p17-allowed.bin", &allowed_bytes);

    let (status, refused) = settings(target, &[0, 32]).await?;
    if status != CommandStatus::Rejected || !refused.is_empty() {
        return Err(format!("malformed SetModes: status={status:?} payload={refused:?}").into());
    }
    let (after_bytes, _) = list(target).await?;
    if after_bytes != allowed_bytes {
        return Err("malformed SetModes changed the allow list".into());
    }

    let mut remove = vec![2];
    remove.extend(probe_id);
    if accepted(target, &remove).await? != [1] {
        return Err("RemovePlayer lacked the accepted receipt".into());
    }
    let (removed_bytes, removed) = list(target).await?;
    if removed.modes != 2 || removed.players != baseline.players {
        return Err(format!("removal List mismatch: {removed:?}").into());
    }
    capture("p17-removed.bin", &removed_bytes);
    let id_hex: String = probe_id.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "denied_modes={}\nallowed_modes={}\nplayer={id_hex} {probe_name}\nremoved_modes={}\nrefused_status=Rejected\n",
        denied.modes, allowed.modes, removed.modes,
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(clients)))
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
