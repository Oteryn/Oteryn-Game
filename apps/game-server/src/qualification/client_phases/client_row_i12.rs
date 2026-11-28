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
        .admission_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("no inventory admission snapshot")?
        .clone();
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(pick),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(stone)),
        use_with_position: None,
    })
    .map_err(|error| format!("pick intent: {error:?}"))?;
    client.take_events();
    client.session_mut().take_inventory_frame();
    let session = client.join_snapshot().game_session_id;
    let baseline_sequence = client.session_mut().last_server_sequence();
    let selected = client.session_mut().selected_capabilities().to_vec();
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("pick reply: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!("pick refine returned {:?}/{disposition:?}", reply.status).into());
    }
    evidence(&format!(
        "I12_REFINE_COMMITTED session={} command_id={} result_server_sequence={}",
        uuid_text(session.as_bytes()),
        reply.command_id,
        reply.server_sequence,
    ));
    client.service_liveness(Duration::from_millis(250)).await?;
    let delta = client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::Inventory(delta) => Some(delta),
            _ => None,
        })
        .ok_or("committed pick refine sent no inventory delta")?;
    let frame = client
        .session_mut()
        .take_inventory_frame()
        .ok_or("no received inventory frame")?;
    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
    if envelope.server_sequence() != delta.server_sequence
        || delta.base_revision != snapshot.revision
        || delta.server_sequence <= baseline_sequence
    {
        return Err("inventory frame does not match refining observation".into());
    }
    let observed = super::full_row_w19a::quantities(&client)?;
    // Global: https://tibia.fandom.com/wiki/Shiny_Stone (documented outputs).
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
    capture(
        &snapshot.payload,
        &frame,
        &reply.payload,
        &format!(
            "{} {} {} {} {} {} {}",
            uuid_text(session.as_bytes()),
            baseline_sequence,
            snapshot.revision,
            delta.base_revision,
            delta.new_revision,
            delta.server_sequence,
            selected
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
        &delta,
    )?;
    let replay = std::env::var_os("OTERYN_I12_REPLAY_BIN")
        .ok_or("I12 requires the named replay executable")?;
    let replay_result = std::process::Command::new(replay)
        .args([
            "--exact",
            "tests::e2e_i12::pick_refining_replays_real_inventory_payload",
        ])
        .env("OTERYN_I12_SESSION", uuid_text(session.as_bytes()))
        .output()?;
    let report = String::from_utf8(replay_result.stdout)?;
    evidence(report.trim());
    if !replay_result.status.success()
        || !report
            .contains("test tests::e2e_i12::pick_refining_replays_real_inventory_payload ... ok")
    {
        return Err(format!("I12 named replay failed: {}", replay_result.status).into());
    }
    Ok("committed-inventory-delta-replayed".into())
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
    metadata: &str,
    applied: &oteryn_dev_client::AppliedDelta<oteryn_dev_client::CharacterInventory>,
) -> TestResult {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("I12 requires a fresh capture directory")?;
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
        std::io::Write::write_all(&mut std::fs::File::create_new(root.join(name))?, bytes)?;
    }
    for (name, value) in [
        ("i12-expected.txt", expected),
        ("i12-revisions.txt", metadata.to_owned()),
    ] {
        std::io::Write::write_all(
            &mut std::fs::File::create_new(root.join(name))?,
            value.as_bytes(),
        )?;
    }
    Ok(())
}
