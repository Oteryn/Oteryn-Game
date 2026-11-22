//! Ordinary acquisition and item lifecycle checks for the E2EF3 item rows.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::npc_shop;
use oteryn_protocol_oteryn::world_object::{self, MapUseTarget, UseIntent, UseTarget};
use oteryn_protocol_oteryn::{item_view as items, npc_service as npc, timed_item as timed};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

pub(super) fn owns(id: &str) -> bool {
    matches!(id, "I08" | "I12" | "I13" | "I15")
}

struct Check {
    id: &'static str,
    action: &'static str,
    started: std::time::Instant,
}

impl Check {
    fn new(id: &'static str, action: &'static str) -> Self {
        Self {
            id,
            action,
            started: std::time::Instant::now(),
        }
    }

    fn finish(self, result: TestResult<()>) {
        let seconds = self.started.elapsed().as_secs_f64();
        let id = self.id;
        match result {
            Ok(()) => evidence(&format!(
                "e2e_all scenario={id}.full status=PASS seconds={seconds:.3}"
            )),
            Err(error) => {
                let cause =
                    verdict::classify(error.as_ref()).suffix(&format!("{id}/{}", self.action));
                evidence(&format!(
                    "e2e_all scenario={id}.full status=FAIL seconds={seconds:.3}{cause}"
                ));
                let observed = error.to_string().replace(['\r', '\n'], " ");
                evidence(&format!(
                    "E2EF3_GAP row={id} unexecuted={} observed={observed}",
                    self.action
                ));
            }
        }
    }
}

pub(super) async fn run(
    observations: TestResult<DevClientSession>,
    clients: &SeamClients<'_>,
    primal: bool,
    covet: bool,
) -> TestResult<DevClientSession> {
    for (id, used, action, route) in [
        (
            "I08",
            primal,
            "obtain-primal-bag",
            "Primal Bag 39546: no carried bag observed, Hazard 12 Primal Ordeal reward held and no Plunder Patriarch spawn in the compiled town regions",
        ),
        (
            "I13",
            covet,
            "obtain-bag-you-covet",
            "Bag You Covet 43895: no carried bag observed, Rotten Blood bosses Bakragore, Chagorz, Ichgahal, Murcion and Vemiath have no spawn in the compiled town regions and Crimson Offering requires a Bakragore kill",
        ),
    ] {
        if !(observations.is_ok() && used) {
            if used {
                let observed = observations
                    .as_ref()
                    .err()
                    .map_or_else(String::new, ToString::to_string);
                Check::new(id, "relogin-reward").finish(Err(observed.into()));
            } else {
                Check::new(id, action).finish(Err(route.into()));
            }
        }
    }
    let mut mining = Check::new("I12", "obtain-pick-and-shiny-stone");
    let mut ring = Check::new("I15", "obtain-energy-ring");
    let mut client = match observations {
        Ok(client) => client,
        Err(error) => match connect_world(clients, 0x79).await {
            Ok(client) => client,
            Err(rejoin) => {
                let detail = format!("starter={error}; recovery-admission={rejoin}");
                mining.finish(Err(detail.clone().into()));
                ring.finish(Err(detail.into()));
                return Err(rejoin);
            }
        },
    };
    let setup = (|| -> TestResult<_> {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let world = load_world_generation(
            bytes,
            &pin,
            WorldId::decode(&clients.world)?,
            ChannelId::decode(&clients.channel)?,
        )?;
        Ok((metadata, world, provisioning::item_index()?))
    })();
    match setup {
        Ok((metadata, world, index)) => {
            let result = refine(&mut client, clients, &mut mining, world, &metadata, &index).await;
            mining.finish(result);
            let result =
                energy_ring(&mut client, clients, &mut ring, world, &metadata, &index).await;
            ring.finish(result);
        }
        Err(error) => {
            mining.finish(Err(error.to_string().into()));
            ring.finish(Err(error));
        }
    }
    Ok(client)
}

fn carried(client: &DevClientSession, reference: NonZeroU32) -> Option<items::ItemHandle> {
    client
        .inventory()?
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == reference)
        .map(|entry| entry.handle)
}

