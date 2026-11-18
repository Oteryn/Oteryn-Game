//! I11 client-capture scenario, derived from the matrix/audit and the production craft owner:
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
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::item_view;

const DOUGH: u32 = 3604;
const BREAD: u32 = 3600;
const OVENS: [&str; 4] = [
    "oteryn:item.tibia.i2535",
    "oteryn:item.tibia.i2537",
    "oteryn:item.tibia.i2539",
    "oteryn:item.tibia.i2541",
];

fn baked_refusal_preserved<T: PartialEq>(before: &T, observed: &T, reload: &T) -> bool {
    before == observed && before == reload
}

fn refusal_preserved<T: PartialEq>(before: &T, at_reply: &T, after_wait: &T, reload: &T) -> bool {
    before == at_reply && before == after_wait && before == reload
}

fn bake_verdict(result: TestResult) -> (&'static str, String) {
    match result {
        Ok(()) => ("PASS", "client-inventory-payload-captured".to_owned()),
        Err(error) => ("FAIL", error.to_string().replace([' ', '\n', '\r'], "_")),
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let (status, detail) = bake_verdict(result.unwrap_or_else(|_| Err("step-timeout-observed=no-reply".into())));
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I11.client status={status} seconds={:.3} evidence={detail}",
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
    let mut ovens = Vec::new();
    let mut water_sources = Vec::new();
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |manifest, sector| {
        for tile in sector.tiles {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                let key = &manifest.palette[item.palette as usize].key;
                let positions = if OVENS.contains(&key.as_str()) {
                    &mut ovens
                } else if crate::map::palette_appearance(key).is_some_and(|id| matches!(id,
                    137 | 629..=634 | 880..=891 | 1399..=1402 | 1439..=1440 | 1922..=1931 | 4597..=4602
                    | 4609..=4614 | 4621..=4652 | 4809..=4820 | 5076..=5079 | 5739..=5740 | 6389
                    | 9582 | 21302..=21322 | 31171..=31182
                )) {
                    &mut water_sources
                } else { continue };
                positions.push((TilePos { x: tile.x, y: tile.y, floor: sector.floor },
                    u8::try_from(ordinal).map_err(|_| oteryn_world_bundle::Error::Limit("I11 placement ordinal".into()))?
                ));
            }
        }
        Ok(())
    })?;
    let mut client = connect_character(clients, 0xb8, 0).await?;
    let index = provisioning::item_index()?;
    let dough = provisioning::item_reference(&index, DOUGH)?;
    let bread = provisioning::item_reference(&index, BREAD)?;
    let acquisition = if quantity(&client, dough)? == 0 {
        prepare_dough(&mut client, world, &metadata, water_sources, dough).await
    } else {
        Ok(())
    };
    let here = tile(client.world_spatial().actor_position)?;
    let ((oven, ordinal), path) = first_reachable(ovens, |(oven, _)| route(world, here, |at| {
        at.floor == oven.floor && at.x.abs_diff(oven.x).max(at.y.abs_diff(oven.y)) <= 1
    })).ok_or("step-1_observed=no-walk-route-to-oven")?;
    walk(&mut client, world, &path).await?;

    let before_dough = quantity(&client, dough)?;
    let before_bread = quantity(&client, bread)?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("I11 inventory snapshot absent")?
        .clone();
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
    let initial_inventory = client.inventory().cloned().ok_or("I11 pre-use inventory absent")?;
    client.take_events();
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("step-2_observed=reply-decode-{error:?}"))?;
    capture("i11-result.bin", &reply.payload);

    if before_dough == 0 {
        if reply.status != CommandStatus::Accepted || disposition != UseDisposition::NothingToUse {
            return Err(format!(
                "step-2_observed=status-{:?}-disposition-{disposition:?}-need=ordinary-dough-acquisition",
                reply.status
            )
            .into());
        }
        let at_reply = client.inventory().cloned().ok_or("I11 inventory absent at refusal")?;
        client.service_liveness(Duration::from_millis(100)).await?;
        let after_wait = client.inventory().cloned().ok_or("I11 inventory absent after refusal")?;
        if at_reply != initial_inventory || after_wait != initial_inventory {
            return Err("step-3_observed=unsupported-use-mutated-inventory".into());
        }
        logout(&mut client).await?;
        let reloaded = connect_character(clients, 0xb9, 0).await?;
        let reload = reloaded.inventory().cloned().ok_or("I11 reload inventory absent")?;
        if !refusal_preserved(
            &inventory_without_handles(initial_inventory),
            &inventory_without_handles(at_reply),
            &inventory_without_handles(after_wait),
            &inventory_without_handles(reload),
        ) {
            return Err("step-4_observed=refused-use-changed-durable-inventory".into());
        }
        acquisition?;
        return Err("I11 acquire-dough: no crafted dough in the carried inventory".into());
    }
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!(
            "step-2_observed=status-{:?}-disposition-{disposition:?}",
            reply.status
        )
        .into());
    }
    let delta = wait_quantities(
        &mut client,
        dough,
        before_dough - 1,
        bread,
        before_bread + 1,
    )
    .await?;
    let delta_payload = item_view::encode_character_inventory(&delta.value)
        .map_err(|error| format!("I11 inventory capture encode: {error:?}"))?;
    capture("i11-snapshot.bin", &snapshot.payload);
    capture("i11-inventory.bin", &delta_payload);
    capture(
        "i11-revisions.txt",
        format!(
            "{} {} {} {}",
            snapshot.revision, delta.base_revision, delta.new_revision, delta.server_sequence
        )
        .as_bytes(),
    );
    capture("i11-expected.txt", visible_inventory(&delta.value).as_bytes());
    let refusal_source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref != bread && entry.item_definition_ref != dough)
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
    let at_reply = client.inventory().cloned();
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
    if at_reply != after_bake || client.inventory() != after_bake.as_ref() {
        return Err("step-3_observed=unsupported-use-mutated-inventory".into());
    }
    logout(&mut client).await?;
    let reloaded = connect_character(clients, 0xb9, 0).await?;
    let baseline = after_bake.map(inventory_without_handles).ok_or("I11 baked inventory absent")?;
    let durable = reloaded.inventory().cloned().map(inventory_without_handles).ok_or("I11 reload inventory absent")?;
    let at_reply = at_reply.map(inventory_without_handles).ok_or("I11 reply inventory absent")?;
    if !baked_refusal_preserved(&baseline, &at_reply, &durable) {
        return Err("step-4_observed=unsupported-use-changed-durable-inventory".into());
    }
    if quantity(&reloaded, dough)? != before_dough - 1
        || quantity(&reloaded, bread)? != before_bread + 1
    {
        return Err("step-4_observed=baked-quantities-not-durable".into());
    }
    Ok(())
}

