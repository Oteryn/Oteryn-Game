//! E03 client replay capture: using a coin stack converts it to the next denomination. Client
//! owner: the CharacterInventory domain in `crates/session` (`Session::inventory`). The inventory
//! views are session-owned and re-encoded with the production encoder (C09/I11 practice).
use super::*;
use oteryn_protocol_oteryn::{item_view, world_object::UseDisposition};
use super::client_row_e02::{backpack_coins, coin_refs, earn_gold, exact_delta};


fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("E03_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn pairs(client: &oteryn_dev_client::DevClientSession) -> String {
    client.inventory().map_or_else(String::new, |inventory| {
        inventory
            .entries
            .iter()
            .map(|item| format!("{}:{};", item.item_definition_ref, item.count))
            .collect()
    })
}

fn encoded(client: &oteryn_dev_client::DevClientSession) -> TestResult<Vec<u8>> {
    let inventory = client.inventory().ok_or("inventory absent")?;
    item_view::encode_character_inventory(inventory)
        .map_err(|error| format!("inventory encode: {error:?}").into())
}

fn observed_conversion(
    disposition: UseDisposition,
    before: [u64; 3],
    after: [u64; 3],
    source: usize,
) -> bool {
    let change = match source {
        0 => [-100, 1, 0],
        1 => [0, -100, 1],
        2 => [0, 100, -1],
        _ => return false,
    };
    disposition == UseDisposition::Committed && exact_delta(before, after, change)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let refs = coin_refs()?;
    let mut client = connect_world(clients, 0x34).await?;
    let qualifying_stack = |client: &oteryn_dev_client::DevClientSession| {
        let inventory = client.inventory()?;
        refs.iter().enumerate().find_map(|(denomination, reference)| {
            inventory.entries.iter().find(|item| {
                item.item_definition_ref == *reference && item.count.get() >= 100
            }).map(|item| (denomination, item.handle))
        })
    };
    if qualifying_stack(&client).is_none() {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
        earn_gold(&mut client, world, &metadata.npcs, refs).await?;
    }
    let (source, handle) = qualifying_stack(&client).ok_or("E03 acquisition did not produce a full coin stack")?;
    let before = backpack_coins(&client, refs)?;
    let before_pairs = pairs(&client);
    let before_payload = encoded(&client)?;
    let outcome = client.use_item(handle).await?;
    if outcome.status != CommandStatus::Accepted || outcome.disposition != UseDisposition::Committed {
        return Err(format!("E03 coin conversion refused: {outcome:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !observed_conversion(outcome.disposition, before, backpack_coins(&client, refs)?, source) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("coin use did not produce the exact exchange: before={before:?} after={:?}", backpack_coins(&client, refs)?).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    let after_pairs = pairs(&client);
    capture("e03-before.bin", &before_payload);
    capture("e03-after.bin", &encoded(&client)?);
    capture(
        "e03-expected.txt",
        format!("before={before_pairs}\nafter={after_pairs}\n").as_bytes(),
    );
    client_row_c08::require_logout(&mut client, Duration::from_secs(15)).await?;
    Ok(format!("before={before_pairs} after={after_pairs}"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E03.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E03.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E03.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_requires_committed_use_and_exact_coin_delta() {
        let before = [100, 0, 3];
        assert!(!observed_conversion(UseDisposition::NothingToUse, before, [0, 1, 3], 0));
        assert!(!observed_conversion(UseDisposition::Committed, before, [100, 0, 4], 0));
        assert!(observed_conversion(UseDisposition::Committed, before, [0, 1, 3], 0));
    }
}
