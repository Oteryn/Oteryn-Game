//! E02 client replay capture: the NPC gold-to-platinum exchange changes the backpack coins.
//! Client owner: the CharacterInventory domain in `crates/session` (`Session::inventory`). The
//! inventory views are session-owned and re-encoded with the production encoder (C09/I11 practice).
use super::*;
use oteryn_dev_client::client_runtime::npc_shop as runtime;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::item_view;
use oteryn_protocol_oteryn::npc_service as wire;
use std::collections::BTreeMap;

const GOLD_ID: u32 = 3031;
const PLATINUM_ID: u32 = 3035;
const CRYSTAL_ID: u32 = 3043;
const PROMPT: &str = "How many platinum coins would you like to get?";

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn pairs(client: &DevClientSession) -> String {
    client.inventory().map_or_else(String::new, |inventory| {
        inventory
            .entries
            .iter()
            .map(|item| format!("{}:{};", item.item_definition_ref, item.count))
            .collect()
    })
}

fn encoded(client: &DevClientSession) -> TestResult<Vec<u8>> {
    let inventory = client.inventory().ok_or("inventory absent")?;
    item_view::encode_character_inventory_with_equip_drop(inventory, true)
        .map_err(|error| format!("inventory encode: {error:?}").into())
}

fn coin_refs() -> TestResult<[std::num::NonZeroU32; 3]> {
    let index = provisioning::item_index()?;
    Ok([
        provisioning::item_reference(&index, GOLD_ID)?,
        provisioning::item_reference(&index, PLATINUM_ID)?,
        provisioning::item_reference(&index, CRYSTAL_ID)?,
    ])
}

fn backpack_coins(
    client: &DevClientSession,
    refs: [std::num::NonZeroU32; 3],
) -> TestResult<[u64; 3]> {
    let inventory = client
        .inventory()
        .ok_or("step=inventory observed=snapshot-absent")?;
    let mut counts = [0; 3];
    for entry in &inventory.entries {
        if let Some(index) = refs
            .iter()
            .position(|reference| *reference == entry.item_definition_ref)
        {
            counts[index] += u64::from(entry.count.get());
        }
    }
    Ok(counts)
}

async fn wait_coins(
    client: &mut DevClientSession,
    refs: [std::num::NonZeroU32; 3],
    expected: [u64; 3],
    step: &'static str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let observed = backpack_coins(client, refs)?;
        if observed == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step={step} observed=coin-delta-timeout:{observed:?} expected={expected:?}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn talk(
    client: &mut DevClientSession,
    actor: EntityRef,
    text: &str,
) -> TestResult<wire::NpcConversation> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let disposition = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step=talk-{text} observed=disposition:{disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    wire::decode_npc_conversation(&delta.payload).map_err(|error| {
        format!("step=talk-{text} observed=malformed-conversation:{error:?}").into()
    })
}

fn visible_npc(
    client: &DevClientSession,
    npc: &oteryn_world_bundle::npc::Npc,
) -> Option<EntityRef> {
    client
        .world_entities()?
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Npc
                && npc.placements.iter().any(|placement| {
                    entity.position.x == i32::from(placement.x)
                        && entity.position.y == i32::from(placement.y)
                        && entity.position.floor == -i16::from(placement.floor)
                })
        })
        .map(|entity| entity.entity)
}

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<EntityRef> {
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements.iter().any(|placement| {
            placement.floor == at.floor
                && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
        })
    })
    .ok_or_else(|| format!("step=walk-npc observed=no-route:{}", npc.key))?;
    walk(client, world, &path).await?;
    visible_npc(client, npc)
        .ok_or_else(|| format!("step=walk-npc observed=actor-not-visible:{}", npc.key).into())
}

async fn find_exchange_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<(EntityRef, String)> {
    let mut attempted = Vec::new();
    for npc in &npcs.npcs {
        if route(world, tile(client.world_spatial().actor_position)?, |at| {
            npc.placements.iter().any(|placement| {
                placement.floor == at.floor
                    && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
            })
        })
        .is_none()
        {
            continue;
        }
        let actor = approach(client, world, npc).await?;
        let greeting = talk(client, actor, "hi").await?;
        attempted.push(format!("{}:hi={:?}", npc.key, greeting.lines));
        let reply = talk(client, actor, "change gold").await?;
        attempted.push(format!("{}:change-gold={:?}", npc.key, reply.lines));
        if reply.lines.iter().any(|line| line == PROMPT) {
            return Ok((actor, npc.key.clone()));
        }
    }
    Err(format!("step=find-exchange-npc observed=no-live-prompt attempts={attempted:?}; need=compiled-world-reachable-banker").into())
}

async fn exchange(
    client: &mut DevClientSession,
    actor: EntityRef,
    words: &[&str],
) -> TestResult<wire::NpcConversation> {
    let mut last = None;
    for word in words {
        last = Some(talk(client, actor, word).await?);
    }
    last.ok_or_else(|| "step=exchange observed=empty-command-sequence".into())
}

