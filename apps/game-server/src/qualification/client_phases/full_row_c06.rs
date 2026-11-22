//! C06 full-row journey, derived from the matrix, Crystal re-audit, and the native
//! `rune_use_bridge`/`actor_spell` owners.
//!
//! Buy a blank rune from a served merchant, conjure Light Magic Missile charges and
//! prove the reagent debit, then spend a charge on a live enemy. Buy an eligible
//! Intense Healing Rune, heal the character, and prove an illegal hostile target
//! preserves the remaining charge. Fresh admission must retain the exact charges.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

const DAMAGE_RUNE_ID: u32 = 3174;
const HEAL_RUNE_ID: u32 = 3152;
const BLANK_RUNE_ID: u32 = 3147;
const STARTER_CONJURE: &str = "adori dis min vis";
const ORDINARY_CONJURE: &str = "adori min vis";
const CONJURED_CHARGES: u64 = 10;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    let result = result.unwrap_or_else(|_| {
        Err("step=journey observed=timeout-240s need=all-native-replies-and-deltas".into())
    });
    if let Err(error) = &result {
        evidence(&format!("C06_GAP {error}"));
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C06.full status={} seconds={:.3}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        if result.is_ok() {
            ""
        } else {
            " evidence=required-actions-unproven-see-C06_GAP"
        }
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let index = provisioning::item_index()?;
    let damage = provisioning::item_reference(&index, DAMAGE_RUNE_ID)?;
    let healing = provisioning::item_reference(&index, HEAL_RUNE_ID)?;
    let blank = provisioning::item_reference(&index, BLANK_RUNE_ID)?;
    let mut client = connect_world(clients, 0xc6).await?;
    // Xodet's pinned price is ten gold per blank rune. The second blank
    // makes the immediate repeat test cooldown rather than reagent absence.
    super::full_row_c02::earn_gold(&mut client, world, &metadata.spawns, 20)
        .await
        .map_err(|error| format!("step=1 observed=fund-blank-runes:{error}"))?;
    acquire(&mut client, world, &metadata.npcs, blank, BLANK_RUNE_ID, 2)
        .await
        .map_err(|error| format!("step=1 item={BLANK_RUNE_ID} {error}"))?;
    check(1, client.last_server_sequence());

    let spell = conjure_index(&mut client)?;
    let blank_before = quantity(&client, blank)?;
    let damage_on_entry = quantity(&client, damage)?;
    let mana = client
        .actor_vitals()
        .ok_or("step=2 observed=no-vitals")?
        .mana;
    client.take_events();
    let cast = client.cast_spell(spell, SpellTarget::None, false).await?;
    if cast.status != CommandStatus::Accepted
        || cast.disposition != SpellCastDisposition::Cast
        || cast
            .actor_vitals_delta
            .as_ref()
            .is_none_or(|delta| delta.value.mana >= mana)
    {
        return Err(format!("step=2 observed={cast:?} need=Cast-mana-delta").into());
    }
    let damage_before = damage_on_entry + CONJURED_CHARGES;
    let blank_after = blank_before.checked_sub(1).ok_or("blank rune absent")?;
    let paid_mana = client
        .actor_vitals()
        .ok_or("step=2 observed=no-post-cast-vitals")?
        .mana;
    let refused = client.cast_spell(spell, SpellTarget::None, false).await?;
    inventory_wait(
        &mut client,
        &[(blank, blank_after), (damage, damage_before)],
    )
    .await?;
    drain_after(&mut client, refused.result_server_sequence).await?;
    if refused.disposition != SpellCastDisposition::CoolingDown
        || client.actor_vitals().is_none_or(|v| v.mana != paid_mana)
        || quantity(&client, damage)? != damage_before
        || quantity(&client, blank)? != blank_before - 1
    {
        return Err(
            format!("step=2 observed=repeat:{refused:?} need=CoolingDown-unchanged").into(),
        );
    }
    check(2, refused.result_server_sequence);

    require_rune_eligibility(&mut client, DAMAGE_RUNE_ID, 3)?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat")
        .await
        .map_err(|error| format!("step=3 observed=reach-rune-target:{error}"))?;
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Creature
                    && e.position.floor == position.floor
                    && e.position
                        .x
                        .abs_diff(position.x)
                        .max(e.position.y.abs_diff(position.y))
                        <= 1
            })
        })
        .cloned()
        .ok_or("step=3 observed=no-visible-creature need=authored-target")?;
    let health_before = target_health(&client, target.entity)
        .ok_or("step=3 observed=no-target-health need=entity-detail")?;
    client.take_events();
    let (disposition, sequence) = use_rune(&mut client, damage, Some(target.entity)).await?;
    if disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("step=3 observed={disposition:?} need=Committed").into());
    }
    wait_paid_effect(&mut client, damage, damage_before - 1, "step=3 targeted damage", |event| {
        let SessionEvent::WorldSpatialEntities(delta) = event else { return false };
        delta.value.update.iter().any(|entry| {
            entry.entity == target.entity && matches!(entry.detail,
                oteryn_dev_client::EntityDetail::Actor { health_percent, .. } if health_percent < health_before)
        }) || (delta.value.leave.contains(&target.entity)
            && delta.value.enter.iter().any(|entry| {
                entry.position == target.position && entry.kind == oteryn_dev_client::EntityKind::Corpse
            }))
    }).await?;
    check(3, sequence);

    require_rune_eligibility(&mut client, HEAL_RUNE_ID, 4)?;
    super::full_row_c02::earn_gold(&mut client, world, &metadata.spawns, 190)
        .await
        .map_err(|e| format!("step=4 observed=fund-healing-runes:{e}"))?;
    acquire(&mut client, world, &metadata.npcs, healing, HEAL_RUNE_ID, 2)
        .await
        .map_err(|error| format!("step=4 item={HEAL_RUNE_ID} {error}"))?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Creature
                    && e.position.floor == client.world_spatial().actor_position.floor
            })
        })
        .ok_or("step=4 observed=no-visible-healing-encounter")?
        .entity;
    let full = client
        .actor_vitals()
        .ok_or("step=4 observed=no-vitals")?
        .health;
    client.session_mut().attack_target(None).await?;
    let damaged = wait_self_health_below(&mut client, full).await?;
    let heal_before = quantity(&client, healing)?;
    client.take_events();
    let (disposition, sequence) = use_rune(&mut client, healing, None).await?;
    if disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("step=4 observed={disposition:?} need=Committed").into());
    }
    wait_paid_effect(
        &mut client,
        healing,
        heal_before - 1,
        "step=4 self healing",
        |event| matches!(event, SessionEvent::ActorVitals(delta) if delta.value.health > damaged),
    )
    .await?;
    check(4, sequence);

    let charges = quantity(&client, healing)?;
    let before = client
        .actor_vitals()
        .ok_or("step=5 observed=no-vitals")?
        .health;
    client.take_events();
    let (illegal, sequence) = use_rune(&mut client, healing, Some(target)).await?;
    drain_after(&mut client, sequence).await?;
    if illegal != oteryn_protocol_oteryn::world_object::UseDisposition::TargetIllegal
        || quantity(&client, healing)? != charges
        || client.actor_vitals().is_none_or(|v| v.health != before)
    {
        return Err(format!(
            "step=5 observed=reply:{illegal:?}/charges:{}/health:{} need=TargetIllegal-unchanged",
            quantity(&client, healing)?,
            client.actor_vitals().map_or(0, |v| v.health)
        )
        .into());
    }
    check(5, sequence);

    let old = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xc7).await?;
    if fresh.join_snapshot().game_session_id == old
        || quantity(&fresh, damage)? != damage_before - 1
        || quantity(&fresh, healing)? != charges
        || quantity(&fresh, blank)? != blank_before - 1
    {
        return Err(
            "step=6 observed=fresh-custody-mismatch need=durable-rune-charges-and-conjured-items"
                .into(),
        );
    }
    check(6, fresh.last_server_sequence());
    logout(&mut fresh).await
}

