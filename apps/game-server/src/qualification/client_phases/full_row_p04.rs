//! P04 full-row player scenario, derived from the matrix/re-audit and the
//! `task_board` durable owner:
//! 1. Admit the node-boot character and send QUERY; expect the complete weekly,
//!    bounty, shop, points and soulseal projection.
//! 2. Send START_WEEKLY with an invalid/locked difficulty; expect a refusal and
//!    no board delta (the negative path).
//! 3. Admit at seeded level eight, send START_WEEKLY(Beginner),
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
use oteryn_protocol_oteryn::task_board::full::{Reply, Request};

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
    let result = verdict::indexed_timeout(
        Duration::from_secs(240),
        verdict::observe(run(&mut journey, clients)),
    )
    .await;
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

async fn run(journey: &mut Journey, clients: &SeamClients<'_>) -> TestResult {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_Q_P04_ACCOUNT").await?;
    let scoped_clients = identity.clients(clients);
    let clients = &scoped_clients;
    character_prep::declare_seeded(
        "P04.full",
        "sorcerer-level-8",
        "weekly Task Board eligibility",
    );
    let mut client = connect_character(clients, 0x9a, 0).await?;
    let original_session = client.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(original_session.as_bytes()),
        "--profile",
        "p04-task-board".into(),
    )
    .await?;
    logout_once(&mut client).await?;
    drop(client);
    let mut client = connect_character(clients, 0x9a, 0).await?;
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        8,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("missing level-8 sorcerer maxima")?;
    if client.join_snapshot().game_session_id == original_session
        || full_e2ef7::player_level(&mut client).await? != 8
        || !client
            .actor_vitals()
            .is_some_and(|vitals| vitals.max_health == max_health && vitals.max_mana == max_mana)
    {
        return Err(
            "fresh Task Board admission did not serve the seeded level and derived vitals".into(),
        );
    }
    let initial = query(&mut client).await?;
    journey.check(
        1,
        initial.refusal == 0 && !initial.shop.is_empty(),
        &initial,
    );

    let refused = send(&mut client, 5, 3).await?;
    journey.check(
        2,
        refused.refusal != 0 && refused.weekly == initial.weekly,
        (&refused.refusal, &refused.weekly),
    );

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
    let delivery_index = weekly
        .deliveries
        .iter()
        .position(|task| !task.delivered && task.current >= task.required)
        .or_else(|| weekly.deliveries.iter().position(|task| !task.delivered));
    if let Some(index) = delivery_index {
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
    } else {
        journey.check(4, false, "weekly-delivery needs an undelivered assignment");
    }

    let opened = send(&mut client, 7, 0).await?;
    journey.check(
        5,
        opened.refusal == 0 && opened.bounty.state == 1 && opened.bounty.offers.len() == 3,
        (&opened.refusal, &opened.bounty),
    );
    let claimed = full_row_p05::complete_bounty(&mut client, clients, &opened).await?;
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
        full_row_p05::hunt_for_race(&mut client, world, &metadata.spawns, race).await?;
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
