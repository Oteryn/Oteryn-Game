//! E08 — Depot search, detail, and retrieve-all from results.
//!
//! Player scenario derived from the E08 audit/matrix row and the named depot-search owner:
//! 1. Walk to an authored depot locker, browse a depot box, and deposit one legitimately
//!    provisioned carried item; expect accepted depot replies, a MOVED result, and an
//!    inventory delta removing that exact handle.
//! 2. Send SEARCH for that item's native client id; expect FOUND with the deposited item
//!    counted in depot custody and no retrieved count.
//! 3. Send DETAIL for a tier returned by SEARCH; expect DETAILED with exactly that item/tier
//!    row and the same depot count.
//! 4. Send RETRIEVE(all=true) from the result; expect RETRIEVED, the complete count, and an
//!    inventory delta restoring the item.
//! 5. Repeat RETRIEVE(all=true) as the refusal/negative path; expect NOT_FOUND, zero retrieved,
//!    and no inventory change (never a duplicate item).
//! 6. Log out and obtain a fresh admission for the same character; walk back to the locker,
//!    SEARCH again, and expect NOT_FOUND while the restored inventory item remains durable.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{CommandStatus, depot, depot_search as wire};
use oteryn_protocol_oteryn::item_view::ItemHandle;

struct Journey {
    started: std::time::Instant,
    step: &'static str,
    observed: String,
}

impl Journey {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            step: "admission",
            observed: "not-started".into(),
        }
    }

    fn at(&mut self, step: &'static str) {
        self.step = step;
        self.observed = "command-not-yet-replied".into();
    }

    fn saw(&mut self, value: impl std::fmt::Debug) {
        self.observed = format!("{value:?}").replace(' ', "_");
    }

    fn finish(self, result: &TestResult) {
        let (status, detail) = match result {
            Ok(()) => ("PASS", None),
            Err(error) => (
                "FAIL",
                Some(format!(
                    "step:{};observed:{};need:accepted-native-depot-search-lifecycle;error:{}",
                    self.step,
                    self.observed,
                    error.to_string().replace(' ', "_")
                )),
            ),
        };
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=E08.full status={status} seconds={:.3}{}",
            self.started.elapsed().as_secs_f64(),
            detail.map_or_else(String::new, |value| format!(" evidence={value}"))
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut journey = Journey::new();
    let result = match row_complete_with_timeout(
        Duration::from_secs(180),
        run(&mut journey, clients),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err("journey-timeout".into()),
    };
    journey.finish(&result);
}

async fn command(client: &mut DevClientSession, request: wire::Request) -> TestResult<wire::Reply> {
    let payload = wire::encode_request(request).map_err(|error| format!("encode:{error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DEPOT_SEARCH_V1,
            wire::COMMAND_TYPE_DEPOT_SEARCH,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("command-status:{:?}", response.status).into());
    }
    wire::decode_reply(&response.payload).map_err(|error| format!("decode:{error:?}").into())
}

fn carried(client: &DevClientSession, handle: ItemHandle) -> bool {
    client.inventory().is_some_and(|inventory| {
        inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .any(|entry| entry.handle == handle)
    })
}

fn carried_count(client: &DevClientSession, definition: std::num::NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .main_backpack
            .iter()
            .chain(&inventory.entries)
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .filter(|entry| entry.item_definition_ref == definition)
            .map(|entry| u64::from(entry.count.get()))
            .sum()
    })
}

