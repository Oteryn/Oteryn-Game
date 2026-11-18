//! I10 client replay capture: real house-furniture wrap / unwrap / stale-revision replies.
//!
//! The replies are replayed through the production reducer `apply` in
//! `apps/client/src/features/0207-house-furniture-wrap/protocol.rs`, which turns the command
//! status and payload into the player-visible feedback. The client acts on one original piece of furniture in the fixture’s controlled house.
use super::*;
use oteryn_protocol_oteryn::house_wrap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::observe(journey(clients)).await;
    match result {
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

async fn restoration_request(
    db: &mut sqlx::PgConnection, mut request: house_wrap::Request,
) -> TestResult<Option<(house_wrap::Request, house_wrap::Reply)>> {
    let (definition, revision): (String, String) = sqlx::query_as(
        "SELECT definition_production_key,state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1"
    ).bind(request.item.as_slice()).fetch_one(db).await?;
    if definition == "oteryn:item.tibia.i6355" {
        return Ok(None);
    }
    if !matches!(definition.as_str(), "oteryn:item.tibia.i6371" | "oteryn:item.tibia.i23398") {
        return Err(format!("I10 unexpected furniture definition during recovery: {definition}").into());
    }
    request.expected_revision = revision.parse()?;
    let next = request.expected_revision.checked_add(1).ok_or("I10 recovery revision exhausted")?;
    Ok(Some((request, house_wrap::Reply::Committed(next))))
}

async fn restore_wrapped(
    client: &mut oteryn_dev_client::DevClientSession, db: &mut sqlx::PgConnection,
    request: house_wrap::Request,
) -> TestResult {
    if let Some((request, wanted)) = restoration_request(db, request).await? {
        wrap_result(client, request, wanted).await?;
    }
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
    let owned_house: Option<String> = sqlx::query_scalar(
        "SELECT h.house_key FROM game_account_housing_slots h
         JOIN game_character_roots c ON c.account_id=h.account_id
         WHERE h.world_id=encode($1,'hex')::uuid AND c.character_id=encode($2,'hex')::uuid",
    ).bind(clients.world.as_slice()).bind(clients.characters[1].as_slice()).fetch_optional(&mut *db).await?;
    let mut candidates = Vec::new();
    for catalogue in [
        include_str!("../../../../../content/houses/houses-00000-00499.json"),
        include_str!("../../../../../content/houses/houses-00500-00994.json"),
    ] {
        let rows: serde_json::Value = serde_json::from_str(catalogue)?;
        for row in rows["houses"].as_array().ok_or("I10 houses missing")? {
            let Some(key) = row["identity"]["key"].as_str() else { continue; };
            if row["kind"] != "private_house" || owned_house.as_deref().is_some_and(|owned| owned != key) {
                continue;
            }
            let entrance = &row["entrance"];
            let Some(at) = serde_json::from_value::<(u16, u16, i8)>(serde_json::json!([
                entrance["x"], entrance["y"], entrance["z"]
            ])).ok().map(|(x, y, z)| TilePos { x, y, floor: -z }) else { continue; };
            if !world.enterable(at) || at.floor != from.floor { continue; }
            let Some(tiles) = row["tiles"].as_array() else { continue; };
            let mut positions: Vec<_> = tiles.iter()
                .filter_map(|value| serde_json::from_value::<[i32; 3]>(value.clone()).ok())
                .filter(|[_, _, floor]| *floor == -i32::from(at.floor))
                .filter(|[x,y,z]| {
                    let Ok(pos) = tile(ActorPosition { x: *x, y: *y, floor: *z as i16 }) else { return false; };
                    world.enterable(pos)
                })
                .collect();
            positions.sort_by_key(|[x,y,_]| x.abs_diff(i32::from(at.x)).max(y.abs_diff(i32::from(at.y))));
            candidates.push((at.x.abs_diff(from.x).max(at.y.abs_diff(from.y)), key.to_owned(), at, positions));
        }
    }
    candidates.sort_unstable();
    let mut selected = None;
    for (_, house, entrance, positions) in candidates {
        if route(world, from, |at| at == entrance).is_none() { continue; }
        let available: bool = sqlx::query_scalar(
            "SELECT CASE WHEN $3 THEN EXISTS (SELECT 1 FROM game_house_properties
                WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND owner_character_id=encode($4,'hex')::uuid)
             ELSE NOT EXISTS (SELECT 1 FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2)
                  AND NOT EXISTS (SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND house_key=$2)
             END",
        ).bind(clients.world.as_slice()).bind(&house).bind(owned_house.is_some())
            .bind(clients.characters[1].as_slice()).fetch_one(&mut *db).await?;
        if !available {
            continue;
        }
        let encoded: Vec<Vec<u8>> = positions.into_iter()
            .map(|position| position.into_iter().flat_map(i32::to_be_bytes).collect())
            .collect();
        let vacant: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT tile.position FROM unnest($1::bytea[]) WITH ORDINALITY tile(position, ordinal)
             WHERE NOT EXISTS (SELECT 1 FROM game_item_house_interior_locations l
               WHERE l.world_id=encode($2,'hex')::uuid AND l.house_key=$3 AND l.spatial_position=tile.position)
             ORDER BY tile.ordinal LIMIT 1",
        ).bind(encoded).bind(clients.world.as_slice()).bind(&house).fetch_optional(&mut *db).await?;
        if let Some(position) = vacant {
            selected = Some((house, position));
            break;
        }
    }
    let (house, encoded) = selected.ok_or("I10 no reachable house for the disposable fixture")?;
    let mut tx = db.begin().await?;
    let generation: String = sqlx::query_scalar(
        "SELECT ownership_generation::text FROM game_runtime_scope_assignments
         WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1 FOR SHARE",
    ).bind(clients.world.as_slice()).bind(clients.channel.as_slice()).fetch_one(&mut *tx).await?;
    if generation.parse::<u64>()? != clients.scope_generation {
        return Err("I10 channel generation changed during fixture planning".into());
    }
    if owned_house.is_none() {
        sqlx::query(include_str!("../../../tests/support/owned_house_fixture.sql"))
            .bind(clients.world.as_slice()).bind(&house).bind(clients.characters[1].as_slice())
            .execute(&mut *tx).await?;
    }
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    let item: Vec<u8> = sqlx::query_scalar(
        "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
         VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i6355','definition-r1',1,1,game_character_uuid_v7()) RETURNING uuid_send(item_instance_id)",
    ).bind(clients.world.as_slice()).fetch_one(&mut *tx).await?;
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
    Ok(())
}

