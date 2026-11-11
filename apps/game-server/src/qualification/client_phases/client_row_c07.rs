//! C07 client replay capture journey, derived from the matrix, the Crystal reaudit, and the
//! `rune_use_bridge`/field owner exercised by `postgres_tests/field_runes.rs`.
//!
//! 1. Admit the node-boot character, walk to a compiled-world rune seller, say
//!    `hi` and `trade`, buy two Fire Field runes (3188), and require the trade
//!    reply plus the pushed inventory delta.
//! 2. While still on a protection-zone tile, send a real rune USE at an adjacent
//!    tile. Require `ProtectionZone`, unchanged vitals, and unchanged rune count.
//! 3. Walk to a reachable non-PZ two-tile corridor and send the same USE at its
//!    adjacent tile. Require `Committed`, a pushed map delta containing field
//!    item 21465, and the exact one-rune inventory debit.
//! 4. Send STEP onto that field. Require `Moved`, the position delta, and a
//!    pushed vitals delta whose health is lower (the field-contact tick).
//! 5. Wait on native liveness for the server-owned decay and require a pushed
//!    map delta removing item 21465; step off and back on, requiring movement
//!    but no further field-contact health loss.
//! 6. Logout, perform a fresh admission, and require a different game session,
//!    the retained position, and the remaining rune count. This re-checks the
//!    durable payment/custody state; the expired field must not reappear.

use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use std::collections::BTreeSet;

const RUNE_ID: u32 = 3188;
const FIELD_ID: u32 = 21465;

pub(super) async fn run_client_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete(client_journey(clients)).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => {
            Err("step=journey observed=timeout-240s need=server-complete-field-lifecycle".into())
        }
    };
    if let Err(error) = &result {
        evidence(&format!("C07_GAP {error}"));
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C07.client status={} seconds={:.3}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        if result.is_ok() {
            ""
        } else {
            " evidence=required-actions-unproven-see-C07_GAP"
        },
    ));
}

async fn client_journey(clients: &SeamClients<'_>) -> TestResult {
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xc7).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let reference = provisioning::item_reference(&provisioning::item_index()?, RUNE_ID)?;
    let field = provisioning::item_reference(&provisioning::item_index()?, FIELD_ID)?;
    let purchase_sequence = acquire(&mut client, world, &metadata.npcs, reference).await?;
    check("acquire", purchase_sequence);

    let pz_path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|t| t.protection_zone())
            && adjacent(world, at).is_some()
    })
    .ok_or("step=protection-zone-refusal observed=no-reachable-pz-corridor need=compiled-pz-route")?;
    walk(&mut client, world, &pz_path).await?;
    let pz_target = adjacent(world, tile(client.world_spatial().actor_position)?)
        .ok_or("step=protection-zone-refusal observed=no-adjacent-tile need=walkable-pz-target")?.1;
    let before_count = quantity(&client, reference)?;
    let before_vitals = *client.actor_vitals().ok_or("step=protection-zone-refusal observed=no-vitals-frame need=actor-vitals")?;
    let (disposition, refusal_sequence) =
        use_rune(&mut client, reference, actor_position(pz_target)).await?;
    if disposition != oteryn_protocol_oteryn::world_object::UseDisposition::ProtectionZone {
        return Err(format!("step=protection-zone-refusal observed={disposition:?} need=ProtectionZone").into());
    }
    later_frame(&mut client).await?;
    if quantity(&client, reference)? != before_count || client.actor_vitals() != Some(&before_vitals) {
        return Err("step=protection-zone-refusal observed=state-changed need=no-payment-or-vitals-delta".into());
    }
    check("pz-refusal", refusal_sequence);

    let site_path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|t| !t.protection_zone())
            && adjacent(world, at).is_some_and(|(_, next)| world.base().tile(next.x, next.y, next.floor).is_some_and(|t| !t.protection_zone()))
    }).ok_or("step=field-cast observed=no-reachable-non-pz-corridor need=compiled-field-site")?;
    walk(&mut client, world, &site_path).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let (direction, target_tile) = adjacent(world, from).ok_or("step=field-cast observed=no-adjacent-target need=walkable-target")?;
    let target = actor_position(target_tile);
    let health_before = client.actor_vitals().ok_or("step=field-step observed=no-vitals need=vitals-frame")?.health;
    client.take_events();
    let (disposition, cast_sequence) = use_rune(&mut client, reference, target).await?;
    if disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("step=field-cast observed={disposition:?} need=Committed-and-field-delta").into());
    }
    let placed = field_frame(&mut client, target, field, true, cast_sequence, Duration::from_secs(5)).await?;
    inventory_count(&mut client, reference, before_count - 1).await?;
    check("attack-rune", placed);

    let step = client.step_retrying(direction).await?;
    if step.disposition != StepDisposition::Moved || tile(client.world_spatial().actor_position)? != target_tile {
        return Err(format!("step=field-step observed={step:?} need=Moved-position-delta").into());
    }
    let damaged = wait_health_below(&mut client, health_before).await?;
    check("field-contact-damage", damaged);
    let expired = field_frame(&mut client, target, field, false, placed, Duration::from_secs(75)).await?;
    check("field-expiry", expired);
    capture_expected(&client, target, field, health_before)?;

    let back = route(world, target_tile, |at| at == from).ok_or("step=post-expiry-step observed=no-return-route need=adjacent-route")?;
    walk(&mut client, world, &back).await?;
    let stable = client.actor_vitals().ok_or("step=post-expiry-step observed=no-vitals need=vitals-frame")?.health;
    walk(&mut client, world, &[direction]).await?;
    client.service_liveness(Duration::from_millis(250)).await?;
    if client.actor_vitals().is_none_or(|v| v.health != stable) {
        return Err("step=post-expiry-step observed=health-changed need=expired-field-no-contact-tick".into());
    }
    check("field-step-expiry", client.last_server_sequence());

    let old_session = client.join_snapshot().game_session_id;
    let retained_position = client.world_spatial().actor_position;
    logout(&mut client).await?; row_needs_logout_0 = false;
    if row_needs_logout_0 { tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??; row_needs_logout_0 = false; }
    let mut fresh = connect_world(clients, 0xc8).await?;
    let mut row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if fresh.join_snapshot().game_session_id == old_session
        || fresh.world_spatial().actor_position != retained_position
        || quantity(&fresh, reference)? != before_count - 1
    {
        return Err("step=fresh-admission observed=session-position-or-rune-mismatch need=durable-payment-custody".into());
    }
    let snapshot = fresh
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("step=fresh-admission observed=no-map-snapshot need=world-map-view")?;
    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|error| format!("C07 fresh map: {error:?}"))?;
    if map.tiles.iter().find(|tile| tile.position == target).is_some_and(|tile| {
        tile.items.iter().any(|item| item.definition == MapDefinition::Item(field))
    }) {
        return Err("step=fresh-admission observed=expired-field-present need=decay-remain-retired".into());
    }
    check("fresh-admission", fresh.last_server_sequence());
    logout(&mut fresh).await?;
    row_needs_logout_1 = false;
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { tokio::time::timeout(Duration::from_secs(10), row_logout(&mut fresh)).await??; }
    row_result

    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

