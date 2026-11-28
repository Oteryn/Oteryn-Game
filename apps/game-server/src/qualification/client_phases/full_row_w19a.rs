//! W19a.full — random-outcome event containers, derived from the W19a audit row,
//! matrix row, and `reward_bag_use`'s generation-fenced consume/mint owner.
//!
//! Player scenario:
//! 1. Walk to the compiled Messenger of Santa, TALK "hi" and "trade", and read the
//!    live offer price. If needed, walk to a real rat spawn, ATTACK and loot until
//!    the character can afford two bags. Return on foot and open the shop again.
//! 2. BUY each red Christmas bundle separately; expect two accepted purchases
//!    and verify the exact inventory and gold change after each.
//! 3. Send USE-WITH-POSITION for one bundle; expect Accepted/NothingToUse and no
//!    inventory change (the refusal must not consume or mint anything).
//! 4. Send ordinary USE for one bundle; expect Accepted/Committed and an inventory
//!    delta that removes exactly one source and adds seven distinct pinned rewards.
//! 5. LOGOUT and freshly admit the character; expect a new session and the exact
//!    post-use inventory quantities, proving the consumed source and reward durable.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{npc_service as npc, world_object as use_wire};
use std::collections::BTreeMap;

// Crystal 201189f8 data-global/npc/messenger_of_santa.lua:62.
pub(super) const BAG_ID: u32 = 6506;
const BUNDLES_TO_BUY: u32 = 2;
const MESSENGER_NAME: &str = "Messenger of Santa";
// Crystal 201189f8 data-global/npc/messenger_of_santa.lua:11.
// Stop walking close enough to talk even at the edge of the spawn radius.
const MESSENGER_WANDER_RADIUS: u32 = 2;

pub(super) fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut result = BTreeMap::new();
    for entry in &client.inventory().ok_or("W19a inventory absent")?.entries {
        *result.entry(entry.item_definition_ref.get()).or_default() += u64::from(entry.count.get());
    }
    Ok(result)
}

fn coin_values() -> TestResult<[(u32, u64); 3]> {
    let index = provisioning::item_index()?;
    Ok([
        (provisioning::item_reference(&index, 3031)?.get(), 1),
        (provisioning::item_reference(&index, 3035)?.get(), 100),
        (provisioning::item_reference(&index, 3043)?.get(), 10_000),
    ])
}

fn gold_and_goods(
    mut items: BTreeMap<u32, u64>,
    coins: &[(u32, u64); 3],
) -> (u64, BTreeMap<u32, u64>) {
    let gold = coins.iter().map(|(key, value)| items.remove(key).unwrap_or(0) * value).sum();
    (gold, items)
}

async fn earn_price(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    cost: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let coins = coin_values()?;
    while gold_and_goods(quantities(client)?, &coins).0 < cost {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step 1: rat ATTACK/loot deadline with carried_gold={}",
                gold_and_goods(quantities(client)?, &coins).0
            )
            .into());
        }
        super::full_row_w22a::travel_to_named_spawn(client, world, spawns, "oteryn:creature.rat", "Rat").await?;
        let entries = super::full_row_w22a::kill_and_open(client, "Rat").await?;
        super::full_row_w22a::move_loot(client, &entries).await?;
    }
    Ok(())
}

