//! G07: house furniture rotation and decorations (apps/client 0445-house-furniture-rotation).
//! 1. Admit the node-boot character; require the rotation capability (needs HOUSE and a native
//!    bundle map) in the negotiated session.
//! 2. QUERY page 0 of the rotatable furniture/decorations of the actor's current house: expect an
//!    accepted catalogue with at least one authoritative target.
//! 3. ROTATE the first target through the same request the panel sends: expect `Applied` with
//!    `Committed`.
//! 4. QUERY again: the same target must have a newer revision and its authored orientation.
//! 5. Log out, freshly admit, QUERY: the revision and orientation must still agree.
//!
//! The production node-boot world exposes no player route that grants an owned house (ownership
//! is only settled Control-side), so the catalogue is empty or refused and the row ends FAIL
//! naming that precondition. It never provisions state.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{house_rotation as wire, world_map::{MapDefinition, MapOrigin}};
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(75), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "rotation-committed-and-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G07.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn exchange(
    client: &mut DevClientSession,
    request: &wire::Request,
) -> TestResult<wire::Reply> {
    let payload = request
        .encode()
        .ok_or("step=encode observed=request-refused")?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1,
            wire::COMMAND,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("step=command observed=status:{:?}", response.status).into());
    }
    wire::Reply::decode(&response.payload)
        .ok_or_else(|| format!("step=decode observed=payload:{:?}", response.payload).into())
}

/// Stable identity of a target and its current revision.
fn identity(target: &wire::Target) -> (Vec<u8>, u64) {
    match target {
        wire::Target::Item {
            item,
            expected_revision,
        } => (item.to_vec(), *expected_revision),
        wire::Target::Base(base) => (base.placement.clone(), base.expected_revision),
    }
}