async fn begin_assignment(db: &mut sqlx::PgConnection) -> TestResult<sqlx::Transaction<'_, sqlx::Postgres>> {
    // Match the durability writer's budget on this dedicated connection before
    // taking the writer lock. The server can release it even if the client stops polling.
    let budget = crate::durability::DB_PASS_DEADLINE;
    let mut tx = db.begin().await?;
    sqlx::query(
        "SELECT set_config('transaction_timeout',$1,true),
                set_config('statement_timeout',$1,true),
                set_config('lock_timeout',$1,true)",
    ).bind(format!("{}ms", budget.as_millis())).execute(&mut *tx).await?;
    Ok(tx)
}

async fn ensure_assignment(
    db: &mut sqlx::PgConnection, clients: &SeamClients<'_>, house: &str, item: &[u8; 16],
) -> TestResult {
    use crate::durability::house_scope_handoff::{
        HouseScopeAssignmentCommand, HouseScopeAssignmentOutcome, HouseScopeAssignmentRequest,
        assign_house_scope_in_transaction,
    };
    use crate::durability::runtime_scope_assignment::{ControlActor, NodeRegistrationFact, OperationKey};
    let world_id = WorldId::decode(&clients.world)?;
    let mut tx = begin_assignment(db).await?;
    // The assignment writer takes this lock before any scope row.
    sqlx::query("SELECT writer_id FROM game_runtime_scope_assignment_writer WHERE writer_id=1 FOR UPDATE")
        .fetch_one(&mut *tx).await?;
    let (node, revision, role, generation): (Vec<u8>, String, String, String) = sqlx::query_as(
        "SELECT uuid_send(holder_node_id),holder_registration_revision::text,session_user::text,ownership_generation::text
         FROM game_runtime_scope_assignments
         WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1 FOR SHARE",
    ).bind(clients.world.as_slice()).bind(clients.channel.as_slice()).fetch_one(&mut *tx).await?;
    if generation.parse::<u64>()? != clients.scope_generation {
        return Err("I10 channel generation changed before house assignment".into());
    }
    let target = NodeRegistrationFact::new(crate::foundation::NodeId::decode(&node)?, revision.parse()?);
    let assignment: Option<(i16, Option<Vec<u8>>, Option<String>)> = sqlx::query_as(
        "SELECT state,uuid_send(holder_node_id),holder_registration_revision::text
         FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND house_key=$2",
    ).bind(clients.world.as_slice()).bind(house).fetch_optional(&mut *tx).await?;
    if let Some((state, holder, holder_revision)) = assignment {
        if state != 1 || holder.as_deref() != Some(node.as_slice()) || holder_revision.as_deref() != Some(revision.as_str()) {
            return Err("I10 fixture house is assigned to a different node or incarnation".into());
        }
        tx.commit().await?;
        return Ok(());
    }
    if std::env::var("NODE_BOOT_DATABASE_URL").ok().as_deref() != Some(clients.url) {
        return Err("I10 fixture assignment requires the disposable node-boot database".into());
    }
    sqlx::query("INSERT INTO game_control_house_scope_grants VALUES(session_user,encode($1,'hex')::uuid,$2,1) ON CONFLICT DO NOTHING")
        .bind(clients.world.as_slice()).bind(house).execute(&mut *tx).await?;
    let mut operation = [0; 32];
    operation[..16].copy_from_slice(&clients.world);
    operation[16..].copy_from_slice(item);
    let request = HouseScopeAssignmentRequest {
        operation_key: OperationKey::from_bytes(operation),
        actor: ControlActor::new(&role).map_err(|error| format!("I10 control actor: {error:?}"))?,
        actor_role: role,
        command: HouseScopeAssignmentCommand::Assign {
            house: crate::durability::house_scope_handoff::HouseId::new(world_id, house).ok_or("I10 house identity")?,
            target,
        },
    };
    let command = request.encode().map_err(|error| format!("I10 control command: {error:?}"))?;
    let outcome = assign_house_scope_in_transaction(&mut tx, &request, &command)
        .await.map_err(|error| format!("I10 control assignment: {error:?}"))?;
    if !matches!(outcome, HouseScopeAssignmentOutcome::Committed(_) | HouseScopeAssignmentOutcome::Replayed) {
        return Err(format!("I10 fixture House assignment failed: {outcome:?}").into());
    }
    tx.commit().await?;
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

async fn recover_on_fresh_session(
    clients: &SeamClients<'_>, world: &BundleWorld, db: &mut sqlx::PgConnection,
    house: &str, entrance: TilePos, request: house_wrap::Request,
) -> TestResult {
    verdict::Harness::global().bounded_wait(tokio::time::Instant::now() + Duration::from_secs(210), "I10 house recovery");
    let (mut client, pending) = verdict::indexed_timeout(Duration::from_secs(210), async {
        let mut client = connect_world(clients, 0xb3).await?;
        // A stalled durable read leaves this admission in channel scope.
        let pending = tokio::time::timeout(Duration::from_secs(18), restoration_request(db, request)).await??;
        if pending.is_some() {
            let path = route(world, tile(client.world_spatial().actor_position)?, |at| at == entrance)
                .ok_or("I10 recovery house entrance unreachable")?;
            walk(&mut client, world, &path).await?;
        }
        Ok::<_, Box<dyn std::error::Error>>((client, pending))
    }).await??;
    let Some((request, wanted)) = pending else { return Ok(()); };
    verdict::Harness::global().bounded_wait(tokio::time::Instant::now() + Duration::from_secs(36), "I10 restore and exit");
    let restored = tokio::time::timeout(Duration::from_secs(18), async {
        transfer(&mut client, house, false).await?;
        wrap_result(&mut client, request, wanted).await
    }).await;
    let result: TestResult = match restored {
        Ok(result) => {
            let exit = tokio::time::timeout(Duration::from_secs(18), transfer(&mut client, house, true)).await;
            match exit {
                Ok(Ok(())) => return result.map(|_| ()),
                Ok(Err(error)) => result.and(Err(error)),
                Err(error) => result.and(Err(verdict::context("I10 recovery house exit deadline".into(), error))),
            }
        }
        Err(error) => Err(verdict::context("I10 unwrap recovery deadline".into(), error)),
    };
    // An unresolved reply can arrive before an exit reply. Close the stream
    // and verify durable release of the house session through the server.
    drop(client);
    let release = wait_character_released(clients, 1).await;
    if let Err(error) = &release {
        evidence(&format!("I10 recovery house release failed: {error}"));
    }
    result.map(|_| ()).and(release)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    if E2eMode::from_env()? == E2eMode::Journey {
        evidence("e2e_game_gap scenario=I10.client cause=reachable-owned-house-and-placed-furniture-unavailable");
        return Err(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!())).into());
    }

    let (mut db, mut client, world, house, entrance, mut original, item, position, acl_revision, needs_restore) =
        verdict::indexed_timeout(Duration::from_secs(250), async {
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
            let original: u64 = revision.parse()?;
            let item = id.try_into().map_err(|_| "I10 invalid item identity")?;
            let position = crate::durability::consume_transform::HouseActionContext::decode_spatial_position(&position)
                .ok_or("I10 invalid furniture position")?;
            let acl_revision = acl.parse()?;
            ensure_assignment(&mut db, clients, &house, &item).await?;
            Ok::<_, Box<dyn std::error::Error>>((db, client, world, house, entrance, original, item, position, acl_revision, needs_restore))
        }).await??;
    let request = |action, expected_revision| house_wrap::Request { action, item, expected_revision, acl_revision, position };
    let recovery_request = request(house_wrap::Action::Unwrap, original);
    verdict::Harness::global().bounded_wait(tokio::time::Instant::now() + Duration::from_secs(18), "I10 wrap sequence");
    let proofs = tokio::time::timeout(Duration::from_secs(18), async {
        transfer(&mut client, &house, false).await?;
        if needs_restore {
            restore_wrapped(&mut client, &mut db, recovery_request).await?;
            original = original.checked_add(1).ok_or("I10 recovered revision exhausted")?;
        }
        let wrapped = original.checked_add(1).ok_or("I10 revision exhausted")?;
        let restored = wrapped.checked_add(1).ok_or("I10 revision exhausted")?;
        let wrap_request = request(house_wrap::Action::Wrap, original);
        let unwrap_request = request(house_wrap::Action::Unwrap, wrapped);
        let stale_request = request(house_wrap::Action::Wrap, original);
        let wrap = wrap_result(&mut client, wrap_request, house_wrap::Reply::Committed(wrapped)).await?;
        let unwrap = wrap_result(&mut client, unwrap_request, house_wrap::Reply::Committed(restored)).await?;
        let stale = wrap_result(&mut client, stale_request, house_wrap::Reply::Refused(house_wrap::Refusal::Stale)).await?;
        Ok::<_, Box<dyn std::error::Error>>([(wrap_request, wrap), (unwrap_request, unwrap), (stale_request, stale)])
    }).await.unwrap_or_else(|error| Err(verdict::context("I10 wrap sequence timed out".into(), error)));
    let [(wrap_request, wrap), (unwrap_request, unwrap), (stale_request, stale)] = match proofs {
        Ok(proofs) => proofs,
        Err(error) => {
            drop(client);
            if let Err(recovery) = recover_on_fresh_session(clients, world, &mut db, &house, entrance, recovery_request).await {
                evidence(&format!("I10 oven restoration failed: {recovery}"));
            }
            return Err(error);
        }
    };
    verdict::indexed_timeout(Duration::from_secs(18), transfer(&mut client, &house, true)).await??;
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

    #[tokio::test]
    async fn qualification_r5_assignment_sets_database_deadlines_before_any_lock() -> TestResult {
        let Some(url) = crate::bestiary_postgres_harness::configured_admin() else { return Ok(()); };
        let mut db = sqlx::PgConnection::connect(&url).await?;
        let mut tx = begin_assignment(&mut db).await?;
        let (transaction, statement, lock): (String, String, String) = sqlx::query_as(
            "SELECT current_setting('transaction_timeout'), current_setting('statement_timeout'), current_setting('lock_timeout')",
        ).fetch_one(&mut *tx).await?;
        assert_eq!(transaction, "2s");
        assert_eq!(statement, transaction);
        assert_eq!(lock, transaction);
        tx.rollback().await?;
        Ok(())
    }

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
