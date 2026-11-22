//! P05 full-row journey, derived from the matrix, the re-audit row, and the
//! Task Board/talisman durable owner.
//!
//! Player scenario:
//! 1. Send the full Task Board query and require an accepted, decodable snapshot.
//! 2. Send Buy Talisman; require success, a 5,000-gold payment, and item 51978
//!    appearing in the ordinary inventory delta.
//! 3. Send Buy Talisman again; require the durable refusal and no second item.
//! 4. Open the bounty board, select an offered creature, kill it through native
//!    combat until complete, and claim; require kill progress and bounty points.
//! 5. Send Upgrade Talisman for the damage path; require the exact point debit,
//!    level/bonus increase, then send an unaffordable repeat and require refusal
//!    without a level or balance change.
//! 6. Equip item 51978 in its player extra slot and fight the selected bounty
//!    race; require an observed talisman combat proc and increased loot result.
//! 7. Log out normally, obtain a fresh admission, query again, and require the
//!    purchased item and upgraded level/balance to remain durable.
//!
//! The journey does not manufacture prerequisites. If an ordinary provisioned
//! character cannot reach a step, its single FAIL names that step and includes
//! the accepted/rejected status, refusal code, or missing delta actually seen.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::task_board::{self as wire, full};

const TALISMAN_ITEM: u32 = 51_978;

#[derive(Debug)]
struct StepFailure {
    step: u8,
    observed: String,
    source: Option<Box<dyn std::error::Error>>,
}

impl std::fmt::Display for StepFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "step-{}:{}", self.step, sanitize(&self.observed))
    }
}

impl std::error::Error for StepFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref()
    }
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:,=".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

async fn command(
    client: &mut DevClientSession,
    action: u8,
    selection: u16,
) -> Result<(CommandStatus, Option<full::Reply>), StepFailure> {
    let request = full::Request {
        action,
        selection,
        race: String::new(),
    };
    let payload = full::encode_request(&request).map_err(|error| StepFailure {
        step: 1,
        observed: format!("action-{action}-request-encode-{error:?}"),
        source: None,
    })?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_TASK_BOARD_V1,
            wire::COMMAND_TYPE_TASK_BOARD,
            &payload,
        )
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            StepFailure {
                step: 1,
                observed: format!("action-{action}-transport-{error}"),
                source: Some(error.into()),
            }
        })?;
    let reply = if response.payload.is_empty() {
        None
    } else {
        Some(
            full::decode_reply(&response.payload).map_err(|error| StepFailure {
                step: 1,
                observed: format!(
                    "action-{action}-status-{:?}-decode-{error:?}",
                    response.status
                ),
                source: None,
            })?,
        )
    };
    Ok((response.status, reply))
}

fn accepted(
    step: u8,
    result: (CommandStatus, Option<full::Reply>),
) -> Result<full::Reply, StepFailure> {
    let (status, reply) = result;
    let reply = reply.ok_or_else(|| StepFailure {
        step,
        observed: format!("status-{status:?}-missing-reply"),
        source: None,
    })?;
    if status != CommandStatus::Accepted || reply.refusal != 0 {
        return Err(StepFailure {
            step,
            observed: format!("status-{status:?}-refusal-{}", reply.refusal),
            source: None,
        });
    }
    Ok(reply)
}

fn item_count(client: &DevClientSession, reference: std::num::NonZeroU32) -> u32 {
    client
        .inventory()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == reference)
                .map(|entry| entry.count.get())
                .sum()
        })
        .unwrap_or(0)
}