fn inventory_without_handles(mut view: item_view::CharacterInventory) -> item_view::CharacterInventory {
    for entry in view.main_backpack.iter_mut().chain(&mut view.entries).chain(view.equipment.iter_mut().map(|row| &mut row.item)) {
        entry.handle = item_view::ItemHandle::MIN;
    }
    view
}

async fn prepare_dough(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
    water_sources: Vec<(TilePos, u8)>,
    dough: std::num::NonZeroU32,
) -> TestResult {
    let index = provisioning::item_index()?;
    let flour = provisioning::item_reference(&index, 3603)?;
    let flask = provisioning::item_reference(&index, 2873)?;
    for (id, reference) in [(3603, flour), (2873, flask)] {
        if quantity(client, reference)? == 0 {
            let cost = full_gold::price(id, 1)?;
            full_gold::earn(client, world, &metadata.spawns, "I11_CLIENT", cost).await?;
            full_gold::buy(client, world, &metadata.npcs, id, 1).await?;
        }
    }
    let find_flask = |client: &oteryn_dev_client::DevClientSession, subtype| client.inventory()
        .and_then(|view| view.entries.iter().find(|entry| entry.item_definition_ref == flask && entry.sub_type == subtype))
        .map(|entry| entry.handle);
    if find_flask(client, 1).is_none() {
        let empty = find_flask(client, 0).ok_or("I11 no empty flask for water")?;
        let from = tile(client.world_spatial().actor_position)?;
        let ((at, ordinal), path) = first_reachable(water_sources, |(at, _)| route(world, from, |cell| {
            cell.floor == at.floor && cell.x.abs_diff(at.x).max(cell.y.abs_diff(at.y)) <= 1
        })).ok_or("I11 no reachable authored water source")?;
        walk(client, world, &path).await?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&crate::map::view::placement_key(at, ordinal).ok_or("I11 water placement")?.to_be_bytes());
        craft_with(client, empty, MapUseTarget::Object(WorldObjectTarget { placement, expected_revision: 0 })).await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while find_flask(client, 1).is_none() {
            if tokio::time::Instant::now() >= deadline {
                return Err("I11 fill did not publish a water flask".into());
            }
            client.service_liveness(Duration::from_millis(25)).await?;
            client.take_events();
        }
    }
    let water = find_flask(client, 1).ok_or("I11 water flask missing")?;
    let ingredient = client.inventory().and_then(|view| view.entries.iter().find(|item| item.item_definition_ref == flour))
        .ok_or("I11 flour missing")?.handle;
    let flour_before = quantity(client, flour)?;
    client.take_events();
    craft_with(client, ingredient, MapUseTarget::Item(water)).await?;
    wait_quantities(client, flour, flour_before.checked_sub(1).ok_or("I11 empty flour baseline")?, dough, 1).await?;
    Ok(())
}

