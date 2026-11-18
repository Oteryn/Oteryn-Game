//! I10 client replay capture: real house-furniture wrap / unwrap / stale-revision replies.
//!
//! The replies are replayed through the production reducer `apply` in
//! `apps/client/src/features/0207-house-furniture-wrap/protocol.rs`, which turns the command
//! status and payload into the player-visible feedback. The client acts on one original piece of furniture in the fixture’s controlled house.
use super::*;
use oteryn_protocol_oteryn::house_wrap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(220), verdict::observe(journey(clients))).await;
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

async fn restore_wrapped(
    client: &mut oteryn_dev_client::DevClientSession, db: &mut sqlx::PgConnection,
    mut request: house_wrap::Request,
) -> TestResult {
    let (definition, revision): (String, String) = sqlx::query_as(
        "SELECT definition_production_key,state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1"
    ).bind(request.item.as_slice()).fetch_one(db).await?;
    if definition == "oteryn:item.tibia.i6355" {
        return Ok(());
    }
    if !matches!(definition.as_str(), "oteryn:item.tibia.i6371" | "oteryn:item.tibia.i23398") {
        return Err(format!("I10 unexpected furniture definition during recovery: {definition}").into());
    }
    request.expected_revision = revision.parse()?;
    let next = request.expected_revision.checked_add(1).ok_or("I10 recovery revision exhausted")?;
    wrap_result(client, request, house_wrap::Reply::Committed(next)).await?;
    Ok(())
}

