//! P04 full-row player scenario, derived from the matrix/re-audit and the
//! `task_board` durable owner:
//! 1. Admit the node-boot character and send QUERY; expect the complete weekly,
//!    bounty, shop, points and soulseal projection.
//! 2. Send START_WEEKLY with an invalid/locked difficulty; expect a refusal and
//!    no board delta (the negative path).
//! 3. Admit the capacity fixture at seeded knight level 650, send START_WEEKLY(Beginner),
//!    and expect an accepted board containing kill and delivery assignments.
//! 4. Send DELIVER for a real assignment; expect either item consumption plus
//!    completion/XP deltas, or the explicit insufficient-items refusal.
//! 5. Send OPEN_BOUNTY, SELECT_BOUNTY, REROLL_BOUNTY and CLAIM_BOUNTY; expect
//!    each accepted transition and its offers/current/reward deltas.
//! 6. Send BUY_SHOP for a projected offer and BUY_TALISMAN/UPGRADE_TALISMAN;
//!    expect the quoted balance/custody deltas, or their resource refusal.
//! 7. Query after the server-owned weekly reset boundary; expect completed
//!    weekly rewards in points and soulseals and a new-startable week.
//! 8. Logout, freshly admit the same character, QUERY again, and require all
//!    durable board, bounty, shop, soulseal and talisman state to match.

use super::*;
use oteryn_dev_client::client_runtime::task_board;
use oteryn_protocol_oteryn::task_board::full::{
    CAPABILITY_TASK_BOARD_DAILY_BOUNTY_V1, Reply, Request,
};

struct Journey {
    started: std::time::Instant,
    failures: Vec<String>,
}

impl Journey {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            failures: Vec::new(),
        }
    }

    fn check(&mut self, step: u8, condition: bool, observed: impl std::fmt::Debug) {
        if !condition {
            verdict::record_assertion("task-board-check");
        }
        let observed = format!("{observed:?}").replace('\n', " ");
        if condition {
            evidence(&format!(
                "P04_CHECK step={step} status=PASS observed={observed}"
            ));
        } else {
            evidence(&format!(
                "P04_CHECK step={step} status=FAIL observed={observed}"
            ));
            self.failures.push(format!("step-{step}:{observed}"));
        }
    }

    fn finish(self) {
        let status = if self.failures.is_empty() {
            "PASS"
        } else {
            "FAIL"
        };
        match self.failures.first() {
            Some(failure) => evidence(&format!(
                "P04_GAP status=FAIL observed={}",
                failure.replace(char::is_whitespace, "_")
            )),
            None => evidence("P04_GAP status=PASS"),
        }
        let suffix = self.failures.first().map_or(String::new(), |failure| {
            format!(" evidence={}", failure.replace(' ', "_"))
        });
        evidence(&format!(
            "e2e_all scenario=P04.full status={status} seconds={:.3}{suffix}",
            self.started.elapsed().as_secs_f64()
        ));
    }
}

fn request(action: u8, selection: u16) -> Request {
    Request {
        action,
        selection,
        race: String::new(),
    }
}

async fn send(
    client: &mut oteryn_dev_client::DevClientSession,
    action: u8,
    selection: u16,
) -> TestResult<Reply> {
    task_board::send_full(client.session_mut(), &request(action, selection))
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(
                format!("task-board action={action} selection={selection}: {error:?}"),
                error,
            )
        })
}