async fn wait_quantities(
    client: &mut DevClientSession,
    wanted: &(u64, BTreeMap<u32, u64>),
    coins: &[(u32, u64); 3],
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut sequence = 0;
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event {
                sequence = delta.server_sequence;
            }
        }
        if &gold_and_goods(quantities(client)?, coins) == wanted && sequence != 0 {
            return Ok(sequence);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step 2 BUY: missing exact bag and price inventory delta; observed={:?}",
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
        .ok_or_else(|| "step 3: red Christmas bundle has no server-issued handle".into())
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

fn is_messenger_in_talk_range(
    entity: &oteryn_dev_client::WorldSpatialEntity,
    at: ActorPosition,
) -> bool {
    entity.kind == oteryn_dev_client::EntityKind::Npc
        && entity.display_name.is_some_and(|name| {
            name.as_str()
                .is_ok_and(|shown| shown.eq_ignore_ascii_case(MESSENGER_NAME))
        })
        && entity.position.floor == at.floor
        && entity.position.x.abs_diff(at.x).max(entity.position.y.abs_diff(at.y))
            <= npc::TALK_RANGE_TILES
}

const MESSENGER: &str = "oteryn:npc.messenger_of_santa";
async fn quote(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<(npc::NpcTradeIntent, u32)> {
    use oteryn_dev_client::client_runtime::npc_shop as shop;
    let seller = npcs.npcs.iter().find(|entry| entry.key == MESSENGER && !entry.placements.is_empty())
        .ok_or("step 2: compiled world has no Messenger of Santa placement")?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        seller
            .placements
            .iter()
            .any(|p| {
                p.floor == at.floor
                    && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)))
                        + MESSENGER_WANDER_RADIUS <= npc::TALK_RANGE_TILES
            })
    })
    .ok_or("step 2: no player-walk route to Messenger of Santa")?;
    walk(client, world, &path).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    verdict::Harness::global().bounded_wait(deadline, "Messenger in talk range");
    let actor = loop {
        let at = client.world_spatial().actor_position;
        if let Some(merchant) = client.world_entities().and_then(|entities| {
            let mut eligible = entities.iter().filter(|entry| {
                is_messenger_in_talk_range(entry, at)
                    && tile(entry.position).is_ok_and(|at| {
                        seller.placements.iter().any(|p| {
                            at.floor == p.floor
                                && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)))
                                    <= MESSENGER_WANDER_RADIUS
                        })
                    })
            });
            let merchant = eligible.next()?;
            eligible.next().is_none().then_some(merchant)
        }) {
            break merchant.entity;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step 2: live Messenger of Santa absent within talk range".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    client.take_events();
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
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    verdict::Harness::global().bounded_wait(deadline, "Messenger shop window");
    let (window, catalogue_revision) = loop {
        if let Some(open) = client.take_events().into_iter().find_map(|event| {
            if let SessionEvent::NpcTradeWindow(delta) = event
                && let Some((merchant, revision)) = delta.value.open
                && merchant == actor
            {
                return Some((delta.value, revision));
            }
            None
        }) {
            break open;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step 2: live Messenger trade window did not open".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    let reference = bag_reference()?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == npc::NpcTradeSide::Buy && offer.item_definition_ref == reference.get()
        })
        .ok_or("step 2: live Messenger trade window has no red Christmas bundle")?;
    evidence(&format!("W19A_CHECK live_shop=open unit_price={}", offer.unit_price));
    Ok((
        npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
        offer.count,
    ))
}

async fn acquire(
    client: &mut DevClientSession,
    intent: &npc::NpcTradeIntent,
    count: u32,
) -> TestResult<u64> {
    let coins = coin_values()?;
    let mut wanted = gold_and_goods(quantities(client)?, &coins);
    let unit_price = u64::from(intent.expected_unit_price);
    let cost = unit_price * u64::from(BUNDLES_TO_BUY);
    if wanted.0 < cost {
        return Err(format!("step 2: two bundles require {cost} gold; starter earned {}", wanted.0).into());
    }
    let reference = bag_reference()?.get();
    let mut previous_sequence = 0;
    for purchase in 1..=BUNDLES_TO_BUY {
        client.take_events();
        let reply = oteryn_dev_client::client_runtime::npc_shop::buy(client.session_mut(), intent)
            .await?;
        if reply != npc::NpcIntentDisposition::Ok {
            return Err(format!("step 2: BUY bundle {purchase} observed={reply:?} unit_price={unit_price}").into());
        }
        wanted.0 -= unit_price;
        *wanted.1.entry(reference).or_default() += u64::from(count);
        let sequence = wait_quantities(client, &wanted, &coins).await?;
        if sequence <= previous_sequence {
            return Err(format!("step 2: BUY bundle {purchase} did not advance inventory sequence").into());
        }
        evidence(&format!("W19A_CHECK bundle_purchase={purchase} inventory_sequence={sequence}"));
        previous_sequence = sequence;
    }
    Ok(previous_sequence)
}

pub(super) fn random_outcome_matches(
    consumed: &BTreeMap<u32, u64>,
    observed: &BTreeMap<u32, u64>,
    rewards: &BTreeMap<u32, (u32, u32)>,
    picks: usize,
) -> bool {
    if consumed.keys().any(|key| !observed.contains_key(key)) {
        return false;
    }
    let mut gains = 0;
    for (key, quantity) in observed {
        let prior = consumed.get(key).copied().unwrap_or(0);
        if *quantity == prior {
            continue;
        }
        let Some((min, max)) = rewards.get(key) else {
            return false;
        };
        let Some(gain) = quantity.checked_sub(prior) else {
            return false;
        };
        if !(u64::from(*min)..=u64::from(*max)).contains(&gain) {
            return false;
        }
        gains += 1;
    }
    gains == picks
}