fn conjure_index(client: &mut DevClientSession) -> TestResult<NonZeroU32> {
    let catalogue = client
        .session_mut()
        .spell_catalog()?
        .ok_or("step=2 observed=no-spell-catalog")?;
    [ORDINARY_CONJURE, STARTER_CONJURE]
        .into_iter()
        .find_map(|words| {
            catalogue
                .entries
                .iter()
                .find(|e| e.words == words && e.available && e.rune_item.is_none())
                .and_then(|e| NonZeroU32::new(e.cast_index))
        })
        .ok_or_else(|| {
            format!("step=2 observed=no-eligible-conjure need={ORDINARY_CONJURE}|{STARTER_CONJURE}")
                .into()
        })
}

fn require_rune_eligibility(
    client: &mut DevClientSession,
    item_id: u32,
    step: usize,
) -> TestResult {
    let catalog = client
        .session_mut()
        .spell_catalog()?
        .ok_or("no served spell catalogue")?;
    let rune = catalog
        .entries
        .iter()
        .find(|entry| entry.rune_item == Some(item_id))
        .ok_or_else(|| {
            format!("step={step} observed=rune-{item_id}-missing-from-served-catalogue")
        })?;
    if rune.available {
        Ok(())
    } else {
        Err(format!(
            "step={step} observed=rune-{item_id}-unavailable;need={}-eligibility;level={}",
            rune.name, rune.level
        )
        .into())
    }
}

