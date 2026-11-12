//! I11 full-row player scenario, derived from the matrix/audit and the production craft owner:
//! 1. Admit the provisioned character and walk beside a compiled-world oven.
//! 2. Acquire dough through ordinary play (loot or an NPC purchase), then send native USE-with
//!    dough -> oven; expect an accepted/Committed reply and inventory deltas consuming one dough
//!    and adding one bread.
//! 3. Send native USE-with using an unsupported carried item -> the same oven; expect a rejected
//!    NothingToUse reply and no inventory delta (the refusal path).
//! 4. Logout, freshly admit the same character, and re-check the durable dough/bread quantities.
//!
//! The node-boot provisioning deliberately supplies only normal tools, not recipe inputs. This
//! journey therefore still sends the station-conversion command with a legitimately carried item
//! when no dough can be acquired, records the observed server refusal, and reports the missing
//! ordinary acquisition surface rather than manufacturing a PASS.
use super::*;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent, CAPABILITY_ITEM_USE_V1, COMMAND_TYPE_USE_INTENT,
};

const DOUGH: u32 = 3604;
const BREAD: u32 = 3600;
const OVENS: [&str; 4] = [
    "oteryn:item.tibia.i2535",
    "oteryn:item.tibia.i2537",
    "oteryn:item.tibia.i2539",
    "oteryn:item.tibia.i2541",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(120), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "baked-refused-and-reloaded".to_owned()),
        Ok(Err(error)) if error.to_string().contains("MISSING-SERVER:") => {
            evidence(&format!(
                "I11_GAP {}",
                error.to_string().replace([' ', '\n', '\r'], "_")
            ));
            (
                "MISSING-SERVER",
                "source:crates/gameplay/dispatch/src/gameplay_transport/item_craft.rs:154".to_owned(),
            )
        }
        Ok(Err(error)) => (
            "FAIL",
            error.to_string().replace([' ', '\n', '\r'], "_"),
        ),
        Err(_) => ("FAIL", "step-timeout-observed=no-reply".to_owned()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I11.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut oven = None;
    oteryn_world_bundle::bundle::visit(&bytes, |manifest, sector| {
        for tile in sector.tiles {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if OVENS.contains(&manifest.palette[item.palette as usize].key.as_str()) {
                    oven.get_or_insert((
                        TilePos { x: tile.x, y: tile.y, floor: sector.floor },
                        u8::try_from(ordinal).map_err(|_| {
                            oteryn_world_bundle::Error::Limit("I11 oven ordinal".into())
                        })?,
                    ));
                }
            }
        }
        Ok(())
    })?;
    let (oven, ordinal) = oven.ok_or("step-1_observed=no-compiled-world-oven")?;
    let mut client = connect_character(clients, 0xb8, 0).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| {
        at.floor == oven.floor && at.x.abs_diff(oven.x).max(at.y.abs_diff(oven.y)) <= 1
    })
    .ok_or("step-1_observed=no-walk-route-to-oven")?;
    walk(&mut client, world, &path).await?;

    let index = provisioning::item_index()?;
    let dough = provisioning::item_reference(&index, DOUGH)?;
    let bread = provisioning::item_reference(&index, BREAD)?;
    let before_dough = quantity(&client, dough)?;
    let before_bread = quantity(&client, bread)?;
    let source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == dough)
                .or_else(|| inventory.entries.first())
        })
        .ok_or("step-2_observed=no-carried-item-for-real-use")?
        .handle;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(oven, ordinal)
            .ok_or("step-2_observed=oven-placement-unavailable")?
            .to_be_bytes(),
    );
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(|error| format!("step-2_observed=encode-{error:?}"))?;
    client.take_events();
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("step-2_observed=reply-decode-{error:?}"))?;

    if before_dough == 0 {
        if reply.status != CommandStatus::Accepted || disposition != UseDisposition::NothingToUse {
            return Err(format!(
                "step-2_observed=status-{:?}-disposition-{disposition:?}-need=ordinary-dough-acquisition",
                reply.status
            )
            .into());
        }
        let snapshot = client.inventory().cloned();
        client.service_liveness(Duration::from_millis(100)).await?;
        if client.inventory() != snapshot.as_ref() {
            return Err("step-3_observed=unsupported-use-mutated-inventory".into());
        }
        logout(&mut client).await?;
        let reloaded = connect_character(clients, 0xb9, 0).await?;
        if quantity(&reloaded, dough)? != 0 || quantity(&reloaded, bread)? != before_bread {
            return Err("step-4_observed=refused-use-changed-durable-quantities".into());
        }
        return Err("MISSING-SERVER:step-2_observed=NothingToUse_no_legitimate_dough_source_need=server-player-acquisition_path".into());
    }
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!(
            "step-2_observed=status-{:?}-disposition-{disposition:?}",
            reply.status
        )
        .into());
    }
    wait_quantities(&mut client, dough, before_dough - 1, bread, before_bread + 1).await?;
    let refusal_source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref != bread)
        })
        .ok_or("step-3_observed=no-unsupported-carried-item")?
        .handle;
    let mut refusal_placement = world.base().digest().to_vec();
    refusal_placement.extend_from_slice(
        &crate::map::view::placement_key(oven, ordinal)
            .ok_or("step-3_observed=oven-placement-unavailable")?
            .to_be_bytes(),
    );
    let refusal_payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(refusal_source),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement: refusal_placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(|error| format!("step-3_observed=encode-{error:?}"))?;
    let after_bake = client.inventory().cloned();
    let refusal = client
        .session_mut()
        .world_context_use(&refusal_payload)
        .await?;
    let refusal_disposition = decode_use_result(&refusal.payload)
        .map_err(|error| format!("step-3_observed=reply-decode-{error:?}"))?;
    if refusal.status != CommandStatus::Accepted
        || refusal_disposition != UseDisposition::NothingToUse
    {
        return Err(format!(
            "step-3_observed=status-{:?}-disposition-{refusal_disposition:?}",
            refusal.status
        )
        .into());
    }
    client.service_liveness(Duration::from_millis(100)).await?;
    if client.inventory() != after_bake.as_ref() {
        return Err("step-3_observed=unsupported-use-mutated-inventory".into());
    }
    logout(&mut client).await?;
    let reloaded = connect_character(clients, 0xb9, 0).await?;
    if quantity(&reloaded, dough)? != before_dough - 1
        || quantity(&reloaded, bread)? != before_bread + 1
    {
        return Err("step-4_observed=baked-quantities-not-durable".into());
    }
    Ok(())
}

fn quantity(
    client: &oteryn_dev_client::DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("I11 inventory frame absent")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == reference)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn wait_quantities(
    client: &mut oteryn_dev_client::DevClientSession,
    first: std::num::NonZeroU32,
    first_expected: u64,
    second: std::num::NonZeroU32,
    second_expected: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if quantity(client, first)? == first_expected && quantity(client, second)? == second_expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step-2_observed=missing-inventory-delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-4_observed=logout-refused".into());
    }
    Ok(())
}
