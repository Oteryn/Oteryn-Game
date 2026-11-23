use super::*;
use crate::policy::OperationPolicy;
use serde_json::Value;

const WORLD: &str = "019a5620-0000-7000-8000-000000000011";
const OTHER_WORLD: &str = "019a5620-0000-7000-8000-000000000012";
const NODE: &str = "019a5620-0000-7000-8000-000000000021";
const CHARACTER: &str = "019a5620-0000-7000-8000-000000000031";
const ACCOUNT: &str = "019a5620-0000-7000-8000-000000000041";
const CHANNEL: &str = "019a5620-0000-7000-8000-000000000051";
const BAG: &str = "019a5620-0000-7000-8000-000000000061";
const ITEM: &str = "019a5620-0000-7000-8000-000000000062";
const SESSION: &str = "019a5620-0000-7000-8000-000000000071";

#[test]
fn postgres_phase_one_reads_validate_exported_schemas() -> Result<(), WireError> {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_on_thread(read_cases)
}

async fn read_cases(pool: &sqlx::PgPool) -> Result<(), WireError> {
    crate::read::database::MIGRATOR.run(pool).await?;
    fixture(pool).await?;
    let mut agent = crate::tests::channel::agent()?;
    agent.read_pool = Some(pool.clone());
    agent.config.worlds = vec![OTHER_WORLD.into(), WORLD.into()];
    agent.config.nodes = vec![NODE.into()];
    for op in Operation::ALL {
        agent.policy.operations.insert(
            op.name().into(),
            OperationPolicy {
                min_risk_class: op.minimum(),
                allowed_permissions: [op.permission().into()].into(),
                worlds: [WORLD.into()].into(),
            },
        );
    }
    let schemas = schemas()?;
    let mut run = async |op: Operation, target: Value, params: Value| {
        let response = agent
            .process(&read_request(op, target, params)?)
            .await
            .map_err(code)?;
        let result = response
            .result
            .ok_or_else(|| format!("{}: {:?}", op.name(), response.error))?;
        let result = serde_json::to_value(result)?;
        let schema: Value =
            serde_json::from_str(&schemas[&format!("{}.result.schema.json", op.name())])?;
        validate_schema(&schema, &schema, &result);
        Ok::<Value, WireError>(result)
    };

    use Operation::*;
    let character = json!({"world_id":WORLD,"character_id":CHARACTER});
    let result = run(OpsOverviewGet, json!({}), json!({})).await?;
    assert_eq!(result["db_available"], true);
    assert_eq!(result["worlds"][0]["world_id"], WORLD);
    assert_eq!(result["nodes"][0]["node_id"], NODE);

    let result = run(WorldList, json!({}), json!({"limit":1})).await?;
    assert_eq!(result["worlds"][0]["world_id"], WORLD);
    assert!(result["next_cursor"].is_null());
    let result = run(WorldGet, json!({"world_id":WORLD}), json!({})).await?;
    assert_eq!(result["channels"], json!([CHANNEL]));

    let result = run(NodeList, json!({}), json!({"limit":1})).await?;
    assert_eq!(result["nodes"][0]["worlds"], json!([WORLD]));
    let result = run(NodeGet, json!({"node_id":NODE}), json!({})).await?;
    assert_eq!(result["ready"], true);

    let result = run(DbStatus, json!({}), json!({})).await?;
    assert!(
        result["tables"]
            .as_array()
            .is_some_and(|tables| tables.len() == 50)
    );
    let result = run(DbMigrationsList, json!({}), json!({"limit":1})).await?;
    assert_eq!(result["migrations"][0]["applied"], true);
    let result = run(
        DbMigrationsList,
        json!({}),
        json!({"limit":1,"cursor":result["next_cursor"]}),
    )
    .await?;
    assert_eq!(result["migrations"][0]["version"], "2");

    let result = run(
        PlayerSearch,
        json!({}),
        json!({"limit":1,"name_prefix":"oTe R","account_ref":ACCOUNT}),
    )
    .await?;
    assert_eq!(result["players"][0]["character_id"], CHARACTER);
    assert_eq!(result["players"][0]["online"], false);

    let result = run(PlayerCharacterGet, character.clone(), json!({})).await?;
    assert_eq!(result["position"], json!({"x":10,"y":20,"z":7}));
    assert_eq!(result["name"], "Oteryn Reader");

    let result = run(PlayerInventoryGet, character.clone(), json!({"limit":1})).await?;
    assert_eq!(result["items"][0]["instance_id"], BAG);
    let result = run(
        PlayerInventoryGet,
        character.clone(),
        json!({"limit":1,"cursor":result["next_cursor"]}),
    )
    .await?;
    assert_eq!(
        result["items"][0],
        json!({"instance_id":ITEM,"type_id":3031,"count":"4","location":"backpack","parent_id":BAG})
    );
    assert!(result["next_cursor"].is_null());

    let result = run(SanctionList, character.clone(), json!({"limit":1})).await?;
    assert_eq!(result["sanctions"][0]["kind"], "character_ban");
    assert_eq!(result["sanctions"][0]["active"], true);

    let filter = json!({"limit":1,"from":"2026-11-05T00:00:00Z","to":"2026-11-06T00:00:00Z","operation":"world.get","character_id":CHARACTER});
    let result = run(AdminLedgerQuery, json!({"world_id":WORLD}), filter.clone()).await?;
    assert_eq!(result["entries"][0]["admin_id"], "fixture");
    let result = run(
        AdminLedgerQuery,
        json!({"world_id":WORLD}),
        json!({
            "cursor":result["next_cursor"], "limit":1,"from":filter["from"],"to":filter["to"],
            "operation":"world.get","character_id":CHARACTER
        }),
    )
    .await?;
    assert!(result["entries"][0]["target"]["world_id"] == WORLD);
    assert!(result["next_cursor"].is_null());
    run(AdminPolicyGet, json!({}), json!({})).await?;

    // A nonterminal session, even without matching advertised presence, excludes both offline reads.
    pool.execute(sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "INSERT INTO game_durability_reconnect_sessions (
            game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,
            runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,
            character_lease_generation,scope_ownership_generation,current_generation,session_state)
         VALUES ('{SESSION}','{ACCOUNT}','{CHARACTER}','{WORLD}',1,'{WORLD}','{CHANNEL}',1,1794000000,1,2,1,1,1)"
    )))).await?;
    for (op, params) in [
        (PlayerCharacterGet, json!({})),
        (PlayerInventoryGet, json!({"limit":1})),
    ] {
        let request = read_request(op, character.clone(), params)?;
        let response = agent.process(&request).await.map_err(code)?;
        assert_eq!(
            response.error.map(|e| e.code),
            Some(ErrorCode::AdmTargetUnavailable)
        );
    }
    let generation: String = sqlx::query_scalar(
        "SELECT character_lease_generation::text FROM game_durability_reconnect_sessions",
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(generation, "2");

    eprintln!(
        "ADMIN_READ_PG_EXECUTED: PostgreSQL 18.6; phase-one result schemas, paging, scope and offline fencing"
    );
    Ok(())
}

