//! P04 full-row player scenario, derived from the matrix/re-audit and the
//! `task_board` durable owner:
//! 1. Admit the node-boot character and send QUERY; expect the complete weekly,
//!    bounty, shop, points and soulseal projection.
//! 2. Send START_WEEKLY with an invalid/locked difficulty; expect a refusal and
//!    no board delta (the negative path).
//! 3. Reach level eight only by ordinary hunting, send START_WEEKLY(Beginner),
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
        if !condition { verdict::record_assertion("task-board-check"); }
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
        .map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("task-board action={action} selection={selection}: {error:?}"), error) })
}

async fn query(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<Reply> {
    send(client, 4, 0).await
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P04 logout refused: {reply:?}").into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut journey = Journey::new();
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(run(&mut journey, clients))).await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => { verdict::record_error(error.as_ref()); journey.check(0, false, format!("journey-error:{error}")) },
        Err(_) => journey.check(0, false, "journey-timeout"),
    }
    journey.finish();
}

fn earned_bounty(current: u32, required: u32, before: u64, after: u64, award: u32) -> bool {
    required > 0 && current == required && award > 0 && before.checked_add(u64::from(award)) == Some(after)
}

fn completed_rollover(initial: u64, reply: u64, world: u64, initial_seals: u64, seals: u64) -> bool {
    reply > initial && reply == world && seals > initial_seals
}

async fn hunt_assignment(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    race: &str,
    name: &str,
) -> TestResult {
    let target = travel_to_spawn_target(client, world, spawns, race).await?;
    client_hunt::hunt_named_target(client, target, name).await
}

async fn observed_world_period(database: &mut sqlx::PgConnection, world: &[u8; 16]) -> TestResult<u64> {
    let period: Option<i64> = sqlx::query_scalar("SELECT game_task_board_period(encode($1,'hex')::uuid)")
        .bind(world.as_slice()).fetch_one(database).await?;
    Ok(u64::try_from(period.ok_or("step=weekly-reset needed=published-world-period")?)?)
}