async fn query(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<Reply> {
    send(client, 4, 0).await
}

async fn await_weekly_reset(
    client: &mut oteryn_dev_client::DevClientSession,
    before: &Reply,
) -> TestResult<Reply> {
    let until = tokio::time::Instant::now() + Duration::from_secs(25);
    loop {
        let current = query(client).await?;
        if current.period > before.period {
            if before.points.checked_add(before.weekly.pending_points) != Some(current.points)
                || before
                    .soulseals
                    .checked_add(before.weekly.pending_soulseals)
                    != Some(current.soulseals)
                || before.weekly.pending_points == 0
                || before.weekly.pending_soulseals == 0
            {
                return Err(format!(
                    "step=weekly-reset award mismatch before={before:?} after={current:?}"
                )
                .into());
            }
            return Ok(current);
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!("step=weekly-reset need=completed-tasks-and-real-weekly-boundary period={} pending-points={} pending-soulseals={}", before.period, before.weekly.pending_points, before.weekly.pending_soulseals).into());
        }
        client.service_liveness(Duration::from_secs(2)).await?;
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut journey = Journey::new();
    let result = {
        let start = tokio::time::Instant::now();
        let base_budget = Duration::from_secs(240);
        let (publish_count, mut count) = tokio::sync::oneshot::channel();
        let journey = verdict::observe(run(&mut journey, clients, publish_count));
        tokio::pin!(journey);
        tokio::select! {
            biased;
            required = &mut count => {
                let bounty_budget = required.map_or(Duration::ZERO, |kills| {
                    full_row_p05::P04_BOUNTY_KILL_BUDGET * kills
                });
                let deadline = start + base_budget + bounty_budget;
                verdict::indexed_timeout(
                    deadline.saturating_duration_since(tokio::time::Instant::now()),
                    journey,
                ).await
            }
            result = verdict::indexed_timeout(base_budget, &mut journey) => result,
        }
    };
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            verdict::record_error(error.as_ref());
            journey.check(0, false, format!("journey-error:{error}"))
        }
        Err(_) => journey.check(0, false, "journey-timeout"),
    }
    journey.finish();
}

