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
//! The shared G01 fixture supplies settled ownership. Control assigns that House to the serving
//! node before the live journey walks to its authored entrance and requests the handoff.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::house_rotation as wire;
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(75), verdict::observe(journey(clients))).await;
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
    let (result, detail, _, _) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={detail}").into());
    }
    Ok(())
}

async fn assign_fixture_scope(clients: &SeamClients<'_>) -> TestResult {
    use crate::durability::house_scope_handoff::{
        HouseId, HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome,
        HouseScopeAssignmentRequest,
    };
    use crate::durability::runtime_scope_assignment::{
        ControlActor, NodeRegistrationFact, OperationKey,
    };
    use full_row_g01::HOUSE;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let (node, revision, role): (Vec<u8>, String, String) = sqlx::query_as(
        "SELECT uuid_send(holder_node_id),holder_registration_revision::text,session_user::text
         FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid
           AND channel_id=encode($2,'hex')::uuid AND state=1",
    )
    .bind(clients.world.as_slice())
    .bind(clients.channel.as_slice())
    .fetch_one(&mut database)
    .await?;
    let house =
        HouseId::new(WorldId::decode(&clients.world)?, HOUSE).ok_or("G07 House identity")?;
    sqlx::query(
        "INSERT INTO game_control_house_scope_grants(control_role,world_id,house_key,operation)
         VALUES(session_user,encode($1,'hex')::uuid,$2,1) ON CONFLICT DO NOTHING",
    )
    .bind(clients.world.as_slice())
    .bind(HOUSE)
    .execute(&mut database)
    .await?;
    let target =
        NodeRegistrationFact::new(crate::foundation::NodeId::decode(&node)?, revision.parse()?);
    let root = crate::durability::DurabilityRoot::connect_test_runtime(clients.url)?;
    root.maintain_ready_once().await?;
    let request = HouseScopeAssignmentRequest {
        operation_key: OperationKey::from_bytes([0xc9; 32]),
        actor: ControlActor::new(&role).map_err(|error| format!("G07 Control actor: {error:?}"))?,
        actor_role: role,
        command: HouseScopeAssignmentCommand::Assign { house, target },
    };
    let outcome = root
        .assign_house_scope(request)
        .await
        .map_err(|error| format!("G07 scope assignment: {error:?}"))?;
    if !matches!(
        outcome,
        HouseScopeAssignmentOutcome::Committed(_) | HouseScopeAssignmentOutcome::Replayed
    ) {
        return Err(format!("G07 scope assignment refused: {outcome:?}").into());
    }
    Ok(())
}

async fn enter_house(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    use full_row_g01::HOUSE;
    use oteryn_dev_client::client_runtime::house_transfer as transfer;
    let world = full_e2ef7::qualification_world(clients)?;
    let house = full_row_g01::authored_house()?;
    let coordinate = |name| {
        house["entrance"][name]
            .as_u64()
            .ok_or("G07 authored entrance coordinate")
    };
    let entrance = TilePos {
        x: u16::try_from(coordinate("x")?)?,
        y: u16::try_from(coordinate("y")?)?,
        floor: -i8::try_from(coordinate("z")?)?,
    };
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == entrance
    })
    .ok_or("step=house-entry observed=no-route-to-authored-entrance")?;
    walk(client, world, &path).await?;
    let request = transfer::encode_house_transfer_intent(&transfer::HouseTransferIntent {
        exit: false,
        house: HOUSE.into(),
    })
    .map_err(|error| format!("G07 house entry encoding: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            transfer::CAPABILITY_HOUSE_TRANSFER_V1,
            transfer::COMMAND_TYPE_HOUSE_TRANSFER,
            &request,
        )
        .await?;
    let ack = transfer::decode_house_transfer_ack(&reply.payload).map_err(|error| {
        format!(
            "step=house-entry status={:?} decode={error:?}",
            reply.status
        )
    })?;
    if reply.status != CommandStatus::Accepted || ack.house.as_deref() != Some(HOUSE) {
        return Err("step=house-entry observed=missing-authorized-house-scope".into());
    }
    evidence("G07_CHECK step=house-transfer status=PASS");
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let accounts = [required("NODE_BOOT_G01_ACCOUNT")?];
    let characters = [full_row_g01::fixture_character(clients, &accounts[0]).await?];
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    full_row_g01::prepare_owned_house(clients).await?;
    assign_fixture_scope(clients).await?;
    let mut client = connect_character(clients, 0xc9, 0).await?;
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
    enter_house(&mut client, clients).await?;
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
    let mut fresh = connect_character(clients, 0xca, 0).await?;
    // Fresh admission may restore the saved House scope. Enter from the
    // authored door only when the join snapshot is in the Channel.
    if fresh
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::house_transfer::STATE_DOMAIN_HOUSE_SCOPE)
        .is_none()
    {
        enter_house(&mut fresh, clients).await?;
    }
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
