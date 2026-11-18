//! C05 client replay capture — Exiva named-target permissions. The phase changes the allow modes,
//! adds a named player and captures the server's two List payloads (modes only, then modes plus
//! player); the previous settings are restored before the status line is printed.
use super::*;
use oteryn_protocol_oteryn::exiva;

const NEW_MODES: u8 = 0b1_0101;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(580), verdict::observe(journey(clients))).await;
    match result.unwrap_or_else(|_| Err("C05 client capture timed out".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=C05.client status=PASS evidence=exiva-mode-change-and-player-list",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C05.client status=FAIL evidence={error}"
        )),
    }
}

async fn settings(client: &mut oteryn_dev_client::DevClientSession, payload: &[u8]) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(
            exiva::CAPABILITY_EXIVA_SETTINGS_V1,
            exiva::COMMAND_TYPE_EXIVA_SETTINGS,
            payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("C05 settings {payload:?} refused: {:?}", reply.status).into());
    }
    Ok(reply.payload)
}

async fn list(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<(Vec<u8>, exiva::PlayerList)> {
    let payload = settings(client, &[5]).await?;
    let decoded = exiva::PlayerList::decode(&payload).ok_or("C05 server List undecodable")?;
    Ok((payload, decoded))
}

async fn restore_after(result: TestResult, cleanup: impl Future<Output = TestResult>) -> TestResult {
    let cleanup = cleanup.await;
    if let (Err(_), Err(error)) = (&result, &cleanup) {
        evidence(&format!("C05 cleanup also failed: {error}"));
    }
    result.and(cleanup)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut target = connect_character(clients, 0x5c, 1).await?;
    let own_name = {
        let entities = target
            .world_entities()
            .ok_or("C05 entity snapshot absent")?;
        entities
            .iter()
            .find(|entry| entry.entity.identity == *entities.own_identity())
            .and_then(|entry| entry.display_name)
            .ok_or("C05 own display name absent")?
            .as_str()
            .map(str::to_owned)
            .map_err(|error| format!("C05 own name invalid UTF-8: {error:?}"))?
    };
    let (_, initial) = list(&mut target).await?;
    if initial.modes == NEW_MODES || !initial.players.is_empty() {
        return Err(format!("C05 unexpected initial Exiva state {initial:?}").into());
    }

    let changed = tokio::time::timeout(Duration::from_secs(35), async {
    if settings(&mut target, &[0, NEW_MODES]).await? != [1] {
        return Err("C05 SetModes lacked an accepted receipt".into());
    }
    let (modes_payload, modes_list) = list(&mut target).await?;
    if modes_list.modes != NEW_MODES || !modes_list.players.is_empty() {
        return Err(format!("C05 mode change not reflected: {modes_list:?}").into());
    }

    let mut add = vec![6];
    add.extend(own_name.as_bytes());
    if settings(&mut target, &add).await? != [1] {
        return Err("C05 AddName lacked an accepted receipt".into());
    }
    let (player_payload, player_list) = list(&mut target).await?;
    let [(player_id, listed_name)] = player_list.players.as_slice() else {
        return Err(format!("C05 AddName did not yield one player: {player_list:?}").into());
    };
    if player_list.modes != NEW_MODES || *listed_name != own_name || *player_id != clients.characters[1] {
        return Err(format!("C05 unexpected List after AddName: {player_list:?}").into());
    }
    let player_id = *player_id;

    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = Path::new(&path);
        let hex: String = player_id.iter().map(|byte| format!("{byte:02x}")).collect();
        std::fs::write(path.join("c05-modes.bin"), &modes_payload)?;
        std::fs::write(path.join("c05-list.bin"), &player_payload)?;
        std::fs::write(
            path.join("c05-expected.txt"),
            format!("modes={}\nplayer={hex} {listed_name}\n", player_list.modes),
        )?;
    }

    Ok(())
    }).await;
    verdict::Harness::global().bounded_wait(
        tokio::time::Instant::now() + Duration::from_secs(360), "exiva-restoration",
    );
    let changed = match changed {
        Ok(result) => result,
        Err(error) => {
            drop(target);
            target = connect_character(clients, 0x5d, 1).await?;
            Err(verdict::context("C05 Exiva mutation deadline".into(), error))
        }
    };
    restore_after(changed, async {
        let restored = restore(&mut target, &initial, clients.characters[1]).await;
        if restored.is_err() {
            drop(target);
            let mut fresh = connect_character(clients, 0x5e, 1).await?;
            restore(&mut fresh, &initial, clients.characters[1]).await?;
            client_row_c08::require_logout(&mut fresh, Duration::from_secs(15)).await
        } else {
            client_row_c08::require_logout(&mut target, Duration::from_secs(15)).await
        }
    }).await
}

async fn restore(
    target: &mut oteryn_dev_client::DevClientSession,
    initial: &exiva::PlayerList,
    player: [u8; 16],
) -> TestResult {
    let modes = settings(target, &[0, initial.modes]).await;
    let (_payload, current) = list(target).await?;
    let removal = if current.players.iter().any(|(id, _)| *id == player) {
        let mut remove = vec![2];
        remove.extend_from_slice(&player);
        Some(settings(target, &remove).await?)
    } else {
        None
    };
    if modes? != [1] || removal.is_some_and(|reply| reply != [1]) {
        return Err("C05 restore lacked accepted receipts".into());
    }
    if list(target).await?.1 != *initial {
        return Err("C05 restore did not return the initial Exiva state".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn exiva_restore_runs_after_action_failure() {
        let called = std::cell::Cell::new(false);
        let result = restore_after(Err("invalid List frame".into()), async {
            called.set(true);
            Ok(())
        }).await;
        assert!(called.get());
        assert!(result.is_err_and(|error| error.to_string() == "invalid List frame"));
    }
}
