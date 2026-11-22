//! P18 Exaltation Forge full-row journey.
//!
//! Concrete player scenario, derived from the P18 audit/matrix row and the
//! native item-Forge owner (`gameplay_transport/item_forge.rs` and
//! `durability/item_forge_live`):
//! 1. Admit the provisioned node-boot character and retain its complete native
//!    inventory snapshot; this is the only source of session item handles.
//! 2. Send HISTORY and require an accepted JSON history result.
//! 3. Acquire two class-one daggers from the player economy and send FUSION with their handles and require either an
//!    applied tier/custody/resource delta or an explicit native refusal which
//!    leaves the complete inventory unchanged.
//! 4. Send TRANSFER, CONVERGENCE_FUSION, and CONVERGENCE_TRANSFER in the same
//!    way, checking every reply and the inventory delta/refusal after each.
//! 5. Send DUST_TO_SLIVERS, SLIVERS_TO_CORES, and DUST_LIMIT; require their
//!    resource result on success, or an explicit no-delta refusal. This is the
//!    row's negative path when legitimately earned resources are insufficient.
//! 6. Query HISTORY again and require the server to describe every applied
//!    operation (an all-refused run must remain an empty history).
//! 7. Log out normally, make a fresh admission, and re-check the complete
//!    inventory plus HISTORY so durable tiers, resources, and receipts cannot
//!    disappear or be duplicated.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_forge::{
    CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, Operation, Request,
};
use std::num::NonZeroU64;

#[derive(Debug)]
struct Observation {
    operation: Operation,
    status: CommandStatus,
    body: serde_json::Value,
    changed: bool,
}

fn occurrence(session: oteryn_protocol_oteryn::GameSessionId, ordinal: u8) -> [u8; 16] {
    use sha2::Digest;
    let digest = sha2::Sha256::new()
        .chain_update(b"E2E-P18/occurrence")
        .chain_update(session.as_bytes())
        .chain_update([ordinal])
        .finalize();
    let mut value = [0; 16];
    value.copy_from_slice(&digest[..16]);
    value[6] = (value[6] & 15) | 0x70;
    value[8] = (value[8] & 63) | 0x80;
    value
}

const FUSION_PAIR: &str = "oteryn:item.tibia.i3267";

