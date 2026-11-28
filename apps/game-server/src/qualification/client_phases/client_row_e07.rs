//! E07 client replay capture: open the Supply Stash at an authored locker, stow a carried stack,
//! browse, withdraw a chosen unit and browse again. Client owner: the Stash panel in
//! `crates/client-ui/src/features/0062-stash`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_ui::stash as ui;
use oteryn_protocol_oteryn::stash;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "E07_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

/// Sends the panel's own command and returns the decoded reply and the raw payload.
async fn send(
    client: &mut DevClientSession,
    panel: &mut ui::StashPanel,
    intent: ui::Intent,
) -> TestResult<(stash::StashReply, Vec<u8>, u64)> {
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
    Ok((reply, result.payload, result.server_sequence))
}

fn total(panel: &ui::StashPanel) -> u64 {
    panel.rows.iter().map(|row| u64::from(row.quantity)).sum()
}

fn carried_quantity(
    client: &DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<u64> {
    let inventory = client.inventory().ok_or("E07 inventory snapshot absent")?;
    Ok(inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == definition)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn await_carried_quantity(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    expected: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let observed = carried_quantity(client, definition)?;
        if observed == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "E07 carried Health Potions expected={expected} observed={observed}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
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
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("no reachable authored depot locker")?;
    walk(&mut client, world, &path).await?;
    let mut panel = ui::StashPanel {
        selected: true,
        ..Default::default()
    };
    let (opened, open_bytes, open_sequence) =
        send(&mut client, &mut panel, ui::Intent::Refresh).await?;
    if opened.outcome != stash::StashOutcome::Opened {
        return Err(format!("stash open {:?}", opened.outcome).into());
    }
    let before = total(&panel);
    // Global Supply Stash explicitly supports Health Potions. Inventory order is
    // unrelated to eligibility and can also put the starter gold stack first.
    let potion = provisioning::item_reference(&definitions, 266)?;
    let item = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == potion && entry.count.get() == 2)
        })
        .copied()
        .ok_or("E07 requires the starter stack of two Health Potions")?;
    let (key, revision) = definitions
        .definition_at(item.item_definition_ref)
        .map(|(key, revision)| (key.to_owned(), revision.to_owned()))
        .ok_or("carried definition unknown")?;
    if panel
        .rows
        .iter()
        .any(|row| row.definition_key == key && row.definition_revision == revision)
    {
        return Err("Health Potion balance already exists before stow".into());
    }
    if carried_quantity(&client, potion)? != 2 {
        return Err("E07 requires exactly two carried Health Potions".into());
    }
    panel.carried = vec![item];
    let (stowed, stow_bytes, stow_sequence) =
        send(&mut client, &mut panel, ui::Intent::Stow(item.handle)).await?;
    if stowed.outcome != stash::StashOutcome::Committed
        || stowed.stowed_quantity != item.count.get()
    {
        return Err(format!("stow reply {stowed:?}").into());
    }
    await_carried_quantity(&mut client, potion, 0).await?;
    let (browsed, browse_bytes, browse_sequence) =
        send(&mut client, &mut panel, ui::Intent::Refresh).await?;
    let after_stow = total(&panel);
    if after_stow != before + u64::from(item.count.get()) {
        return Err("stash total differs from stowed quantity".into());
    }
    if browsed.outcome != stash::StashOutcome::Opened {
        return Err(format!("stash browse after stow {:?}", browsed.outcome).into());
    }
    let row = browsed
        .rows
        .iter()
        .position(|r| {
            r.definition_key == key && r.definition_revision == revision && r.quantity == 2
        })
        .ok_or("stowed definition absent from stash page")?;
    let (withdrawn, withdraw_bytes, withdraw_sequence) = send(
        &mut client,
        &mut panel,
        ui::Intent::Withdraw { row, amount: 1 },
    )
    .await?;
    if withdrawn.outcome != stash::StashOutcome::Committed {
        return Err(format!("withdraw outcome {:?}", withdrawn.outcome).into());
    }
    await_carried_quantity(&mut client, potion, 1).await?;
    let (after, after_bytes, after_sequence) =
        send(&mut client, &mut panel, ui::Intent::Refresh).await?;
    let after_withdraw = total(&panel);
    if after.outcome != stash::StashOutcome::Opened
        || after_withdraw != after_stow - 1
        || !after.rows.iter().any(|r| {
            r.definition_key == key && r.definition_revision == revision && r.quantity == 1
        })
    {
        return Err("one-unit withdrawal changed the wrong quantity".into());
    }
    let previous_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("E07 logout refused".into());
    }
    let mut fresh = connect_character(clients, 0x3a, 1).await?;
    if fresh.join_snapshot().game_session_id == previous_session {
        return Err("E07 relog reused the old session".into());
    }
    let (persisted, _, _) = send(&mut fresh, &mut panel, ui::Intent::Refresh).await?;
    if carried_quantity(&fresh, potion)? != 1
        || persisted.outcome != stash::StashOutcome::Opened
        || total(&panel) != after_withdraw
        || !persisted.rows.iter().any(|r| {
            r.definition_key == key && r.definition_revision == revision && r.quantity == 1
        })
    {
        return Err("E07 relog did not preserve exactly one stashed Health Potion".into());
    }
    let session = uuid_text(previous_session.as_bytes());
    let sequences = format!(
        "{open_sequence},{stow_sequence},{browse_sequence},{withdraw_sequence},{after_sequence}"
    );
    let expected = format!(
        "key={key}\nrevision={revision}\nstowed={}\nopen_total={before}\nafter_stow={after_stow}\nafter_withdraw={after_withdraw}\nsession={session}\nsequences={sequences}\nhandle={}\ndefinition_ref={}\n",
        item.count.get(),
        item.handle,
        item.item_definition_ref
    );
    capture("e07-open.bin", &open_bytes);
    capture("e07-stow.bin", &stow_bytes);
    capture("e07-browse.bin", &browse_bytes);
    capture("e07-withdraw.bin", &withdraw_bytes);
    capture("e07-after.bin", &after_bytes);
    capture("e07-expected.txt", expected.as_bytes());
    Ok(format!(
        "session={session} sequences={sequences} key={key} stowed={} after_stow={after_stow} after_withdraw={after_withdraw}",
        item.count.get()
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
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
