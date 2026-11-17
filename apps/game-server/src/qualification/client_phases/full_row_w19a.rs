//! W19a.full — random-outcome event containers, derived from the W19a audit row,
//! matrix row, and `reward_bag_use`'s generation-fenced consume/mint owner.
//!
//! Player scenario:
//! 1. Admit the node-boot character, walk to a real rat spawn, ATTACK and loot
//!    until the character has the catalogue price of one bag (`full_gold::earn`, deadline sized
//!    from that price); expect movement/combat/inventory deltas.
//! 2. Walk to the compiled Messenger of Santa, TALK "hi" and "trade", then BUY
//!    one blue surprise bag; expect accepted talk replies, a trade-window delta,
//!    an accepted purchase reply, and an inventory delta containing the bag.
//! 3. Send USE-WITH-POSITION for one bag; expect Accepted/NothingToUse and no
//!    inventory change (the refusal must not consume or mint anything).
//! 4. Send ordinary USE for one bag; expect Accepted/Committed and an inventory
//!    delta that removes exactly one source and adds only a pinned random outcome.
//! 5. LOGOUT and freshly admit the character; expect a new session and the exact
//!    post-use inventory quantities, proving the consumed source and reward durable.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{npc_service as npc, world_object as use_wire};
use std::collections::BTreeMap;

const BAG_ID: u32 = 6506;
/// One bag is all the row consumes: USE-WITH refuses without consuming and ordinary USE takes one.
const BAG_QUANTITY: u32 = 1;
fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut result = BTreeMap::new();
    for entry in &client.inventory().ok_or("W19a inventory absent")?.entries {
        *result.entry(entry.item_definition_ref.get()).or_default() += u64::from(entry.count.get());
    }
    Ok(result)
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
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (payload, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("W19a refusal detail: {error:?}"))?;
    let disposition = use_wire::decode_use_result(payload)
        .map_err(|error| format!("W19a USE result: {error:?}"))?;
    Ok((reply.status, disposition, reply.server_sequence))
}

fn purchase_matches(
    before: &BTreeMap<u32, u64>,
    observed: &BTreeMap<u32, u64>,
    bag: u32,
    count: u64,
    coins: &[(u32, u64)],
    cost: u64,
) -> bool {
    let value = |map: &BTreeMap<u32, u64>| -> Option<u64> {
        coins.iter().try_fold(0_u64, |sum, (key, denomination)| {
            sum.checked_add(
                map.get(key)
                    .copied()
                    .unwrap_or(0)
                    .checked_mul(*denomination)?,
            )
        })
    };
    let noncoins = |map: &BTreeMap<u32, u64>| {
        map.iter()
            .filter(|(key, _)| !coins.iter().any(|(coin, _)| coin == *key))
            .map(|(&key, &value)| (key, value))
            .collect::<BTreeMap<_, _>>()
    };
    let mut wanted = noncoins(before);
    let Some(total) = wanted.get(&bag).copied().unwrap_or(0).checked_add(count) else {
        return false;
    };
    wanted.insert(bag, total);
    value(before).and_then(|gold| gold.checked_sub(cost)) == value(observed)
        && value(observed).is_some()
        && noncoins(observed) == wanted
}

fn reward_matches(
    consumed: &BTreeMap<u32, u64>,
    observed: &BTreeMap<u32, u64>,
    rewards: &BTreeMap<u32, (u32, u32)>,
) -> bool {
    // The source selects seven distinct entries without replacement from ten candidates.
    let mut remaining = observed.clone();
    let mut picks = 0;
    for (&key, &(low, high)) in rewards {
        let before = consumed.get(&key).copied().unwrap_or(0);
        let after = remaining.get(&key).copied().unwrap_or(0);
        let Some(delta) = after.checked_sub(before) else {
            return false;
        };
        if delta != 0 {
            if !(u64::from(low)..=u64::from(high)).contains(&delta) {
                return false;
            }
            picks += 1;
            if before == 0 {
                remaining.remove(&key);
            } else {
                remaining.insert(key, before);
            }
        }
    }
    picks == 7 && &remaining == consumed
}