async fn run(
    journey: &mut Journey,
    clients: &SeamClients<'_>,
    publish_count: tokio::sync::oneshot::Sender<u32>,
) -> TestResult {
    let underlevel =
        character_prep::dedicated_character(clients, "NODE_BOOT_Q_P04_399_ACCOUNT").await?;
    let underlevel_clients = underlevel.clients(clients);
    let client = connect_character(&underlevel_clients, 0x9a, 0).await?;
    let mut client = seed_build(&underlevel_clients, client, "p04-master-399", ("sorcerer", 399), "Master refusal").await?;
    let initial = query(&mut client).await?;
    journey.check(1, initial.refusal == 0 && !initial.shop.is_empty(), &initial);
    let refused = send(&mut client, 5, 3).await?;
    let mut unchanged = refused.clone();
    unchanged.refusal = 0;
    let refused_session = client.join_snapshot().game_session_id;
    logout_once(&mut client).await?;
    drop(client);
    let mut client = connect_character(&underlevel_clients, 0x9a, 0).await?;
    let after_refusal = query(&mut client).await?;
    journey.check(
        2,
        refused.refusal == 3 && unchanged == initial && after_refusal == initial
            && client.join_snapshot().game_session_id != refused_session,
        (&refused.refusal, &refused.weekly),
    );
    logout_once(&mut client).await?;

    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_Q_P04_ACCOUNT").await?;
    let scoped_clients = identity.clients(clients);
    let clients = &scoped_clients;
    let client = connect_character(clients, 0x9a, 0).await?;
    let mut client = seed_build(clients, client, "p04-task-board", ("knight", 650), "authored maximum delivery capacity").await?;
    let capacity_board = query(&mut client).await?;
    if capacity_board.refusal != 0 || capacity_board.weekly.started || capacity_board.shop.is_empty() {
        return Err("P04 capacity fixture did not start with a fresh board".into());
    }

    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let started = send(&mut client, 5, 0).await?;
    journey.check(
        3,
        started.refusal == 0
            && started.weekly.started
            && !started.weekly.kills.is_empty()
            && !started.weekly.deliveries.is_empty(),
        (&started.refusal, &started.weekly),
    );

    // Sample one weekly credit after delivery and bounty commands have been exercised.
    let weekly = started.weekly;
    let index = weekly
        .deliveries
        .iter()
        .position(|task| !task.delivered)
        .ok_or("weekly-delivery needs an undelivered assignment")?;
    let task = &weekly.deliveries[index];
    let item_index = provisioning::item_index()?;
    let reference = delivery_reference(&item_index, task.item, client.selected_capabilities())?;
    let required = u32::from(task.required);
    let bag = provisioning::item_reference(&item_index, DELIVERY_BAG)?;
    let inventory_before = u32::from(task.current);
    let (key, _) = item_index.definition_at(reference).ok_or("P04 definition absent")?;
    let legacy_id: u16 = key.strip_prefix("oteryn:item.tibia.i").ok_or("P04 legacy ID absent")?.parse()?;
    let maximum = *oteryn_gameplay_quests::quest::task_board::full::DELIVERY_ITEMS
        .iter()
        .find(|item| item.item_id == legacy_id)
        .ok_or("P04 authored range absent")?
        .quantity.end();
    client = fund_delivery(clients, client, reference).await?;
    let inventory_funded = inventory_before + maximum;
    let containers = delivery_inventory(&client)?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == bag)
        .count();
    evidence(&format!(
        "FIXTURE row=P04.full fixture_containers={containers} item={legacy_id} authored_max={maximum} required={required}"
    ));
    let funded = query(&mut client).await?;
    if !funded.weekly.deliveries.get(index).is_some_and(|funded| {
        funded.item == task.item
            && funded.required == task.required
            && u32::from(funded.current) == inventory_funded
            && !funded.delivered
    }) {
        return Err("weekly-delivery did not recognize the exact carried fixture items".into());
    }
    if carried_delivery_count(&mut client, reference, bag).await? != inventory_funded {
        return Err("weekly-delivery carried inventory differs from the required items".into());
    }
    let delivered = send(&mut client, 6, u16::try_from(index)?).await?;
    journey.check(
        4,
        delivered.refusal == 0
            && delivered
                .weekly
                .deliveries
                .get(index)
                .is_some_and(|task| task.delivered),
        (&delivered.refusal, delivered.weekly.deliveries.get(index)),
    );
    if delivered.refusal == 0 {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let consumed = carried_delivery_count(&mut client, reference, bag).await?;
            if consumed == inventory_funded - required {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("weekly-delivery left {consumed} carried items").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    }

    let opened = send(&mut client, 7, 0).await?;
    journey.check(
        5,
        opened.refusal == 0 && opened.bounty.state == 1 && opened.bounty.offers.len() == 3,
        (&opened.refusal, &opened.bounty),
    );
    let claimed = full_row_p05::complete_p04_bounty(&mut client, clients, &opened, publish_count).await?;
    let rerolled = send(&mut client, 11, 0).await?;
    journey.check(
        5,
        rerolled.refusal == 0
            && rerolled.bounty.state == 1
            && rerolled.bounty.offers.len() == 3
            && rerolled.bounty_points == claimed.bounty_points,
        (&rerolled.refusal, &rerolled.bounty),
    );

    let talisman = send(&mut client, 17, 0).await?;
    journey.check(
        6,
        talisman.refusal == 0,
        (&talisman.refusal, &talisman.talisman),
    );
    let upgraded = send(&mut client, 18, 0).await?;
    journey.check(
        6,
        upgraded.refusal == 0 && upgraded.talisman[0].level > talisman.talisman[0].level,
        (&upgraded.refusal, upgraded.talisman[0]),
    );

    let before_sample = query(&mut client).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let sample = before_sample
        .weekly
        .kills
        .iter()
        .enumerate()
        .filter(|(_, task)| task.current < task.required)
        .find_map(|(index, task)| {
            let race = if task.race.is_empty() {
                "oteryn:creature.rat"
            } else {
                &task.race
            };
            full_row_p05::has_reachable_spawn(world, from, &metadata.spawns, race)
                .then_some((index, task, race))
        });
    if let Some((index, task, race)) = sample {
        full_row_p05::hunt_p04_fixture(&mut client, world, race).await?;
        let next = query(&mut client).await?;
        journey.check(
            4,
            next.weekly.kills.get(index).is_some_and(|found| {
                found.race == task.race && task.current.checked_add(1) == Some(found.current)
            }),
            ("weekly-credit-sample", task, next.weekly.kills.get(index)),
        );
    } else {
        journey.check(
            4,
            false,
            "weekly-credit-sample needs a reachable unfinished assignment",
        );
    }

    let before_reset = query(&mut client).await?;
    let reset = await_weekly_reset(&mut client, &before_reset).await?;
    let offer = reset
        .shop
        .iter()
        .filter(|offer| !offer.bought && offer.price > 0)
        .min_by_key(|offer| offer.price)
        .ok_or("step=shop need=unbought-authored-offer")?;
    let bought = send(&mut client, 16, offer.index).await?;
    journey.check(
        6,
        bought.refusal == 0
            && reset.points.checked_sub(u64::from(offer.price)) == Some(bought.points),
        (&bought.refusal, reset.points, bought.points, offer),
    );
    let next_week = send(&mut client, 5, 0).await?;
    journey.check(
        7,
        next_week.refusal == 0 && next_week.period == reset.period && next_week.weekly.started,
        ("next-week-start", &next_week),
    );
    let reset = query(&mut client).await?;
    let prior_session = client.join_snapshot().game_session_id;
    logout_once(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0x9b, 0).await?;
    if fresh.join_snapshot().game_session_id == prior_session {
        return Err("Task Board relog reused its original session".into());
    }
    let persisted = query(&mut fresh).await?;
    journey.check(8, persisted == reset, (&reset, &persisted));
    logout_once(&mut fresh).await?;
    Ok(())
}

