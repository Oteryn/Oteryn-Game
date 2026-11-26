//! C19b: starting items, per-vocation equipment and backpack contents.
//! Admits the provisioned character (vocation `none`) and requires the Crystal vocationless kit
//! in the backpack (rope 3003, shovel 3457) with nothing worn, and the same after a fresh admission.
//! The sorcerer/druid/paladin/knight/monk kits are granted only by a Dawnport vocation door
//! (`starting_loadout::departure`), which no player route from the town start reaches, so the
//! row ends FAIL `needed:vocation-change-route...` after the proven leg.
use super::*;
use oteryn_dev_client::DevClientSession;
use std::collections::BTreeMap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
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

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

const NONE_KIT: [u32; 2] = [3003, 3457];

fn kit_report(client: &DevClientSession) -> TestResult<(Vec<u32>, usize)> {
    let inventory = client
        .inventory()
        .ok_or("step=inventory observed=snapshot-absent")?;
    let held = quantities(client)?;
    let index = provisioning::item_index()?;
    let mut missing = Vec::new();
    for id in NONE_KIT {
        // Held quantities are keyed by the generation's definition reference, not the native id.
        let reference = provisioning::item_reference(&index, id)?;
        if held.get(&reference.get()).copied().unwrap_or(0) == 0 {
            missing.push(id);
        }
    }
    Ok((missing, inventory.equipment.len()))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xc9).await?;
    let (missing, worn) = kit_report(&client)?;
    if !missing.is_empty() || worn != 0 {
        return Err(format!("step=none-kit observed=missing:{missing:?};worn={worn};needed=rope3003-shovel3457-and-no-equipment").into());
    }
    let held = quantities(&client)?;
    let mut client = {
        logout(&mut client).await?;
        connect_world(clients, 0xc9).await?
    };
    if quantities(&client)? != held {
        return Err("step=fresh-admission observed=starter-kit-changed".into());
    }
    logout(&mut client).await?;
    Err("vocationless-kit-proven;needed:vocation-change-route-for-sorcerer-druid-paladin-knight-monk-kits(dawnport_departure_unreachable_from_town)".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_kit_is_rope_and_shovel() {
        assert_eq!(NONE_KIT, [3003, 3457]);
    }
}