fn read_request(op: Operation, target: Value, params: Value) -> Result<Request, WireError> {
    let mut r = request()?;
    let id = uuid::Uuid::now_v7();
    r.request_id = id.to_string();
    r.admin_operation_id.clone_from(&r.request_id);
    r.nonce = URL_SAFE_NO_PAD.encode(id.as_bytes());
    r.operation = op.name().into();
    r.target = serde_json::from_value(target)?;
    r.params = params;
    r.actor.permission = op.permission().into();
    r.risk_class = op.minimum();
    resign(&mut r)?;
    Ok(r)
}

// The exported result schemas use only these validation keywords.
fn validate_schema(root: &Value, schema: &Value, value: &Value) {
    if let Some(reference) = schema["$ref"].as_str() {
        let target = root.pointer(reference.trim_start_matches('#'));
        assert!(target.is_some(), "unknown reference {reference}");
        if let Some(target) = target {
            validate_schema(root, target, value);
        }
        return;
    }
    if let Some(types) = schema["type"].as_array() {
        if value.is_null() {
            assert!(types.contains(&json!("null")));
            return;
        }
        for ty in types.iter().filter(|ty| **ty != "null") {
            let mut schema = schema.clone();
            schema["type"] = ty.clone();
            validate_schema(root, &schema, value);
        }
        return;
    }
    if schema["anyOf"].is_array() {
        // The result exports use anyOf solely for nullable references.
        let variants = schema["anyOf"].as_array().into_iter().flatten();
        let nonnull: Vec<_> = variants.filter(|s| s["type"] != "null").collect();
        assert_eq!(nonnull.len(), 1);
        if !value.is_null() {
            validate_schema(root, nonnull[0], value);
        }
        return;
    }
    if let Some(allowed) = schema["enum"].as_array() {
        assert!(allowed.contains(value), "{value}");
    }
    match schema["type"].as_str() {
        Some("object") => {
            assert!(value.is_object(), "{value}");
            for name in schema["required"].as_array().into_iter().flatten() {
                assert!(
                    name.as_str().is_some_and(|name| value.get(name).is_some()),
                    "{name} missing from {value}"
                );
            }
            for (name, field) in value.as_object().into_iter().flatten() {
                assert_eq!(schema["additionalProperties"], false);
                let property = &schema["properties"][name];
                assert!(!property.is_null(), "unexpected field {name}");
                validate_schema(root, property, field);
            }
        }
        Some("array") => {
            assert!(value.is_array(), "{value}");
            for item in value.as_array().into_iter().flatten() {
                validate_schema(root, &schema["items"], item);
            }
        }
        Some("string") => assert!(value.is_string(), "{value}"),
        Some("boolean") => assert!(value.is_boolean(), "{value}"),
        Some("number") => assert!(value.is_number(), "{value}"),
        Some("integer") => {
            assert!(value.is_i64() || value.is_u64(), "{value}");
            if let Some(number) = value.as_f64() {
                assert!(schema["minimum"].as_f64().is_none_or(|min| number >= min));
                assert!(schema["maximum"].as_f64().is_none_or(|max| number <= max));
            }
        }
        other => assert!(other.is_some(), "unsupported schema {schema}"),
    }
}

