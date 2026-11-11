//! C09 client replay capture: create real HP/mana deficits, purchase and consume real food, then
//! capture the ACTOR_VITALS payload applied by `crates/session/src/domain_dispatch.rs` through
//! `Session::apply_domain_delta` and exposed by `Session::actor_vitals` in `session_access.rs`.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{ActorVitals, SpellCastDisposition, SpellTarget};
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(journey(clients)).await;
    match result {
        Ok(Ok(())) => evidence(
            "e2e_all scenario=C09.client status=PASS evidence=food-funded-HP-mana-payloads",
        ),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C09.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=C09.client status=FAIL evidence=journey-timeout"),
    }
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> u64 {
    quantity_from(client.inventory(), reference)
}

fn quantity_from(
    input: Option<&oteryn_dev_client::CharacterInventory>,
    reference: std::num::NonZeroU32,
) -> u64 {
    input.map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == reference)
            .map(|item| u64::from(item.count.get()))
            .sum()
    })
}

async fn acquire_meat(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<std::num::NonZeroU32> {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, 3577)?;
    if quantity(client, reference) > 0 {
        return Ok(reference);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let key = "oteryn:item.tibia.i3577";
    let sellers: std::collections::BTreeSet<_> = trades["records"]
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
        .ok_or("no reachable meat seller")?;
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
        .ok_or("meat seller not visible")?;
    for text in ["hi", "trade"] {
        if runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?
            != wire::NpcIntentDisposition::Ok
        {
            return Err(format!("NPC {text} refused").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("trade window: {error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
        })
        .ok_or("meat offer absent")?;
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!("meat purchase refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while quantity(client, reference) == 0 {
        if tokio::time::Instant::now() >= deadline {
            return Err("meat inventory delta timeout".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(reference)
}

fn capture(before: ActorVitals, after: ActorVitals) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let write = || -> TestResult {
        let path = Path::new(&path);
        std::fs::write(
            path.join("c09-before.bin"),
            oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(&before)
                .map_err(|error| format!("encode before: {error:?}"))?,
        )?;
        std::fs::write(
            path.join("c09-regenerated.bin"),
            oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(&after)
                .map_err(|error| format!("encode regenerated: {error:?}"))?,
        )?;
        std::fs::write(
            path.join("c09-expected.txt"),
            format!(
                "{} {} {} {} {} {} {}\n",
                after.health,
                after.max_health,
                after.mana,
                after.max_mana,
                after.soul,
                after.harmony,
                u8::from(after.serene)
            ),
        )?;
        Ok(())
    };
    write()?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
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
    let mut client = connect_world(clients, 0xc9).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let meat = acquire_meat(&mut client, world).await?;
        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        let full = *client.actor_vitals().ok_or("C09 initial vitals absent")?;
        for _ in 0..3 {
            client_hunt::hunt(&mut client)
                .await
                .map_err(|error| format!("C09 combat: {error}"))?;
            if client
                .actor_vitals()
                .is_some_and(|v| v.health < full.health)
            {
                break;
            }
        }
        let snapshot = client
            .session_mut()
            .gated_snapshot(
                oteryn_protocol_oteryn::actor_spell_catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG,
            )
            .ok_or("spell catalogue absent")?;
        let spell = oteryn_protocol_oteryn::actor_spell_catalog::decode_catalog(&snapshot.payload)
            .map_err(|error| format!("spell catalogue: {error:?}"))?
            .into_iter()
            .find(|spell| {
                spell.available
                    && spell.words == "utevo lux"
                    && !spell.mana_percent
                    && spell.mana > 0
            })
            .ok_or("no mana-costing learned spell")?;
        let cast = client
            .cast_spell(
                std::num::NonZeroU32::new(spell.cast_index).ok_or("zero spell index")?,
                SpellTarget::None,
                false,
            )
            .await?;
        if cast.disposition != SpellCastDisposition::Cast {
            return Err(format!("mana deficit cast={cast:?}").into());
        }
        let before = *client.actor_vitals().ok_or("deficit vitals absent")?;
        if before.health >= full.health || before.mana >= full.mana {
            return Err(format!("HP/mana deficits missing full={full:?} before={before:?}").into());
        }
        let handle = client
            .inventory()
            .and_then(|inventory| {
                inventory
                    .entries
                    .iter()
                    .find(|item| item.item_definition_ref == meat)
            })
            .map(|item| item.handle)
            .ok_or("meat handle absent")?;
        if client.use_item(handle).await?.disposition != UseDisposition::Committed {
            return Err("food use was not committed".into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            client.service_liveness(Duration::from_millis(50)).await?;
            for event in client.take_events() {
                if let SessionEvent::ActorVitals(delta) = event
                    && delta.value.health > before.health
                    && delta.value.mana > before.mana
                {
                    capture(before, delta.value)?;
                    return Ok(());
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "food regeneration timeout before={before:?} current={:?}",
                    client.actor_vitals()
                )
                .into());
            }
        }
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_projection_cannot_supply_evidence() {
        assert_eq!(quantity_from(None, std::num::NonZeroU32::MIN), 0);
    }
}
