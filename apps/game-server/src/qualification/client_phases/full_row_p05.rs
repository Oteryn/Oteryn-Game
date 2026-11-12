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
}

impl std::fmt::Display for StepFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "step-{}:{}", self.step, sanitize(&self.observed))
    }
}

impl std::error::Error for StepFailure {}

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
        observed: format!("request-encode-{error:?}"),
    })?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_TASK_BOARD_V1,
            wire::COMMAND_TYPE_TASK_BOARD,
            &payload,
        )
        .await
        .map_err(|error| StepFailure {
            step: 1,
            observed: format!("transport-{error}"),
        })?;
    let reply = if response.payload.is_empty() {
        None
    } else {
        Some(full::decode_reply(&response.payload).map_err(|error| StepFailure {
            step: 1,
            observed: format!("status-{:?}-decode-{error:?}", response.status),
        })?)
    };
    Ok((response.status, reply))
}

fn accepted(step: u8, result: (CommandStatus, Option<full::Reply>)) -> Result<full::Reply, StepFailure> {
    let (status, reply) = result;
    let reply = reply.ok_or_else(|| StepFailure {
        step,
        observed: format!("status-{status:?}-missing-reply"),
    })?;
    if status != CommandStatus::Accepted || reply.refusal != 0 {
        return Err(StepFailure {
            step,
            observed: format!("status-{status:?}-refusal-{}", reply.refusal),
        });
    }
    Ok(reply)
}

fn item_count(client: &DevClientSession) -> TestResult<u32> {
    let definitions = provisioning::item_index()?;
    let reference = provisioning::item_reference(&definitions, TALISMAN_ITEM)?;
    Ok(client
        .inventory()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == reference)
                .map(|entry| entry.count.get())
                .sum()
        })
        .unwrap_or(0))
}

async fn journey(clients: &SeamClients<'_>) -> Result<(), StepFailure> {
    let mut client = connect_character(clients, 0xd5, 0)
        .await
        .map_err(|error| StepFailure { step: 1, observed: format!("admission-{error}") })?;

    let before = accepted(1, command(&mut client, 4, 0).await?)?;
    evidence(&format!("P05_CHECK step=1 status=PASS bounty_points={} talisman_level={}", before.bounty_points, before.talisman[0].level));

    let items_before = item_count(&client).map_err(|error| StepFailure { step: 2, observed: format!("inventory-before-{error}") })?;
    let _bought = accepted(2, command(&mut client, 17, 0).await?)?;
    let items_after = item_count(&client).map_err(|error| StepFailure { step: 2, observed: format!("inventory-after-{error}") })?;
    if items_after != items_before + 1 {
        return Err(StepFailure { step: 2, observed: format!("status-Accepted-refusal-0-missing-inventory-delta-before-{items_before}-after-{items_after}") });
    }
    evidence("P05_CHECK step=2 status=PASS purchase-and-inventory-delta");

    let duplicate = command(&mut client, 17, 0).await?;
    let duplicate_reply = duplicate.1.ok_or_else(|| StepFailure { step: 3, observed: format!("status-{:?}-missing-reply", duplicate.0) })?;
    let duplicate_items = item_count(&client).map_err(|error| StepFailure { step: 3, observed: format!("inventory-{error}") })?;
    if duplicate.0 != CommandStatus::Accepted || duplicate_reply.refusal == 0 || duplicate_items != items_after {
        return Err(StepFailure { step: 3, observed: format!("status-{:?}-refusal-{}-items-{duplicate_items}-expected-{items_after}", duplicate.0, duplicate_reply.refusal) });
    }
    evidence("P05_CHECK step=3 status=PASS duplicate-purchase-refused-no-delta");

    let opened = accepted(4, command(&mut client, 7, 0).await?)?;
    let offer = opened.bounty.offers.first().ok_or_else(|| StepFailure { step: 4, observed: "status-Accepted-refusal-0-missing-bounty-offer".into() })?;
    let race = offer.race.clone();
    let required = offer.required;
    let selected = accepted(4, command(&mut client, 8, 0).await?)?;
    if selected.bounty.active.as_ref().map(|task| task.race.as_str()) != Some(race.as_str()) {
        return Err(StepFailure { step: 4, observed: "status-Accepted-refusal-0-missing-selected-bounty-delta".into() });
    }
    Err(StepFailure { step: 4, observed: format!("status-Accepted-refusal-0-combat-route-needed-race-{race}-required-{required}") })
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence(&format!("e2e_all scenario=P05.full status=PASS seconds={:.3}", started.elapsed().as_secs_f64())),
        Ok(Err(error)) => evidence(&format!("e2e_all scenario=P05.full status=FAIL seconds={:.3} evidence={error}", started.elapsed().as_secs_f64())),
        Err(_) => evidence(&format!("e2e_all scenario=P05.full status=FAIL seconds={:.3} evidence=step-timeout-observed-no-reply", started.elapsed().as_secs_f64())),
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn evidence_is_one_token() {
        assert_eq!(sanitize("status Accepted / missing delta"), "status-Accepted---missing-delta");
    }
}
