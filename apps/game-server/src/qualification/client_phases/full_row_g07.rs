//! G07: house furniture rotation and decorations (apps/client 0445-house-furniture-rotation).
//! 1. Admit the node-boot character; require the rotation capability (needs HOUSE and a native
//!    bundle map) in the negotiated session.
//! 2. QUERY page 0 of the rotatable furniture/decorations of the actor's current house: expect an
//!    accepted catalogue with at least one authoritative target.
//! 3. ROTATE the first target through the same request the panel sends: expect `Applied` with
//!    `Committed`.
//! 4. QUERY again: the rotated target must be listed with a newer revision (the state delta).
//! 5. Log out, freshly admit, QUERY: the durable revision equals the one seen after the rotate.
//!
//! The production node-boot world exposes no player route that grants an owned house (ownership
//! is only settled Control-side), so the catalogue is empty or refused and the row ends FAIL
//! naming that precondition. It never provisions state.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::house_rotation as wire;

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
    let query = first_entry(&mut client).await;
    let entry = match query {
        Ok(Some(entry)) => entry,
        other => {
            let observed = match other {
                Ok(_) => "empty-catalogue;needed=owned-house-with-rotatable-furniture".to_owned(),
                Err(error) => error.to_string(),
            };
            logout(&mut client).await?;
            return Err(format!("step=query observed={observed}").into());
        }
    };
    let (key, before) = identity(&entry.target);
    let applied = exchange(&mut client, &wire::Request::Rotate(entry.target.clone())).await?;
    if applied != wire::Reply::Applied(wire::UseDisposition::Committed) {
        return Err(format!("step=rotate observed={applied:?};needed=committed").into());
    }
    let after = first_entry_with(&mut client, &key).await?;
    if after <= before {
        return Err(
            format!("step=rotate observed=revision:{before}->{after};needed=advance").into(),
        );
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xca).await?;
    let durable = first_entry_with(&mut fresh, &key).await;
    logout(&mut fresh).await?;
    if durable? != after {
        return Err("step=fresh observed=revision-not-durable".into());
    }
    Ok(())
}

/// Revision of the target with identity `key` in the current page-0 catalogue.
async fn first_entry_with(client: &mut DevClientSession, key: &[u8]) -> TestResult<u64> {
    match exchange(client, &wire::Request::Query(0)).await? {
        wire::Reply::Catalogue(catalogue) => catalogue
            .entries
            .iter()
            .map(|entry| identity(&entry.target))
            .find(|(candidate, _)| candidate == key)
            .map(|(_, revision)| revision)
            .ok_or_else(|| "step=query observed=rotated-target-not-listed".into()),
        wire::Reply::Applied(disposition) => {
            Err(format!("step=query observed=applied:{disposition:?}").into())
        }
    }
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
}
