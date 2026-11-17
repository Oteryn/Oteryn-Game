//! W19a client replay capture: the Character carries a blue surprise bag (bought from the
//! Messenger of Santa by the full row), is refused a USE-WITH-position on it, then uses it. The
//! client-visible reward is the character inventory: the before and after views are the
//! session-owned views read through `DevClientSession::inventory`, re-encoded with the production
//! encoder (the C09/I11/I08 practice; `Session` does not retain the wire inventory delta).
//! `crates/session/src/tests/e2e_w19a.rs` replays them through `Session::apply_state_delta`.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{item_view, world_object as use_wire};
use std::collections::BTreeMap;

const BAG_ID: u32 = 6506;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W19A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn units(inventory: &oteryn_dev_client::CharacterInventory) -> BTreeMap<u32, u64> {
    let mut held = BTreeMap::new();
    for item in &inventory.entries {
        *held.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    held
}

fn visible(inventory: &oteryn_dev_client::CharacterInventory) -> String {
    inventory
        .entries
        .iter()
        .map(|item| format!("{} {}\n", item.item_definition_ref, item.count))
        .collect()
}

async fn wait_inventory(
    client: &mut DevClientSession,
    after: u64,
    refusal_baseline: &oteryn_dev_client::CharacterInventory,
) -> TestResult<oteryn_dev_client::CharacterInventory> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event {
                if delta.server_sequence > after {
                    return Ok(delta.value);
                }
                if &delta.value != refusal_baseline {
                    return Err("step=use-with inventory changed before the valid USE reply".into());
                }
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("blue surprise bag inventory delta timeout".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

fn exact_bag_outcome(source_spent: bool, reward: u64, minimum: u32, maximum: u32) -> bool {
    source_spent && (u64::from(minimum)..=u64::from(maximum)).contains(&reward)
}

#[test]
fn surprise_bag_use_requires_exact_source_and_allowed_reward_count() {
    assert!(!exact_bag_outcome(false, 2, 1, 2));
    assert!(!exact_bag_outcome(true, 3, 1, 2));
    assert!(exact_bag_outcome(true, 2, 1, 2));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x8f, 1).await?;
    let index = provisioning::item_index()?;
    let bag_ref = provisioning::item_reference(&index, BAG_ID)?;
    let rules = oteryn_gameplay_items::reward_bag::event_containers::event_container_rules()
        .ok_or("step=rules event outcome catalogue absent")?;
    let rule = rules
        .iter()
        .find(|rule| rule.item_id == BAG_ID)
        .ok_or("step=rules blue surprise bag rule absent")?;
    let rewards = rule
        .rewards
        .iter()
        .map(|reward| {
            let reference = provisioning::item_reference(&index, reward.item_id)?.get();
            Ok((reference, (reward.minimum, reward.maximum)))
        })
        .collect::<TestResult<BTreeMap<u32, (u32, u32)>>>()?;
    let before = client
        .inventory()
        .cloned()
        .ok_or("step=inventory snapshot absent")?;
    let bag = before
        .entries
        .iter()
        .find(|item| item.item_definition_ref == bag_ref)
        .copied()
        .ok_or("step=carry the Character holds no blue surprise bag")?;
    let before_payload = item_view::encode_character_inventory_with_equip_drop(&before, true)
        .map_err(|error| format!("inventory encode: {error:?}"))?;
    let held_before = units(&before);

    // USE-WITH-position is refused without consuming or minting anything.
    let position = client.world_spatial().actor_position;
    let payload = use_wire::encode_item_use_intent(&use_wire::UseIntent {
        target: use_wire::UseTarget::Item(bag.handle),
        use_with: None,
        use_with_map_item: None,
        use_with_position: Some(oteryn_protocol_oteryn::actor_spell::SpellTargetPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        }),
    })
    .map_err(|error| format!("use-with encode: {error:?}"))?;
    client.take_events();
    let refused = client.session_mut().world_context_use(&payload).await?;
    let (detail, _) =
        oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&refused.payload)
            .map_err(|error| format!("step=use-with refusal detail: {error:?}"))?;
    let disposition = use_wire::decode_use_result(detail)
        .map_err(|error| format!("step=use-with result: {error:?}"))?;
    client.service_liveness(Duration::from_millis(250)).await?;
    let changed_on_refusal = client
        .take_events()
        .into_iter()
        .any(|event| matches!(event, SessionEvent::Inventory(delta) if delta.value != before));
    if refused.status != CommandStatus::Accepted
        || disposition != use_wire::UseDisposition::NothingToUse
        || changed_on_refusal
        || client.inventory() != Some(&before)
    {
        return Err(format!(
            "step=use-with status={:?} disposition={disposition:?}",
            refused.status
        )
        .into());
    }

    let reply = client.use_item(bag.handle).await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != use_wire::UseDisposition::Committed
    {
        return Err(format!("step=use reply={reply:?}").into());
    }
    let after = wait_inventory(&mut client, reply.result_server_sequence, &before).await?;
    let held_after = units(&after);
    let changed: Vec<u32> = held_before
        .keys()
        .chain(held_after.keys())
        .copied()
        .filter(|key| held_before.get(key) != held_after.get(key))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let bag_count_after = after
        .entries
        .iter()
        .find(|item| item.handle == bag.handle)
        .map_or(0, |item| item.count.get());
    let spent = bag.count.get().checked_sub(1) == Some(bag_count_after)
        && held_before
            .get(&bag_ref.get())
            .copied()
            .unwrap_or(0)
            .checked_sub(1)
            == Some(held_after.get(&bag_ref.get()).copied().unwrap_or(0));
    let minted: Vec<u32> = changed
        .iter()
        .copied()
        .filter(|key| *key != bag_ref.get())
        .collect();
    let [reward] = minted.as_slice() else {
        return Err(format!("step=outcome expected one reward definition: {minted:?}").into());
    };
    let delta = held_after
        .get(reward)
        .copied()
        .unwrap_or(0)
        .checked_sub(held_before.get(reward).copied().unwrap_or(0));
    if !rewards.get(reward).is_some_and(|&(min, max)| {
        delta.is_some_and(|count| exact_bag_outcome(spent, count, min, max))
    }) {
        return Err(format!("step=outcome before={held_before:?} after={held_after:?}").into());
    }
    let after_payload = item_view::encode_character_inventory_with_equip_drop(&after, true)
        .map_err(|error| format!("inventory encode: {error:?}"))?;
    capture("w19a-before.bin", &before_payload);
    capture("w19a-after.bin", &after_payload);
    capture("w19a-expected.txt", visible(&after).as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!("bag={} reward={}", bag_ref.get(), minted[0]))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W19a.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W19a.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W19a.client status=FAIL evidence=timeout"),
    }
}