async fn run(journey: &mut Journey, clients: &SeamClients<'_>) -> TestResult {
    use sqlx::Connection;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let initial_period = observed_world_period(&mut database, &clients.world).await?;
    let mut client = connect_world(clients, 0x9a).await?;
    let initial = query(&mut client).await?;
    if initial.period != initial_period {
        return Err("step=weekly-query observed=board-period-disagrees-with-committed-world-period".into());
    }
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

    // The node-boot character is never advanced by a fixture. Repeated ordinary
    // hunts provide XP, loot and any delivery goods used below.
    let mut started = send(&mut client, 5, 0).await?;
    for _ in 0..96 {
        if started.refusal == 0 {
            break;
        }
        hunt_assignment(&mut client, world, &metadata.spawns, "oteryn:creature.rat", "Rat").await?;
        started = send(&mut client, 5, 0).await?;
    }
    journey.check(
        3,
        started.refusal == 0
            && started.weekly.started
            && !started.weekly.kills.is_empty()
            && !started.weekly.deliveries.is_empty(),
        (&started.refusal, &started.weekly),
    );

    let delivered = send(&mut client, 6, 0).await?;
    journey.check(
        4,
        delivered.refusal == 0
            && delivered
                .weekly
                .deliveries
                .first()
                .is_some_and(|task| task.delivered),
        (&delivered.refusal, delivered.weekly.deliveries.first()),
    );

    let opened = send(&mut client, 7, 0).await?;
    journey.check(
        5,
        opened.refusal == 0 && opened.bounty.state == 1 && opened.bounty.offers.len() == 3,
        (&opened.refusal, &opened.bounty),
    );
    let selected = send(&mut client, 8, 0).await?;
    journey.check(
        5,
        selected.refusal == 0 && selected.bounty.state == 2 && selected.bounty.active.is_some(),
        (&selected.refusal, &selected.bounty),
    );
    let rerolled = send(&mut client, 11, 0).await?;
    journey.check(
        5,
        rerolled.refusal == 0 && rerolled.bounty.state == 1 && rerolled.bounty.offers.len() == 3,
        (&rerolled.refusal, &rerolled.bounty),
    );
    let (number, offer) = rerolled.bounty.offers.iter().enumerate()
        .find(|(_, offer)| metadata.spawns.sources.iter().any(|source| source.points.iter().any(|point| {
            metadata.spawns.creatures.get(point.creature as usize).is_some_and(|creature| creature.key == offer.race)
        }))).ok_or("step=bounty needed=rerolled-offer-with-authored-spawn")?;
    let name = super::full_row_p02::creature_display_name(&offer.race)?;
    let mut active = send(&mut client, 8, u16::try_from(number)?).await?;
    if active.refusal != 0 || active.bounty.state != 2 || active.bounty.current != 0
        || active.bounty.active.as_ref() != Some(offer)
    {
        return Err(format!("step=bounty-select observed={:?};needed={offer:?}", active.bounty).into());
    }
    while active.bounty.current < offer.required {
        hunt_assignment(&mut client, world, &metadata.spawns, &offer.race, &name).await?;
        let after = query(&mut client).await?;
        if active.bounty.current.checked_add(1) != Some(after.bounty.current)
            || after.bounty.active.as_ref() != Some(offer) || after.bounty_points != active.bounty_points
        {
            return Err(format!("step=bounty-kill observed={:?};needed=one-credited-kill-for-{}", after.bounty, offer.race).into());
        }
        active = after;
    }
    let claimed = send(&mut client, 9, 0).await?;
    journey.check(5, claimed.refusal == 0 && claimed.bounty.active.is_none()
        && earned_bounty(active.bounty.current, offer.required, active.bounty_points, claimed.bounty_points, offer.points),
        (&claimed.refusal, claimed.bounty_points, &claimed.bounty));

    let bought = send(
        &mut client,
        16,
        initial.shop.first().map_or(0, |offer| offer.index),
    )
    .await?;
    journey.check(
        6,
        bought.refusal == 0 && bought.points < initial.points,
        (&bought.refusal, bought.points),
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

    let before_reset = query(&mut client).await?;
    if before_reset.period != initial_period || before_reset.weekly.pending_soulseals == 0 {
        return Err(format!("step=weekly-reset observed=period:{},pending-soulseals:{};needed=earned-weekly-reward-before-rollover",
            before_reset.period, before_reset.weekly.pending_soulseals).into());
    }
    logout(&mut client).await?;
    drop(client);
    let until = tokio::time::Instant::now() + Duration::from_secs(60);
    let period = loop {
        let period = observed_world_period(&mut database, &clients.world).await?;
        if period > initial_period {
            break period;
        }
        if tokio::time::Instant::now() >= until {
            return Err("step=weekly-reset needed=Control-published-next-week".into());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    };
    let mut rolled = connect_world(clients, 0x9b).await?;
    let reset = query(&mut rolled).await?;
    journey.check(7, completed_rollover(initial_period, reset.period, period, initial.soulseals, reset.soulseals)
        && !reset.weekly.started && reset.weekly_results.as_ref().is_some_and(|results| {
            results.period == initial_period
                && u64::from(results.soulseals) == before_reset.weekly.pending_soulseals
                && before_reset.soulseals.checked_add(u64::from(results.soulseals)) == Some(reset.soulseals)
        }), (&before_reset.weekly, &reset.weekly_results, reset.period, period));
    logout(&mut rolled).await?;
    drop(rolled);
    let mut fresh = connect_world(clients, 0x9b).await?;
    let persisted = query(&mut fresh).await?;
    journey.check(8, persisted == reset, (&reset, &persisted));
    logout(&mut fresh).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounty_claim_and_weekly_rollover_require_independent_lifecycle_evidence() {
        assert!(!earned_bounty(0, 3, 10, 12, 2));
        assert!(earned_bounty(3, 3, 10, 12, 2));
        assert!(!completed_rollover(7, 14, 7, 3, 4));
        assert!(completed_rollover(7, 14, 14, 3, 4));
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