const DELIVERY_BAG: u32 = 2854;

fn delivery_inventory(
    client: &oteryn_dev_client::DevClientSession,
) -> TestResult<&oteryn_protocol_oteryn::item_view::CharacterInventory> {
    client
        .inventory()
        .ok_or_else(|| "weekly-delivery inventory is absent".into())
}

fn delivery_reference(
    index: &crate::content::item_ref::ItemDefinitionIndex,
    item: u32,
    capabilities: &[u32],
) -> TestResult<std::num::NonZeroU32> {
    // The legacy board carries donor item IDs. Inventory and the fixture use
    // the activated generation's compact references in either projection.
    if !capabilities.contains(&CAPABILITY_TASK_BOARD_DAILY_BOUNTY_V1) {
        return provisioning::item_reference(index, item);
    }
    let reference =
        std::num::NonZeroU32::new(item).ok_or("weekly-delivery has a zero item definition")?;
    index
        .definition_at(reference)
        .ok_or("weekly-delivery item definition is absent")?;
    Ok(reference)
}

async fn open_delivery_bag(
    client: &mut oteryn_dev_client::DevClientSession,
    handle: oteryn_protocol_oteryn::item_view::ItemHandle,
) -> TestResult<oteryn_protocol_oteryn::container_tree::ContainerView> {
    if let Some(view) = client.session_mut().container_views().and_then(|views| {
        views
            .views
            .iter()
            .find(|view| view.container_handle == handle)
    }) {
        return Ok(view.clone());
    }
    let result = client.use_item(handle).await?;
    if result.status != CommandStatus::Accepted
        || result.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("delivery bag use refused: {result:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(view) = client.session_mut().container_views().and_then(|views| {
            views
                .views
                .iter()
                .find(|view| view.container_handle == handle)
        }) {
            return Ok(view.clone());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("delivery bag view absent".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn carried_delivery_count(
    client: &mut oteryn_dev_client::DevClientSession,
    reference: std::num::NonZeroU32,
    bag_reference: std::num::NonZeroU32,
) -> TestResult<u32> {
    let inventory = delivery_inventory(client)?;
    let mut count = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == reference)
        .map(|entry| entry.count.get())
        .sum::<u32>();
    let bags: Vec<_> = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == bag_reference)
        .map(|entry| entry.handle)
        .collect();
    for bag in bags {
        count += open_delivery_bag(client, bag)
            .await?
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == reference)
            .map(|entry| entry.count.get())
            .sum::<u32>();
    }
    Ok(count)
}

