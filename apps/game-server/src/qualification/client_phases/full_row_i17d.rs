//! I17d: hotkey equip, jewelry delay and the feared gate.
//! Buys an amulet (3572), equips it (Moved, Necklace) and immediately repeats the hotkey: the
//! server must refuse `Rejected` during the jewelry delay and keep it worn; once the delay has
//! passed the toggle must succeed. The feared gate needs the feared condition, which no player
//! route in this run applies, so the row ends FAIL `needed:feared-condition` after the proven legs.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use std::collections::{BTreeMap, BTreeSet};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(300), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I17d.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut values = BTreeMap::new();
    for item in &client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
    {
        *values.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(values)
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(*quantities(client)?.get(&reference.get()).unwrap_or(&0))
}

async fn wait_quantity(
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
                "inventory delta timeout definition={reference} expected={expected} observed={}",
                quantity(client, reference)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    id: u32,
    amount: u32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, id)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let key = format!("oteryn:item.tibia.i{id}");
    let sellers: BTreeSet<_> = trades["records"]
        .as_array()
        .ok_or("trade records absent")?
        .iter()
        .filter_map(|record| {
            let declaration = &record["declaration"];
            declaration["offers"]
                .as_array()?
                .iter()
                .any(|offer| offer["direction"] == "SellToPlayer" && offer["item"]["key"] == key)
                .then(|| declaration["identity"]["key"].as_str())
                .flatten()?
                .strip_prefix("oteryn:service.trade.")
                .map(|name| format!("oteryn:npc.{name}"))
        })
        .collect();
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let here = tile(client.world_spatial().actor_position)?;
    let (npc, path) = metadata
        .npcs
        .npcs
        .iter()
        .filter(|npc| sellers.contains(&npc.key))
        .filter_map(|npc| {
            route(world, here, |at| {
                npc.placements
                    .iter()
                    .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
            })
            .map(|path| (npc, path))
        })
        .min_by_key(|(_, path)| path.len())
        .ok_or_else(|| format!("no walk route to seller for item {id}; sellers={sellers:?}"))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entity.position.x == i32::from(p.x)
                            && entity.position.y == i32::from(p.y)
                            && entity.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or("seller not visible after walk")?;
    for text in ["hi", "trade"] {
        let reply = runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != wire::NpcIntentDisposition::Ok {
            return Err(format!("NPC {text} reply={reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("trade window decode: {error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
        })
        .ok_or("live seller omitted requested offer")?;
    let before = quantity(client, reference)?;
    client.take_events();
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: amount,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "buy item={id} reply={reply:?} price={} quantity={amount}",
            offer.unit_price
        )
        .into());
    }
    let expected = before + u64::from(offer.count) * u64::from(amount);
    wait_quantity(client, reference, expected).await
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

async fn open_world(clients: &SeamClients<'_>) -> TestResult<&'static BundleWorld> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )
}

fn reference(id: u32) -> TestResult<std::num::NonZeroU32> {
    let index = provisioning::item_index()?;
    provisioning::item_reference(&index, id)
}

fn worn(client: &DevClientSession, definition: std::num::NonZeroU32) -> Option<String> {
    client
        .inventory()?
        .equipment
        .iter()
        .find(|entry| entry.item.item_definition_ref == definition)
        .map(|entry| format!("{:?}", entry.slot))
}

async fn settle(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    want_worn: bool,
) -> TestResult<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    loop {
        client.take_events();
        if worn(client, definition).is_some() == want_worn {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=settle observed=worn:{:?};wanted={want_worn}",
                worn(client, definition)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn hotkey(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
) -> TestResult<oteryn_dev_client::ItemMoveOutcome> {
    client.take_events();
    Ok(
        oteryn_dev_client::client_runtime::hotkey_equip::equip_definition(
            client.session_mut(),
            definition,
        )
        .await?,
    )
}

async fn expect(
    client: &mut DevClientSession,
    definition: std::num::NonZeroU32,
    step: &str,
    want: oteryn_dev_client::ItemMoveOutcome,
) -> TestResult<()> {
    let outcome = hotkey(client, definition).await?;
    if outcome != want {
        return Err(format!("step={step} observed={outcome:?};needed={want:?}").into());
    }
    Ok(())
}

async fn buy(
    client: &mut DevClientSession,
    world: &BundleWorld,
    id: u32,
    amount: u32,
) -> TestResult<()> {
    acquire(client, world, id, amount)
        .await
        .map(|_| ())
        .map_err(|error| {
            format!("step=buy-{id} observed={error};needed:player-gold-and-reachable-seller").into()
        })
}

async fn wait_for(client: &mut DevClientSession, window: Duration) -> TestResult<()> {
    let deadline = tokio::time::Instant::now() + window;
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = open_world(clients).await?;
    use oteryn_dev_client::ItemMoveOutcome as Out;
    let amulet = reference(3572)?;
    let mut client = connect_world(clients, 0xe4).await?;
    buy(&mut client, world, 3572, 1).await?;
    expect(&mut client, amulet, "equip-amulet", Out::Moved).await?;
    settle(&mut client, amulet, true).await?;
    let slot = worn(&client, amulet).ok_or("step=equip observed=not-worn")?;
    if slot != "Necklace" {
        return Err(format!("step=equip observed=slot:{slot};needed=Necklace").into());
    }
    let refused = hotkey(&mut client, amulet).await?;
    if refused != Out::Rejected || worn(&client, amulet).is_none() {
        return Err(format!(
            "step=immediate-repeat observed={refused:?};needed=Rejected-and-still-worn"
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        wait_for(&mut client, Duration::from_millis(500)).await?;
        match hotkey(&mut client, amulet).await? {
            Out::Moved => break,
            Out::Rejected if tokio::time::Instant::now() < deadline => {}
            other => return Err(format!("step=after-delay observed={other:?};needed=Moved").into()),
        }
    }
    settle(&mut client, amulet, false).await?;
    logout(&mut client).await?;
    Err("jewelry-delay-refusal-and-recovery-proven;needed:feared-condition-player-route".into())
}
