//! I19: gem and weapon enchanting at elemental shrines (`items/src/item_craft/enchanting.rs`).
//! Buys the operands from ordinary shops, verifies targetless refusal, then
//! crafts a ruby at an authored altar and applies it to a carried sword.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::{BTreeMap, BTreeSet};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I19.full status={status} seconds={:.3} evidence={detail}",
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    use super::full_row_i11::{carried_item, use_ingredient, walk_to_map_item};
    use oteryn_gameplay_items::item_craft::enchanting::rule;

    const RUBY: u16 = 3030;
    const SWORD: u16 = 3271;
    let world = open_world(clients).await?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let gem = reference(u32::from(RUBY))?;
    let sword = reference(u32::from(SWORD))?;
    let mut client = connect_world(clients, 0xdf).await?;
    let unit_rule = rule(RUBY, 146, 1, 1).ok_or("I19 ruby altar rule absent")?;
    if client.actor_vitals().ok_or("I19 vitals absent")?.max_mana < unit_rule.mana {
        super::full_row_q01::provision(clients, &mut client, 0xdf, 1, "sorcerer", "I19").await?;
    }
    acquire(&mut client, world, u32::from(RUBY), 1).await?;
    let ruby = carried_item(&client, gem)?;
    let before = quantities(&client)?;
    let targetless = client.use_item(ruby.handle).await?;
    client.service_liveness(Duration::from_millis(100)).await?;
    if targetless.disposition == UseDisposition::Committed || quantities(&client)? != before {
        return Err(format!("step=targetless-ruby expected refusal and unchanged inventory; reply={targetless:?}").into());
    }

    let (altar, site) = walk_to_map_item(&mut client, world, &bytes, "ruby-altar", |key| {
        crate::map::palette_appearance(key).is_some_and(|id| rule(RUBY, id, 1, 1).is_some())
    }).await?;
    let ruby = carried_item(&client, gem)?;
    let altar_rule = rule(RUBY, 146, ruby.count.get(), 1).ok_or("I19 invalid ruby stack")?;
    let enchanted = reference(u32::from(altar_rule.output))?;
    let input_before = quantity(&client, gem)?;
    let output_before = quantity(&client, enchanted)?;
    let before = *client.actor_vitals().ok_or("I19 altar vitals absent")?;
    let expected_mana = before.mana.checked_sub(altar_rule.mana).ok_or("I19 altar needs more mana")?;
    let expected_soul = before.soul.checked_sub(altar_rule.soul).ok_or("I19 altar needs more soul")?;
    client.take_events();
    let reply = use_ingredient(&mut client, ruby.handle, None, Some(altar)).await?;
    if reply != UseDisposition::Committed {
        return Err(format!("step=ruby-altar position={site:?} reply={reply:?}").into());
    }
    wait_quantity(&mut client, enchanted, output_before + u64::from(altar_rule.count)).await?;
    if !observed_conversion(
        reply, (input_before, quantity(&client, gem)?),
        (output_before, quantity(&client, enchanted)?),
        altar_rule.source_debit, altar_rule.count,
    ) {
        return Err("step=ruby-altar inventory conversion mismatch".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !client.actor_vitals().is_some_and(|now| now.mana == expected_mana && now.soul == expected_soul) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=ruby-altar expected mana {expected_mana} and soul {expected_soul}; observed {:?}", client.actor_vitals()).into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }

    acquire(&mut client, world, u32::from(SWORD), 1).await?;
    let ruby = carried_item(&client, enchanted)?;
    let blade = carried_item(&client, sword)?;
    let weapon_rule = rule(altar_rule.output, SWORD, ruby.count.get(), blade.count.get())
        .ok_or("I19 enchanted ruby sword rule absent")?;
    let weapon = reference(u32::from(weapon_rule.output))?;
    let blade_before = quantity(&client, sword)?;
    let ruby_before = quantity(&client, enchanted)?;
    let weapon_before = quantity(&client, weapon)?;
    client.take_events();
    let reply = use_ingredient(&mut client, ruby.handle, Some(blade.handle), None).await?;
    if reply != UseDisposition::Committed {
        return Err(format!("step=enchant-sword reply={reply:?}").into());
    }
    wait_quantity(&mut client, weapon, weapon_before + u64::from(weapon_rule.count)).await?;
    if !observed_conversion(
        reply, (ruby_before, quantity(&client, enchanted)?),
        (weapon_before, quantity(&client, weapon)?),
        weapon_rule.source_debit, weapon_rule.count,
    ) || blade_before.checked_sub(u64::from(blade.count.get())) != Some(quantity(&client, sword)?) {
        return Err("step=enchant-sword inventory conversion mismatch".into());
    }
    let expected = quantities(&client)?;
    logout(&mut client).await?;
    let mut fresh = connect_world(clients, 0xdf).await?;
    if quantities(&fresh)? != expected {
        return Err("step=enchant-relog inventory conversion did not persist".into());
    }
    logout(&mut fresh).await?;
    Ok(format!("altar={site:?} ruby_output={} weapon_output={} mana_debit={} soul_debit={}",
        altar_rule.output, weapon_rule.output, altar_rule.mana, altar_rule.soul))
}

fn observed_conversion(
    disposition: UseDisposition,
    input: (u64, u64),
    output: (u64, u64),
    source_units: u32,
    output_units: u32,
) -> bool {
    disposition == UseDisposition::Committed
        && input.0.checked_sub(u64::from(source_units)) == Some(input.1)
        && output.0.checked_add(u64::from(output_units)) == Some(output.1)
}

#[test]
fn altar_enchant_requires_committed_conversion() {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    assert!(observed_conversion(UseDisposition::Committed, (3, 2), (0, 1), 1, 1));
    assert!(!observed_conversion(UseDisposition::Committed, (3, 3), (0, 1), 1, 1));
    assert!(!observed_conversion(UseDisposition::NothingToUse, (3, 2), (0, 1), 1, 1));
}