async fn first_entry(client: &mut DevClientSession) -> TestResult<Option<wire::Entry>> {
    match exchange(client, &wire::Request::Query(0)).await? {
        wire::Reply::Catalogue(catalogue) => Ok(catalogue.entries.into_iter().next()),
        wire::Reply::Applied(disposition) => Err(format!(
            "step=query observed=applied:{disposition:?};needed=owned-house;settlement-only:crates/gameplay/durability/src/durability/house_ownership/settlement.rs"
        )
        .into()),
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

fn rotation_matches(before: u64, after: u64, expected_definition: &str, after_definition: &str) -> bool {
    after > before && expected_definition == after_definition
}

async fn orientation(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    target: &wire::Target,
) -> TestResult<String> {
    match target {
        wire::Target::Item { item, .. } => {
            let mut database = sqlx::PgConnection::connect(clients.url).await?;
            Ok(sqlx::query_scalar(
                "SELECT definition_production_key FROM game_item_instances                  WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND lifecycle=1",
            ).bind(item.as_slice()).bind(clients.world.as_slice()).fetch_one(&mut database).await?)
        }
        wire::Target::Base(base) => {
            let (digest, key) = base.placement.split_at_checked(32).ok_or("step=orientation observed=short-placement")?;
            let (position, ordinal) = crate::map::view::placement_of(u64::from_be_bytes(key.try_into()?))
                .ok_or("step=orientation observed=invalid-placement")?;
            let view = client.session_mut().world_map_snapshot().filter(|view| view.header.bundle_digest == digest)
                .ok_or("step=orientation observed=no-map-for-placement")?;
            let item = view.tiles.iter().find(|tile| tile.position.x == i32::from(position.x)
                && tile.position.y == i32::from(position.y) && tile.position.floor == i16::from(position.floor))
                .and_then(|tile| tile.items.iter().find(|item| matches!(item.origin, MapOrigin::BaseOrdinal { ordinal: actual, .. } if actual == ordinal)))
                .ok_or("step=orientation observed=base-item-not-visible")?;
            let MapDefinition::Item(reference) = item.definition else {
                return Err("step=orientation observed=placement-not-an-item".into());
            };
            let index = provisioning::item_index()?;
            Ok(index.definition_at(reference).ok_or("step=orientation observed=unknown-definition")?.0.to_owned())
        }
    }
}

fn rotated_definition(before: &str) -> TestResult<String> {
    let id = before.strip_prefix("oteryn:item.tibia.i").ok_or("step=orientation observed=non-native-item")?.parse::<u64>()?;
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tools/content-schema/native-gameplay/house-decoration-actions.json"
    ))?;
    let target = catalogue["native_transforms"].as_array()
        .and_then(|rules| rules.iter().find(|rule| rule["kind"] == "ROTATE" && rule["item_id"].as_u64() == Some(id)))
        .and_then(|rule| rule["target"].as_u64())
        .filter(|target| *target != id)
        .ok_or("step=orientation observed=no-authored-rotation-target")?;
    Ok(format!("oteryn:item.tibia.i{target}"))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xc9).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_HOUSE_FURNITURE_ROTATION_V1)
    {
        let selected = format!("{:?}", client.selected_capabilities());
        logout(&mut client).await?;
        return Err(format!(
            "step=admit observed=capability-445-not-selected;selected={selected};needed=house-capability"
        )
        .into());
    }
    let entry = first_entry(&mut client).await?.ok_or("step=query needed=owned-house-with-rotatable-furniture")?;
    let (key, before) = identity(&entry.target);
    let expected = rotated_definition(&orientation(&mut client, clients, &entry.target).await?)?;
    let applied = exchange(&mut client, &wire::Request::Rotate(entry.target.clone())).await?;
    if applied != wire::Reply::Applied(wire::UseDisposition::Committed) {
        return Err(format!("step=rotate observed={applied:?};needed=committed").into());
    }
    let after_entry = first_entry_with(&mut client, &key).await?;
    let (_, after) = identity(&after_entry.target);
    let current = orientation(&mut client, clients, &after_entry.target).await?;
    if !rotation_matches(before, after, &expected, &current) {
        return Err(format!("step=rotate observed=revision:{before}->{after},definition:{current};needed=advance-and-{expected}").into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xca).await?;
    let fresh_entry = first_entry_with(&mut fresh, &key).await?;
    let (_, durable) = identity(&fresh_entry.target);
    let durable_definition = orientation(&mut fresh, clients, &fresh_entry.target).await?;
    logout(&mut fresh).await?;
    if durable != after || durable_definition != expected {
        return Err(format!("step=fresh observed=revision:{durable},definition:{durable_definition};needed={after},{expected}").into());
    }
    Ok(())
}

/// Find the same target even if its new orientation changes its sort position.
async fn first_entry_with(client: &mut DevClientSession, key: &[u8]) -> TestResult<wire::Entry> {
    for page in 0..=wire::MAX_PAGE {
        match exchange(client, &wire::Request::Query(page)).await? {
            wire::Reply::Catalogue(catalogue) if catalogue.page == page => {
                if let Some(entry) = catalogue.entries.into_iter().find(|entry| identity(&entry.target).0 == key) {
                    return Ok(entry);
                }
                if !catalogue.more {
                    break;
                }
            }
            other => return Err(format!("step=query observed={other:?};needed=page-{page}").into()),
        }
    }
    Err("step=query observed=rotated-target-not-listed".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_separates_item_and_base_targets() {
        let item = wire::Target::Item {
            item: [7; 16],
            expected_revision: 9,
        };
        assert_eq!(identity(&item), (vec![7; 16], 9));
        assert_eq!(token("a  b"), "a-b");
    }

    #[test]
    fn rotation_requires_the_expected_orientation_as_well_as_revision() {
        assert!(!rotation_matches(1, 2, "east", "north"));
        assert!(!rotation_matches(1, 1, "east", "east"));
        assert!(rotation_matches(1, 2, "east", "east"));
    }
}