async fn earn_gold(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    refs: [std::num::NonZeroU32; 3],
) -> TestResult<[u64; 3]> {
    let mut observed = backpack_coins(client, refs)?;
    let mut attempts = Vec::new();
    for npc in &npcs.npcs {
        if observed[0] >= 100
            || route(world, tile(client.world_spatial().actor_position)?, |at| {
                npc.placements.iter().any(|placement| {
                    placement.floor == at.floor
                        && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
                })
            })
            .is_none()
        {
            continue;
        }
        let actor = approach(client, world, npc).await?;
        let greeting = talk(client, actor, "hi").await?;
        let trade = talk(client, actor, "trade").await?;
        attempts.push(format!(
            "{}:hi={:?}:trade={:?}",
            npc.key, greeting.lines, trade.lines
        ));
        let delta = match tokio::time::timeout(
            Duration::from_secs(2),
            client
                .session_mut()
                .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1),
        )
        .await
        {
            Ok(Ok(delta)) => delta,
            _ => continue,
        };
        let window = wire::decode_npc_trade_window(&delta.payload)
            .map_err(|error| format!("step=earn-gold observed=malformed-trade-window:{error:?}"))?;
        let Some((_, revision)) = window.open else {
            continue;
        };
        let carried: BTreeMap<_, _> = client
            .inventory()
            .ok_or("step=earn-gold observed=inventory-absent")?
            .entries
            .iter()
            .map(|entry| (entry.item_definition_ref.get(), entry.count.get()))
            .collect();
        for (index, offer) in window.offers.iter().enumerate() {
            if observed[0] >= 100 {
                break;
            }
            let Some(available) = carried.get(&offer.item_definition_ref).copied() else {
                continue;
            };
            if offer.side != wire::NpcTradeSide::Sell
                || refs.iter().any(|r| r.get() == offer.item_definition_ref)
            {
                continue;
            }
            let needed = u32::try_from(100 - observed[0]).unwrap_or(u32::MAX);
            let quantity = available
                .min(needed.div_ceil(offer.unit_price.max(1)))
                .max(1);
            let disposition = runtime::buy(
                client.session_mut(),
                &wire::NpcTradeIntent {
                    npc_actor: actor,
                    catalogue_revision: revision,
                    offer_index: u32::try_from(index)?,
                    side: offer.side,
                    quantity,
                    expected_unit_price: offer.unit_price,
                },
            )
            .await?;
            let before = observed;
            if disposition != wire::NpcIntentDisposition::Ok {
                attempts.push(format!(
                    "{}:sell-ref={}:qty={quantity}:reply={disposition:?}",
                    npc.key, offer.item_definition_ref
                ));
                continue;
            }
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            loop {
                observed = backpack_coins(client, refs)?;
                if observed[0] > before[0] {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("step=earn-gold observed=sale-accepted-without-gold-delta npc={} ref={} before={before:?} after={observed:?}", npc.key, offer.item_definition_ref).into());
                }
                client.service_liveness(Duration::from_millis(25)).await?;
                client.take_events();
            }
            attempts.push(format!(
                "{}:sold-ref={}:qty={quantity}:price={}:coins={observed:?}",
                npc.key, offer.item_definition_ref, offer.unit_price
            ));
        }
    }
    if observed[0] < 100 {
        return Err(format!("step=legitimate-gold observed=backpack-coins:{observed:?},sale-attempts={attempts:?}; need=reachable-compiled-world-buyer-for-starter-items").into());
    }
    evidence(&format!(
        "E2EF20_ACQUISITION legitimate-player-sales={attempts:?}"
    ));
    Ok(observed)
}

fn exact_delta(before: [u64; 3], after: [u64; 3], delta: [i64; 3]) -> bool {
    before
        .into_iter()
        .zip(after)
        .zip(delta)
        .all(|((before, after), delta)| i128::from(after) - i128::from(before) == i128::from(delta))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(150);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let refs = coin_refs()?;
    let mut client = connect_world(clients, 0x33).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let initial = earn_gold(&mut client, world, &metadata.npcs, refs).await?;
        let (actor, npc_key) = find_exchange_npc(&mut client, world, &metadata.npcs).await?;
        let before_pairs = pairs(&client);
        let before_payload = encoded(&client)?;
        let reply = exchange(&mut client, actor, &["1", "yes"]).await?;
        let platinum = [initial[0] - 100, initial[1] + 1, initial[2]];
        wait_coins(&mut client, refs, platinum, "gold-to-platinum").await?;
        if !exact_delta(initial, platinum, [-100, 1, 0])
            || !reply.lines.iter().any(|line| line == "Here you are.")
        {
            return Err(format!(
                "step=gold-to-platinum observed=reply:{:?},coins:{platinum:?}",
                reply.lines
            )
            .into());
        }
        let after_pairs = pairs(&client);
        capture("e02-before.bin", &before_payload)?;
        capture("e02-after.bin", &encoded(&client)?)?;
        capture(
            "e02-expected.txt",
            format!("before={before_pairs}\nafter={after_pairs}\n").as_bytes(),
        )?;

        Ok(format!(
            "npc={npc_key} before={initial:?} after={platinum:?}"
        ))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E02.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E02.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E02.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_arithmetic_does_not_wrap() {
        assert!(exact_delta([100, 0, 0], [0, 1, 0], [-100, 1, 0]));
        assert!(!exact_delta([0, 0, 0], [u64::MAX, 0, 0], [-1, 0, 0]));
        assert!(!exact_delta([u64::MAX, 0, 0], [0, 0, 0], [1, 0, 0]));
        assert!(exact_delta([u64::MAX, 0, 0], [u64::MAX, 0, 0], [0, 0, 0]));
    }
}
