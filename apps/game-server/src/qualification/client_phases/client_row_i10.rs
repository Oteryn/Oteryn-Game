//! I10 client replay capture: real house-furniture wrap / unwrap / stale-revision replies.
//!
//! The replies are replayed through the production reducer `apply` in
//! `apps/client/src/features/0207-house-furniture-wrap/protocol.rs`, which turns the command
//! status and payload into the player-visible feedback. The client acts on one original piece of furniture in the fixture’s controlled house.
use super::*;
use oteryn_protocol_oteryn::house_wrap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    match result.unwrap_or_else(|_| Err("I10 client capture timed out".into())) {
        Ok(()) => evidence(
            "e2e_all scenario=I10.client status=PASS evidence=house-wrap-unwrap-and-stale-refusals",
        ),
        Err(error) => evidence(&format!(
            "e2e_all scenario=I10.client status=FAIL evidence={error}"
        )),
    }
}

fn wrap_matches(status: CommandStatus, actual: house_wrap::Reply, wanted: house_wrap::Reply) -> bool {
    actual == wanted && match wanted {
        house_wrap::Reply::Committed(_) => status == CommandStatus::Accepted,
        house_wrap::Reply::Refused(_) => status == CommandStatus::Rejected,
    }
}

async fn transfer(client: &mut oteryn_dev_client::DevClientSession, house: &str, exit: bool) -> TestResult {
    use oteryn_dev_client::client_runtime::house_transfer as wire;
    let payload = wire::encode_house_transfer_intent(&wire::HouseTransferIntent {
        exit,
        house: house.to_owned(),
    }).map_err(|error| format!("I10 house transfer intent: {error:?}"))?;
    let result = client.session_mut()
        .gated_command(wire::CAPABILITY_HOUSE_TRANSFER_V1, wire::COMMAND_TYPE_HOUSE_TRANSFER, &payload)
        .await?;
    let ack = wire::decode_house_transfer_ack(&result.payload)
        .map_err(|error| format!("I10 transfer status={:?}: {error:?}", result.status))?;
    if result.status != CommandStatus::Accepted || ack.house.as_deref() != (!exit).then_some(house) {
        return Err(format!("I10 wrong transfer scope: {ack:?}").into());
    }
    Ok(())
}

async fn wrap_result(
    client: &mut oteryn_dev_client::DevClientSession,
    request: house_wrap::Request,
    wanted: house_wrap::Reply,
) -> TestResult<Vec<u8>> {
    let result = client.session_mut().gated_command(
        house_wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
        house_wrap::COMMAND_WRAP,
        &request.encode(),
    ).await?;
    let actual = house_wrap::Reply::decode(&result.payload);
    if !actual.is_some_and(|actual| wrap_matches(result.status, actual, wanted)) {
        return Err(format!("I10 action={:?} expected={wanted:?} observed={:?}/{actual:?}", request.action, result.status).into());
    }
    Ok(result.payload)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let (house, id, revision, acl, position): (String, Vec<u8>, String, String, Vec<u8>) =
        sqlx::query_as("SELECT l.house_key,uuid_send(l.item_instance_id),i.state_revision::text,p.acl_revision::text,l.spatial_position
            FROM game_item_house_interior_locations l
            JOIN game_item_instances i USING (world_id,item_instance_id)
            JOIN game_house_properties p USING (world_id,house_key)
            WHERE l.world_id=encode($1,'hex')::uuid AND p.owner_character_id=encode($2,'hex')::uuid
              AND i.definition_production_key='oteryn:item.tibia.i6355' AND i.lifecycle=1
            ORDER BY l.house_key,l.stack_ordinal LIMIT 1")
        .bind(clients.world.as_slice()).bind(clients.characters[1].as_slice())
        .fetch_optional(&mut db).await?.ok_or("I10 fixture owns no house with a placed wrappable oven")?;
    let mut client = connect_world(clients, 0xb2).await?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let entrance = full_row_s05::entrance(&house)?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |candidate| candidate == entrance)
        .ok_or("I10 controlled house entrance unreachable")?;
    walk(&mut client, world, &path).await?;
    let original: u64 = revision.parse()?;
    let wrapped = original.checked_add(1).ok_or("I10 revision exhausted")?;
    let restored = wrapped.checked_add(1).ok_or("I10 revision exhausted")?;
    let item = id.try_into().map_err(|_| "I10 invalid item identity")?;
    let position = crate::durability::consume_transform::HouseActionContext::decode_spatial_position(&position)
        .ok_or("I10 invalid furniture position")?;
    let acl_revision = acl.parse()?;
    transfer(&mut client, &house, false).await?;
    let request = |action, expected_revision| house_wrap::Request { action, item, expected_revision, acl_revision, position };
    let wrap_request = request(house_wrap::Action::Wrap, original);
    let unwrap_request = request(house_wrap::Action::Unwrap, wrapped);
    let stale_request = request(house_wrap::Action::Wrap, original);
    let proofs = async {
        let wrap = wrap_result(&mut client, wrap_request, house_wrap::Reply::Committed(wrapped)).await?;
        let unwrap = wrap_result(&mut client, unwrap_request, house_wrap::Reply::Committed(restored)).await?;
        let stale = wrap_result(&mut client, stale_request, house_wrap::Reply::Refused(house_wrap::Refusal::Stale)).await?;
        Ok::<_, Box<dyn std::error::Error>>((wrap, unwrap, stale))
    }.await;
    let exit = transfer(&mut client, &house, true).await;
    let (wrap, unwrap, stale) = proofs?;
    exit?;
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = Path::new(&directory);
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        for (name, request, reply) in [
            ("i10-wrap", wrap_request, &wrap),
            ("i10-unwrap", unwrap_request, &unwrap),
            ("i10-refusal", stale_request, &stale),
        ] {
            std::fs::write(directory.join(format!("{name}.bin")), reply)?;
            std::fs::write(directory.join(format!("{name}-request.bin")), request.encode())?;
        }
        std::fs::write(
            directory.join("i10-expected.txt"),
            format!(
                "wrap={}\nunwrap={}\nrefusal={}\n",
                hex(&wrap),
                hex(&unwrap),
                hex(&stale)
            ),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_wrap_requires_commits_and_exact_stale_refusal() {
        let stale = house_wrap::Reply::Refused(house_wrap::Refusal::Stale);
        let outside = house_wrap::Reply::Refused(house_wrap::Refusal::NotInHouse);
        assert!(!wrap_matches(CommandStatus::Rejected, outside, stale));
        assert!(wrap_matches(CommandStatus::Rejected, stale, stale));
        assert!(wrap_matches(CommandStatus::Accepted, house_wrap::Reply::Committed(2), house_wrap::Reply::Committed(2)));
        assert!(!wrap_matches(CommandStatus::Accepted, house_wrap::Reply::Committed(3), house_wrap::Reply::Committed(2)));
    }
}