// This setup is confined to the disposable node-boot database. Reuse the
// existing owned-House test data and the production Control assignment owner.
async fn prepare_fixture(
    db: &mut sqlx::PgConnection, clients: &SeamClients<'_>, world: &BundleWorld, from: TilePos,
) -> TestResult {
    if std::env::var("NODE_BOOT_DATABASE_URL").ok().as_deref() != Some(clients.url) {
        return Err("I10 fixture preparation requires the disposable node-boot database".into());
    }
    use crate::durability::house_scope_handoff::{
        HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome, HouseScopeAssignmentRequest,
    };
    use crate::durability::runtime_scope_assignment::{ControlActor, NodeRegistrationFact, OperationKey};
    let world_id = WorldId::decode(&clients.world)?;
    let existing: Vec<String> = sqlx::query_scalar(
        "SELECT house_key FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid
         UNION SELECT house_key FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=2",
    ).bind(clients.world.as_slice()).fetch_all(&mut *db).await?;
    let mut candidates = Vec::new();
    for catalogue in [
        include_str!("../../../../../content/houses/houses-00000-00499.json"),
        include_str!("../../../../../content/houses/houses-00500-00994.json"),
    ] {
        let rows: serde_json::Value = serde_json::from_str(catalogue)?;
        for row in rows["houses"].as_array().ok_or("I10 houses missing")? {
            let Some(key) = row["identity"]["key"].as_str() else { continue; };
            if row["kind"] != "private_house" || existing.iter().any(|occupied| occupied == key) {
                continue;
            }
            let entrance = &row["entrance"];
            let Some(at) = serde_json::from_value::<(u16, u16, i8)>(serde_json::json!([
                entrance["x"], entrance["y"], entrance["z"]
            ])).ok().map(|(x, y, z)| TilePos { x, y, floor: -z }) else { continue; };
            if !world.enterable(at) || at.floor != from.floor { continue; }
            let Some(tiles) = row["tiles"].as_array() else { continue; };
            let Some(position) = tiles.iter()
                .filter_map(|value| serde_json::from_value::<[i32; 3]>(value.clone()).ok())
                .filter(|[_, _, floor]| *floor == -i32::from(at.floor))
                .filter(|[x,y,z]| {
                    let Ok(pos) = tile(ActorPosition { x: *x, y: *y, floor: *z as i16 }) else { return false; };
                    world.enterable(pos)
                })
                .min_by_key(|[x,y,_]| x.abs_diff(i32::from(at.x)).max(y.abs_diff(i32::from(at.y))))
            else { continue; };
            candidates.push((at.x.abs_diff(from.x).max(at.y.abs_diff(from.y)), key.to_owned(), at, position));
        }
    }
    candidates.sort_unstable();
    let (_, house, _, position) = candidates.into_iter()
        .find(|(_, _, entrance, _)| route(world, from, |at| at == *entrance).is_some())
        .ok_or("I10 no reachable unused house for the disposable fixture")?;

    let (node, revision, role): (Vec<u8>, String, String) = sqlx::query_as(
        "SELECT uuid_send(holder_node_id),holder_registration_revision::text,session_user::text
         FROM game_runtime_scope_assignments
         WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1",
    ).bind(clients.world.as_slice()).bind(clients.channel.as_slice()).fetch_one(&mut *db).await?;
    let target = NodeRegistrationFact::new(crate::foundation::NodeId::decode(&node)?, revision.parse()?);
    let mut tx = db.begin().await?;
    sqlx::query(include_str!("../../../tests/support/owned_house_fixture.sql"))
        .bind(clients.world.as_slice()).bind(&house).bind(clients.characters[1].as_slice())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_control_house_scope_grants VALUES(session_user,encode($1,'hex')::uuid,$2,1)")
        .bind(clients.world.as_slice()).bind(&house).execute(&mut *tx).await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    let item: Vec<u8> = sqlx::query_scalar(
        "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
         VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i6355','definition-r1',1,1,game_character_uuid_v7()) RETURNING uuid_send(item_instance_id)",
    ).bind(clients.world.as_slice()).fetch_one(&mut *tx).await?;
    let encoded: Vec<u8> = position.into_iter().flat_map(i32::to_be_bytes).collect();
    sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
        .bind(clients.world.as_slice()).bind(&house).bind(&encoded).execute(&mut *tx).await?;
    let placement: Vec<u8> = sqlx::query_scalar(
        "INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,game_character_uuid_v7()) RETURNING uuid_send(placed_transaction_id)",
    ).bind(&item).bind(clients.world.as_slice()).bind(&house).bind(&encoded).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1)")
        .bind(&item).bind(clients.world.as_slice()).bind(&house).bind(clients.characters[1].as_slice()).bind(&placement)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    let mut operation = [0; 32];
    operation[..16].copy_from_slice(&clients.world);
    operation[16..].copy_from_slice(&item);
    let control = crate::durability::DurabilityRoot::connect_test_runtime(clients.url)?;
    let outcome = control.assign_house_scope(HouseScopeAssignmentRequest {
        operation_key: OperationKey::from_bytes(operation),
        actor: ControlActor::new(&role).map_err(|error| format!("I10 control actor: {error:?}"))?,
        actor_role: role,
        command: HouseScopeAssignmentCommand::Assign {
            house: crate::durability::house_scope_handoff::HouseId::new(world_id, &house).ok_or("I10 house identity")?,
            target,
        },
    }).await.map_err(|error| format!("I10 control assignment: {error:?}"))?;
    if !matches!(outcome, HouseScopeAssignmentOutcome::Committed(_) | HouseScopeAssignmentOutcome::Replayed) {
        return Err(format!("I10 fixture House assignment failed: {outcome:?}").into());
    }
    Ok(())
}

type PlacedOven = (String, Vec<u8>, String, String, Vec<u8>, bool);