pub(super) async fn acquire_bundles(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult<u64> {
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
    let (mut intent, mut count) = quote(client, world, &metadata.npcs).await?;
    let cost = u64::from(intent.expected_unit_price) * u64::from(BUNDLES_TO_BUY);
    if gold_and_goods(quantities(client)?, &coin_values()?).0 < cost {
        earn_price(client, world, &metadata.spawns, cost).await?;
        (intent, count) = quote(client, world, &metadata.npcs).await?;
    }
    acquire(client, &intent, count).await
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W19A_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients { accounts: &accounts, characters: &characters, ..*clients };
    let mut client = connect_world(clients, 0xa9).await?;
    let source_sequence = acquire_bundles(&mut client, clients).await?;
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
        .ok_or("step 4: red Christmas bundle rule absent")?;
    let index = provisioning::item_index()?;
    let rewards: BTreeMap<u32, (u32, u32)> = rule
        .rewards
        .iter()
        .map(|reward| {
            provisioning::item_reference(&index, reward.item_id)
                .map(|reference| (reference.get(), (reward.minimum, reward.maximum)))
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
        if random_outcome_matches(&after, &observed, &rewards, rule.picks) {
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
    let mut client = connect_world(clients, 0xaa).await?;
    if client.join_snapshot().game_session_id == old_session || quantities(&client)? != saved {
        return Err(format!(
            "step 5: fresh admission lost durable outcome; observed={:?}",
            quantities(&client)?
        )
        .into());
    }
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step 5: final LOGOUT refused".into());
    }
    evidence(&format!(
        "W19A_CHECK source_sequence={source_sequence} purchased=true refusal_sequence={} committed_sequence={} durable_outcomes={}",
        refused.2, committed.result_server_sequence, rule.picks
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", String::new()),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => ("FAIL", "step=journey observed=timeout-after-180s".to_owned()),
    };
    let suffix = if detail.is_empty() {
        String::new()
    } else {
        evidence(&format!("W19A_GAP {detail}"));
        format!(" evidence={}", detail.replace(char::is_whitespace, "_"))
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=W19a.full status={status} seconds={:.3}{suffix}",
        started.elapsed().as_secs_f64()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_selects_a_wandering_messenger_within_talk_range() -> TestResult {
        use oteryn_dev_client::{EntityDetail, EntityKind, EntityRef, WorldSpatialEntity};
        use oteryn_protocol_oteryn::entity_names::EntityDisplayName;

        let at = ActorPosition { x: 100, y: 100, floor: 7 };
        let mut merchant = WorldSpatialEntity {
            motion: None,
            display_name: Some(EntityDisplayName::new("Messenger Of Santa")
                .map_err(|error| format!("Messenger name: {error:?}"))?),
            kind: EntityKind::Npc,
            entity: EntityRef { identity: [1; 16], generation: 1 },
            position: ActorPosition { x: at.x + i32::try_from(npc::TALK_RANGE_TILES)?, ..at },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        assert!(is_messenger_in_talk_range(&merchant, at));
        merchant.position.x += 1;
        assert!(!is_messenger_in_talk_range(&merchant, at));
        merchant.position = at;
        merchant.display_name = Some(EntityDisplayName::new("Santa Claus")
            .map_err(|error| format!("Santa name: {error:?}"))?);
        assert!(!is_messenger_in_talk_range(&merchant, at));
        Ok(())
    }

    #[test]
    fn journey_regression_rejects_loss_of_unrelated_inventory() {
        let consumed = BTreeMap::from([(6506, 1), (3031, 20)]);
        let rewards = BTreeMap::from([(6570, (1, 1))]);
        assert!(!random_outcome_matches(&consumed, &BTreeMap::from([(6506, 1), (6570, 1)]), &rewards, 1));
        assert!(random_outcome_matches(&consumed, &BTreeMap::from([(6506, 1), (3031, 20), (6570, 1)]), &rewards, 1));
    }

    #[test]
    fn journey_regression_accepts_all_seven_distinct_bundle_rewards() {
        let consumed = BTreeMap::from([(6506, 1)]);
        let rewards = BTreeMap::from_iter((1..=7).map(|key| (key, (1, 1))));
        let mut observed = consumed.clone();
        observed.extend((1..=7).map(|key| (key, 1)));
        assert!(random_outcome_matches(&consumed, &observed, &rewards, 7));
        assert!(!random_outcome_matches(&consumed, &observed, &rewards, 6));
    }

}
