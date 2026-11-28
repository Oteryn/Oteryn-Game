//! C19b: exact Newhaven first-entry inventory and equipment across fresh admission.
use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "newhaven-kit-proven;dawnport-vocation-door=excluded;reason=owner-excluded-dawnport".into()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=C19b.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_C19B_ACCOUNT").await?;
    let scoped = fixture.clients(clients);
    let profile = std::env::var("NODE_BOOT_PROFILE").unwrap_or_else(|_| "sorcerer".into());
    let vocation = if profile == "explorer" { "knight" } else { &profile };
    let index = provisioning::item_index()?;
    // CrystalServer donor: data-global/lib/others/newhaven.lua:3-95,126-129,
    // transcribed in crates/gameplay/items/src/starter_grant.rs:103-149.
    let expected = starter_inventory::newhaven_loadout(vocation, |id| {
        provisioning::item_reference(&index, id)
    })?;
    let mut client = connect_character(&scoped, 0xc9, 0).await?;
    let prior = client.join_snapshot().game_session_id;
    starter_inventory::check_starter(
        client.inventory().ok_or("C19b initial inventory snapshot absent")?, &expected,
    )?;
    release(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&scoped, 0xc9, 0).await?;
    if fresh.join_snapshot().game_session_id == prior {
        return Err("C19b reused the first admission".into());
    }
    starter_inventory::check_starter(
        fresh.inventory().ok_or("C19b fresh inventory snapshot absent")?, &expected,
    )?;
    release(&mut fresh).await
}

async fn release(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let (result, detail, _, _) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C19b logout refused: {detail}").into());
    }
    Ok(())
}
