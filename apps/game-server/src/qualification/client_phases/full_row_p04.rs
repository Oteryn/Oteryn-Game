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
        .map_err(|error| {
            format!("task-board action={action} selection={selection}: {error:?}").into()
        })
}

async fn query(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<Reply> {
    send(client, 4, 0).await
}

async fn logout(client: &mut RowClient) -> TestResult {
    let reply = client.row_logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P04 logout refused: {reply:?}").into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut journey = Journey::new();
    let result = row_complete_with_timeout(Duration::from_secs(240), run(&mut journey, clients)).await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => journey.check(0, false, format!("journey-error:{error}")),
        Err(_) => journey.check(0, false, "journey-timeout"),
    }
    journey.finish();
}

async fn run(journey: &mut Journey, clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0x9a).await?;
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

    // The node-boot character is never advanced by a fixture. Repeated ordinary
    // hunts provide XP, loot and any delivery goods used below.
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut started = send(&mut client, 5, 0).await?;
    for _ in 0..96 {
        if started.refusal == 0 {
            break;
        }
        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(&mut client).await?;
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
    let claimed = send(&mut client, 9, 0).await?;
    journey.check(
        5,
        claimed.refusal == 0 && claimed.bounty_points > rerolled.bounty_points,
        (&claimed.refusal, claimed.bounty_points),
    );

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

    let reset = query(&mut client).await?;
    journey.check(
        7,
        reset.period > initial.period && reset.soulseals > initial.soulseals,
        (
            initial.period,
            reset.period,
            initial.soulseals,
            reset.soulseals,
        ),
    );
    logout(&mut client).await?;
    drop(client);
    let mut fresh = row_connect_world(clients, 0x9b).await?;
    let persisted = query(&mut fresh).await?;
    journey.check(8, persisted == reset, (&reset, &persisted));
    logout(&mut fresh).await?;
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
