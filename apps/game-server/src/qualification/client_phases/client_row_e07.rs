//! E07 client replay capture: open the Supply Stash at an authored locker, stow a carried stack,
//! browse, withdraw a chosen unit and browse again. Client owner: the Stash panel in
//! `crates/client-ui/src/features/0062-stash`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_ui::stash as ui;
use oteryn_protocol_oteryn::stash;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

/// Sends the panel's own command and returns the decoded reply and the raw payload.
async fn send(
    client: &mut DevClientSession,
    panel: &mut ui::StashPanel,
    intent: ui::Intent,
) -> TestResult<(stash::StashReply, Vec<u8>)> {
    let command = ui::request(panel, intent).ok_or("stash request refused by the panel")?;
    let result = client
        .session_mut()
        .gated_command(
            stash::CAPABILITY_STASH_V1,
            stash::COMMAND_TYPE_STASH_INTENT,
            &command.payload,
        )
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("stash transport status {:?}", result.status).into());
    }
    let reply =
        stash::decode_stash_reply(&result.payload).map_err(|error| format!("decode {error:?}"))?;
    ui::apply_result(
        panel,
        &ui::StashResponse {
            status: result.status,
            reply: reply.clone(),
        },
    );
    Ok((reply, result.payload))
}

fn total(panel: &ui::StashPanel) -> u64 {
    panel.rows.iter().map(|row| u64::from(row.quantity)).sum()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let definitions = provisioning::item_index()?;
    let mut client = connect_character(clients, 0x39, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("no reachable authored depot locker")?;
    walk(&mut client, world, &path).await?;
    let mut panel = ui::StashPanel {
        selected: true,
        ..Default::default()
    };
    let (opened, open_bytes) = send(&mut client, &mut panel, ui::Intent::Refresh).await?;
    if opened.outcome != stash::StashOutcome::Opened {
        return Err(format!("stash open {:?}", opened.outcome).into());
    }
    let before = total(&panel);
    let item = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.count.get() >= 2)
                .or(inventory.entries.first())
        })
        .copied()
        .ok_or("no carried stack to stow")?;
    let (key, revision) = definitions
        .definition_at(item.item_definition_ref)
        .map(|(key, revision)| (key.to_owned(), revision.to_owned()))
        .ok_or("carried definition unknown")?;
    panel.carried = vec![item];
    let (stowed, stow_bytes) = send(&mut client, &mut panel, ui::Intent::Stow(item.handle)).await?;
    if stowed.outcome != stash::StashOutcome::Committed
        || stowed.stowed_quantity != item.count.get()
    {
        return Err(format!("stow reply {stowed:?}").into());
    }
    let (browsed, browse_bytes) = send(&mut client, &mut panel, ui::Intent::Refresh).await?;
    let after_stow = total(&panel);
    if after_stow != before + u64::from(item.count.get()) {
        return Err("stash total differs from stowed quantity".into());
    }
    let row = browsed
        .rows
        .iter()
        .position(|r| r.definition_key == key && r.definition_revision == revision)
        .ok_or("stowed definition absent from stash page")?;
    let (withdrawn, withdraw_bytes) = send(
        &mut client,
        &mut panel,
        ui::Intent::Withdraw { row, amount: 1 },
    )
    .await?;
    if withdrawn.outcome != stash::StashOutcome::Committed {
        return Err(format!("withdraw outcome {:?}", withdrawn.outcome).into());
    }
    let (_, after_bytes) = send(&mut client, &mut panel, ui::Intent::Refresh).await?;
    let after_withdraw = total(&panel);
    if after_withdraw != after_stow - 1 {
        return Err("one-unit withdrawal changed the wrong quantity".into());
    }
    let expected = format!(
        "key={key}\nrevision={revision}\nstowed={}\nopen_total={before}\nafter_stow={after_stow}\nafter_withdraw={after_withdraw}\n",
        item.count.get()
    );
    capture("e07-open.bin", &open_bytes)?;
    capture("e07-stow.bin", &stow_bytes)?;
    capture("e07-browse.bin", &browse_bytes)?;
    capture("e07-withdraw.bin", &withdraw_bytes)?;
    capture("e07-after.bin", &after_bytes)?;
    capture("e07-expected.txt", expected.as_bytes())?;

    Ok(format!(
        "key={key} stowed={} after_stow={after_stow} after_withdraw={after_withdraw}",
        item.count.get()
    ))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E07.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E07.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E07.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stash_totals_use_wide_arithmetic() {
        let mut panel = ui::StashPanel::default();
        assert_eq!(total(&panel), 0);
        let row = stash::StashRow {
            handle: [1; 16],
            definition_key: "item".into(),
            definition_revision: "1".into(),
            quantity: u32::MAX,
            migration_required: false,
        };
        panel.rows = vec![row.clone(), row];
        assert_eq!(total(&panel), 2 * u64::from(u32::MAX));
    }
}
