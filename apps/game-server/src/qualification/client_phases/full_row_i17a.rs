//! I17a: hotkey equip, a worn item by definition id from the backpack and from a loot pouch.
//! Prepares a dedicated character with its starter leather helmet (3355) and a fixture bag (2853),
//! equips the helmet from the backpack (Moved, Head slot, backpack debit), toggles it back,
//! then moves it into the bag and equips from there: with capability 180 (carried selection)
//! the server must equip it; without it the server must refuse `NotSupported` with no change
//! (FAIL `needed:capability-180`). The worn state is re-read after logout and a fresh admission.
use super::*;
use oteryn_dev_client::DevClientSession;
use std::collections::BTreeMap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I17a.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut values = BTreeMap::new();
    for item in &client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
    {
        *values.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(values)
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(*quantities(client)?.get(&reference.get()).unwrap_or(&0))
}

async fn wait_quantity(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    expected: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        // The expected state may already be in the snapshot: `settle` drained its event.
        client.take_events();
        if quantity(client, reference)? == expected {
            return Ok(client.last_server_sequence());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "inventory delta timeout definition={reference} expected={expected} observed={}",
                quantity(client, reference)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

fn reference(id: u32) -> TestResult<std::num::NonZeroU32> {
    let index = provisioning::item_index()?;
    provisioning::item_reference(&index, id)
}

fn worn(client: &DevClientSession, definition: std::num::NonZeroU32) -> Option<String> {
    client
        .inventory()?
        .equipment
        .iter()
        .find(|entry| entry.item.item_definition_ref == definition)
        .map(|entry| format!("{:?}", entry.slot))
}

async fn settle(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    want_worn: bool,
) -> TestResult<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        client.take_events();
        if worn(client, definition).is_some() == want_worn {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=settle observed=worn:{:?};wanted={want_worn}",
                worn(client, definition)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn hotkey(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<oteryn_dev_client::ItemMoveOutcome> {
    client.take_events();
    Ok(
        oteryn_dev_client::client_runtime::hotkey_equip::equip_definition(
            client.session_mut(),
            definition,
        )
        .await?,
    )
}

async fn expect(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    step: &str,
    want: oteryn_dev_client::ItemMoveOutcome,
) -> TestResult<()> {
    let outcome = hotkey(client, definition).await?;
    if outcome != want {
        return Err(format!("step={step} observed={outcome:?};needed={want:?}").into());
    }
    Ok(())
}

async fn relog(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<DevClientSession> {
    logout(client).await?;
    connect_character(clients, tag, 0).await
}

async fn expect_bag_helmet(
    client: &mut DevClientSession,
    bag: oteryn_protocol_oteryn::item_view::ItemHandle,
    helmet: std::num::NonZeroU32,
    expected: bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(view) = client
            .open_container()
            .filter(|view| view.container_handle == Some(bag))
        {
            let matches = if expected {
                matches!(view.entries.as_slice(), [item] if item.item_definition_ref == helmet && item.count.get() == 1)
            } else {
                view.entries.is_empty()
            };
            if matches {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=bag-contents needed=helmet:{expected};observed={:?}",
                client.open_container()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_dev_client::ItemMoveOutcome as Out;
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_I17A_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let helmet = reference(3355)?;
    let bag = reference(2853)?;
    let mut client = connect_character(clients, 0xe1, 0).await?;
    character_prep::grant_item(&client, 2853, 1).await?;
    evidence("row=I17a.full fixture=bag item=2853 count=1");
    let mut client = relog(&mut client, clients, 0xe1).await?;
    let spare = quantity(&client, helmet)?;
    if spare != 0
        || worn(&client, helmet).as_deref() != Some("Head")
        || quantity(&client, bag)? != 1
    {
        return Err(format!("step=precondition observed=helmet-backpack:{spare};needed:one-worn-starter-helmet-and-one-bag").into());
    }
    let source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .equipment
                .iter()
                .find(|entry| entry.item.item_definition_ref == helmet)
        })
        .ok_or("step=prepare observed=no-starter-helmet-handle")?
        .item
        .handle;
    let moved = client
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source,
            destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != Out::Moved {
        return Err(format!(
            "step=prepare-helmet observed={:?};needed=Moved",
            moved.outcome
        )
        .into());
    }
    settle(&mut client, helmet, false).await?;
    wait_quantity(&mut client, helmet, 1).await?;
    expect(&mut client, helmet, "equip-from-backpack", Out::Moved).await?;
    settle(&mut client, helmet, true).await?;
    if quantity(&client, helmet)? != 0 {
        return Err("step=equip-from-backpack observed=item-still-in-backpack".into());
    }
    let head = worn(&client, helmet).ok_or("step=equip observed=not-worn")?;
    if head != "Head" {
        return Err(format!("step=equip observed=slot:{head};needed=Head").into());
    }
    expect(&mut client, helmet, "toggle-back", Out::Moved).await?;
    settle(&mut client, helmet, false).await?;
    if quantity(&client, helmet)? != 1 {
        return Err("step=toggle-back observed=item-not-in-backpack".into());
    }
    let bag_handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == bag)
        })
        .map(|item| item.handle)
        .ok_or("step=bag observed=no-bag-handle")?;
    let used = client.use_item(bag_handle).await?;
    if used.status != oteryn_protocol_oteryn::CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("step=open-bag observed={used:?}").into());
    }
    expect_bag_helmet(&mut client, bag_handle, helmet, false).await?;
    let helmet_handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == helmet)
        })
        .map(|item| item.handle)
        .ok_or("step=pouch observed=no-helmet-handle")?;
    client.take_events();
    let moved = client
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source: helmet_handle,
            destination: oteryn_dev_client::ItemMoveDestination::Container(bag_handle, None),
        })
        .await?;
    if moved.outcome != Out::Moved {
        return Err(format!("step=into-pouch observed={:?};needed=Moved", moved.outcome).into());
    }
    wait_quantity(&mut client, helmet, 0).await?;
    expect_bag_helmet(&mut client, bag_handle, helmet, true).await?;
    let carried = client.selected_capabilities().contains(&180);
    let outcome = hotkey(&mut client, helmet).await?;
    if !carried {
        if outcome != Out::NotSupported
            || worn(&client, helmet).is_some()
            || quantity(&client, helmet)? != 0
        {
            return Err(format!(
                "step=pouch-refusal observed={outcome:?};needed=NotSupported-and-unchanged"
            )
            .into());
        }
        expect_bag_helmet(&mut client, bag_handle, helmet, true).await?;
        logout(&mut client).await?;
        return Err(format!("step=equip-from-pouch observed={outcome:?};unchanged=true;needed:capability-180-carried-selection").into());
    }
    if outcome != Out::Moved {
        return Err(format!("step=equip-from-pouch observed={outcome:?};needed=Moved").into());
    }
    settle(&mut client, helmet, true).await?;
    if worn(&client, helmet).as_deref() != Some("Head") || quantity(&client, helmet)? != 0 {
        return Err("step=equip-from-pouch observed=wrong-helmet-custody".into());
    }
    expect_bag_helmet(&mut client, bag_handle, helmet, false).await?;
    let session = client.join_snapshot().game_session_id;
    let mut client = relog(&mut client, clients, 0xe1).await?;
    if client.join_snapshot().game_session_id == session {
        return Err("step=fresh-admission observed=reused-session".into());
    }
    if worn(&client, helmet).as_deref() != Some("Head") {
        return Err("step=fresh-admission observed=helmet-not-worn".into());
    }
    expect(&mut client, helmet, "restore", Out::Moved).await?;
    settle(&mut client, helmet, false).await?;
    wait_quantity(&mut client, helmet, 1).await?;
    let session = client.join_snapshot().game_session_id;
    let mut client = relog(&mut client, clients, 0xe1).await?;
    if client.join_snapshot().game_session_id == session
        || worn(&client, helmet).is_some()
        || quantity(&client, helmet)? != 1
    {
        return Err("step=restore-relog needed=fresh-session-with-one-helmet-in-backpack".into());
    }
    logout(&mut client).await?;
    Ok("equipped-from-backpack-and-pouch-and-durable".to_owned())
}