fn handles(
    client: &DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<(NonZeroU64, NonZeroU64)> {
    let inventory = client
        .inventory()
        .ok_or("step=forge-pair inventory frame missing")?;
    pair_handles(
        inventory
            .entries
            .iter()
            .chain(&inventory.main_backpack)
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .filter(|item| item.item_definition_ref == reference)
            .map(|entry| entry.handle),
    )
    .ok_or_else(|| {
        format!("step=forge-pair need=two-distinct-carried-{FUSION_PAIR}-handles").into()
    })
}

fn pair_handles(mut handles: impl Iterator<Item = NonZeroU64>) -> Option<(NonZeroU64, NonZeroU64)> {
    let first = handles.next()?;
    handles
        .find(|handle| *handle != first)
        .map(|second| (first, second))
}

/// Item handles are issued by each session. Compare the complete carried view
/// after removing only those handle values.
pub(super) fn durable_inventory(
    mut inventory: oteryn_protocol_oteryn::item_view::CharacterInventory,
) -> oteryn_protocol_oteryn::item_view::CharacterInventory {
    for item in inventory
        .entries
        .iter_mut()
        .chain(&mut inventory.main_backpack)
        .chain(inventory.equipment.iter_mut().map(|entry| &mut entry.item))
    {
        item.handle = NonZeroU64::MIN;
    }
    inventory
}

async fn command(
    client: &mut DevClientSession,
    operation: Operation,
    occurrence: [u8; 16],
    pair: Option<(NonZeroU64, NonZeroU64)>,
    history_limit: u8,
) -> TestResult<Observation> {
    let before = client
        .inventory()
        .cloned()
        .ok_or("inventory frame missing")?;
    let payload = Request {
        operation,
        occurrence,
        first_item: pair.map(|value| value.0),
        second_item: pair.map(|value| value.1),
        boost: false,
        protect: false,
        history_limit,
    }
    .encode()
    .ok_or("P18 request was not canonical")?;
    let reply = client
        .session_mut()
        .gated_command(CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, &payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    let body = serde_json::from_slice(&reply.payload).unwrap_or(serde_json::Value::Null);
    Ok(Observation {
        operation,
        status: reply.status,
        body,
        changed: client.inventory() != Some(&before),
    })
}

fn validate(observation: &Observation) -> TestResult<bool> {
    if observation.status != CommandStatus::Accepted || !observation.body.is_object() {
        return Err(format!(
            "step-{:?} observed status={:?} payload={}",
            observation.operation, observation.status, observation.body
        )
        .into());
    }
    let applied = observation.body["applied"].as_bool().unwrap_or(false);
    if applied != observation.changed && observation.operation.changes_inventory() {
        return Err(format!(
            "step-{:?} reply/delta mismatch applied={applied} inventory_changed={}",
            observation.operation, observation.changed
        )
        .into());
    }
    if !applied
        && (observation.body["reason"].as_str().is_none() || observation.changed)
        && observation.operation != Operation::QueryHistory
    {
        return Err(format!(
            "step-{:?} refusal lacked reason or changed inventory: {}",
            observation.operation, observation.body
        )
        .into());
    }
    Ok(applied)
}

fn validate_history(step: &str, observation: &Observation, applied: &[&Observation]) -> TestResult {
    if observation.status != CommandStatus::Accepted {
        return Err(format!(
            "{step} HISTORY observed status={:?} payload={}",
            observation.status, observation.body
        )
        .into());
    }
    let entries = observation.body.as_array().ok_or_else(|| {
        format!(
            "{step} HISTORY payload was not an array: {}",
            observation.body
        )
    })?;
    if entries.len() != applied.len() {
        return Err(format!(
            "{step} HISTORY receipt count mismatch expected={} observed={} payload={}",
            applied.len(),
            entries.len(),
            observation.body
        )
        .into());
    }
    for (entry, expected) in entries.iter().zip(applied.iter().rev()) {
        if entry["receipt"] != expected.body {
            return Err(format!(
                "{step} HISTORY receipt mismatch for {:?}: observed={} expected={}",
                expected.operation, entry, expected.body
            )
            .into());
        }
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-7 ordinary logout refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xd8).await?;
    let session = client.join_snapshot().game_session_id;
    let first_history = command(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    validate_history("step-2", &first_history, &[])?;

    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let definitions = provisioning::item_index()?;
    let pair_ref = super::full_e2ef7::acquire(&mut client, world, &definitions, FUSION_PAIR, 2)
        .await
        .map_err(|error| format!("step=forge-pair-acquisition need=two-class-one-{FUSION_PAIR} observed={error}; source:content/services/trade/trade-00000-00323.json:1"))?;

    let mut applied = Vec::new();
    let operations = [
        Operation::Fusion,
        Operation::Transfer,
        Operation::ConvergenceFusion,
        Operation::ConvergenceTransfer,
        Operation::DustToSlivers,
        Operation::SliversToCores,
        Operation::DustLimit,
    ];
    let mut observations = Vec::new();
    for (index, operation) in operations.into_iter().enumerate() {
        let pair = if operation.requires_pair() {
            if operation != Operation::Fusion {
                super::full_e2ef7::acquire(&mut client, world, &definitions, FUSION_PAIR, 2)
                    .await?;
            }
            Some(handles(&client, pair_ref)?)
        } else {
            None
        };
        let observed = command(
            &mut client,
            operation,
            occurrence(session, u8::try_from(index + 1)?),
            pair,
            0,
        )
        .await?;
        if validate(&observed)? {
            applied.push(operation);
        }
        observations.push(observed);
    }
    let history = command(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let applied_observations = observations
        .iter()
        .filter(|observation| observation.body["applied"] == true)
        .collect::<Vec<_>>();
    validate_history("step-6", &history, &applied_observations)?;
    let retained = client
        .inventory()
        .cloned()
        .ok_or("step-6 inventory absent")?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xd9).await?;
    if fresh.join_snapshot().game_session_id == session
        || fresh.inventory().cloned().map(durable_inventory) != Some(durable_inventory(retained))
    {
        return Err("step-7 fresh admission session/inventory durability mismatch".into());
    }
    let fresh_history = command(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
    logout(&mut fresh).await?;
    validate_history("step-7", &fresh_history, &applied_observations)?;
    if fresh_history.body != history.body {
        return Err(format!("step-7 fresh HISTORY mismatch: {fresh_history:?}").into());
    }
    if applied.len() != 7 {
        let refused = observations
            .iter()
            .filter(|value| !value.body["applied"].as_bool().unwrap_or(false))
            .map(|value| format!("{:?}:{}", value.operation, value.body["reason"]))
            .collect::<Vec<_>>()
            .join(",");
        return Err(format!(
            "steps-3-5 real mechanic attempts refused=[{refused}]; need=tiered-transfer-donor,class-four-convergence-pair,and-ordinary-dust/sliver/core-acquisition; pair-source:crates/gameplay/items/src/forge.rs:103; dust-source:crates/gameplay/domain/src/domain/forge_creature.rs:93"
        )
        .into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("P18 journey timeout after real native commands".into()),
    };
    match result {
        Ok(()) => evidence(&format!(
            "e2e_all scenario=P18.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Err(error) => {
            evidence(&format!("P18_GAP {error}"));
            evidence(&format!(
                "e2e_all scenario=P18.full status=FAIL seconds={:.3} evidence={}",
                started.elapsed().as_secs_f64(),
                error
                    .to_string()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join("_")
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forge_pair_requires_two_distinct_instances() -> TestResult {
        let first = NonZeroU64::new(1).ok_or("first handle")?;
        let second = NonZeroU64::new(2).ok_or("second handle")?;
        assert_eq!(pair_handles([first, first].into_iter()), None);
        assert_eq!(
            pair_handles([first, first, second].into_iter()),
            Some((first, second))
        );
        Ok(())
    }

    #[test]
    fn occurrences_are_v7_session_scoped() -> TestResult {
        let mut first_id = [1; 16];
        first_id[6] = 0x71;
        first_id[8] = 0x81;
        let mut second_id = [2; 16];
        second_id[6] = 0x72;
        second_id[8] = 0x82;
        let first = oteryn_protocol_oteryn::GameSessionId::decode(&first_id)?;
        let second = oteryn_protocol_oteryn::GameSessionId::decode(&second_id)?;
        assert_eq!(occurrence(first, 1), occurrence(first, 1));
        assert_ne!(occurrence(first, 1), occurrence(first, 2));
        assert_ne!(occurrence(first, 1), occurrence(second, 1));
        oteryn_protocol_oteryn::GameSessionId::decode(&occurrence(first, 1))?;
        Ok(())
    }
}