async fn craft_with(client: &mut oteryn_dev_client::DevClientSession, handle: item_view::ItemHandle, target: MapUseTarget) -> TestResult {
    let payload = encode_item_use_intent(&UseIntent { target: UseTarget::Item(handle),
        use_with: None, use_with_map_item: Some(target), use_with_position: None,
    }).map_err(|error| format!("I11 craft encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&reply.payload).map_err(|error| format!("I11 craft reply: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!("I11 dough preparation refused: {:?}/{disposition:?}", reply.status).into());
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
) -> TestResult<oteryn_dev_client::AppliedDelta<oteryn_dev_client::CharacterInventory>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::Inventory(delta) = event
                && quantity(client, first)? == first_expected
                && quantity(client, second)? == second_expected
            {
                return Ok(delta);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step-2_observed=missing-inventory-delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

fn visible_inventory(inventory: &oteryn_dev_client::CharacterInventory) -> String {
    inventory
        .entries
        .iter()
        .map(|item| format!("{} {}\n", item.item_definition_ref, item.count))
        .collect()
}

fn capture(name: &str, bytes: &[u8]) {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(std::path::Path::new(&path).join(name), bytes) {
        evidence(&format!(
            "I11_CAPTURE_WRITE_ERROR file={name} error={}",
            error.to_string().replace([' ', '\n', '\r'], "_")
        ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_fixture_dough_is_an_acquisition_failure() {
        assert_eq!(bake_verdict(Err("NothingToUse without fixture dough".into())).0, "FAIL");
    }

    #[test]
    fn refusal_checks_inventory_before_command_and_after_reload() {
        assert!(!baked_refusal_preserved(&[1,2,3], &[1,2,3], &[1,2,4]));
        assert!(!refusal_preserved(&[1, 2], &[1, 3], &[1, 3], &[1, 3]));
        assert!(!refusal_preserved(&[1, 2], &[1, 2], &[1, 2], &[1, 3]));
        assert!(refusal_preserved(&[1, 2], &[1, 2], &[1, 2], &[1, 2]));
    }
}