fn quantity(client: &DevClientSession, reference: NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("inventory absent")?
        .entries
        .iter()
        .filter(|e| e.item_definition_ref == reference)
        .map(|e| u64::from(e.count.get()))
        .sum())
}
fn handle(
    client: &DevClientSession,
    reference: NonZeroU32,
) -> TestResult<oteryn_protocol_oteryn::item_view::ItemHandle> {
    client
        .inventory()
        .and_then(|v| {
            v.entries
                .iter()
                .find(|e| e.item_definition_ref == reference)
        })
        .map(|e| e.handle)
        .ok_or_else(|| "rune handle absent".into())
}
fn target_health(
    client: &DevClientSession,
    target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> Option<u8> {
    match client.world_entities()?.get(&target)?.detail {
        oteryn_protocol_oteryn::world_spatial_entities::EntityDetail::Actor {
            health_percent,
            ..
        } => Some(health_percent),
        _ => None,
    }
}
fn check(step: usize, sequence: u64) {
    evidence(&format!(
        "C06_CHECK step={step} PASS server_sequence={sequence}"
    ));
}

fn seller_gap(item_id: u32, declared: usize, placed: usize) -> String {
    format!(
        "observed=no-reachable-seller-{item_id};declared={declared};placed={placed};need=walkable-path-to-item-seller"
    )
}

#[test]
fn c06_gap_distinguishes_missing_placement_from_missing_route() {
    assert!(seller_gap(3174, 30, 0).contains("declared=30;placed=0"));
    assert!(seller_gap(3174, 30, 1).contains("declared=30;placed=1"));
}

async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    reference: NonZeroU32,
    item_id: u32,
    units: u32,
) -> TestResult<u64> {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service as wire;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let trades: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/services/trade/trade-00000-00323.json"),
    )?)?;
    let key = format!("oteryn:item.tibia.i{item_id}");
    let sellers: BTreeSet<String> = trades["records"]
        .as_array()
        .ok_or("trade records absent")?
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
    let placed = npcs
        .npcs
        .iter()
        .filter(|n| sellers.contains(&n.key))
        .count();
    let (npc, path) = npcs
        .npcs
        .iter()
        .filter(|n| sellers.contains(&n.key))
        .filter_map(|n| {
            route(world, here, |at| {
                n.placements.iter().any(|p| {
                    p.floor == at.floor
                        && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)))
                            <= wire::TALK_RANGE_TILES
                })
            })
            .map(|p| (n, p))
        })
        .min_by_key(|(_, p)| p.len())
        .ok_or_else(|| seller_gap(item_id, sellers.len(), placed))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|es| {
            es.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        let distance = e
                            .position
                            .x
                            .abs_diff(i32::from(p.x))
                            .max(e.position.y.abs_diff(i32::from(p.y)));
                        e.position.floor == -i16::from(p.floor)
                            && (distance == 0
                                || (distance <= 2
                                    && npc.key == "oteryn:npc.xodet"
                                    && e.display_name.is_some_and(|name| {
                                        name.as_str().is_ok_and(|name| name == "Xodet")
                                    })))
                    })
            })
        })
        .map(|e| e.entity)
        .ok_or("vendor not visible")?;
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
            return Err(format!("observed=talk-{text}:{reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|e| format!("trade window: {e:?}"))?;
    let (i, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, o)| {
            o.side == wire::NpcTradeSide::Buy && o.item_definition_ref == reference.get()
        })
        .ok_or_else(|| format!("observed=item-{item_id}-not-offered"))?;
    let before = quantity(client, reference)?;
    client.take_events();
    let reply = npc_shop::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade closed")?.1,
            offer_index: u32::try_from(i)?,
            side: offer.side,
            quantity: units,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!("observed=buy-{item_id}:{reply:?} need=earned-player-funds").into());
    }
    inventory_count(
        client,
        reference,
        before + u64::from(units) * u64::from(offer.count),
    )
    .await
}

