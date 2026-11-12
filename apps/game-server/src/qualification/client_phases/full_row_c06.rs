//! C06 full-row journey, derived from the matrix, Crystal re-audit, and the native
//! `rune_use_bridge`/`actor_spell` owners.
//!
//! 1. Admit the provisioned character, walk to compiled-world rune merchants, say `hi` and
//!    `trade`, buy charged Light Magic Missile (3174) and Intense Healing (3152) runes, and
//!    require each NPC reply and pushed inventory delta.
//! 2. Walk to an authored creature, send rune USE with its entity as target, and require an
//!    accepted `Committed` reply, a lower target-health entity delta, and exactly one charge paid.
//! 3. Let that creature damage the character, send the healing-rune USE on the character, and
//!    require `Committed`, a higher-health vitals delta, and exactly one charge paid.
//! 4. Send healing-rune USE at the hostile creature (an illegal healing target); drain/await the
//!    pending server deltas before reading state, then require `TargetIllegal`, no health change,
//!    and unchanged healing-rune charges.
//! 5. Send the served level-one `exevo infir con` conjure CAST, require `Cast`, its mana delta,
//!    and an inventory delta adding the conjured arrows; immediately repeat it and require the
//!    observed cooldown refusal with unchanged mana and inventory.
//! 6. Logout and freshly admit the character; require a new game session and the exact remaining
//!    damage/healing rune charges plus conjured arrows, proving durable custody/payment state.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

const DAMAGE_RUNE_ID: u32 = 3174;
const HEAL_RUNE_ID: u32 = 3152;
const CONJURED_ID: u32 = 21470;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await;
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
    let arrows = provisioning::item_reference(&index, CONJURED_ID)?;
    let mut client = connect_world(clients, 0xc6).await?;
    acquire(&mut client, world, &metadata.npcs, damage, DAMAGE_RUNE_ID).await?;
    acquire(&mut client, world, &metadata.npcs, healing, HEAL_RUNE_ID).await?;
    check(1, client.last_server_sequence());

    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|e| e.kind == oteryn_dev_client::EntityKind::Creature)
        })
        .cloned()
        .ok_or("step=2 observed=no-visible-creature need=authored-target")?;
    let damage_before = quantity(&client, damage)?;
    let health_before = target_health(&client, target.entity)
        .ok_or("step=2 observed=no-target-health need=entity-detail")?;
    let (disposition, sequence) = use_rune(
        &mut client,
        damage,
        Some(target.entity),
        Some(target.position),
    )
    .await?;
    if disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("step=2 observed={disposition:?} need=Committed").into());
    }
    wait_target_health_below(&mut client, target.entity, health_before).await?;
    inventory_count(&mut client, damage, damage_before - 1).await?;
    check(2, sequence);

    let own = *client
        .world_entities()
        .ok_or("step=3 observed=no-entities")?
        .own_identity();
    let self_ref = client
        .world_entities()
        .and_then(|entities| entities.iter().find(|e| e.entity.identity == own))
        .map(|e| e.entity)
        .ok_or("step=3 observed=no-own-entity")?;
    let full = client
        .actor_vitals()
        .ok_or("step=3 observed=no-vitals")?
        .health;
    client.session_mut().attack_target(None).await?;
    let damaged = wait_self_health_below(&mut client, full).await?;
    let heal_before = quantity(&client, healing)?;
    let own_position = client.world_spatial().actor_position;
    let (disposition, sequence) =
        use_rune(&mut client, healing, Some(self_ref), Some(own_position)).await?;
    if disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("step=3 observed={disposition:?} need=Committed").into());
    }
    wait_self_health_above(&mut client, damaged).await?;
    inventory_count(&mut client, healing, heal_before - 1).await?;
    check(3, sequence);

    let charges = quantity(&client, healing)?;
    let before = client
        .actor_vitals()
        .ok_or("step=4 observed=no-vitals")?
        .health;
    client.take_events();
    let (illegal, sequence) = use_rune(
        &mut client,
        healing,
        Some(target.entity),
        Some(target.position),
    )
    .await?;
    drain_after(&mut client, sequence).await?;
    if illegal != oteryn_protocol_oteryn::world_object::UseDisposition::TargetIllegal
        || quantity(&client, healing)? != charges
        || client.actor_vitals().is_none_or(|v| v.health != before)
    {
        return Err(format!(
            "step=4 observed=reply:{illegal:?}/charges:{}/health:{} need=TargetIllegal-unchanged",
            quantity(&client, healing)?,
            client.actor_vitals().map_or(0, |v| v.health)
        )
        .into());
    }
    check(4, sequence);

    let spell = spell_index(&mut client, "exevo infir con")?;
    let mana = client
        .actor_vitals()
        .ok_or("step=5 observed=no-vitals")?
        .mana;
    let arrow_before = quantity(&client, arrows)?;
    let cast = client.cast_spell(spell, SpellTarget::None, false).await?;
    if cast.disposition != SpellCastDisposition::Cast
        || client.actor_vitals().is_none_or(|v| v.mana >= mana)
    {
        return Err(format!("step=5 observed={cast:?} need=Cast-mana-delta").into());
    }
    inventory_at_least(&mut client, arrows, arrow_before + 1).await?;
    let paid_mana = client
        .actor_vitals()
        .ok_or("step=5 observed=no-post-cast-vitals")?
        .mana;
    let made = quantity(&client, arrows)?;
    let refused = client.cast_spell(spell, SpellTarget::None, false).await?;
    drain_after(&mut client, refused.result_server_sequence).await?;
    if refused.disposition != SpellCastDisposition::CoolingDown
        || client.actor_vitals().is_none_or(|v| v.mana != paid_mana)
        || quantity(&client, arrows)? != made
    {
        return Err(
            format!("step=5 observed=repeat:{refused:?} need=CoolingDown-unchanged").into(),
        );
    }
    check(5, refused.result_server_sequence);

    let old = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xc7).await?;
    if fresh.join_snapshot().game_session_id == old
        || quantity(&fresh, damage)? != damage_before - 1
        || quantity(&fresh, healing)? != charges
        || quantity(&fresh, arrows)? != made
    {
        return Err(
            "step=6 observed=fresh-custody-mismatch need=durable-rune-charges-and-conjured-items"
                .into(),
        );
    }
    check(6, fresh.last_server_sequence());
    logout(&mut fresh).await
}

