//! I11 full-row player scenario, derived from the matrix/audit and the production craft owner:
//! Buy and mill wheat, fill a carried bucket with water, and mix flour into dough.
//! Bake one dough at an authored oven, verify a rope is refused without mutation,
//! and check the exact bake quantities after fresh admission.
use super::*;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};

const DOUGH: u32 = 3604;
const BREAD: u32 = 3600;
const WHEAT: u32 = 3605;
const FLOUR: u32 = 3603;
const BUCKET: u32 = 2874;
const ROPE: u32 = 3003;
const WATER_SOURCES: [&str; 6] = [
    "oteryn:item.tibia.i1922",
    "oteryn:item.tibia.i1923",
    "oteryn:item.tibia.i1924",
    "oteryn:item.tibia.i1925",
    "oteryn:item.tibia.i1926",
    "oteryn:item.tibia.i1927",
];
const MILLSTONES: [&str; 4] = [
    "oteryn:item.tibia.i1943",
    "oteryn:item.tibia.i1944",
    "oteryn:item.tibia.i1945",
    "oteryn:item.tibia.i1946",
];
const OVENS: [&str; 4] = [
    "oteryn:item.tibia.i2535",
    "oteryn:item.tibia.i2537",
    "oteryn:item.tibia.i2539",
    "oteryn:item.tibia.i2541",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "baked-refused-and-reloaded".to_owned()),
        Ok(Err(error)) => (
            "FAIL",
            error.to_string().replace([' ', '\n', '\r'], "_"),
        ),
        Err(_) => ("FAIL", "step-timeout-observed=no-reply".to_owned()),
    };
    if status == "FAIL" {
        evidence(&format!("I11_GAP {detail}"));
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I11.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn nearby_station(
    at: TilePos,
    stations: &std::collections::BTreeMap<TilePos, u8>,
) -> Option<(TilePos, u8)> {
    for x in at.x.saturating_sub(1)..=at.x.saturating_add(1) {
        for y in at.y.saturating_sub(1)..=at.y.saturating_add(1) {
            let candidate = TilePos { x, y, floor: at.floor };
            if let Some(&ordinal) = stations.get(&candidate) {
                return Some((candidate, ordinal));
            }
        }
    }
    None
}

async fn reach_station(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
    here: TilePos,
    keys: &[&str],
    name: &str,
) -> TestResult<(TilePos, u8)> {
    let mut stations = std::collections::BTreeMap::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        if here.floor.abs_diff(sector.floor) > 2 {
            return Ok(());
        }
        for tile in sector.tiles {
            if here.x.abs_diff(tile.x) > 161 || here.y.abs_diff(tile.y) > 161 {
                continue;
            }
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if keys.contains(&manifest.palette[item.palette as usize].key.as_str()) {
                    let at = TilePos { x: tile.x, y: tile.y, floor: sector.floor };
                    let ordinal = u8::try_from(ordinal).map_err(|_| {
                        oteryn_world_bundle::Error::Limit("I11 oven ordinal".into())
                    })?;
                    stations.entry(at).or_insert(ordinal);
                }
            }
        }
        Ok(())
    })?;
    let path = route(world, here, |at| nearby_station(at, &stations).is_some())
        .ok_or_else(|| format!("step={name}-route missing=reachable-compiled-world-station"))?;
    walk(client, world, &path).await?;
    let arrived = tile(client.world_spatial().actor_position)?;
    nearby_station(arrived, &stations).ok_or_else(|| format!("step={name}-route observed=no-station-within-use-distance-after-STEP").into())
}

pub(super) async fn mill_wheat(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
) -> TestResult {
    super::full_row_i14::acquire(client, world, WHEAT, 1).await.map_err(|error|
        verdict::context(format!("step=wheat-purchase observed={error}"), error)
    )?;
    let here = tile(client.world_spatial().actor_position)?;
    let (mill, ordinal) = reach_station(client, world, bytes, here, &MILLSTONES, "millstone").await?;
    let index = provisioning::item_index()?;
    let wheat = provisioning::item_reference(&index, WHEAT)?;
    let flour = provisioning::item_reference(&index, FLOUR)?;
    let before_wheat = quantity(client, wheat)?;
    let before_flour = quantity(client, flour)?;
    let handle = client.inventory().and_then(|inventory| inventory.entries.iter().find(|entry| entry.item_definition_ref == wheat))
        .ok_or("step=millstone observed=purchased-wheat-handle-missing")?.handle;
    let key = crate::map::view::placement_key(mill, ordinal).ok_or("step=millstone observed=placement-key-missing")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget { placement, expected_revision: 0 })),
        use_with_position: None,
    }).map_err(|error| format!("step=millstone encode={error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&reply.payload).map_err(|error| format!("step=millstone decode={error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!("step=millstone status={:?} disposition={disposition:?}", reply.status).into());
    }
    wait_quantities(client, wheat, before_wheat - 1, flour, before_flour + 1).await?;
    evidence("I11_CHECK wheat-to-flour PASS");
    Ok(())
}

fn dough_created(before: u64, after: u64) -> bool {
    before.checked_add(1) == Some(after)
}

fn oven_refusal_source(definition: u32, rope: u32) -> bool {
    definition == rope
}

