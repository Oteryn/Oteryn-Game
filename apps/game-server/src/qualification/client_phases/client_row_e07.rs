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

async fn walk_once(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("E07 planned step has no destination")?;
        let result = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if !valid_step(
            result.disposition,
            result.world_spatial_delta.is_some(),
            expected == actual,
        ) {
            let occupants: Vec<_> = client
                .world_entities()
                .into_iter()
                .flat_map(|entities| entities.iter())
                .filter(|entity| tile(entity.position).ok() == Some(expected))
                .map(|entity| entity.entity)
                .collect();
            return Err(format!("E07 step direction={direction:?} before={before:?} expected={expected:?} actual={actual:?} occupants={occupants:?} reply={result:?}").into());
        }
        verdict::Harness::global().progress("walk");
        // Pace the next first attempt using the server's committed movement duration.
        client
            .service_liveness(Duration::from_millis(u64::from(result.step_wait_ms)))
            .await?;
        client.take_events();
    }
    Ok(())
}

fn valid_step(disposition: StepDisposition, has_delta: bool, reached_destination: bool) -> bool {
    disposition == StepDisposition::Moved && has_delta && reached_destination
}

fn exact_page(reply: &stash::StashReply, expected: Option<(&str, &str, u32)>) -> TestResult {
    let observed: Vec<_> = reply
        .rows
        .iter()
        .map(|row| {
            (
                row.definition_key.as_str(),
                row.definition_revision.as_str(),
                row.quantity,
            )
        })
        .collect();
    let expected = expected.as_slice();
    if reply.page != 0
        || reply.has_more
        || observed != expected
        || reply.rows.iter().any(|row| row.migration_required)
    {
        return Err(format!("E07 stash page expected={expected:?} actual={reply:?}").into());
    }
    Ok(())
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
    walk_once(&mut client, world, &path).await?;
    let mut panel = ui::StashPanel {
        selected: true,
        ..Default::default()
    };
    let (opened, open_bytes, open_sequence) =
        send(&mut client, &mut panel, ui::Intent::Refresh).await?;
    if opened.outcome != stash::StashOutcome::Opened {
        return Err(format!("stash open {:?}", opened.outcome).into());
    }
    exact_page(&opened, None)?;
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
    exact_page(&stowed, Some((&key, &revision, 2)))?;
    await_carried_quantity(&mut client, potion, 0).await?;
    let (browsed, browse_bytes, browse_sequence) =
        send(&mut client, &mut panel, ui::Intent::Refresh).await?;
    if browsed.outcome != stash::StashOutcome::Opened {
        return Err(format!("stash browse after stow {:?}", browsed.outcome).into());
    }
    exact_page(&browsed, Some((&key, &revision, 2)))?;
    let intent = ui::visible_withdraw(&panel, 0, 1).ok_or("E07 visible withdrawal unavailable")?;
    let (withdrawn, withdraw_bytes, withdraw_sequence) =
        send(&mut client, &mut panel, intent).await?;
    if withdrawn.outcome != stash::StashOutcome::Committed
        || withdrawn.stowed_quantity != 0
        || panel.pending
        || panel.feedback.is_some()
    {
        return Err(format!(
            "E07 immediate withdrawal reply={withdrawn:?} feedback={:?}",
            panel.feedback
        )
        .into());
    }
    exact_page(&withdrawn, Some((&key, &revision, 1)))?;
    await_carried_quantity(&mut client, potion, 1).await?;
    let (after, after_bytes, after_sequence) =
        send(&mut client, &mut panel, ui::Intent::Refresh).await?;
    if after.outcome != stash::StashOutcome::Opened {
        return Err(format!("stash browse after withdrawal {:?}", after.outcome).into());
    }
    exact_page(&after, Some((&key, &revision, 1)))?;
    let previous_session = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("E07 logout refused: {logout:?}").into());
    }
    let mut fresh = connect_character(clients, 0x3a, 1).await?;
    if fresh.join_snapshot().game_session_id == previous_session {
        return Err("E07 relog reused the old session".into());
    }
    let (persisted, _, _) = send(&mut fresh, &mut panel, ui::Intent::Refresh).await?;
    if carried_quantity(&fresh, potion)? != 1 || persisted.outcome != stash::StashOutcome::Opened {
        return Err("E07 relog did not preserve the withdrawal".into());
    }
    exact_page(&persisted, Some((&key, &revision, 1)))?;
    let session = uuid_text(previous_session.as_bytes());
    let sequences = format!(
        "{open_sequence},{stow_sequence},{browse_sequence},{withdraw_sequence},{after_sequence}"
    );
    let expected = format!(
        "key={key}\nrevision={revision}\nstowed={}\nsession={session}\nsequences={sequences}\nhandle={}\ndefinition_ref={}\n",
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
        "session={session} sequences={sequences} key={key} stowed={} after_stow=2 after_withdraw=1",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e07_first_step_refusals_cannot_count_as_movement() {
        for disposition in [
            StepDisposition::Rejected,
            StepDisposition::Blocked,
            StepDisposition::TooEarly,
        ] {
            assert!(!valid_step(disposition, true, true));
        }
        assert!(!valid_step(StepDisposition::Moved, false, true));
        assert!(!valid_step(StepDisposition::Moved, true, false));
        assert!(valid_step(StepDisposition::Moved, true, true));
    }

    #[test]
    fn e07_stash_rejects_equal_total_replacements_and_extra_rows() -> TestResult {
        let mut reply = stash::StashReply {
            outcome: stash::StashOutcome::Opened,
            stowed_quantity: 0,
            transaction: None,
            page: 0,
            has_more: false,
            rows: vec![],
        };
        exact_page(&reply, None)?;
        let potion = ("potion", "r1", 2);
        reply.rows.push(stash::StashRow {
            handle: [1; 16],
            definition_key: "unrelated".into(),
            definition_revision: "r1".into(),
            quantity: 2,
            migration_required: false,
        });
        assert!(exact_page(&reply, None).is_err());
        assert!(exact_page(&reply, Some(potion)).is_err());
        reply.rows[0].definition_key = "potion".into();
        exact_page(&reply, Some(potion))?;
        reply.has_more = true;
        assert!(exact_page(&reply, Some(potion)).is_err());
        Ok(())
    }
}