async fn read_fixture(db: &mut sqlx::PgConnection, clients: &SeamClients<'_>) -> TestResult<Option<PlacedOven>> {
    let fixture: Option<PlacedOven> =
        sqlx::query_as("SELECT * FROM (
            SELECT l.house_key,uuid_send(l.item_instance_id),i.state_revision::text,p.acl_revision::text,l.spatial_position,false AS wrapped
            FROM game_item_house_interior_locations l
            JOIN game_item_instances i USING (world_id,item_instance_id)
            JOIN game_house_properties p USING (world_id,house_key)
            WHERE l.world_id=encode($1,'hex')::uuid AND p.owner_character_id=encode($2,'hex')::uuid
              AND i.definition_production_key='oteryn:item.tibia.i6355' AND i.lifecycle=1
            UNION ALL
            SELECT r.house_key,uuid_send(r.item_instance_id),i.state_revision::text,p.acl_revision::text,
                   decode(substr(r.house_location->>'spatial_position',3),'hex'),true
            FROM game_house_furniture_wrap_receipts r
            JOIN game_item_instances i USING (world_id,item_instance_id)
            JOIN game_house_properties p USING (world_id,house_key)
            JOIN game_item_container_entries e USING (world_id,item_instance_id)
            WHERE r.world_id=encode($1,'hex')::uuid AND r.character_id=encode($2,'hex')::uuid
              AND e.character_id=r.character_id AND p.owner_character_id=r.character_id
              AND i.last_transaction_id=r.transaction_id AND i.lifecycle=1 AND r.action=1
              AND r.before_item->>'definition_production_key'='oteryn:item.tibia.i6355'
              AND i.definition_production_key IN ('oteryn:item.tibia.i6371','oteryn:item.tibia.i23398')
            ) fixture ORDER BY wrapped DESC,house_key LIMIT 1")
        .bind(clients.world.as_slice()).bind(clients.characters[1].as_slice())
        .fetch_optional(&mut *db).await?;
    Ok(fixture)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let mut client = connect_world(clients, 0xb2).await?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let fixture = read_fixture(&mut db, clients).await?;
    let (house, id, revision, acl, position, needs_restore) = if let Some(fixture) = fixture {
        fixture
    } else {
        prepare_fixture(&mut db, clients, world, tile(client.world_spatial().actor_position)?).await?;
        read_fixture(&mut db, clients).await?.ok_or("I10 setup did not publish the fixture")?
    };
    let entrance = full_row_s05::entrance(&house)?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |candidate| candidate == entrance)
        .ok_or("I10 controlled house entrance unreachable")?;
    walk(&mut client, world, &path).await?;
    let mut original: u64 = revision.parse()?;
    let item = id.try_into().map_err(|_| "I10 invalid item identity")?;
    let position = crate::durability::consume_transform::HouseActionContext::decode_spatial_position(&position)
        .ok_or("I10 invalid furniture position")?;
    let acl_revision = acl.parse()?;
    transfer(&mut client, &house, false).await?;
    let request = |action, expected_revision| house_wrap::Request { action, item, expected_revision, acl_revision, position };
    if needs_restore {
        restore_wrapped(&mut client, &mut db, request(house_wrap::Action::Unwrap, original)).await?;
        original = original.checked_add(1).ok_or("I10 recovered revision exhausted")?;
    }
    let wrapped = original.checked_add(1).ok_or("I10 revision exhausted")?;
    let restored = wrapped.checked_add(1).ok_or("I10 revision exhausted")?;
    let wrap_request = request(house_wrap::Action::Wrap, original);
    let unwrap_request = request(house_wrap::Action::Unwrap, wrapped);
    let stale_request = request(house_wrap::Action::Wrap, original);
    let proofs = tokio::time::timeout(Duration::from_secs(18), async {
        let wrap = wrap_result(&mut client, wrap_request, house_wrap::Reply::Committed(wrapped)).await?;
        let unwrap = wrap_result(&mut client, unwrap_request, house_wrap::Reply::Committed(restored)).await?;
        let stale = wrap_result(&mut client, stale_request, house_wrap::Reply::Refused(house_wrap::Refusal::Stale)).await?;
        Ok::<_, Box<dyn std::error::Error>>((wrap, unwrap, stale))
    }).await.unwrap_or_else(|_| Err("I10 wrap sequence timed out".into()));
    let recovery = if proofs.is_err() {
        restore_wrapped(&mut client, &mut db, unwrap_request).await
    } else {
        Ok(())
    };
    let exit = transfer(&mut client, &house, true).await;
    if let Err(error) = &recovery {
        evidence(&format!("I10 oven restoration failed: {error}"));
    }
    let (wrap, unwrap, stale) = proofs?;
    recovery?;
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