fn adjacent(world: &BundleWorld, at: TilePos) -> Option<(StepDirection, TilePos)> {
    [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ]
    .into_iter()
    .find_map(|direction| {
        destination(world, at, direction)
            .filter(|next| next.floor == at.floor)
            .map(|next| (direction, next))
    })
}
fn actor_position(at: TilePos) -> ActorPosition {
    ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}
fn check(step: &str, sequence: u64) {
    evidence(&format!("C07_CHECK {step} PASS server_sequence={sequence}"));
}
fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("C07 inventory absent")?
        .entries
        .iter()
        .filter(|e| e.item_definition_ref == reference)
        .map(|e| u64::from(e.count.get()))
        .sum())
}
fn handle(
    client: &DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<oteryn_protocol_oteryn::item_view::ItemHandle> {
    client
        .inventory()
        .and_then(|v| {
            v.entries
                .iter()
                .find(|e| e.item_definition_ref == reference)
        })
        .map(|e| e.handle)
        .ok_or_else(|| {
            "step=rune-use observed=no-carried-handle need=purchased-rune-inventory-frame".into()
        })
}

async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    reference: std::num::NonZeroU32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service as wire;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let key = format!("oteryn:item.tibia.i{RUNE_ID}");
    let sellers: BTreeSet<String> = trades["records"]
        .as_array()
        .ok_or("C07 trade records absent")?
        .iter()
        .filter_map(|r| {
            r["declaration"]["offers"]
                .as_array()?
                .iter()
                .any(|o| o["direction"] == "SellToPlayer" && o["item"]["key"] == key)
                .then(|| {
                    r["declaration"]["identity"]["key"]
                        .as_str()?
                        .strip_prefix("oteryn:service.trade.")
                        .map(|n| format!("oteryn:npc.{n}"))
                })?
        })
        .collect();
    let here = tile(client.world_spatial().actor_position)?;
    let (npc, path) = npcs
        .npcs
        .iter()
        .filter(|n| sellers.contains(&n.key))
        .filter_map(|n| {
            route(world, here, |at| {
                n.placements
                    .iter()
                    .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
            })
            .map(|p| (n, p))
        })
        .min_by_key(|(_, p)| p.len())
        .ok_or("step=acquire observed=no-reachable-rune-seller need=compiled-vendor-route")?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|es| {
            es.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        e.position.x == i32::from(p.x)
                            && e.position.y == i32::from(p.y)
                            && e.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|e| e.entity)
        .ok_or("step=acquire observed=vendor-not-visible need=npc-entity-delta")?;
    for text in ["hi", "trade"] {
        let reply = npc_shop::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != wire::NpcIntentDisposition::Ok {
            return Err(format!("step=acquire-{text} observed={reply:?} need=Ok").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|e| format!("C07 trade window: {e:?}"))?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| {
            o.side == wire::NpcTradeSide::Buy && o.item_definition_ref == reference.get()
        })
        .ok_or("step=acquire observed=rune-not-offered need=live-3188-offer")?;
    let before = quantity(client, reference)?;
    let reply = npc_shop::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("C07 trade closed")?.1,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity: 2,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "step=acquire-buy observed={reply:?} need=Ok-and-earned-funds unit_price={}",
            offer.unit_price
        )
        .into());
    }
    inventory_count(client, reference, before + u64::from(offer.count) * 2).await
}

