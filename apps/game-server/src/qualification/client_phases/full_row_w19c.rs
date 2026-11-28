//! W19c novelty items (fireworks, toys), derived from the novelty-item owner
//! (`gameplay_transport/novelty_item_owner.rs`, `items/src/novelty_items`).
//!
//! 1. Walk to the authored NPC seller (Stan) and buy one trumpet (6572, 80 gold) through the real
//!    talk and buy commands; assert the exact inventory credit.
//! 2. USE the trumpet by its server-issued handle; assert `Committed` and the source transform:
//!    the trumpet is replaced by the played trumpet (6573).
//! 3. Keep answering liveness; the server's four-second decay must restore the trumpet (6572).
//! 4. Log out, obtain a fresh admission and assert the durable quantities are unchanged.
//! 5. A purchase or use that the server refuses reports FAIL with the exact reply.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::{BTreeMap, BTreeSet};

const TRUMPET: u32 = 6572;
const PLAYED_TRUMPET: u32 = 6573;
const TRUMPET_PRICE: u64 = 80;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(note)) => ("PASS", compact(&note)),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W19c.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(400)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
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
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if quantity(client, reference)? == expected {
            return Ok(0);
        }
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

fn carried_gold(client: &DevClientSession) -> TestResult<u64> {
    let index = provisioning::item_index()?;
    let inventory = quantities(client)?;
    let mut gold = 0;
    for (id, value) in [(3031, 1), (3035, 100), (3043, 10_000)] {
        let reference = provisioning::item_reference(&index, id)?;
        gold += inventory.get(&reference.get()).copied().unwrap_or_default() * value;
    }
    Ok(gold)
}

async fn acquire(client: &mut DevClientSession, world: &BundleWorld) -> TestResult<(u64, u64)> {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, TRUMPET)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let key = format!("oteryn:item.tibia.i{TRUMPET}");
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
    let npc = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.stan" && sellers.contains(&npc.key))
        .ok_or("Stan is missing from the live qualification bundle")?;
    let approach = TilePos { x: 32947, ..START };
    if here != START || destination(world, here, StepDirection::West) != Some(approach) {
        return Err("Stan's exact westbound approach is absent from the loaded map".into());
    }
    let step = client.session_mut().step(StepDirection::West).await?;
    if step.disposition != StepDisposition::Moved
        || step.world_spatial_delta.is_none()
        || tile(client.world_spatial().actor_position)? != approach
    {
        return Err(format!("step=stan-approach expected={approach:?} observed={step:?}").into());
    }
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
    if u64::from(offer.unit_price) != TRUMPET_PRICE || offer.count != 1 {
        return Err(format!(
            "Stan trumpet offer price={} count={} expected={TRUMPET_PRICE}:1",
            offer.unit_price, offer.count
        )
        .into());
    }
    let before = quantity(client, reference)?;
    let gold_before = carried_gold(client)?;
    let expected_gold = gold_before
        .checked_sub(TRUMPET_PRICE)
        .ok_or("insufficient carried gold for the trumpet fixture")?;
    client.take_events();
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade window closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "buy item={TRUMPET} reply={reply:?} price={} quantity=1",
            offer.unit_price
        )
        .into());
    }
    let expected = before + 1;
    let sequence = wait_quantity(client, reference, expected).await?;
    let gold_after = carried_gold(client)?;
    if gold_after != expected_gold {
        return Err(format!(
            "step=purchase gold_before={gold_before} gold_after={gold_after} expected={expected_gold}"
        )
        .into());
    }
    Ok((sequence, expected_gold))
}

async fn use_one(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<(UseDisposition, u64)> {
    let before = quantity(client, reference)?;
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference)
        })
        .map(|item| item.handle)
        .ok_or("purchased item handle absent")?;
    client.take_events();
    let reply = client.use_item(handle).await?;
    if reply.disposition == UseDisposition::Committed {
        wait_quantity(client, reference, before - 1).await?;
    }
    Ok((reply.disposition, before))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("logout refused: {:?}", reply.result).into());
    }
    Ok(())
}

