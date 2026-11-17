//! I11 full-row player scenario, derived from the matrix/audit and the production craft owner:
//! 1. Admit the provisioned character and walk beside a compiled-world oven.
//! 2. Acquire dough through ordinary play (loot or an NPC purchase), then send native USE-with
//!    dough -> oven; expect an accepted/Committed reply and inventory deltas consuming one dough
//!    and adding one bread.
//! 3. Send native USE-with using an unsupported carried item -> the same oven; expect a rejected
//!    NothingToUse reply and no inventory delta (the refusal path).
//! 4. Logout, freshly admit the same character, and re-check the durable dough/bread quantities.
//!
use super::*;
use oteryn_dev_client::{DevClientSession, ItemHandle};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use std::num::NonZeroU32;

const DOUGH: u32 = 3604;
const BREAD: u32 = 3600;
const FLOUR: u32 = 3603;
pub(super) const VESSEL: u32 = 2874;
pub(super) const OVENS: [&str; 4] = [
    "oteryn:item.tibia.i2535",
    "oteryn:item.tibia.i2537",
    "oteryn:item.tibia.i2539",
    "oteryn:item.tibia.i2541",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = acquisition_verdict(result.map_err(|_| ()));
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I11.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn acquisition_verdict(result: Result<TestResult, ()>) -> (&'static str, String) {
    match result {
        Ok(Ok(())) => ("PASS", "baked-refused-and-reloaded".to_owned()),
        Ok(Err(error)) => ("FAIL", error.to_string().replace([' ', '\n', '\r'], "_")),
        Err(_) => ("FAIL", "step-timeout-observed=no-reply".to_owned()),
    }
}

#[test]
fn unrelated_oven_refusal_does_not_prove_an_acquisition_gap() {
    assert_eq!(
        acquisition_verdict(Ok(Err(
            "MISSING-SERVER:step-2_observed=NothingToUse_no_legitimate_dough_source".into()
        )))
        .0,
        "FAIL"
    );
    assert_eq!(acquisition_verdict(Ok(Ok(()))).0, "PASS");
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
    let index = provisioning::item_index()?;
    let dough = provisioning::item_reference(&index, DOUGH)?;
    let bread = provisioning::item_reference(&index, BREAD)?;
    if quantity(&client, dough)? == 0 {
        acquire_dough(&mut client, world, &bytes, &index, dough).await?;
    }
    let (oven, _) = walk_to_map_item(&mut client, world, &bytes, "oven", |key| {
        OVENS.contains(&key)
    })
    .await?;
    let before_dough = quantity(&client, dough)?;
    let before_bread = quantity(&client, bread)?;
    let source = carried_item(&client, dough)?.handle;
    let disposition = use_ingredient(&mut client, source, None, Some(oven.clone())).await?;
    if disposition != UseDisposition::Committed {
        return Err(format!("step=bake observed={disposition:?} need=Committed").into());
    }
    wait_quantities(
        &mut client,
        dough,
        before_dough - 1,
        bread,
        before_bread + 1,
    )
    .await?;
    let refusal_source = client
        .inventory()
        .and_then(|inventory| {
            inventory.entries.iter().find(|entry| {
                entry.item_definition_ref != bread && entry.item_definition_ref != dough
            })
        })
        .ok_or("step-3_observed=no-unsupported-carried-item")?
        .handle;
    let after_bake = client.inventory().cloned();
    let refusal = use_ingredient(&mut client, refusal_source, None, Some(oven)).await?;
    if refusal != UseDisposition::NothingToUse {
        return Err(format!("step=unsupported-use observed={refusal:?} need=NothingToUse").into());
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

async fn acquire_dough(
    client: &mut DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
    index: &crate::content::item_ref::ItemDefinitionIndex,
    dough: NonZeroU32,
) -> TestResult {
    let flour = provisioning::item_reference(index, FLOUR)?;
    if quantity(client, flour)? == 0 {
        super::full_row_i17c::acquire(client, world, FLOUR, 1).await?;
    }
    let vessel = water_vessel(client, world, bytes, index, false).await?;
    let before = quantity(client, flour)?;
    let source = carried_item(client, flour)?.handle;
    let outcome = use_ingredient(client, source, Some(vessel), None).await?;
    if outcome != UseDisposition::Committed {
        return Err(format!(
            "step=make-dough observed={outcome:?} need=Committed-flour-with-water"
        )
        .into());
    }
    wait_quantities(client, flour, before - 1, dough, 1).await
}

pub(super) fn carried_item(
    client: &DevClientSession,
    reference: NonZeroU32,
) -> TestResult<oteryn_protocol_oteryn::item_view::ItemEntry> {
    client
        .inventory()
        .and_then(|items| {
            items
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference)
        })
        .copied()
        .ok_or_else(|| format!("carried item {reference} absent").into())
}

pub(super) async fn water_vessel(
    client: &mut DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
    index: &crate::content::item_ref::ItemDefinitionIndex,
    require_fill: bool,
) -> TestResult<ItemHandle> {
    let reference = provisioning::item_reference(index, VESSEL)?;
    if quantity(client, reference)? == 0 {
        super::full_row_i17c::acquire(client, world, VESSEL, 1).await?;
    }
    let vessel = carried_item(client, reference)?;
    if !require_fill && vessel.sub_type == 1 {
        return Ok(vessel.handle);
    }
    if vessel.sub_type != 0 {
        return Err(format!(
            "step=fill-water observed=vessel-subtype-{} need=empty-vessel",
            vessel.sub_type
        )
        .into());
    }
    let (source, _) = walk_to_map_item(client, world, bytes, "water-source", |key| {
        key.strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u32>().ok())
            .is_some_and(|id| matches!(id, 137 | 629..=634 | 880..=891 | 1399..=1402
                | 1439..=1440 | 1922..=1931 | 4597..=4602 | 4609..=4614 | 4621..=4652
                | 4809..=4820 | 5076..=5079 | 5739..=5740 | 6389 | 9582 | 21302..=21322 | 31171..=31182))
    }).await?;
    let outcome = use_ingredient(client, vessel.handle, None, Some(source)).await?;
    if outcome != UseDisposition::Committed {
        return Err(format!("step=fill-water observed={outcome:?} need=Committed").into());
    }
    wait_subtype(client, vessel.handle, 1).await?;
    Ok(vessel.handle)
}

pub(super) async fn wait_subtype(
    client: &mut DevClientSession,
    handle: ItemHandle,
    expected: u32,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if client.inventory().is_some_and(|items| {
            items
                .entries
                .iter()
                .any(|item| item.handle == handle && item.sub_type == expected)
        }) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("handle {handle}: missing subtype {expected} state").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

pub(super) async fn walk_to_map_item(
    client: &mut DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
    label: &str,
    matches: impl Fn(&str) -> bool,
) -> TestResult<(WorldObjectTarget, TilePos)> {
    let mut sites = BTreeMap::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if matches(&manifest.palette[item.palette as usize].key) {
                    let at = TilePos {
                        x: tile.x,
                        y: tile.y,
                        floor: sector.floor,
                    };
                    if let Ok(ordinal) = u8::try_from(ordinal) {
                        sites.entry(at).or_insert(ordinal);
                    }
                }
            }
        }
        Ok(())
    })?;
    let reachable = |at: TilePos| {
        (-1_i16..=1)
            .flat_map(|dx| (-1_i16..=1).map(move |dy| (dx, dy)))
            .find_map(|(dx, dy)| {
                let site = TilePos {
                    x: at.x.checked_add_signed(dx)?,
                    y: at.y.checked_add_signed(dy)?,
                    floor: at.floor,
                };
                Some((site, *sites.get(&site)?))
            })
    };
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        reachable(at).is_some()
    })
    .ok_or_else(|| {
        format!(
            "step=walk-to-{label} observed=no-route to any of {} compiled targets",
            sites.len()
        )
    })?;
    walk(client, world, &path).await?;
    let (site, ordinal) = reachable(tile(client.world_spatial().actor_position)?)
        .ok_or("walk did not reach chosen map target")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(site, ordinal)
            .ok_or("map target placement unavailable")?
            .to_be_bytes(),
    );
    let expected_revision = client
        .world_object_overlay()
        .iter()
        .find(|entry| entry.placement == placement)
        .map_or(0, |entry| entry.revision);
    Ok((
        WorldObjectTarget {
            placement,
            expected_revision,
        },
        site,
    ))
}

pub(super) async fn use_ingredient(
    client: &mut DevClientSession,
    source: ItemHandle,
    carried: Option<ItemHandle>,
    map: Option<WorldObjectTarget>,
) -> TestResult<UseDisposition> {
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: map
            .map(MapUseTarget::Object)
            .or_else(|| carried.map(MapUseTarget::Item)),
        use_with_position: None,
    })
    .map_err(|error| format!("item-use encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("item-use status={:?} need=Accepted", reply.status).into());
    }
    let (payload, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("item-use refusal detail: {error:?}"))?;
    decode_use_result(payload).map_err(|error| format!("item-use decode: {error:?}").into())
}

pub(super) fn quantity(
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