fn failure_status(_error: &(dyn std::error::Error + 'static)) -> &'static str {
    "FAIL"
}

fn missing_live_offer() -> String {
    "step 2: live Messenger trade window has no blue surprise bag".into()
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
            "step 2: compiled world has no Messenger of Santa acquisition actor".to_owned()
        })?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        seller
            .placements
            .iter()
            .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
    })
    .ok_or_else(|| "step 2: no player-walk route to Messenger of Santa".to_owned())?;
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
            "step 2: Messenger of Santa absent after walking to compiled placement".to_owned()
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
        .ok_or_else(missing_live_offer)?;
    let before = quantities(client)?;
    client.take_events();
    let reply = shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("step 2: trade window closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: BAG_QUANTITY,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != npc::NpcIntentDisposition::Ok {
        return Err(format!(
            "step 2: BUY bag observed={reply:?} unit_price={}",
            offer.unit_price
        )
        .into());
    }
    let coins = [3031, 3035, 3043]
        .into_iter()
        .zip([1, 100, 10_000])
        .map(|(id, value)| {
            provisioning::item_reference(&provisioning::item_index()?, id)
                .map(|reference| (reference.get(), value))
        })
        .collect::<TestResult<Vec<_>>>()?;
    let cost = u64::from(offer.unit_price)
        .checked_mul(u64::from(BAG_QUANTITY))
        .ok_or("purchase-cost-overflow")?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let sequence = client
            .take_events()
            .into_iter()
            .filter_map(|event| match event {
                SessionEvent::Inventory(delta) => Some(delta.server_sequence),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        if sequence != 0
            && purchase_matches(
                &before,
                &quantities(client)?,
                reference.get(),
                u64::from(offer.count) * u64::from(BAG_QUANTITY),
                &coins,
                cost,
            )
        {
            return Ok(sequence);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step BUY: exact purchase/payment delta absent".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
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
    full_gold::earn(
        &mut client,
        world,
        &metadata.spawns,
        "W19A",
        full_gold::price(BAG_ID, BAG_QUANTITY)?,
    )
    .await?;
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
    if rule.selection
        != oteryn_gameplay_items::reward_bag::event_containers::Selection::WithoutReplacement
        || rule.picks != 7
    {
        return Err("step 4: blue bag selection contract changed".into());
    }
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
        if reward_matches(&after, &observed, &rewards) {
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
    // Earning the price is real hunting, so its budget is part of the row's own deadline; a
    // catalogue error surfaces from the journey itself.
    let price = full_gold::price(BAG_ID, BAG_QUANTITY).unwrap_or_default();
    let result = verdict::indexed_timeout(
        full_gold::allowance(price) + Duration::from_secs(180),
        verdict::observe(journey(clients)),
    )
    .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W19a.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => {
            let detail = error.to_string();
            evidence(&format!("W19A_FAIL {detail}"));
            let status = failure_status(error.as_ref());
            evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=W19a.full status={status} seconds={:.3}",
                started.elapsed().as_secs_f64()
            ));
        }
        Err(error) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W19a.full status=FAIL seconds={:.3} evidence={}",
            started.elapsed().as_secs_f64(),
            verdict::sanitize_cause(&error.to_string())
        )),
    }
}

#[cfg(test)]
mod review_tests {
    use super::*;
    fn rewards() -> TestResult<BTreeMap<u32, (u32, u32)>> {
        Ok(
            oteryn_gameplay_items::reward_bag::event_containers::event_container_rules()
                .ok_or("authored rules absent")?
                .iter()
                .find(|rule| rule.item_id == BAG_ID)
                .ok_or("blue bag absent")?
                .rewards
                .iter()
                .map(|reward| (reward.item_id, (reward.minimum, reward.maximum)))
                .collect(),
        )
    }
    #[test]
    fn review_purchase_accepts_payment_and_denomination_change() {
        let before = BTreeMap::from([(3035, 1), (3031, 3)]);
        let after = BTreeMap::from([(3031, 98), (BAG_ID, 1)]);
        let coins = [(3031, 1), (3035, 100), (3043, 10_000)];
        assert!(purchase_matches(&before, &after, BAG_ID, 1, &coins, 5));
        assert!(!purchase_matches(
            &before,
            &BTreeMap::from([(3035, 1), (3031, 3), (BAG_ID, 1)]),
            BAG_ID,
            1,
            &coins,
            5
        ));
    }
    #[test]
    fn review_blue_bag_requires_every_authored_gift() -> TestResult {
        let rewards = rewards()?;
        assert!(rewards.len() >= 7);
        let consumed = BTreeMap::new();
        let gifts = rewards
            .iter()
            .take(7)
            .map(|(&key, &(low, high))| {
                assert_eq!(low, high);
                (key, u64::from(low))
            })
            .collect();
        assert!(reward_matches(&consumed, &gifts, &rewards));
        Ok(())
    }
    #[test]
    fn review_blue_bag_rejects_wrong_quantity_and_unrelated_deletion() -> TestResult {
        let rewards = rewards()?;
        let (&key, &(low, _)) = rewards.iter().next().ok_or("gifts absent")?;
        assert!(!reward_matches(
            &BTreeMap::new(),
            &BTreeMap::from([(key, u64::from(low) + 100)]),
            &rewards
        ));
        assert!(!reward_matches(
            &BTreeMap::from([(BAG_ID, 1)]),
            &BTreeMap::from([(key, u64::from(low))]),
            &rewards
        ));
        Ok(())
    }
    #[test]
    fn review_live_offer_mismatch_is_a_failed_observation() {
        let error: Box<dyn std::error::Error> = missing_live_offer().into();
        assert_eq!(failure_status(error.as_ref()), "FAIL");
    }
}
