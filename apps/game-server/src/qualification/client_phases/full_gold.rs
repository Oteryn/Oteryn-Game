//! Shared real-play gold for full rows: hunt rats, loot every coin stack, buy from a catalogue
//! seller and equip, with a target the caller computes from what its row actually buys.
//!
//! Every kill and loot move counts as progress of the running row (the 90 s no-progress watchdog),
//! and every step logs observed against needed gold.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::{
    item_view::{self as items, EquipmentSlot},
    npc_service as npc,
};
use std::{collections::BTreeSet, num::NonZeroU32};

const COINS: [(u32, u64); 3] = [(3031, 1), (3035, 100), (3043, 10_000)];
/// Lower bound of gold per rat kill: `oteryn:loot.creature.rat` rolls 1-4 gold at 90.3 %
/// (mean 2.26), so two is what an ordinary hunt reliably yields.
pub(super) const GOLD_PER_KILL: u64 = 2;
/// Walk to the spawn, kill, open the corpse and loot, per rat.
pub(super) const SECONDS_PER_KILL: u64 = 12;
const SETUP_SECONDS: u64 = 60;

/// Honest wall budget for earning `needed` gold from rats: the kills it takes at the yield floor.
pub(super) fn budget(needed: u64) -> Duration {
    Duration::from_secs(
        SETUP_SECONDS.saturating_add(
            needed
                .div_ceil(GOLD_PER_KILL)
                .saturating_mul(SECONDS_PER_KILL),
        ),
    )
}

fn reference(id: u32) -> TestResult<NonZeroU32> {
    provisioning::item_reference(&provisioning::item_index()?, id)
}

/// Gold the character carries, from its coin stacks.
pub(super) fn coin_value(client: &DevClientSession) -> TestResult<u64> {
    let coins: Vec<(NonZeroU32, u64)> = COINS
        .iter()
        .map(|&(id, value)| reference(id).map(|key| (key, value)))
        .collect::<TestResult<_>>()?;
    Ok(client
        .inventory()
        .ok_or("gold-inventory-absent")?
        .entries
        .iter()
        .map(|entry| {
            coins
                .iter()
                .find(|(key, _)| *key == entry.item_definition_ref)
                .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
        })
        .sum())
}

/// Every `SellToPlayer` offer of the compiled trade catalogue for item `id`: (npc key, unit price).
fn offers(id: u32) -> TestResult<Vec<(String, u64)>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let item_key = format!("oteryn:item.tibia.i{id}");
    let mut found = Vec::new();
    for record in catalog["records"]
        .as_array()
        .ok_or("trade-records-absent")?
    {
        let Some(list) = record["declaration"]["offers"].as_array() else {
            continue;
        };
        let Some(seller) = record["declaration"]["identity"]["key"]
            .as_str()
            .and_then(|key| key.strip_prefix("oteryn:service.trade."))
        else {
            continue;
        };
        for offer in list {
            if offer["direction"] == "SellToPlayer" && offer["item"]["key"] == item_key {
                let price = offer["unit_price"].as_u64().ok_or("trade-price-absent")?;
                found.push((format!("oteryn:npc.{seller}"), price));
            }
        }
    }
    if found.is_empty() {
        return Err(format!("trade-catalogue-has-no-seller:item={id}").into());
    }
    Ok(found)
}

/// Gold a row must hold to buy `quantity` of item `id` from any catalogue seller (dearest seller).
pub(super) fn price(id: u32, quantity: u32) -> TestResult<u64> {
    let dearest = offers(id)?
        .into_iter()
        .map(|(_, price)| price)
        .max()
        .ok_or("trade-catalogue-has-no-seller")?;
    dearest
        .checked_mul(u64::from(quantity))
        .ok_or_else(|| "trade-price-overflow".into())
}

