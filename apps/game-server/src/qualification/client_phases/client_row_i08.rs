//! Capture the real inventory payload applied by the native client for SCENARIO_MATRIX I08.
use super::*;
use super::{
    row_connect_character as connect_character,
    row_finish as row_logout,
};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::item_view;

fn visible(inventory: &oteryn_dev_client::CharacterInventory) -> String {
    inventory
        .entries
        .iter()
        .map(|item| format!("{} {}\n", item.item_definition_ref, item.count))
        .collect()
}

fn capture(path: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::write(path.join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>, path: &Path) -> TestResult {
    row_complete(async {
    let mut client = connect_character(clients, 0xc8, 0).await?;
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let bags = oteryn_gameplay_items::reward_bag::reward_bags().ok_or("reward pool absent")?;
        let bag = bags
            .iter()
            .find(|bag| bag.constant == "PRIMAL_BAG" && bag.playable)
            .ok_or("playable Primal Bag absent")?;
        let index = provisioning::item_index()?;
        let bag_ref = index
            .revision(&bag.bag)
            .and_then(|revision| index.definition_ref("Item", &bag.bag, revision))
            .ok_or("Primal Bag definition absent")?;
        let reward_refs = bag
            .rewards
            .iter()
            .map(|reward| {
                index
                    .revision(&reward.item)
                    .and_then(|revision| index.definition_ref("Item", &reward.item, revision))
                    .ok_or_else(|| format!("reward definition absent: {}", reward.item).into())
            })
            .collect::<TestResult<Vec<_>>>()?;
        let before = client
            .inventory()
            .cloned()
            .ok_or("inventory snapshot absent")?;
        let item = before
            .entries
            .iter()
            .find(|item| item.item_definition_ref == bag_ref)
            .copied()
            .ok_or("real player does not carry a Primal Bag")?;
        let before_payload = item_view::encode_character_inventory_with_equip_drop(&before, true)
            .map_err(|error| format!("inventory encode: {error:?}"))?;
        client.take_events();
        let reply = client.use_item(item.handle).await?;
        if reply.status != CommandStatus::Accepted
            || reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        {
            return Err(format!("Primal Bag USE reply={reply:?}").into());
        }
        let after = wait_inventory(&mut client).await?;
        let bag_before = before
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == bag_ref)
            .map(|item| u64::from(item.count.get()))
            .sum::<u64>();
        let bag_after = after
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == bag_ref)
            .map(|item| u64::from(item.count.get()))
            .sum::<u64>();
        let minted = reward_refs.iter().any(|reference| {
            let count = |inventory: &oteryn_dev_client::CharacterInventory| {
                inventory
                    .entries
                    .iter()
                    .filter(|item| item.item_definition_ref == *reference)
                    .map(|item| u64::from(item.count.get()))
                    .sum::<u64>()
            };
            count(&after) == count(&before) + 1
        });
        if bag_before != bag_after + 1 || !minted {
            return Err("inventory payload did not consume one bag and mint a pool reward".into());
        }
        let after_payload = item_view::encode_character_inventory_with_equip_drop(&after, true)
            .map_err(|error| format!("inventory encode: {error:?}"))?;
        capture(path, "i08-before.bin", &before_payload)?;
        capture(path, "i08-after.bin", &after_payload)?;
        capture(path, "i08-expected.txt", visible(&after).as_bytes())?;
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    row_result

    }).await?
}

async fn wait_inventory(
    client: &mut DevClientSession,
) -> TestResult<oteryn_dev_client::CharacterInventory> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event {
                return Ok(delta.value);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("Primal Bag inventory delta timeout".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let started = std::time::Instant::now();
    let result = journey(clients, Path::new(&path)).await;
    evidence(&format!(
        "e2e_all scenario=I08.client status={} seconds={:.3} evidence={}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        result
            .as_ref()
            .map(|_| "real-use-inventory-payload-captured".into())
            .unwrap_or_else(|error| error.to_string())
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inventory_text_keeps_definition_and_count_boundaries() {
        let mut inventory = oteryn_dev_client::CharacterInventory::default();
        assert_eq!(visible(&inventory), "");
        inventory.entries.push(oteryn_dev_client::ItemEntry {
            handle: std::num::NonZeroU64::MIN,
            item_definition_ref: std::num::NonZeroU32::MAX,
            count: std::num::NonZeroU32::MAX,
            sub_type: 0,
        });
        assert_eq!(visible(&inventory), "4294967295 4294967295\n");
    }
}
