//! Capture the real inventory payload applied by the native client for SCENARIO_MATRIX I13.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::item_view;

fn visible(inventory: &oteryn_dev_client::CharacterInventory) -> String {
    let mut rows = inventory
        .entries
        .iter()
        .map(|item| (item.item_definition_ref.get(), item.count.get()))
        .collect::<Vec<_>>();
    rows.sort_unstable();
    rows.into_iter()
        .map(|(item, count)| format!("{item} {count}\n"))
        .collect()
}

fn capture(path: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::write(path.join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>, path: &Path) -> TestResult {
    let mut client = row_connect_character(clients, 0xcd, 0).await?;
    let bags = oteryn_gameplay_items::reward_bag::reward_bags().ok_or("reward pool absent")?;
    let bag = bags
        .iter()
        .find(|bag| bag.constant == "BAG_YOU_COVET" && bag.playable)
        .ok_or("playable Bag You Covet absent")?;
    let index = provisioning::item_index()?;
    let bag_ref = index
        .revision(&bag.bag)
        .and_then(|revision| index.definition_ref("Item", &bag.bag, revision))
        .ok_or("Bag You Covet definition absent")?;
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
        .ok_or("needed:carried-bag-you-covet-i43895-from-quest-or-loot")?;
    let before_payload = item_view::encode_character_inventory_with_equip_drop(&before, true)
        .map_err(|error| format!("inventory encode: {error:?}"))?;
    client.take_events();
    let reply = client.use_item(item.handle).await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("Bag You Covet USE reply={reply:?}").into());
    }
    let delta = wait_inventory(&mut client).await?;
    let after = &delta.value;
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
        count(after) == count(&before) + 1
    });
    if bag_before != bag_after + 1 || !minted {
        return Err("inventory payload did not consume one bag and mint a pool reward".into());
    }
    let after_payload = item_view::encode_character_inventory_with_equip_drop(after, true)
        .map_err(|error| format!("inventory encode: {error:?}"))?;
    capture(path, "i13-before.bin", &before_payload)?;
    capture(path, "i13-after.bin", &after_payload)?;
    capture(path, "i13-expected.txt", visible(after).as_bytes())?;
    capture(
        path,
        "i13-revisions.txt",
        format!(
            "{} {} {} {}\n",
            delta.base_revision, delta.base_revision, delta.new_revision, delta.server_sequence
        )
        .as_bytes(),
    )?;
    row_finish(&mut client).await?;
    let mut fresh = row_connect_character(clients, 0xce, 0).await?;
    let persisted = fresh
        .inventory()
        .is_some_and(|inventory| visible(inventory) == visible(after));
    row_finish(&mut fresh).await?;
    if !persisted {
        return Err("Bag You Covet reward inventory changed after fresh admission".into());
    }
    Ok(())
}

async fn wait_inventory(
    client: &mut DevClientSession,
) -> TestResult<oteryn_dev_client::AppliedDelta<oteryn_dev_client::CharacterInventory>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event {
                return Ok(delta);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("Bag You Covet inventory delta timeout".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let started = std::time::Instant::now();
    let result =
        row_complete_with_timeout(Duration::from_secs(90), journey(clients, Path::new(&path)))
            .await
            .and_then(|result| result);
    evidence(&format!(
        "e2e_all scenario=I13.client status={} seconds={:.3} evidence={}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        result
            .as_ref()
            .map(|_| "real-use-inventory-payload-captured".into())
            .unwrap_or_else(|error| error.to_string().replace(' ', "_"))
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_row_i13_inventory_text_keeps_definition_and_count_boundaries() {
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