async fn seed_build(
    clients: &SeamClients<'_>,
    mut client: oteryn_dev_client::DevClientSession,
    profile: &'static str,
    (vocation, level): (&'static str, u32),
    reason: &'static str,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    character_prep::declare_seeded(
        "P04.full",
        &format!("{vocation}-level-{level}"),
        reason,
    );
    let original_session = client.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(original_session.as_bytes()),
        "--profile",
        profile.into(),
    )
    .await?;
    logout_once(&mut client).await?;
    drop(client);
    let mut client = connect_character(clients, 0x9a, 0).await?;
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        vocation,
        level,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("missing P04 vocation maxima")?;
    if client.join_snapshot().game_session_id == original_session
        || full_e2ef7::player_level(&mut client).await? != level
        || !client
            .actor_vitals()
            .is_some_and(|vitals| vitals.max_health == max_health && vitals.max_mana == max_mana)
    {
        return Err(
            "fresh Task Board admission did not serve the seeded level and derived vitals".into(),
        );
    }
    evidence(&format!("FIXTURE row=P04.full fixture_level={level} vocation={vocation} reason={reason}"));
    Ok(client)
}

async fn fund_delivery(
    clients: &SeamClients<'_>,
    mut client: oteryn_dev_client::DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let prior = client.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(prior.as_bytes()),
        "--profile",
        format!("p04-max-item-{reference}"),
    ).await?;
    logout_once(&mut client).await?;
    drop(client);
    let fresh = connect_character(clients, 0x9a, 0).await?;
    if fresh.join_snapshot().game_session_id == prior {
        return Err("delivery fixture reused its original admission".into());
    }
    Ok(fresh)
}

async fn logout_once(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let result = client.session_mut().logout().await?.result;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("Task Board logout refused: {result:?}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_resolves_the_negotiated_id_even_when_both_id_spaces_overlap() -> TestResult {
        let shard: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../../content/items/definitions/items-00000-00499.json"
        ))?;
        let mut records: Vec<(&str, &str)> = shard["records"]
            .as_array()
            .ok_or("item shard records")?
            .iter()
            .map(|record| {
                let identity = &record["definition"]["identity"];
                let key = identity["key"].as_str().ok_or("item key")?;
                let revision = identity["revision"].as_str().ok_or("item revision")?;
                Ok::<_, &'static str>((key, revision))
            })
            .collect::<Result<_, _>>()?;
        records.sort_unstable();
        let index = crate::content::item_ref::ItemDefinitionIndex::decode(&serde_json::to_vec(
            &serde_json::json!({"schema": "OTERYN_NATIVE_ITEM_KEYS/v1", "records": records}),
        )?)?;
        let canonical = &[CAPABILITY_TASK_BOARD_DAILY_BOUNTY_V1];
        let legacy_item = delivery_reference(&index, 100, &[])?;
        let canonical_item = delivery_reference(&index, 100, canonical)?;
        assert_eq!(
            index
                .definition_at(legacy_item)
                .ok_or("legacy definition")?
                .0,
            "oteryn:item.tibia.i100"
        );
        assert_eq!(canonical_item.get(), 100);
        assert_ne!(legacy_item, canonical_item);
        assert!(delivery_reference(&index, 0, &[]).is_err());
        assert!(delivery_reference(&index, u32::MAX, canonical).is_err());
        assert!(delivery_reference(&index, 0, canonical).is_err());
        Ok(())
    }

    #[test]
    fn scenario_commands_use_full_task_board_action_numbers() {
        for (action, selection) in [
            (4, 0),
            (5, 0),
            (6, 0),
            (7, 0),
            (8, 0),
            (9, 0),
            (11, 0),
            (16, 0),
            (17, 0),
            (18, 0),
        ] {
            assert!(
                oteryn_protocol_oteryn::task_board::full::encode_request(&request(
                    action, selection
                ))
                .is_ok()
            );
        }
    }
}

#[cfg(test)]
#[test]
fn row_04_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p04.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P04_GAP status="))
    );
}

#[cfg(test)]
#[test]
fn review_p04_guards_the_journey() {
    assert!(
        include_str!("full_row_p04.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("weekly-credit-sample"))
    );
}

#[cfg(test)]
#[test]
fn review_2_p04() {
    assert!(
        include_str!("full_row_p04.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("await_weekly_reset"))
    );
}
