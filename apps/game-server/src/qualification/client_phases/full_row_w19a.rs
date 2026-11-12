//! W19a.full — random-outcome event containers, derived from the W19a audit row,
//! matrix row, and `reward_bag_use`'s generation-fenced consume/mint owner.
//!
//! Player scenario:
//! 1. Admit the node-boot character, walk to a real rat spawn, ATTACK and loot
//!    until the character has the shop price; expect movement/combat/inventory deltas.
//! 2. Walk to the compiled Messenger of Santa, TALK "hi" and "trade", then BUY
//!    two blue surprise bags; expect accepted talk replies, a trade-window delta,
//!    an accepted purchase reply, and an inventory delta containing both bags.
//! 3. Send USE-WITH-POSITION for one bag; expect Accepted/NothingToUse and no
//!    inventory change (the refusal must not consume or mint anything).
//! 4. Send ordinary USE for one bag; expect Accepted/Committed and an inventory
//!    delta that removes exactly one source and adds only a pinned random outcome.
//! 5. LOGOUT and freshly admit the character; expect a new session and the exact
//!    post-use inventory quantities, proving the consumed source and reward durable.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{npc_service as npc, world_object as use_wire};
use std::collections::{BTreeMap, BTreeSet};

const BAG_ID: u32 = 6506;
const OWNER_GAP: &str =
    "source:crates/gameplay/dispatch/src/gameplay_transport/reward_bag_use.rs:167";

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut result = BTreeMap::new();
    for entry in &client.inventory().ok_or("W19a inventory absent")?.entries {
        *result.entry(entry.item_definition_ref.get()).or_default() += u64::from(entry.count.get());
    }
    Ok(result)
}

fn carried_gold(client: &DevClientSession) -> TestResult<u64> {
    let index = provisioning::item_index()?;
    let coins = [(3031, 1), (3035, 100), (3043, 10_000)];
    let references: Vec<_> = coins
        .into_iter()
        .map(|(id, value)| provisioning::item_reference(&index, id).map(|key| (key, value)))
        .collect::<TestResult<_>>()?;
    Ok(client
        .inventory()
        .ok_or("step 1: inventory absent")?
        .entries
        .iter()
        .map(|entry| {
            references
                .iter()
                .find(|(key, _)| *key == entry.item_definition_ref)
                .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
        })
        .sum())
}

async fn earn_price(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while carried_gold(client)? < 160 {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step 1: rat ATTACK/loot deadline with carried_gold={}",
                carried_gold(client)?
            )
            .into());
        }
        travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(client)
            .await
            .map_err(|error| format!("step 1: rat ATTACK/loot observed={error}"))?;
    }
    Ok(())
}

async fn wait_quantities(
    client: &mut DevClientSession,
    wanted: &BTreeMap<u32, u64>,
    step: &str,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut sequence = 0;
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event {
                sequence = delta.server_sequence;
            }
        }
        if &quantities(client)? == wanted && sequence != 0 {
            return Ok(sequence);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step {step}: missing inventory delta; observed={:?}",
                quantities(client)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

fn bag_reference() -> TestResult<std::num::NonZeroU32> {
    let index = provisioning::item_index()?;
    provisioning::item_reference(&index, BAG_ID)
}

fn bag_handle(
    client: &DevClientSession,
) -> TestResult<oteryn_protocol_oteryn::item_view::ItemHandle> {
    let reference = bag_reference()?;
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == reference)
        })
        .map(|entry| entry.handle)
        .ok_or_else(|| "step 3: purchased blue surprise bag has no server-issued handle".into())
}