async fn use_rune(
    client: &mut DevClientSession,
    reference: NonZeroU32,
    entity: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
) -> TestResult<(oteryn_protocol_oteryn::world_object::UseDisposition, u64)> {
    use oteryn_protocol_oteryn::world_object as wire;
    let payload = wire::encode_item_use_intent(&wire::UseIntent {
        target: wire::UseTarget::Item(handle(client, reference)?),
        use_with: entity,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|e| format!("rune encode: {e:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (payload, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|e| format!("rune refusal: {e:?}"))?;
    let disposition =
        wire::decode_use_result(payload).map_err(|e| format!("rune result: {e:?}"))?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "rune command status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    Ok((disposition, reply.server_sequence))
}

async fn drain_after(client: &mut DevClientSession, sequence: u64) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
        if client.last_server_sequence() > sequence {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("pending deltas did not drain after sequence {sequence}").into());
        }
    }
}
async fn inventory_count(client: &mut DevClientSession, r: NonZeroU32, n: u64) -> TestResult<u64> {
    inventory_wait(client, &[(r, n)]).await
}
async fn inventory_wait(
    client: &mut DevClientSession,
    expected: &[(NonZeroU32, u64)],
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut sequence = None;
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event {
                sequence = Some(delta.server_sequence);
            }
        }
        if let Some(sequence) = sequence {
            let mut matched = true;
            for &(item, count) in expected {
                matched &= quantity(client, item)? == count;
            }
            if matched {
                return Ok(sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            let observed = expected
                .iter()
                .map(|&(item, _)| quantity(client, item))
                .collect::<TestResult<Vec<_>>>()?;
            return Err(format!("inventory delta observed={observed:?} need={expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}
async fn wait_self_health_below(client: &mut DevClientSession, n: u32) -> TestResult<u32> {
    wait_self_health(client, |v| v < n, "below").await
}
async fn wait_self_health(
    client: &mut DevClientSession,
    ok: impl Fn(u32) -> bool,
    label: &str,
) -> TestResult<u32> {
    let d = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        client.service_liveness(Duration::from_millis(30)).await?;
        for e in client.take_events() {
            if let SessionEvent::ActorVitals(x) = e
                && ok(x.value.health)
            {
                return Ok(x.value.health);
            }
        }
        if tokio::time::Instant::now() >= d {
            return Err(format!("health delta not {label} threshold").into());
        }
    }
}
async fn wait_paid_effect(
    client: &mut DevClientSession,
    rune: NonZeroU32,
    expected: u64,
    action: &str,
    proves_effect: impl Fn(&SessionEvent) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut effect_seen = false;
    let mut inventory_seen = false;
    loop {
        for event in client.take_events() {
            effect_seen |= proves_effect(&event);
            inventory_seen |= matches!(event, SessionEvent::Inventory(_));
        }
        let observed = quantity(client, rune)?;
        if effect_seen && inventory_seen && observed == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "{action}: effect={effect_seen};inventory-delta={inventory_seen};charges={observed};expected={expected}"
            ).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let d = tokio::time::Instant::now() + Duration::from_secs(75);
    loop {
        let r = client.session_mut().logout().await?;
        if r.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= d {
            return Err(format!("logout observed={:?}", r.result).into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn production_item_ids_are_distinct() -> TestResult {
        let i = provisioning::item_index()?;
        let refs = [DAMAGE_RUNE_ID, HEAL_RUNE_ID, BLANK_RUNE_ID]
            .map(|id| provisioning::item_reference(&i, id))
            .into_iter()
            .collect::<Result<BTreeSet<_>, _>>()?;
        assert_eq!(refs.len(), 3);
        Ok(())
    }
}