pub(super) async fn mix_dough(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
    index: &crate::content::item_ref::ItemDefinitionIndex,
    dough: std::num::NonZeroU32,
) -> TestResult {
    let flour = provisioning::item_reference(index, FLOUR)?;
    let bucket = provisioning::item_reference(index, BUCKET)?;
    let carried_bucket = |client: &oteryn_dev_client::DevClientSession, subtype| {
        client.inventory()?.entries.iter()
            .find(|item| item.item_definition_ref == bucket && item.sub_type == subtype)
            .map(|item| item.handle)
    };
    if carried_bucket(client, 1).is_none() {
        if carried_bucket(client, 0).is_none() {
            super::full_row_i14::acquire(client, world, BUCKET, 1).await.map_err(|error|
                verdict::context(format!("step=dough-water-bucket observed={error}"), error)
            )?;
        }
        let here = tile(client.world_spatial().actor_position)?;
        let (water, ordinal) = reach_station(client, world, bytes, here, &WATER_SOURCES, "dough-water").await?;
        let key = crate::map::view::placement_key(water, ordinal)
            .ok_or("step=dough-water missing=placement-key")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let empty = carried_bucket(client, 0).ok_or("step=dough-water missing=empty-bucket")?;
        use_for_mixing(client, empty, MapUseTarget::Object(WorldObjectTarget {
            placement, expected_revision: 0,
        }), "dough-water").await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while carried_bucket(client, 1).is_none() {
            if tokio::time::Instant::now() >= deadline {
                return Err("step=dough-water missing=filled-bucket-inventory-delta".into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    }
    let water = carried_bucket(client, 1).ok_or("step=dough-mix missing=water-bucket")?;
    let source = client.inventory().and_then(|inventory| inventory.entries.iter()
        .find(|item| item.item_definition_ref == flour))
        .ok_or("step=dough-mix missing=milled-flour")?.handle;
    let bucket_units = |client: &oteryn_dev_client::DevClientSession, subtype| -> TestResult<u64> {
        Ok(client.inventory().ok_or("step=dough-mix missing=inventory")?.entries.iter()
            .filter(|item| item.item_definition_ref == bucket && item.sub_type == subtype)
            .map(|item| u64::from(item.count.get()))
            .sum())
    };
    let before_water = bucket_units(client, 1)?;
    let before_empty = bucket_units(client, 0)?;
    let before_flour = quantity(client, flour)?;
    let before_dough = quantity(client, dough)?;
    use_for_mixing(client, source, MapUseTarget::Item(water), "dough-mix").await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if dough_created(before_dough, quantity(client, dough)?)
            && quantity(client, flour)?.checked_add(1) == Some(before_flour)
            && bucket_units(client, 1)?.checked_add(1) == Some(before_water)
            && before_empty.checked_add(1) == Some(bucket_units(client, 0)?)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=dough-mix missing=flour-debit-dough-credit-and-empty-bucket".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    evidence("I11_CHECK flour-and-water-to-dough PASS");
    Ok(())
}

async fn use_for_mixing(
    client: &mut oteryn_dev_client::DevClientSession,
    source: oteryn_protocol_oteryn::item_view::ItemHandle,
    target: MapUseTarget,
    step: &str,
) -> TestResult {
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(target),
        use_with_position: None,
    }).map_err(|error| format!("step={step} encode={error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&reply.payload).map_err(|error| format!("step={step} decode={error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!("step={step} status={:?} disposition={disposition:?}", reply.status).into());
    }
    Ok(())
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
    let mut client = connect_character(clients, 0xb8, 0).await?;
    mill_wheat(&mut client, world, &bytes).await?;
    let index = provisioning::item_index()?;
    let dough = provisioning::item_reference(&index, DOUGH)?;
    let bread = provisioning::item_reference(&index, BREAD)?;
    let rope = provisioning::item_reference(&index, ROPE)?;
    mix_dough(&mut client, world, &bytes, &index, dough).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let (oven, ordinal) = reach_station(&mut client, world, &bytes, here, &OVENS, "oven").await?;
    let before_dough = quantity(&client, dough)?;
    let before_bread = quantity(&client, bread)?;
    let source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == dough)
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
                .find(|entry| oven_refusal_source(entry.item_definition_ref.get(), rope.get()))
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_repair_existing_dough_cannot_prove_mixing() {
        assert!(!dough_created(3, 3));
        assert!(dough_created(3, 4));
    }

    #[test]
    fn review_repair_oven_refusal_selects_only_rope() {
        assert!(!oven_refusal_source(DOUGH, 3003));
        assert!(!oven_refusal_source(6276, 3003));
        assert!(oven_refusal_source(3003, 3003));
    }


    #[test]
    fn oven_planning_uses_diagonal_reach_but_never_another_floor() {
        let at = TilePos { x: 10, y: 10, floor: -7 };
        let adjacent = TilePos { x: 11, y: 9, floor: -7 };
        let below = TilePos { floor: -8, ..at };
        let mut ovens = std::collections::BTreeMap::from([(below, 1)]);
        assert_eq!(nearby_station(at, &ovens), None);
        ovens.insert(adjacent, 2);
        assert_eq!(nearby_station(at, &ovens), Some((adjacent, 2)));
    }
}