pub(super) async fn wait_for_inventory(
    client: &mut DevClientSession,
    step: &str,
    committed: impl Fn(&DevClientSession) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    while !committed(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step={step} accepted-command-inventory-delta-missing").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
}

pub(super) async fn complete_bounty(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    opened: &full::Reply,
) -> TestResult<full::Reply> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let from = tile(client.world_spatial().actor_position)?;
    let (index, offer) = reachable_offer(&opened.bounty.offers, |race| {
        has_reachable_spawn(world, from, &metadata.spawns, race)
    })
    .ok_or_else(|| {
        format!(
            "step=bounty-route no reachable authored spawn for {:?}",
            opened.bounty.offers
        )
    })?;

    let mut state = accepted(4, command(client, 8, u16::try_from(index)?).await?)?;
    for previous in 0..offer.required {
        if state.bounty.active.as_ref().is_none_or(|active| {
            active.race != offer.race
                || active.required != offer.required
                || active.points != offer.points
        }) || state.bounty.current != previous
        {
            return Err(format!(
                "step=bounty-selection-or-credit expected={} kills={previous}; observed={:?}",
                offer.race, state.bounty
            )
            .into());
        }
        travel_to_spawn(client, world, &metadata.spawns, &offer.race).await?;
        client_hunt::hunt(client).await?;
        state = accepted(4, command(client, 4, 0).await?)?;
    }
    if state.bounty.current != offer.required || state.bounty.state != 3 {
        return Err(format!(
            "step=bounty-completion expected={} kills={}; observed={:?}",
            offer.race, offer.required, state.bounty
        )
        .into());
    }
    let claimed = accepted(4, command(client, 9, 0).await?)?;
    if state.bounty_points.checked_add(u64::from(offer.points)) != Some(claimed.bounty_points) {
        return Err(format!(
            "step=bounty-claim balance={}->{} expected-credit={}",
            state.bounty_points, claimed.bounty_points, offer.points
        )
        .into());
    }
    Ok(claimed)
}

pub(super) fn has_reachable_spawn(
    world: &BundleWorld,
    from: TilePos,
    spawns: &oteryn_world_bundle::spawn::Table,
    race: &str,
) -> bool {
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|creature| creature.key == race)
        })
        .collect();
    !points.is_empty()
        && route(world, from, |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
                && points.iter().any(|(floor, point)| {
                    at.floor == *floor && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= 1
                })
        })
        .is_some()
}

fn reachable_offer(
    offers: &[full::BountyOffer],
    mut reachable: impl FnMut(&str) -> bool,
) -> Option<(usize, &full::BountyOffer)> {
    offers
        .iter()
        .enumerate()
        .filter(|(_, offer)| !offer.elite && offer.required > 0 && reachable(&offer.race))
        .min_by_key(|(_, offer)| offer.required)
}

async fn journey(clients: &SeamClients<'_>) -> Result<(), StepFailure> {
    let mut client = connect_character(clients, 0xd5, 0).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        StepFailure {
            step: 1,
            observed: format!("admission-{error}"),
            source: Some(error),
        }
    })?;

    let before = accepted(1, command(&mut client, 4, 0).await?)?;
    evidence(&format!(
        "P05_CHECK step=1 status=PASS bounty_points={} talisman_level={}",
        before.bounty_points, before.talisman[0].level
    ));

    let opened = accepted(4, command(&mut client, 7, 0).await?)?;
    let claimed = complete_bounty(&mut client, clients, &opened)
        .await
        .map_err(|error| {
            verdict::record_error(error.as_ref());
            StepFailure {
                step: 4,
                observed: error.to_string(),
                source: Some(error),
            }
        })?;
    let reference = provisioning::item_index()
        .and_then(|definitions| provisioning::item_reference(&definitions, TALISMAN_ITEM))
        .map_err(|error| StepFailure {
            step: 2,
            observed: format!("talisman-definition-{error}"),
            source: Some(error),
        })?;
    let items_before = item_count(&client, reference);
    let bought = accepted(2, command(&mut client, 17, 0).await?).map_err(|mut failure| {
        failure.observed.push_str(";need=5000-spendable-gold;source=crates/gameplay/quests/src/quest/task_board_full.rs:604");
        failure
    })?;
    let items_after = items_before.checked_add(1).ok_or_else(|| StepFailure {
        step: 2,
        observed: "talisman-count-overflow".into(),
        source: None,
    })?;
    wait_for_inventory(&mut client, "talisman-purchase", |client| {
        item_count(client, reference) == items_after
    })
    .await
    .map_err(|error| StepFailure {
        step: 2,
        observed: error.to_string(),
        source: Some(error),
    })?;
    evidence("P05_CHECK step=2 status=PASS purchase-and-inventory-delta");

    let duplicate = command(&mut client, 17, 0).await?;
    let duplicate_reply = duplicate.1.ok_or_else(|| StepFailure {
        step: 3,
        observed: format!("status-{:?}-missing-reply", duplicate.0),
        source: None,
    })?;
    let duplicate_items = item_count(&client, reference);
    if duplicate.0 != CommandStatus::Accepted
        || duplicate_reply.refusal == 0
        || duplicate_items != items_after
    {
        return Err(StepFailure {
            step: 3,
            observed: format!(
                "status-{:?}-refusal-{}-items-{duplicate_items}-expected-{items_after}",
                duplicate.0, duplicate_reply.refusal
            ),
            source: None,
        });
    }
    evidence("P05_CHECK step=3 status=PASS duplicate-purchase-refused-no-delta");

    let prior = bought.talisman[0];
    let upgraded = accepted(5, command(&mut client, 18, 0).await?)?;
    if prior.level.checked_add(1) != Some(upgraded.talisman[0].level)
        || upgraded.talisman[0].bonus <= prior.bonus
        || bought.bounty_points.checked_sub(u64::from(prior.next_cost))
            != Some(upgraded.bounty_points)
    {
        return Err(StepFailure {
            step: 5,
            observed: format!("upgrade-delta {prior:?} -> {:?}", upgraded.talisman[0]),
            source: None,
        });
    }
    if bought.bounty_points != claimed.bounty_points {
        return Err(StepFailure {
            step: 5,
            observed: "purchase changed bounty points".into(),
            source: None,
        });
    }
    let saved = accepted(7, command(&mut client, 4, 0).await?)?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xd6, 0)
        .await
        .map_err(|error| StepFailure {
            step: 7,
            observed: format!("fresh-admission-{error}"),
            source: Some(error),
        })?;
    let persisted = accepted(7, command(&mut fresh, 4, 0).await?)?;
    if persisted != saved || item_count(&fresh, reference) != items_after {
        return Err(StepFailure {
            step: 7,
            observed: "fresh-admission lost board or talisman custody".into(),
            source: None,
        });
    }
    logout(&mut fresh).await?;
    Err(StepFailure {
        step: 6,
        observed: "need=wire-equipment-slot-10-for-talisman-combat-and-loot;source=crates/protocol-oteryn/src/item_view.rs:209".into(),
        source: None,
    })
}