const START: TilePos = TilePos {
    x: 32948,
    y: 32110,
    floor: -6,
};

async fn prepare_buyer(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    let mut client = connect_character(clients, 0xe6, 0).await?;
    if carried_gold(&client)? < TRUMPET_PRICE {
        character_prep::grant_item(&client, 3035, 1).await?;
    }
    logout(&mut client).await?;
    drop(client);
    wait_character_released(clients, 0).await?;
    // Move only the released, dedicated fixture's saved position. Keep the
    // server-authored map, artifact and lease lineage from the real logout.
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let updated = sqlx::query(
        "UPDATE game_character_last_positions
         SET tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
         WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
           AND tile_x=$6 AND tile_y=$7 AND tile_floor=$8
           AND NOT EXISTS (
             SELECT 1 FROM game_durability_reconnect_sessions
             WHERE character_id=game_character_last_positions.character_id AND session_state<>3
           )",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .bind(i32::from(START.x))
    .bind(i32::from(START.y))
    .bind(-i16::from(START.floor))
    .bind(i32::from(TOWN.x))
    .bind(i32::from(TOWN.y))
    .bind(-i16::from(TOWN.floor))
    .execute(&mut connection)
    .await?;
    if updated.rows_affected() != 1 {
        return Err("W19c fixture has no persisted position from preparation logout".into());
    }
    evidence("row=W19c.full fixture=q_w19c staging=(32948,32110,-6) funding=carried-gold");
    let client = connect_character(clients, 0xe6, 0).await?;
    if tile(client.world_spatial().actor_position)? != START {
        return Err("W19c fixture did not admit at the verified Venore staging tile".into());
    }
    Ok(client)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let index = provisioning::item_index()?;
    let trumpet = provisioning::item_reference(&index, TRUMPET)?;
    let played = provisioning::item_reference(&index, PLAYED_TRUMPET)?;
    let buyer = character_prep::dedicated_character(clients, "NODE_BOOT_W19C_ACCOUNT").await?;
    let clients = &buyer.clients(clients);
    let mut client = prepare_buyer(clients).await?;
    let trumpet_base = quantity(&client, trumpet)?;
    let played_base = quantity(&client, played)?;
    let (bought, expected_gold) = acquire(&mut client, world).await?;
    let (reply, _) = use_one(&mut client, trumpet).await?;
    if reply != UseDisposition::Committed {
        return Err(format!("step=use observed=reply-{reply:?}").into());
    }
    // Source debit and transform credit, then the decay: restored trumpet back, played one gone.
    wait_quantity(&mut client, played, played_base + 1).await?;
    wait_quantity(&mut client, trumpet, trumpet_base + 1).await?;
    wait_quantity(&mut client, played, played_base).await?;
    let durable = quantities(&client)?;
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    let mut client = connect_character(clients, 0xe6, 0).await?;
    if client.join_snapshot().game_session_id == session
        || quantities(&client)? != durable
        || carried_gold(&client)? != expected_gold
        || quantity(&client, trumpet)? != trumpet_base + 1
        || quantity(&client, played)? != played_base
    {
        return Err("step=fresh-admission observed=durable-quantities-changed".into());
    }
    logout(&mut client).await?;
    Ok(format!(
        "trumpet-bought-sequence-{bought}-played-then-decayed-durable"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_token() {
        assert_eq!(compact("a  b"), "a-b");
        assert_eq!(compact(" "), "unknown");
    }

    #[test]
    fn venore_staging_uses_the_same_cell_as_the_legacy_floor() -> TestResult {
        assert_eq!(
            tile(ActorPosition {
                x: 32948,
                y: 32110,
                floor: 6,
            })?,
            START
        );
        Ok(())
    }

    #[test]
    fn the_trumpet_ids_are_the_novelty_rule_pair() {
        assert_eq!((TRUMPET, PLAYED_TRUMPET), (6572, 6573));
    }
}