async fn rejoin(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult<()> {
    let before = client.join_snapshot().game_session_id;
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(
            format!("logout={logout} attempts={attempts} busy_timed_out={busy_timed_out}").into(),
        );
    }
    *client = connect_world(clients, 0x79).await?;
    if client.join_snapshot().game_session_id == before {
        return Err(format!("reused game session {before:?}").into());
    }
    Ok(())
}

async fn buy_pick(
    client: &mut DevClientSession,
    world: &BundleWorld,
    pick: NonZeroU32,
    coin: NonZeroU32,
) -> TestResult<()> {
    // Crystal's Lubo sells the pick for 50 gp. This capture has his placed shop at floor 7.
    let shop = TilePos {
        x: 32489,
        y: 32119,
        floor: -7,
    };
    walk_near(client, world, shop, 2).await?;
    let target = client
        .world_entities()
        .and_then(|entries| {
            entries.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && tile(entry.position).ok() == Some(shop)
            })
        })
        .ok_or("Lubo not visible at 32489,32119,7")?
        .entity;
    for text in ["hi", "trade"] {
        let response = npc_shop::talk(
            client.session_mut(),
            &npc::NpcTalkIntent {
                npc_actor: target,
                text: text.into(),
            },
        )
        .await?;
        if response != npc::NpcIntentDisposition::Ok {
            return Err(format!("Lubo {text}: {response:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(npc::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = npc::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("Lubo trade window: {error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == npc::NpcTradeSide::Buy
                && offer.item_definition_ref == pick.get()
                && offer.unit_price == 50
        })
        .ok_or("Lubo live window lacks 50 gp pick offer")?;
    let before = quantities(client)?.get(&coin.get()).copied().unwrap_or(0);
    let response = npc_shop::buy(
        client.session_mut(),
        &npc::NpcTradeIntent {
            npc_actor: target,
            catalogue_revision: window.open.ok_or("Lubo trade window closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if response != npc::NpcIntentDisposition::Ok {
        return Err(
            format!("Lubo pick buy: reply={response:?} carried_gold={before} price=50").into(),
        );
    }
    let stock = wait(client, Duration::from_secs(5), |client| {
        let stock = quantities(client).ok()?;
        stock.contains_key(&pick.get()).then_some(stock)
    })
    .await?;
    let after = stock.get(&coin.get()).copied().unwrap_or(0);
    if stock.get(&pick.get()) != Some(&1) || before.checked_sub(after) != Some(50) {
        return Err(format!("Lubo buy expected one pick and 50 gp debit; gold before={before} inventory after={stock:?}").into());
    }
    Ok(())
}

async fn walk_near(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: TilePos,
    range: u16,
) -> TestResult<()> {
    let here = tile(client.world_spatial().actor_position)?;
    let path = route_avoiding(
        world,
        here,
        |at| {
            at.floor == target.floor
                && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= range
        },
        |at| at != WOLF_BLOCKED_LANDING,
    )
    .ok_or_else(|| format!("no ordinary route from {here:?} to {target:?}"))?;
    walk(client, world, &path).await
}

async fn wait<T>(
    client: &mut DevClientSession,
    limit: Duration,
    mut observe: impl FnMut(&DevClientSession) -> Option<T>,
) -> TestResult<T> {
    let deadline = tokio::time::Instant::now() + limit;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        if let Some(value) = observe(client) {
            return Ok(value);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "deadline; inventory={:?} vitals={:?}",
                client.inventory(),
                client.actor_vitals()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

const WOLF_BLOCKED_LANDING: TilePos = TilePos {
    x: 32428,
    y: 32158,
    floor: -7,
};

async fn hunt_for(
    client: &mut DevClientSession,
    world: &QualificationWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    item: u32,
    ready: impl Fn(&DevClientSession) -> bool,
) -> TestResult<()> {
    if ready(client) {
        return Ok(());
    }
    let key = format!("oteryn:item.tibia.i{item}");
    let sources: BTreeSet<_> = spawns
        .creatures
        .iter()
        .filter(|creature| {
            world.rewards.row(&creature.key).is_ok_and(|reward| {
                reward.loot_table.entries.iter().any(|entry| {
                    entry.item.production_key == key && entry.probability_ppm != Some(0)
                })
            })
        })
        .map(|creature| creature.key.as_str())
        .collect();
    let goals: BTreeSet<_> = spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(|point| {
                let creature = &spawns.creatures.get(point.creature as usize)?.key;
                sources
                    .contains(creature.as_str())
                    .then_some((source.floor, point))
            })
        })
        .flat_map(|(floor, point)| {
            (-1..=1).flat_map(move |dy| {
                (-1..=1).filter_map(move |dx| {
                    if dx == 0 && dy == 0 {
                        return None;
                    }
                    Some(TilePos {
                        x: point.x.checked_add_signed(dx)?,
                        y: point.y.checked_add_signed(dy)?,
                        floor,
                    })
                })
            })
        })
        .filter(|at| {
            world
                .bundle
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|cell| !cell.protection_zone())
        })
        .collect();
    if goals.is_empty() {
        return Err(format!(
            "{key}: no compiled walk target for admitted corpse loot sources {sources:?}"
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut attempts = 0;
    while !ready(client) && tokio::time::Instant::now() < deadline {
        let here = tile(client.world_spatial().actor_position)?;
        let path = find_route(here, |at| goals.contains(&at), |at, step| {
            destination(&world.bundle, at, step).filter(|next| *next != WOLF_BLOCKED_LANDING)
        }).map_err(|visited| format!("{key}: no ordinary step route from {here:?} to corpse loot sources {sources:?}; visited {visited} cells"))?;
        walk(client, &world.bundle, &path).await?;
        client_hunt::hunt(client).await?;
        attempts += 1;
    }
    if ready(client) {
        return Ok(());
    }
    Err(format!(
        "{key}: acquisition missing after {attempts} hunts; inventory={:?}",
        client.inventory()
    )
    .into())
}

async fn refine(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    check: &mut Check,
    world: &QualificationWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult<()> {
    let pick = provisioning::item_reference(index, 3456)?;
    let stone = provisioning::item_reference(index, 10310)?;
    let gold = provisioning::item_reference(index, 3031)?;
    if carried(client, pick).is_none() {
        hunt_for(client, world, &metadata.spawns, 3031, |client| {
            quantities(client)
                .is_ok_and(|stock| stock.get(&gold.get()).is_some_and(|coins| *coins >= 50))
        })
        .await?;
        buy_pick(client, &world.bundle, pick, gold).await?;
    }
    if carried(client, stone).is_none() {
        hunt_for(client, world, &metadata.spawns, 10310, |client| {
            carried(client, stone).is_some()
        })
        .await?;
    }
    let source = carried(client, pick).ok_or("pick 3456 absent after acquisition")?;
    let target = carried(client, stone).ok_or("Shiny Stone 10310 absent after acquisition")?;
    let mut expected = quantities(client)?;
    let quantity = expected
        .get_mut(&stone.get())
        .ok_or("stone quantity absent")?;
    *quantity -= 1;
    if *quantity == 0 {
        expected.remove(&stone.get());
    }
    let allowed = [3043, 3031, 3035, 3028]
        .map(|id| provisioning::item_reference(index, id))
        .into_iter()
        .collect::<TestResult<Vec<_>>>()?;
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(target)),
        use_with_position: None,
    })
    .map_err(|error| format!("pick USE intent: {error:?}"))?;
    check.action = "use-with";
    let response = client.session_mut().world_context_use(&payload).await?;
    let disposition = world_object::decode_use_result(&response.payload)
        .map_err(|error| format!("pick USE result: {error:?}"))?;
    if response.status != CommandStatus::Accepted
        || disposition != world_object::UseDisposition::Committed
    {
        return Err(format!(
            "pick 3456 USE with stone 10310: {:?}/{disposition:?}",
            response.status
        )
        .into());
    }
    check.action = "stone-consumed";
    let saved = wait(client, Duration::from_secs(5), |client| {
        let observed = quantities(client).ok()?;
        (observed.get(&stone.get()) == expected.get(&stone.get())).then_some(observed)
    })
    .await?;
    check.action = "retained-draw";
    if !allowed.into_iter().any(|reference| {
        let mut candidate = expected.clone();
        *candidate.entry(reference.get()).or_default() += 1;
        candidate == saved
    }) {
        return Err(
            format!("pick refinement before reward={expected:?} observed={saved:?}").into(),
        );
    }
    check.action = "relogin-refinement";
    rejoin(client, clients).await?;
    let observed = quantities(client)?;
    if observed != saved {
        return Err(format!("refinement before={saved:?} after={observed:?}").into());
    }
    check.action = "retained-draw";
    Err("exact replay of the acknowledged USE command ID is unavailable through the dev client; pool output, stone consumption and reloaded inventory matched".into())
}

fn equipped(client: &DevClientSession, active: NonZeroU32) -> Option<items::ItemHandle> {
    client
        .inventory()?
        .equipment
        .iter()
        .find(|entry| {
            entry.slot == items::EquipmentSlot::Ring && entry.item.item_definition_ref == active
        })
        .map(|entry| entry.item.handle)
}

async fn move_ring(
    client: &mut DevClientSession,
    source: items::ItemHandle,
    destination: items::ItemMoveDestination,
) -> TestResult<()> {
    let result = client
        .move_item(&items::ItemMoveIntent {
            source,
            destination,
        })
        .await?;
    if result.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("ring MOVE: {result:?}").into());
    }
    Ok(())
}

async fn inspect_timer(
    client: &mut DevClientSession,
    target: items::ItemHandle,
    revision: u64,
) -> TestResult<timed::TimedItemResult> {
    let payload = timed::encode_timed_item_intent(timed::TimedItemIntent {
        action: timed::TimedAction::Inspect,
        target,
        expected_revision: revision,
        npc: None,
    })
    .map_err(|error| format!("ring timer intent: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            timed::CAPABILITY_TIMED_ITEM_V1,
            timed::COMMAND_TYPE_TIMED_ITEM_INTENT,
            &payload,
        )
        .await?;
    let result = timed::decode_timed_item_result(&response.payload)
        .map_err(|error| format!("ring timer result: {error:?}"))?;
    if response.status != CommandStatus::Accepted
        || result.disposition != timed::TimedDisposition::Ok
    {
        return Err(format!(
            "ring timer revision {revision}: {:?}/{result:?}",
            response.status
        )
        .into());
    }
    Ok(result)
}

async fn energy_ring(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    check: &mut Check,
    world: &QualificationWorld,
    metadata: &oteryn_world_bundle::bundle::Visited,
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult<()> {
    let base = provisioning::item_reference(index, 3051)?;
    let active = provisioning::item_reference(index, 3088)?;
    if carried(client, base).is_none() && equipped(client, active).is_none() {
        // The Demona chest is outside the town crop. Hunt a compiled loot source instead.
        hunt_for(client, world, &metadata.spawns, 3051, |client| {
            carried(client, base).is_some()
        })
        .await?;
    }
    check.action = "equip";
    if let Some(source) = carried(client, base) {
        move_ring(
            client,
            source,
            items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
        )
        .await?;
    }
    let handle = wait(client, Duration::from_secs(5), |client| {
        equipped(client, active)
    })
    .await?;
    let timer = inspect_timer(client, handle, 0).await?;
    if !timer
        .remaining_ms
        .is_some_and(|ms| (1..=600_000).contains(&ms))
    {
        return Err(format!("equipped timer: {timer:?}").into());
    }

    check.action = "damage-mana-at-two-to-one";
    observe_shield(client, check, &world.bundle).await?;
    check.action = "unequip-pause";
    move_ring(client, handle, items::ItemMoveDestination::MainBackpack).await?;
    let paused_handle = wait(client, Duration::from_secs(5), |client| {
        equipped(client, active)
            .is_none()
            .then(|| carried(client, base))
            .flatten()
    })
    .await?;
    let paused = inspect_timer(client, paused_handle, 0).await?;
    client.service_liveness(Duration::from_secs(2)).await?;
    let still_paused = inspect_timer(client, paused_handle, paused.revision).await?;
    if paused.remaining_ms.is_none_or(|ms| ms == 0)
        || paused.remaining_ms >= timer.remaining_ms
        || still_paused != paused
    {
        return Err(format!("unequipped timer first={paused:?} later={still_paused:?}").into());
    }

    check.action = "relogin-timer";
    rejoin(client, clients).await?;
    let paused_handle =
        carried(client, base).ok_or("paused Energy Ring 3051 missing after login")?;
    let restored = inspect_timer(client, paused_handle, paused.revision).await?;
    if restored != paused {
        return Err(format!("paused timer before={paused:?} restored={restored:?}").into());
    }

    check.action = "expiry";
    let previous = quantities(client)?.get(&base.get()).copied().unwrap_or(0);
    move_ring(
        client,
        paused_handle,
        items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
    )
    .await?;
    wait(client, Duration::from_secs(5), |client| {
        equipped(client, active)
    })
    .await?;
    let remaining = paused
        .remaining_ms
        .ok_or("paused timer missing remaining budget")?;
    let retired = |client: &DevClientSession| {
        let count = quantities(client)
            .ok()?
            .get(&base.get())
            .copied()
            .unwrap_or(0);
        let ring_slot_empty = client
            .inventory()?
            .equipment
            .iter()
            .all(|entry| entry.slot != items::EquipmentSlot::Ring);
        (ring_slot_empty && count + 1 == previous).then_some(())
    };
    wait(
        client,
        Duration::from_millis(remaining) + Duration::from_secs(5),
        retired,
    )
    .await?;
    check.action = "relogin-timer";
    rejoin(client, clients).await?;
    retired(client).ok_or_else(|| {
        format!(
            "expired ring reappeared on fresh admission; inventory={:?}",
            client.inventory()
        )
        .into()
    })
}

async fn observe_shield(
    client: &mut DevClientSession,
    check: &mut Check,
    world: &BundleWorld,
) -> TestResult<()> {
    // Native 2126 applies one 100-point energy hit per entry, with no residual tick.
    let field = |at: TilePos| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|cell| {
            !cell.protection_zone()
                && (0..cell.depths().iter().filter(|depth| **depth == 0).count())
                    .filter_map(|ordinal| u8::try_from(ordinal).ok())
                    .any(|ordinal| cell.appearance(ordinal) == Some(2126))
        })
    };
    let blocked = WOLF_BLOCKED_LANDING;
    let here = tile(client.world_spatial().actor_position)?;
    let mut path = route_avoiding(world, here, field, |at| at != blocked).ok_or_else(|| {
        format!("native Energy Field 2126 has no walk route from {here:?} avoiding {blocked:?}")
    })?;
    let last = path.pop().ok_or("already standing on Energy Field 2126")?;
    let retreat = match last {
        StepDirection::North => StepDirection::South,
        StepDirection::South => StepDirection::North,
        StepDirection::East => StepDirection::West,
        StepDirection::West => StepDirection::East,
        _ => return Err("energy field route ended in a diagonal step".into()),
    };
    walk(client, world, &path).await?;
    for _ in 0..10 {
        let before = *client
            .actor_vitals()
            .ok_or("energy field approach did not project actor vitals")?;
        let absorbed = 100.min(before.mana / 2);
        let mana = before.mana - 2 * absorbed;
        let health = before.health.saturating_sub(100 - absorbed);
        if absorbed == 0 || health == 0 {
            return Err(format!(
                "Energy Field 2126 cannot safely observe shield: vitals={before:?}"
            )
            .into());
        }
        walk(client, world, &[last]).await?;
        let after = wait(client, Duration::from_secs(5), |client| {
            client
                .actor_vitals()
                .copied()
                .filter(|after| after.health < before.health || after.mana < before.mana)
        })
        .await?;
        if after.mana != mana {
            return Err(format!("Energy Field 2126 damage=100 expected mana={mana} before={before:?} after={after:?}").into());
        }
        check.action = "health-spill";
        if after.health != health {
            return Err(format!("Energy Field 2126 damage=100 expected health={health} before={before:?} after={after:?}").into());
        }
        walk(client, world, &[retreat]).await?;
        if health < before.health {
            return Ok(());
        }
    }
    Err(format!(
        "Energy Field 2126 did not spill into health; last vitals={:?}",
        client.actor_vitals()
    )
    .into())
}
