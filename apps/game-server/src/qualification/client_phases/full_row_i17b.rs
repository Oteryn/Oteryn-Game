//! I17b: the same hotkey equips the starter leather helmet and toggles it into the main backpack.
//! Check both states over the wire and on fresh admissions to the dedicated character.
use super::*;
use oteryn_dev_client::{DevClientSession, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
use oteryn_protocol_oteryn::item_view::EquipmentSlot;
use std::num::NonZeroU32;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I17b.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn custody(client: &DevClientSession, helmet: NonZeroU32, equipped: bool) -> TestResult<bool> {
    let inventory = client.inventory().ok_or("inventory snapshot absent")?;
    let head = inventory
        .equipment
        .iter()
        .find(|entry| entry.slot == EquipmentSlot::Head);
    let carried: u64 = inventory
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == helmet)
        .map(|item| u64::from(item.count.get()))
        .sum();
    let helmets_worn = inventory
        .equipment
        .iter()
        .filter(|entry| entry.item.item_definition_ref == helmet)
        .count();
    Ok(inventory.main_backpack.is_some()
        && if equipped {
            carried == 0
                && helmets_worn == 1
                && head.is_some_and(|entry| {
                    entry.item.item_definition_ref == helmet && entry.item.count.get() == 1
                })
        } else {
            carried == 1 && helmets_worn == 0 && head.is_none()
        })
}

async fn settle(
    client: &mut DevClientSession,
    helmet: NonZeroU32,
    equipped: bool,
    step: &str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        client.take_events();
        if custody(client, helmet, equipped)? {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step={step} observed=wrong-helmet-custody;needed:equipped={equipped}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn expect(client: &mut DevClientSession, helmet: NonZeroU32, step: &str) -> TestResult {
    client.take_events();
    let outcome = oteryn_dev_client::client_runtime::hotkey_equip::equip_definition(
        client.session_mut(),
        helmet,
    )
    .await?;
    if outcome != ItemMoveOutcome::Moved {
        return Err(format!("step={step} observed={outcome:?};needed=Moved").into());
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

async fn relog(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let session = client.join_snapshot().game_session_id;
    logout(client).await?;
    let fresh = connect_character(clients, 0xe2, 0).await?;
    if fresh.join_snapshot().game_session_id == session {
        return Err("relog reused its original session".into());
    }
    Ok(fresh)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_I17B_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    let index = provisioning::item_index()?;
    let helmet = provisioning::item_reference(&index, 3355)?;
    let mut client = connect_character(&scope, 0xe2, 0).await?;

    // Prepare the starter helmet with an ordinary move before invoking the hotkey.
    if !custody(&client, helmet, true)? {
        return Err(
            "step=starter observed=wrong-helmet-custody;needed:single-worn-starter-helmet".into(),
        );
    }
    let source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .equipment
                .iter()
                .find(|entry| entry.slot == EquipmentSlot::Head)
        })
        .ok_or("starter helmet has no equipped handle")?
        .item
        .handle;
    let moved = client
        .move_item(&ItemMoveIntent {
            source,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=prepare observed={:?};needed=Moved", moved.outcome).into());
    }
    settle(&mut client, helmet, false, "precondition").await?;

    expect(&mut client, helmet, "equip").await?;
    settle(&mut client, helmet, true, "equip").await?;
    expect(&mut client, helmet, "toggle-off").await?;
    settle(&mut client, helmet, false, "toggle-off").await?;
    expect(&mut client, helmet, "equip-again").await?;
    settle(&mut client, helmet, true, "equip-again").await?;

    let mut client = relog(&mut client, &scope).await?;
    if !custody(&client, helmet, true)? {
        return Err("step=fresh-admission observed=worn-state-lost".into());
    }
    expect(&mut client, helmet, "final-toggle-off").await?;
    settle(&mut client, helmet, false, "final-toggle-off").await?;
    let mut client = relog(&mut client, &scope).await?;
    if !custody(&client, helmet, false)? {
        return Err("step=fresh-admission observed=backpack-state-lost".into());
    }
    logout(&mut client).await?;
    Ok("toggle-unequips-to-main-backpack-and-durable".to_owned())
}
