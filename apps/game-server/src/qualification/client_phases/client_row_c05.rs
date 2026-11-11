//! C05 client replay capture — Exiva named-target permissions. The phase changes the allow modes,
//! adds a named player and captures the server's two List payloads (modes only, then modes plus
//! player); the previous settings are restored before the status line is printed.
use super::*;
use oteryn_protocol_oteryn::exiva;

const NEW_MODES: u8 = 0b1_0101;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(journey(clients)).await;
    match result.unwrap_or_else(|_| Err("C05 client capture timed out".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=C05.client status=PASS evidence=exiva-mode-change-and-player-list",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=C05.client status=FAIL evidence={error}"
        )),
    }
}

async fn settings(
    client: &mut oteryn_dev_client::DevClientSession,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
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

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let mut target = connect_character(clients, 0x5c, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
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
        if player_list.modes != NEW_MODES || *listed_name != own_name {
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

        // Restore the character's previous Exiva settings, then prove the restore took.
        let mut remove = vec![2];
        remove.extend(player_id);
        if settings(&mut target, &remove).await? != [1]
            || settings(&mut target, &[0, initial.modes]).await? != [1]
        {
            return Err("C05 restore lacked accepted receipts".into());
        }
        if list(&mut target).await?.1 != initial {
            return Err("C05 restore did not return the initial Exiva state".into());
        }

        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut target).await;
    logout_result?;
    row_result
}