async fn use_rune(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    target: ActorPosition,
) -> TestResult<(oteryn_protocol_oteryn::world_object::UseDisposition, u64)> {
    use oteryn_protocol_oteryn::{actor_spell::SpellTargetPosition, world_object as wire};
    let payload = wire::encode_item_use_intent(&wire::UseIntent {
        target: wire::UseTarget::Item(handle(client, reference)?),
        use_with: None,
        use_with_map_item: None,
        use_with_position: Some(SpellTargetPosition {
            x: target.x,
            y: target.y,
            floor: target.floor,
        }),
    })
    .map_err(|e| format!("C07 rune encode: {e:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (payload, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|e| format!("C07 refusal: {e:?}"))?;
    let disposition =
        wire::decode_use_result(payload).map_err(|e| format!("C07 use result: {e:?}"))?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=rune-use observed=status:{:?}/disposition:{disposition:?} need=Accepted-command-frame", reply.status).into());
    }
    Ok((disposition, reply.server_sequence))
}

async fn inventory_count(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    expected: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && quantity(client, reference)? == expected
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=inventory-delta observed={} need={expected}",
                quantity(client, reference)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}
async fn later_frame(client: &mut DevClientSession) -> TestResult {
    client
        .service_liveness(Duration::from_millis(100))
        .await
        .map_err(Into::into)
}
async fn wait_health_below(client: &mut DevClientSession, before: u32) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health < before
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=field-step observed=health:{} need=health-below:{before}",
                client.actor_vitals().map_or(0, |v| v.health)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}
async fn field_frame(
    client: &mut DevClientSession,
    at: ActorPosition,
    reference: std::num::NonZeroU32,
    present: bool,
    after: u64,
    wait: Duration,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + wait;
    loop {
        for event in client.take_events() {
            if let SessionEvent::WorldMap(delta) = event
                && delta.server_sequence > after
                && let Some(tile) = delta.value.tiles.iter().find(|t| t.position == at)
                && tile
                    .items
                    .iter()
                    .any(|item| item.definition == MapDefinition::Item(reference))
                    == present
            {
                capture_map_delta(&delta, present)?;
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step={} observed=missing-map-delta tile={at:?} need=field-{present}",
                if present {
                    "field-cast"
                } else {
                    "field-expiry"
                }
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn logout(client: &mut RowClient) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    loop {
        let reply = client.row_logout().await?;
        if reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Ok(());
        } else if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=logout observed={:?} need=Accepted", reply.result).into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
}

fn capture_map_delta(
    delta: &oteryn_dev_client::AppliedDelta<oteryn_protocol_oteryn::world_map::WorldMapViewDelta>,
    present: bool,
) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let result = oteryn_protocol_oteryn::world_map::encode_world_map_delta(
        &delta.value.header,
        &delta.value,
    )
    .map_err(|error| format!("C07 map encode: {error:?}"))
    .and_then(|payload| {
        std::fs::write(
            Path::new(&path).join(if present {
                "c07-map-placed.bin"
            } else {
                "c07-map-expired.bin"
            }),
            payload,
        )
        .map_err(|error| error.to_string())
    });
    result?;
    Ok(())
}

fn capture_expected(
    client: &DevClientSession,
    target: ActorPosition,
    field: std::num::NonZeroU32,
    health_before: u32,
) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let result = (|| -> TestResult {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()?;
        let path = Path::new(&path).canonicalize()?;
        if !path.starts_with(root) {
            return Err("E2E_ALL_CAPTURE_DIR must be inside the Game worktree".into());
        }
        let expected = format!(
            "field_ref={} target={} {} {} health_before={} health_after={} field_present={}\n",
            field,
            target.x,
            target.y,
            target.floor,
            health_before,
            client.actor_vitals().map_or(0, |vitals| vitals.health),
            false,
        );
        let vitals = client.actor_vitals().ok_or("C07 capture vitals absent")?;
        std::fs::write(
            path.join("c07-vitals.bin"),
            oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(vitals)
                .map_err(|error| format!("C07 vitals encode: {error:?}"))?,
        )?;
        std::fs::write(path.join("c07-expected.txt"), expected)?;
        Ok(())
    })();
    result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn field_ids_resolve_from_the_production_item_index() -> TestResult {
        let index = provisioning::item_index()?;
        let rune = provisioning::item_reference(&index, RUNE_ID)?;
        let field = provisioning::item_reference(&index, FIELD_ID)?;
        assert_ne!(rune, field);
        Ok(())
    }
}
