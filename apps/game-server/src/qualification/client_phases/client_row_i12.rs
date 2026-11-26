//! I12 client replay capture: use the real pick on a legitimately carried shiny stone and retain
//! the command result plus the inventory snapshot/delta bytes consumed by `oteryn-session`.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::item_view;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

const PICK: u32 = 3456;
const SHINY_STONE: u32 = 10310;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(30), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", error.to_string().replace([' ', '\n', '\r'], "_")),
        Err(_) => ("FAIL", "timeout".to_owned()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I12.client status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_Q_I12_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let mut client = connect_character(clients, 0xc2, 0).await?;
    character_prep::grant_item(&client, PICK, 1).await?;
    character_prep::grant_item(&client, SHINY_STONE, 1).await?;
    relog(&mut client, clients).await?;
    let index = provisioning::item_index()?;
    let pick_ref = provisioning::item_reference(&index, PICK)?;
    let stone_ref = provisioning::item_reference(&index, SHINY_STONE)?;
    let inventory = client.inventory().ok_or("no inventory snapshot")?;
    let pick = inventory
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == pick_ref)
        .ok_or("no legitimately carried pick")?
        .handle;
    let stone = inventory
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == stone_ref)
        .ok_or("no legitimately carried shiny stone")?
        .handle;
    let mut expected = super::full_row_w19a::quantities(&client)?;
    if expected.remove(&stone_ref.get()) != Some(1) || expected.get(&pick_ref.get()) != Some(&1) {
        return Err("I12 requires exactly one carried pick and shiny stone".into());
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("no inventory gated snapshot")?
        .clone();
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(pick),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(stone)),
        use_with_position: None,
    })
    .map_err(|error| format!("pick intent: {error:?}"))?;
    client.take_events();
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("pick reply: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!("pick refine returned {:?}/{disposition:?}", reply.status).into());
    }
    client.service_liveness(Duration::from_millis(250)).await?;
    let delta = client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::Inventory(delta) => Some(delta),
            _ => None,
        })
        .ok_or("committed pick refine sent no inventory delta")?;
    let observed = super::full_row_w19a::quantities(&client)?;
    // Crystal 201189f8, data-global/scripts/lib/register_actions.lua:614-628.
    let mut matches_reward = false;
    for id in [3028, 3031, 3035, 3043] {
        let reward = provisioning::item_reference(&index, id)?.get();
        let mut candidate = expected.clone();
        *candidate.entry(reward).or_default() += 1;
        matches_reward |= candidate == observed;
    }
    if !matches_reward {
        return Err(format!("unexpected refining inventory: {observed:?}").into());
    }
    relog(&mut client, clients).await?;
    if super::full_row_w19a::quantities(&client)? != observed {
        return Err("refining inventory changed after fresh admission".into());
    }
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("refining final logout refused".into());
    }
    let delta_payload = item_view::encode_character_inventory(&delta.value)
        .map_err(|error| format!("inventory capture: {error:?}"))?;
    capture(
        &snapshot.payload,
        &delta_payload,
        &reply.payload,
        snapshot.revision,
        &delta,
    )?;
    Ok("committed-inventory-delta-captured".into())
}

async fn relog(
    client: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult {
    let original = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("refining logout refused".into());
    }
    *client = connect_character(clients, 0xc2, 0).await?;
    if client.join_snapshot().game_session_id == original {
        return Err("refining reused its original session".into());
    }
    Ok(())
}

fn capture(
    snapshot: &[u8],
    delta: &[u8],
    reply: &[u8],
    snapshot_revision: u64,
    applied: &oteryn_dev_client::AppliedDelta<oteryn_dev_client::CharacterInventory>,
) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let root = Path::new(&root);
    let expected = applied
        .value
        .entries
        .iter()
        .map(|entry| format!("{} {}\n", entry.item_definition_ref, entry.count))
        .collect::<String>();
    for (name, bytes) in [
        ("i12-snapshot.bin", snapshot),
        ("i12-inventory.bin", delta),
        ("i12-result.bin", reply),
    ] {
        std::fs::write(root.join(name), bytes)?;
    }
    for (name, value) in [
        ("i12-expected.txt", expected),
        (
            "i12-revisions.txt",
            format!(
                "{} {} {} {}",
                snapshot_revision,
                applied.base_revision,
                applied.new_revision,
                applied.server_sequence
            ),
        ),
    ] {
        std::fs::write(root.join(name), value)?;
    }
    Ok(())
}