pub(super) async fn logout_after_combat(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    verdict::Harness::global().bounded_wait(deadline, "logout-combat-lock");
    loop {
        let result = client.session_mut().logout().await?.result;
        let retry = match result {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::Busy) => Duration::from_millis(250),
            Some(LogoutResult::InFight { retry_after_ms }) => {
                Duration::from_millis(u64::from(retry_after_ms).max(100))
            }
            _ => return Err(format!("step=logout observed={result:?}").into()),
        };
        let until = (tokio::time::Instant::now() + retry).min(deadline);
        while tokio::time::Instant::now() < until {
            client.service_liveness(Duration::from_millis(100)).await?;
            client.take_events();
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=logout retry deadline observed={result:?}").into());
        }
    }
}

async fn logout(client: &mut DevClientSession) -> Result<(), StepFailure> {
    logout_after_combat(client)
        .await
        .map_err(|error| StepFailure {
            step: 7,
            observed: error.to_string(),
            source: Some(error),
        })
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    match &result {
        Ok(Ok(())) => evidence("P05_GAP status=PASS"),
        Ok(Err(error)) => evidence(&format!("P05_GAP status=FAIL observed={error}")),
        Err(_) => evidence("P05_GAP status=FAIL step=deadline observed=no-reply"),
    }
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=P05.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P05.full status=FAIL seconds={:.3} evidence={error}",
            started.elapsed().as_secs_f64()
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=P05.full status=FAIL seconds={:.3} evidence=step-timeout-observed-no-reply",
            started.elapsed().as_secs_f64()
        )),
    }
}

impl verdict::IndexedError for StepFailure {
    fn as_error(&self) -> &(dyn std::error::Error + 'static) {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{full, reachable_offer, sanitize};

    #[test]
    fn selects_shortest_reachable_ordinary_offer_at_its_wire_index() {
        let offer = |race: &str, required, elite| full::BountyOffer {
            elite,
            dust: 0,
            race: race.into(),
            required,
            experience: 1,
            points: 1,
            grade: 0,
        };
        let offers = [
            offer("unreachable", 1, false),
            offer("reachable", 1, true),
            offer("reachable", 6, false),
            offer("reachable", 4, false),
        ];
        assert_eq!(
            reachable_offer(&offers, |race| race == "reachable"),
            Some((3, &offers[3]))
        );
        assert_eq!(reachable_offer(&offers, |_| false), None);
    }

    #[test]
    fn evidence_is_one_token() {
        assert_eq!(
            sanitize("status Accepted / missing delta"),
            "status-Accepted---missing-delta"
        );
    }
}

#[cfg(test)]
#[test]
fn row_05_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p05.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P05_GAP status="))
    );
}