fn spell_index(client: &mut DevClientSession, words: &str) -> TestResult<NonZeroU32> {
    use oteryn_protocol_oteryn::actor_spell_catalog as wire;
    let snapshot = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("step=5 observed=no-spell-catalog")?;
    let entries = wire::decode_catalog(&snapshot.payload)
        .map_err(|e| format!("step=5 observed=bad-catalog:{e:?}"))?;
    entries
        .iter()
        .find(|e| e.words == words && e.available)
        .and_then(|e| NonZeroU32::new(e.cast_index))
        .ok_or_else(|| {
            format!("step=5 observed=unavailable-{words} need=served-level-one-conjure").into()
        })
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

async fn acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    reference: NonZeroU32,
    item_id: u32,
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
    let (npc, path) = npcs
        .npcs
        .iter()
        .filter(|n| sellers.contains(&n.key))
        .filter_map(|n| {
            route(world, here, |at| {
                n.placements
                    .iter()
                    .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
            })
            .map(|p| (n, p))
        })
        .min_by_key(|(_, p)| p.len())
        .ok_or_else(|| format!("step=1 observed=no-reachable-seller-{item_id}"))?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|es| {
            es.iter().find(|e| {
                e.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        e.position.x == i32::from(p.x)
                            && e.position.y == i32::from(p.y)
                            && e.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|e| e.entity)
        .ok_or("step=1 vendor not visible")?;
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
            return Err(format!("step=1 observed=talk-{text}:{reply:?}").into());
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
        .ok_or_else(|| format!("step=1 observed=item-{item_id}-not-offered"))?;
    let before = quantity(client, reference)?;
    let reply = npc_shop::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("trade closed")?.1,
            offer_index: u32::try_from(i)?,
            side: offer.side,
            quantity: 2,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(
            format!("step=1 observed=buy-{item_id}:{reply:?} need=earned-player-funds").into(),
        );
    }
    inventory_at_least(client, reference, before + 2).await
}

async fn use_rune(
    client: &mut DevClientSession,
    reference: NonZeroU32,
    entity: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    position: Option<ActorPosition>,
) -> TestResult<(oteryn_protocol_oteryn::world_object::UseDisposition, u64)> {
    use oteryn_protocol_oteryn::{actor_spell::SpellTargetPosition, world_object as wire};
    let payload = wire::encode_item_use_intent(&wire::UseIntent {
        target: wire::UseTarget::Item(handle(client, reference)?),
        use_with: entity,
        use_with_map_item: None,
        use_with_position: position.map(|p| SpellTargetPosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        }),
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
    inventory_wait(client, r, |v| v == n, n).await
}
async fn inventory_at_least(
    client: &mut DevClientSession,
    r: NonZeroU32,
    n: u64,
) -> TestResult<u64> {
    inventory_wait(client, r, |v| v >= n, n).await
}
async fn inventory_wait(
    client: &mut DevClientSession,
    r: NonZeroU32,
    ok: impl Fn(u64) -> bool,
    n: u64,
) -> TestResult<u64> {
    let d = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for e in client.take_events() {
            if let SessionEvent::Inventory(x) = e
                && ok(quantity(client, r)?)
            {
                return Ok(x.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= d {
            return Err(
                format!("inventory delta observed={} need={n}", quantity(client, r)?).into(),
            );
        }
        client.service_liveness(Duration::from_millis(20)).await?
    }
}
async fn wait_self_health_below(client: &mut DevClientSession, n: u32) -> TestResult<u32> {
    wait_self_health(client, |v| v < n, "below").await
}
async fn wait_self_health_above(client: &mut DevClientSession, n: u32) -> TestResult<u32> {
    wait_self_health(client, |v| v > n, "above").await
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
async fn wait_target_health_below(
    client: &mut DevClientSession,
    target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    n: u8,
) -> TestResult {
    let d = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
        if target_health(client, target).is_some_and(|v| v < n) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= d {
            return Err(format!("step=2 target health did not fall below {n}").into());
        }
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
        let refs = [DAMAGE_RUNE_ID, HEAL_RUNE_ID, CONJURED_ID]
            .map(|id| provisioning::item_reference(&i, id))
            .into_iter()
            .collect::<Result<BTreeSet<_>, _>>()?;
        assert_eq!(refs.len(), 3);
        Ok(())
    }
}