async fn wait_carried(
    client: &mut DevClientSession,
    handle: ItemHandle,
    expected: bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while carried(client, handle) != expected {
        if tokio::time::Instant::now() >= deadline {
            return Err("inventory-delta-timeout".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

async fn depot_intent(
    client: &mut DevClientSession,
    intent: depot::DepotIntent,
) -> TestResult<depot::DepotReply> {
    let payload = depot::encode_depot_intent(intent).map_err(|error| format!("depot-encode:{error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(depot::CAPABILITY_DEPOT_V1, depot::COMMAND_TYPE_DEPOT_INTENT, &payload)
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("depot-status:{:?}", response.status).into());
    }
    depot::decode_depot_reply(&response.payload)
        .map_err(|error| format!("depot-decode:{error:?}").into())
}

async fn to_locker(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| near_locker(world, at))
        .ok_or("no-reachable-authored-depot-locker")?;
    walk(client, world, &path).await
}

async fn logout(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout-refused".into());
    }
    Ok(())
}

async fn run(journey: &mut Journey, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let definitions = provisioning::item_index()?;
    let mut client = row_connect_character(clients, 0xe8, 1).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_DEPOT_SEARCH_V1)
    {
        return Err("capability-153-not-negotiated".into());
    }

    journey.at("1-walk-browse-deposit");
    to_locker(&mut client, world).await?;
    let item = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .copied()
        .ok_or("no-legitimately-provisioned-carried-item")?;
    let carried_before = carried_count(&client, item.item_definition_ref);
    let (key, _) = definitions
        .definition_at(item.item_definition_ref)
        .ok_or("carried-item-definition-unknown")?;
    let client_id: u32 = key
        .strip_prefix("oteryn:item.tibia.i")
        .ok_or("carried-item-has-no-native-client-id")?
        .parse()?;
    let browse = depot_intent(
        &mut client,
        depot::DepotIntent {
            town: None,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Browse,
        },
    )
    .await?;
    journey.saw(&browse);
    if browse.outcome != depot::DepotOutcome::Opened || browse.town.is_none() {
        return Err("browse-did-not-open".into());
    }
    let moved = depot_intent(
        &mut client,
        depot::DepotIntent {
            town: browse.town,
            box_number: 1,
            page: 0,
            action: depot::DepotAction::Deposit(item.handle),
        },
    )
    .await?;
    journey.saw(&moved);
    if moved.outcome != depot::DepotOutcome::Moved {
        return Err("deposit-not-moved".into());
    }
    wait_carried(&mut client, item.handle, false).await?;

    journey.at("2-search");
    let found = command(&mut client, wire::Request::Search { client_id }).await?;
    journey.saw(&found);
    let row = found.results.first().ok_or("search-missing-result-row")?;
    if found.outcome != wire::Outcome::Found
        || found.retrieved != 0
        || row.client_id != client_id
        || row.depot == 0
    {
        return Err("search-result-mismatch".into());
    }
    let tier = row.tiers.first().ok_or("search-missing-tier")?.tier;
    let expected = row.depot + row.stash + row.inbox;
    let tier_count = row.tiers.first().ok_or("search-missing-tier")?.count;

    journey.at("3-detail");
    let detail = command(&mut client, wire::Request::Detail { client_id, tier }).await?;
    journey.saw(&detail);
    if detail.outcome != wire::Outcome::Detailed
        || detail.results.len() != 1
        || detail.results[0].client_id != client_id
        || detail.results[0].depot + detail.results[0].stash + detail.results[0].inbox
            != tier_count
        || detail.results[0].tiers.as_slice()
            != [wire::TierCount {
                tier,
                count: tier_count,
            }]
    {
        return Err("detail-result-mismatch".into());
    }

    journey.at("4-retrieve-all");
    let retrieved = command(
        &mut client,
        wire::Request::Retrieve {
            client_id,
            all: true,
        },
    )
    .await?;
    journey.saw(&retrieved);
    if retrieved.outcome != wire::Outcome::Retrieved || retrieved.retrieved != expected {
        return Err("retrieve-all-count-mismatch".into());
    }
    let inventory_deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while carried_count(&client, item.item_definition_ref) != carried_before - u64::from(item.count.get()) + u64::from(retrieved.retrieved) {
        if tokio::time::Instant::now() >= inventory_deadline {
            return Err("retrieved-definition-count-delta-timeout".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let durable_count = carried_count(&client, item.item_definition_ref);

    journey.at("5-empty-retrieve-refusal");
    let refused = command(
        &mut client,
        wire::Request::Retrieve {
            client_id,
            all: true,
        },
    )
    .await?;
    journey.saw(&refused);
    if refused.outcome != wire::Outcome::NotFound
        || refused.retrieved != 0
        || carried_count(&client, item.item_definition_ref) != durable_count
    {
        return Err("empty-retrieve-was-not-stable-not-found".into());
    }

    journey.at("6-logout-fresh-admission-durability");
    let old_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = row_connect_character(clients, 0xe9, 1).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("fresh-admission-reused-session".into());
    }
    to_locker(&mut fresh, world).await?;
    let after = command(&mut fresh, wire::Request::Search { client_id }).await?;
    journey.saw(&after);
    if after.outcome != wire::Outcome::NotFound
        || after.retrieved != 0
        || !after.results.is_empty()
        || carried_count(&fresh, item.item_definition_ref) != durable_count
    {
        return Err("fresh-admission-durable-state-mismatch".into());
    }
    logout(&mut fresh).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn client_id_is_derived_only_from_native_item_keys() {
        assert_eq!(
            "oteryn:item.tibia.i3031"
                .strip_prefix("oteryn:item.tibia.i")
                .and_then(|value| value.parse::<u32>().ok()),
            Some(3031)
        );
        assert!("oteryn:item.other".strip_prefix("oteryn:item.tibia.i").is_none());
    }
}
