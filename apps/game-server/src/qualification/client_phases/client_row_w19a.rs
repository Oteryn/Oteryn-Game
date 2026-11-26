//! W19a client replay capture: a dedicated Character carries a seeded red Christmas bundle,
//! is refused a USE-WITH-position on it, then uses it. The
//! client-visible reward is the character inventory: the before and after views are the
//! session-owned views read through `DevClientSession::inventory`, re-encoded with the production
//! encoder (the C09/I11/I08 practice; `Session` does not retain the wire inventory delta).
//! `crates/session/src/tests/e2e_w19a.rs` replays them through `Session::apply_state_delta`.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{item_view, world_object as use_wire};
use std::collections::BTreeMap;

use super::full_row_w19a::{BAG_ID, random_outcome_matches};

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    std::fs::File::create_new(root.join(name))?.write_all(bytes)?;
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("W19a.client logout refused".into());
    }
    Ok(())
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
) -> TestResult<oteryn_dev_client::CharacterInventory> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event {
                return Ok(delta.value);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("red Christmas bundle inventory delta timeout".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let capture_root = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("W19a.client requires a fresh capture directory")?;
    let capture_root = Path::new(&capture_root);
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W19A_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    let mut client = connect_character(clients, 0x8f, 0).await?;
    let index = provisioning::item_index()?;
    let bag_ref = provisioning::item_reference(&index, BAG_ID)?;
    let rules = oteryn_gameplay_items::reward_bag::event_containers::event_container_rules()
        .ok_or("step=rules event outcome catalogue absent")?;
    let rule = rules
        .iter()
        .find(|rule| rule.item_id == BAG_ID)
        .ok_or("step=rules red Christmas bundle rule absent")?;
    let rewards = rule
        .rewards
        .iter()
        .map(|reward| {
            provisioning::item_reference(&index, reward.item_id)
                .map(|reference| (reference.get(), (reward.minimum, reward.maximum)))
        })
        .collect::<TestResult<BTreeMap<_, _>>>()?;
    let mut expected = units(client.inventory().ok_or("step=seed inventory absent")?);
    *expected.entry(bag_ref.get()).or_default() += 2;
    character_prep::declare_seeded(
        "W19a.client",
        "two-red-christmas-bundles",
        "client inventory projection",
    );
    character_prep::grant_item(&client, BAG_ID, 2).await?;
    let original_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(clients, 0x8f, 0).await?;
    let before = client
        .inventory()
        .cloned()
        .ok_or("step=inventory snapshot absent")?;
    if client.join_snapshot().game_session_id == original_session || units(&before) != expected {
        return Err("step=seed fresh admission did not hold exactly two additional bundles".into());
    }
    let bag = before
        .entries
        .iter()
        .find(|item| item.item_definition_ref == bag_ref)
        .copied()
        .ok_or("step=carry the Character holds no red Christmas bundle")?;
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
    let refused = client.session_mut().world_context_use(&payload).await?;
    let (detail, _) =
        oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&refused.payload)
            .map_err(|error| format!("step=use-with refusal detail: {error:?}"))?;
    let disposition = use_wire::decode_use_result(detail)
        .map_err(|error| format!("step=use-with result: {error:?}"))?;
    if refused.status != CommandStatus::Accepted
        || disposition != use_wire::UseDisposition::NothingToUse
        || client.inventory().map(units).as_ref() != Some(&held_before)
    {
        return Err(format!(
            "step=use-with status={:?} disposition={disposition:?}",
            refused.status
        )
        .into());
    }

    client.take_events();
    let reply = client.use_item(bag.handle).await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != use_wire::UseDisposition::Committed
    {
        return Err(format!("step=use reply={reply:?}").into());
    }
    let after = wait_inventory(&mut client).await?;
    let held_after = units(&after);
    let mut consumed = held_before.clone();
    let id = bag_ref.get();
    let remaining = consumed.get_mut(&id).ok_or("step=outcome source missing")?;
    *remaining -= 1;
    if *remaining == 0 {
        consumed.remove(&id);
    }
    if !random_outcome_matches(&consumed, &held_after, &rewards, rule.picks) {
        return Err(format!("step=outcome before={held_before:?} after={held_after:?} need={} distinct admitted rewards", rule.picks).into());
    }
    let after_payload = item_view::encode_character_inventory_with_equip_drop(&after, true)
        .map_err(|error| format!("inventory encode: {error:?}"))?;
    let used_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(clients, 0x8f, 0).await?;
    if client.join_snapshot().game_session_id == used_session
        || client.inventory().map(units).as_ref() != Some(&held_after)
    {
        return Err("step=outcome fresh admission lost the checked inventory".into());
    }
    logout(&mut client).await?;

    capture(capture_root, "w19a-before.bin", &before_payload)?;
    capture(capture_root, "w19a-after.bin", &after_payload)?;
    capture(
        capture_root,
        "w19a-expected.txt",
        visible(&after).as_bytes(),
    )?;
    Ok(format!("bundle={} rewards={}", bag_ref.get(), rule.picks))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
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
