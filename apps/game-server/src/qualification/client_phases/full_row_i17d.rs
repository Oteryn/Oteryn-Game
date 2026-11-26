//! I17d: hotkey equip, jewelry delay and the feared gate.
//! The disposable character receives its amulet before the wire checks.
//! The row requires an authored player-facing fear source to complete.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_view::EquipmentSlot;

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
        "e2e_all scenario=I17d.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quantity(client: &DevClientSession, definition: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == definition)
        .map(|item| u64::from(item.count.get()))
        .sum())
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

fn worn(client: &DevClientSession, definition: std::num::NonZeroU32) -> Option<EquipmentSlot> {
    client
        .inventory()?
        .equipment
        .iter()
        .find(|entry| entry.item.item_definition_ref == definition)
        .map(|entry| entry.slot)
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
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let original = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    wait_character_released(clients, 0).await?;
    let fresh = connect_character(clients, 0xe4, 0).await?;
    if fresh.join_snapshot().game_session_id == original {
        return Err("relog reused the original session".into());
    }
    Ok(fresh)
}

async fn wait_for(client: &mut DevClientSession, window: Duration) -> TestResult<()> {
    let deadline = tokio::time::Instant::now() + window;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use oteryn_dev_client::ItemMoveOutcome as Out;
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_I17D_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let amulet = reference(3572)?;
    let client = connect_character(clients, 0xe4, 0).await?;
    if quantity(&client, amulet)? != 0 || worn(&client, amulet).is_some() {
        return Err("step=prerequisite observed=amulet-already-present".into());
    }
    character_prep::grant_item(&client, 3572, 1).await?;
    let mut client = relog(client, clients).await?;
    if quantity(&client, amulet)? != 1 || worn(&client, amulet).is_some() {
        return Err("step=prerequisite needed=one-carried-amulet-after-relog".into());
    }
    evidence("I17D_CHECK prerequisite PASS fixture=one-carried-amulet-after-relog");

    expect(&mut client, amulet, "equip-amulet", Out::Moved).await?;
    settle(&mut client, amulet, true).await?;
    let slot = worn(&client, amulet);
    if slot != Some(EquipmentSlot::Necklace) || quantity(&client, amulet)? != 0 {
        return Err(format!(
            "step=equip observed=slot:{slot:?};needed=Necklace-and-no-carried-copy"
        )
        .into());
    }
    let refused = hotkey(&mut client, amulet).await?;
    if refused != Out::Rejected
        || worn(&client, amulet) != Some(EquipmentSlot::Necklace)
        || quantity(&client, amulet)? != 0
    {
        return Err(format!(
            "step=immediate-repeat observed={refused:?};needed=Rejected-and-still-worn"
        )
        .into());
    }
    // Crystal 201189f8 configmanager.cpp:377 defaults the necklace clock to 200 ms.
    wait_for(&mut client, Duration::from_millis(200)).await?;
    expect(&mut client, amulet, "after-delay", Out::Moved).await?;
    settle(&mut client, amulet, false).await?;
    let mut client = relog(client, clients).await?;
    if worn(&client, amulet).is_some() || quantity(&client, amulet)? != 1 {
        return Err("step=durable-reload needed=one-carried-amulet-and-empty-necklace-slot".into());
    }
    logout(&mut client).await?;
    wait_character_released(clients, 0).await?;
    evidence("I17D_CHECK jewelry-delay PASS immediate=Rejected recovery=Moved relog=carried");
    Err("jewelry-delay-refusal-and-recovery-proven;needed:feared-condition-player-route".into())
}