async fn use_intent(
    client: &mut DevClientSession,
    intent: use_wire::UseIntent,
) -> TestResult<(CommandStatus, use_wire::UseDisposition, u64)> {
    let payload = use_wire::encode_item_use_intent(&intent)
        .map_err(|error| format!("W19a USE encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    let (payload, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("W19a refusal detail: {error:?}"))?;
    let disposition = use_wire::decode_use_result(payload)
        .map_err(|error| format!("W19a USE result: {error:?}"))?;
    Ok((reply.status, disposition, reply.server_sequence))
}

async fn acquire(client: &mut DevClientSession, world: &BundleWorld) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop as shop;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let seller = metadata
        .npcs
        .npcs
        .iter()
        .find(|entry| entry.key == "oteryn:npc.messenger_of_santa")
        .ok_or_else(|| {
            format!(
                "step 2: compiled world has no Messenger of Santa acquisition actor; {OWNER_GAP}"
            )
        })?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        seller
            .placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    })
    .ok_or_else(|| format!("step 2: no player-walk route to Messenger of Santa; {OWNER_GAP}"))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && seller.placements.iter().any(|p| {
                        entry.position
                            == ActorPosition {
                                x: i32::from(p.x),
                                y: i32::from(p.y),
                                floor: -i16::from(p.floor),
                            }
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or_else(|| {
            format!(
                "step 2: Messenger of Santa absent after walking to compiled placement; {OWNER_GAP}"
            )
        })?;
    for text in ["hi", "trade"] {
        let reply = shop::talk(
            client.session_mut(),
            &npc::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("step 2: TALK {text:?} observed={reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("step 2: trade window decode: {error:?}"))?;
    let reference = bag_reference()?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
        })
        .ok_or_else(|| {
            format!("step 2: live Messenger trade window has no blue surprise bag; {OWNER_GAP}")
        })?;
    let before = quantities(client)?;
    client.take_events();
    let reply = shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("step 2: trade window closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 2,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!(
            "step 2: BUY two bags observed={reply:?} unit_price={}",
            offer.unit_price
        )
        .into());
    }
    let mut wanted = before;
    *wanted.entry(reference.get()).or_default() += u64::from(offer.count) * 2;
    wait_quantities(client, &wanted, "2 BUY").await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xa9).await?;
    earn_price(&mut client, world, &metadata.spawns).await?;
    let purchase_sequence = acquire(&mut client, world).await?;
    let before_refusal = quantities(&client)?;
    let position = client.world_spatial().actor_position;
    let source = bag_handle(&client)?;
    let refused = use_intent(
        &mut client,
        use_wire::UseIntent {
            target: use_wire::UseTarget::Item(source),
            use_with: None,
            use_with_map_item: None,
            use_with_position: Some(oteryn_protocol_oteryn::actor_spell::SpellTargetPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            }),
        },
    )
    .await?;
    if refused.0 != CommandStatus::Accepted
        || refused.1 != use_wire::UseDisposition::NothingToUse
        || quantities(&client)? != before_refusal
    {
        return Err(format!(
            "step 3: USE-WITH refusal observed={refused:?} inventory={:?}",
            quantities(&client)?
        )
        .into());
    }
    let rules = oteryn_gameplay_items::reward_bag::event_containers::event_container_rules()
        .ok_or("step 4: event outcome catalogue absent")?;
    let rule = rules
        .iter()
        .find(|rule| rule.item_id == BAG_ID)
        .ok_or("step 4: blue surprise bag rule absent")?;
    let index = provisioning::item_index()?;
    let rewards: BTreeSet<u32> = rule
        .rewards
        .iter()
        .map(|reward| {
            provisioning::item_reference(&index, reward.item_id).map(std::num::NonZeroU32::get)
        })
        .collect::<TestResult<_>>()?;
    client.take_events();
    let committed = client.use_item(bag_handle(&client)?).await?;
    if committed.status != CommandStatus::Accepted
        || committed.disposition != use_wire::UseDisposition::Committed
    {
        return Err(format!("step 4: ordinary USE observed={committed:?}").into());
    }
    let mut after = before_refusal.clone();
    *after
        .get_mut(&bag_reference()?.get())
        .ok_or("step 4: source count absent")? -= 1;
    if after[&bag_reference()?.get()] == 0 {
        after.remove(&bag_reference()?.get());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let saved = loop {
        let observed = quantities(&client)?;
        let delta: Vec<_> = observed
            .iter()
            .filter(|(key, value)| after.get(key).copied().unwrap_or(0) != **value)
            .collect();
        if delta.len() == 1
            && rewards.contains(delta[0].0)
            && *delta[0].1 > after.get(delta[0].0).copied().unwrap_or(0)
        {
            break observed;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step 4: missing exact consume/random-outcome delta; before={before_refusal:?} observed={observed:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    let old_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step 5: LOGOUT refused".into());
    }
    drop(client);
    let client = connect_world(clients, 0xaa).await?;
    if client.join_snapshot().game_session_id == old_session || quantities(&client)? != saved {
        return Err(format!(
            "step 5: fresh admission lost durable outcome; observed={:?}",
            quantities(&client)?
        )
        .into());
    }
    evidence(&format!(
        "W19A_CHECK purchase_sequence={purchase_sequence} refusal_sequence={} committed_sequence={} durable_outcome=1",
        refused.2, committed.result_server_sequence
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W19a.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => {
            let detail = error.to_string();
            evidence(&format!("W19A_FAIL {detail}"));
            let status = if detail.contains(OWNER_GAP) {
                "MISSING-SERVER"
            } else {
                "FAIL"
            };
            let suffix = if status == "MISSING-SERVER" {
                format!(" evidence={OWNER_GAP}")
            } else {
                String::new()
            };
            evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=W19a.full status={status} seconds={:.3}{suffix}",
                started.elapsed().as_secs_f64()
            ));
        }
        Err(_) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W19a.full status=FAIL seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
    }
}