async fn fixture(pool: &sqlx::PgPool) -> Result<(), WireError> {
    // Seed committed snapshots directly in the isolated test DB. The gameplay
    // writer/receipt paths have their own tests and are not part of ADM-G3.
    pool.execute(sqlx::raw_sql(sqlx::AssertSqlSafe(format!(r"
        BEGIN;
        SET LOCAL session_replication_role=replica;
        INSERT INTO game_node_bootstrap_authorizations(authorization_digest,launch_binding,issued_at)
        VALUES (decode(repeat('00',32),'hex'),'adm-test',1);
        INSERT INTO game_node_registrations(node_id,registration_revision,authorization_digest,launch_binding,state,registered_at)
        VALUES ('{NODE}',1,decode(repeat('00',32),'hex'),'adm-test',1,1);
        INSERT INTO game_runtime_scope_assignments (
            scope_key,world_id,channel_id,ownership_generation,state,holder_node_id,holder_registration_revision,
            source_revision,decision_identity,operation_key,decided_at
        ) VALUES (decode('01','hex') || uuid_send('{WORLD}') || uuid_send('{CHANNEL}'),
            '{WORLD}','{CHANNEL}',1,1,'{NODE}',1,1,'adm-assign',decode(repeat('00',32),'hex'),1);
        INSERT INTO game_durability_admission_runtime_guards VALUES (
            decode('01','hex') || uuid_send('{WORLD}') || uuid_send('{CHANNEL}'),1,true,
            1,'adm-test',1,'adm-ready',1,0,'{{}}');

        INSERT INTO game_character_account_guards VALUES ('{ACCOUNT}');
        INSERT INTO game_character_roots(
            character_id,account_id,world_id,lifecycle,character_revision,profile_revision,
            ruleset_revision,content_revision,starter_template_revision,name)
        VALUES ('{CHARACTER}','{ACCOUNT}','{WORLD}',1,1,'p','r','c','s','Oteryn Reader');
        INSERT INTO game_character_last_positions (
            character_id,world_id,tile_x,tile_y,tile_floor,coordinate_frame_digest,map_revision_digest,
            server_artifact_digest,character_lease_generation,write_sequence
        ) VALUES ('{CHARACTER}','{WORLD}',10,20,7,decode(repeat('00',32),'hex'),decode(repeat('00',32),'hex'),
                  decode(repeat('00',32),'hex'),1,1);
        INSERT INTO game_item_instances (
            item_instance_id,world_id,definition_family,definition_production_key,
            definition_revision_ref,quantity,lifecycle,minted_transaction_id
        ) VALUES ('{BAG}','{WORLD}','Item','oteryn:item.tibia.i2854','r1',1,1,'{BAG}'),
                 ('{ITEM}','{WORLD}','Item','oteryn:item.tibia.i3031','r1',4,1,'{ITEM}');
        INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id)
        VALUES ('{CHARACTER}','{BAG}','{WORLD}','{BAG}');
        INSERT INTO game_item_container_entries (
            item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id
        ) VALUES ('{ITEM}','{WORLD}','{CHARACTER}','{BAG}',1,'{ITEM}');
        INSERT INTO game_account_sanction_projections (
            sanction_stream_id,decision_revision,account_id,character_id,kind,reason_code,reason_text,starts_at_ms,revoked
        ) VALUES ('{ITEM}',1,'{ACCOUNT}','{CHARACTER}',3,1,'test',1,false);

        INSERT INTO admin_command_ledger (
            ledger_id,request_id,admin_operation_id,operation,operation_version,params_digest,
            canonical_request,signer_key_id,admin_id,risk_class,target,status,effects,started_at,finished_at,agent_id
        ) SELECT id,id,id,'world.get',1,'test',decode('00','hex'),'test','fixture','R0',
            jsonb_build_object('world_id',world,'character_id',character), 'succeeded','[]',
            '2026-11-05T11:00:00Z','2026-11-05T11:00:01Z','test'
        FROM (VALUES ('{BAG}'::uuid,'{WORLD}','{CHARACTER}'),
                     ('{ITEM}'::uuid,'{WORLD}','{CHARACTER}'),
                     ('{ACCOUNT}'::uuid,'{OTHER_WORLD}','{CHARACTER}')) AS rows(id,world,character);
        COMMIT;
    ")))).await?;
    Ok(())
}