/// Move every coin stack still in the open corpse into the backpack.
async fn loot_coins(client: &mut DevClientSession) -> TestResult {
    let coins: Vec<NonZeroU32> = COINS
        .iter()
        .map(|&(id, _)| reference(id))
        .collect::<TestResult<_>>()?;
    for _ in 0..8 {
        let Some(handle) = client.open_container().and_then(|view| {
            view.entries
                .iter()
                .find(|entry| coins.contains(&entry.item_definition_ref))
                .map(|entry| entry.handle)
        }) else {
            return Ok(());
        };
        let before = coin_value(client)?;
        let moved = client
            .move_item(&oteryn_dev_client::ItemMoveIntent {
                source: handle,
                destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
            })
            .await?;
        if moved.outcome != items::ItemMoveOutcome::Moved {
            return Err(format!("gold-loot-move:{moved:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while coin_value(client)? <= before {
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("gold-loot-delta-missing:carried={before}").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
        verdict::Harness::global().progress("gold-loot");
    }
    Ok(())
}

/// Hunt rats and loot their gold until the character carries `needed`; returns the carried gold.
///
/// The deadline is [`budget`] for the gold still missing, never a larger fixed number.
pub(super) async fn earn(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    row: &str,
    needed: u64,
) -> TestResult<u64> {
    let started = tokio::time::Instant::now();
    let start_gold = coin_value(client)?;
    let deadline = started + budget(needed.saturating_sub(start_gold));
    let mut kills = 0u64;
    loop {
        let observed = coin_value(client)?;
        evidence(&format!(
            "{row}_GOLD observed={observed} needed={needed} gained={} kills={kills} elapsed_s={}",
            observed.saturating_sub(start_gold),
            started.elapsed().as_secs()
        ));
        if observed >= needed {
            return Ok(observed);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "{row}: gold-earning-budget observed={observed} needed={needed} kills={kills} budget_s={}",
                budget(needed.saturating_sub(start_gold)).as_secs()
            )
            .into());
        }
        let hunted = async {
            travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
            client_hunt::hunt(client).await?;
            loot_coins(client).await
        }
        .await;
        hunted.map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(format!("{row}: rat ATTACK/loot observed={error}"), error)
        })?;
        kills += 1;
        verdict::Harness::global().progress("gold");
    }
}

fn count(client: &DevClientSession, key: NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("inventory-frame-absent")?
        .entries
        .iter()
        .chain(
            client
                .inventory()
                .into_iter()
                .flat_map(|view| view.equipment.iter().map(|entry| &entry.item)),
        )
        .filter(|entry| entry.item_definition_ref == key)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

fn carried(client: &DevClientSession, key: NonZeroU32) -> TestResult<items::ItemHandle> {
    client
        .inventory()
        .and_then(|view| {
            view.entries
                .iter()
                .find(|entry| entry.item_definition_ref == key)
        })
        .map(|entry| entry.handle)
        .ok_or_else(|| format!("carried-item-absent:{key}").into())
}

/// Walk to the nearest catalogue seller of item `id`, TALK `hi` and `trade`, and BUY `quantity`.
pub(super) async fn buy(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    id: u32,
    quantity: u32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop;
    let key = reference(id)?;
    let sellers: BTreeSet<String> = offers(id)?.into_iter().map(|(seller, _)| seller).collect();
    let here = tile(client.world_spatial().actor_position)?;
    let (seller, path) =
        npcs.npcs
            .iter()
            .filter(|entry| sellers.contains(&entry.key))
            .filter_map(|entry| {
                route(world, here, |at| {
                    entry.placements.iter().any(|p| {
                        p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3
                    })
                })
                .map(|path| (entry, path))
            })
            .min_by_key(|(_, path)| path.len())
            .ok_or_else(|| format!("reachable-seller-absent:item={id}"))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|view| {
            view.iter().find(|entry| {
                entry.kind == EntityKind::Npc
                    && seller.placements.iter().any(|p| {
                        entry.position.x == i32::from(p.x)
                            && entry.position.y == i32::from(p.y)
                            && entry.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or("seller-entity-absent")?;
    for text in ["hi", "trade"] {
        let reply = npc_shop::talk(
            client.session_mut(),
            &npc::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("trade-talk:{text}:reply={reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window =
        npc::decode_npc_trade_window(&delta.payload).map_err(|e| format!("trade-window:{e:?}"))?;
    let (index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| o.side == npc::NpcTradeSide::Buy && o.item_definition_ref == key.get())
        .ok_or("live-offer-absent")?;
    let before = count(client, key)?;
    let gold_before = coin_value(client)?;
    let reply = npc_shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade-window-closed")?.1,
            offer_index: u32::try_from(index)?,
            side: offer.side,
            quantity,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!(
            "trade-buy:item={id}:reply={reply:?}:carried_gold={gold_before}:unit_price={}",
            offer.unit_price
        )
        .into());
    }
    let wanted = before + u64::from(offer.count) * u64::from(quantity);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && count(client, key)? == wanted
            {
                verdict::Harness::global().progress("gold-buy");
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "inventory-delta-timeout:item={key}:observed={}:wanted={wanted}",
                count(client, key)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

/// ITEM_MOVE a carried item into an equipment slot and require the served equipment to show it.
pub(super) async fn equip(
    client: &mut DevClientSession,
    key: NonZeroU32,
    slot: EquipmentSlot,
) -> TestResult {
    let reply = client
        .move_item(&items::ItemMoveIntent {
            source: carried(client, key)?,
            destination: items::ItemMoveDestination::Equipment(slot),
        })
        .await?;
    if reply.outcome != items::ItemMoveOutcome::Moved
        || client.inventory().is_none_or(|view| {
            !view
                .equipment
                .iter()
                .any(|entry| entry.slot == slot && entry.item.item_definition_ref == key)
        })
    {
        return Err(format!("equip:item={key}:slot={slot:?}:reply={reply:?}").into());
    }
    Ok(())
}
