use super::*;
const PREPARE_ENTRANT_SQL: &str = "INSERT INTO game_house_scope_handoffs(handoff_id,direction,character_id,account_id,world_id,origin_channel_id,source_game_session_id,source_connection_generation,source_character_lease_generation,source_scope_ownership_generation,house_key,destination_scope_ownership_generation,acl_revision,reserved_tile,state,prepared_at) SELECT game_character_uuid_v7(),1,character_id,account_id,world_id,runtime_scope_channel_id,game_session_id,current_generation,character_lease_generation,scope_ownership_generation,$1,1,0,'entry',1,1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($2,'hex')::uuid";
fn uuid(id: &[u8; 16]) -> String {
    let h = id.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}
pub(super) async fn reclaim_character(
    h: &Harness,
    character: u8,
    account: u8,
    now: i64,
) -> TestResult<crate::domain::CharacterId> {
    // The independent producer fixture supplies a bounded authenticated decision. Only the
    // real bootstrap writer creates the root, issuer floor, operation receipt and audit data.
    use crate::durability::native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
        SourceObservation,
    };
    if character == 60 {
        let descriptor = DescriptorRegistration {
            revision: 1,
            facts: vec![1],
            installed_at: now,
        };
        let provenance = FreshStoreProvenance {
            namespace: "house-fixture-source".into(),
            authorization: "independent-platform-fixture".into(),
            source_authority: "platform".into(),
            initialized_at: now,
        };
        assert!(
            h.root
                .record_native_source_descriptor_issuance(
                    "platform",
                    descriptor.clone(),
                    Some(provenance.clone())
                )
                .await
                .map_err(debug)?
        );
        h.root
            .initialize_native_admission_source(&h.node, provenance, descriptor)
            .await
            .map_err(debug)?;
    }
    let account_text = uuid(&id(account));
    let revision = u64::from(character);
    let security = serde_json::json!({
        "version": 1, "operation": "ReadAccountSecurityV1", "result": "observed",
        "source_authority": "platform", "source_revision": revision.to_string(),
        "decision_identity": revision.to_string(), "source_observed_at": now.to_string(),
        "clock_uncertainty_seconds": "0", "account_id": account_text,
        "purpose": "platform_security", "scope": "fresh_admission", "allowed": true,
        "minimum_valid_generation": "1",
    });
    h.root
        .accept_native_source_observation(
            &h.node,
            SourceObservation {
                source_authority: "platform".into(),
                operation: NativeSourceOperation::ReadAccountSecurityV1,
                subject: NativeSourceSubject::account_security(account_text.clone())
                    .map_err(debug)?,
                source_revision: revision,
                decision_identity: revision.to_string(),
                observed_at: now,
                semantic_facts: serde_json::to_vec(&security)?,
            },
        )
        .await
        .map_err(debug)?;
    let wire = serde_json::json!({
        "contract_version": 3, "variant": "OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority": "OTERYN_PLATFORM_CHARACTER_AUTHORITY",
        "issuer_decision_id": uuid(&id(character + 20)),
        "source_revision": revision.to_string(),
        "operation_id": uuid(&id(character)),
        "operation": "INITIAL_CHARACTER_BOOTSTRAP", "account_id": account_text,
        "target_world_id": uuid(&id(WORLD)),
        "requested_sex":"Male","requested_name": format!("House Reclaimer {}{}", char::from(b'A' + character / 26), char::from(b'a' + character % 26)),
        "interpretation_context": {"profile_revision":"profile-1", "ruleset_revision":"ruleset-1",
            "content_revision":"content-1", "starter_template_revision":"starter-1"},
        "issued_at_source": (now - 1).to_string(), "expires_at_source": (now + 120).to_string(),
        "audience":"OTERYN_GAME_CHARACTER_AUTHORITY",
    });
    let intent = crate::character_bootstrap_intent::decode_producer_response(
        &serde_json::to_vec(&wire)?,
        id(character),
    )
    .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let bootstrapped = h
        .root
        .bootstrap_character(&recovery, &h.node, &intent)
        .await
        .map_err(debug)?;
    assert_eq!(bootstrapped.revision.get(), 1);
    Ok(bootstrapped.character_id)
}

pub(super) async fn house_and_items(h: &Harness, recipient: [u8; 16], count: i32) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/tests/support/owned_house_fixture.sql"
    )))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("CREATE TEMP TABLE house_items ON COMMIT DROP AS SELECT game_character_uuid_v7() AS item,game_character_uuid_v7() AS mint,game_character_uuid_v7() AS placement,n AS ordinal FROM generate_series(1,$1) n")
        .bind(count).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT item,encode($1,'hex')::uuid,'Item','oteryn:content.item.vase','r1',1,1,mint FROM house_items")
        .bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_interior_locations SELECT item,encode($1,'hex')::uuid,$2,'\x01'::bytea,ordinal,placement FROM house_items")
        .bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) SELECT item,encode($1,'hex')::uuid,$2,CASE WHEN ordinal%2=0 THEN encode($3,'hex')::uuid ELSE encode($4,'hex')::uuid END,placement,1 FROM house_items")
        .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).bind(recipient.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn overdue(h: &Harness) -> TestResult {
    sqlx::query("INSERT INTO game_house_dispositions(world_id,house_key,owner_character_id,ownership_revision,state,noticed_at,due_at) SELECT world_id,house_key,owner_character_id,ownership_revision,1,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-86400001,floor(extract(epoch FROM clock_timestamp())*1000)::bigint-1 FROM game_house_ownership WHERE house_key=$1")
        .bind(HOUSE).execute(&h.pool).await?;
    Ok(())
}

pub(super) fn invalid_commands(
    original: AdmittedSession,
    admitted: AdmittedSession,
    due: i64,
) -> TestResult<Vec<(AdmittedSession, u32, Vec<u8>)>> {
    let encoded = |house: &str, action, due| {
        encode_house_move_out_request(&HouseMoveOutRequest {
            house: house.into(),
            action,
            due_at_unix_ms: due,
        })
        .map_err(debug)
    };
    let mut no_cap = admitted;
    no_cap.continuity.selected_capabilities = SelectedCapabilities::NONE;
    Ok(vec![
        (
            original,
            COMMAND_TYPE_HOUSE_MOVE_OUT,
            encoded(HOUSE, 1, due)?,
        ),
        (no_cap, COMMAND_TYPE_HOUSE_MOVE_OUT, encoded(HOUSE, 1, due)?),
        (admitted, COMMAND_TYPE_HOUSE_MOVE_OUT, vec![0]),
        (admitted, COMMAND_TYPE_HOUSE_MOVE_OUT, encoded(HOUSE, 1, 1)?),
        (
            admitted,
            COMMAND_TYPE_HOUSE_MOVE_OUT,
            encoded(HOUSE, 1, i64::MAX)?,
        ),
        (
            admitted,
            COMMAND_TYPE_HOUSE_MOVE_OUT,
            encoded("oteryn:content.house.other", 1, due)?,
        ),
        (admitted, COMMAND_TYPE_HOUSE_MOVE_OUT, encoded(HOUSE, 0, 0)?),
        (
            admitted,
            COMMAND_TYPE_HOUSE_MOVE_OUT_QUERY,
            encoded(HOUSE, 2, 0)?,
        ),
    ])
}

pub(super) async fn assert_frozen(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    let error = sqlx::query(PREPARE_ENTRANT_SQL)
        .bind(HOUSE)
        .bind(id(50).as_slice())
        .execute(&mut *tx)
        .await
        .err()
        .ok_or("new entry was not fenced")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("55P03")
    );
    tx.rollback().await?;

    let mut tx = h.pool.begin().await?;
    let error=sqlx::query("DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=(SELECT item_instance_id FROM game_item_house_reclaim_provenance LIMIT 1)").execute(&mut *tx).await.err().ok_or("provenance fence missing")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("55P03")
    );
    tx.rollback().await?;
    let mut tx = h.pool.begin().await?;
    let error=sqlx::query("UPDATE game_item_instances SET quantity=quantity+1 WHERE item_instance_id=(SELECT item_instance_id FROM game_item_house_interior_locations LIMIT 1)").execute(&mut *tx).await.err().ok_or("item fence missing")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("55P03")
    );
    tx.rollback().await?;
    Ok(())
}

pub(super) async fn assert_final(h: &Harness, recipient: [u8; 16]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    let error=sqlx::query("INSERT INTO game_item_house_interior_locations SELECT item_instance_id,world_id,$1,'frozen',1,game_character_uuid_v7() FROM game_character_inbox_deliveries LIMIT 1")
        .bind(HOUSE).execute(&mut *tx).await.err().ok_or("DRAINED fence missing")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("55P03")
    );
    tx.rollback().await?;

    let rows:Vec<(Vec<u8>,i64)>=sqlx::query_as("SELECT uuid_send(character_id),count(*) FROM game_character_inbox_deliveries WHERE cause_kind='HOUSE_DISPOSITION' GROUP BY character_id ORDER BY character_id")
        .fetch_all(&h.pool).await?;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .find(|(id, _)| id == &recipient)
            .map(|(_, n)| *n),
        Some(51)
    );
    assert_eq!(
        rows.iter().find(|(c, _)| c == &id(41)).map(|(_, n)| *n),
        Some(50)
    );
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_house_interior_locations")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(remaining, 0);
    let retained: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_ownership WHERE house_key=$1)")
            .bind(HOUSE)
            .fetch_one(&h.pool)
            .await?;
    assert!(retained);
    let events:i64=sqlx::query_scalar("SELECT count(*) FROM game_house_disposition_audit_outbox WHERE event_type_id=8 AND envelope_sha256=sha256(envelope)").fetch_one(&h.pool).await?;
    assert_eq!(events, 7); // notice/cancel twice, freeze, 100, 1
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT game_house_disposition_guild_fence_abi()=1")
            .fetch_one(&h.pool)
            .await?
    );
    Ok(())
}

pub(super) async fn assert_runtime_privileges(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT game_house_disposition_guild_fence_abi()=1")
            .fetch_one(&mut *tx)
            .await?
    );
    for table in [
        "game_house_dispositions",
        "game_house_disposition_commands",
        "game_house_disposition_census",
        "game_house_disposition_steps",
    ] {
        let access: bool = sqlx::query_scalar(
            "SELECT has_table_privilege(current_user,$1,'SELECT,INSERT,UPDATE,DELETE,TRUNCATE')",
        )
        .bind(table)
        .fetch_one(&mut *tx)
        .await?;
        assert!(!access, "private {table}");
    }
    for signature in [
        "game_house_disposition_advance(uuid,integer,integer,integer)",
        "game_house_move_out_apply(uuid,text,uuid,boolean,bigint,bigint)",
        "game_house_disposition_fence_check(uuid,text,uuid,boolean)",
        "game_character_inbox_deliver(uuid,uuid,uuid,text,text)",
    ] {
        let access: bool =
            sqlx::query_scalar("SELECT has_function_privilege(current_user,$1,'EXECUTE')")
                .bind(signature)
                .fetch_one(&mut *tx)
                .await?;
        assert!(!access, "private {signature}");
    }
    tx.rollback().await?;
    Ok(())
}

pub(super) async fn prepare_entrant(h: &Harness) -> TestResult {
    sqlx::query(PREPARE_ENTRANT_SQL)
        .bind(HOUSE)
        .bind(id(50).as_slice())
        .execute(&h.pool)
        .await?;
    Ok(())
}

pub(super) async fn blocked_steps_rollback(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult {
    // A retained PREPARED entrant prevents any delivery after the census is frozen.
    owner
        .drain_source_house_dispositions()
        .await
        .map_err(debug)?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_disposition_steps")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    sqlx::query("DELETE FROM game_house_scope_handoffs WHERE house_key=$1 AND state=1")
        .bind(HOUSE)
        .execute(&h.pool)
        .await?;
    // Inject an Inbox failure without replacing/disabling any production guard.
    sqlx::raw_sql("CREATE FUNCTION test_house_inbox_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected Inbox failure' USING ERRCODE='23514'; END $$; CREATE TRIGGER test_house_inbox_failure BEFORE INSERT ON game_character_inbox_deliveries FOR EACH ROW EXECUTE FUNCTION test_house_inbox_failure();").execute(&h.pool).await?;
    assert!(owner.drain_source_house_dispositions().await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_house_disposition_steps")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_house_interior_locations")
            .fetch_one(&h.pool)
            .await?,
        101
    );
    sqlx::raw_sql("DROP TRIGGER test_house_inbox_failure ON game_character_inbox_deliveries; DROP FUNCTION test_house_inbox_failure();").execute(&h.pool).await?;
    Ok(())
}

pub(super) async fn runtime_root(
    h: &Harness,
    admin: &str,
) -> TestResult<crate::durability::DurabilityRoot> {
    let database: String = sqlx::query_scalar("SELECT current_database()")
        .fetch_one(&h.pool)
        .await?;
    let prefix = admin.strip_suffix("/postgres").ok_or("admin route")?;
    let url = format!("{prefix}/{database}?options=-c%20role%3Doteryn_game_runtime");
    let probe = sqlx::PgPool::connect(&url).await?;
    let role: String = sqlx::query_scalar("SELECT current_user::text")
        .fetch_one(&probe)
        .await?;
    assert_eq!(role, "oteryn_game_runtime");
    probe.close().await;
    let root = crate::durability::DurabilityRoot::connect_test_runtime(&url)?;
    assert!(root.maintain_ready_once().await?);
    Ok(root)
}

pub(super) async fn assert_unproven_removal_closed(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("DELETE FROM game_item_house_interior_locations WHERE item_instance_id=(SELECT item_instance_id FROM game_item_house_interior_locations LIMIT 1)").execute(&mut *tx).await?;
    let error = sqlx::query("SET CONSTRAINTS game_house_location_proven IMMEDIATE")
        .execute(&mut *tx)
        .await
        .err()
        .ok_or("unproven native removal committed")?;
    assert!(
        error
            .to_string()
            .contains("house removal lacks actual source shape")
    );
    tx.rollback().await?;
    Ok(())
}

pub(super) async fn assert_unproven_checkpoint_closed(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SELECT game_house_disposition_advance(operation_id,2000,100,0) FROM game_house_dispositions WHERE state=1").execute(&mut *tx).await?;
    let error = sqlx::query("SET CONSTRAINTS game_house_disposition_checkpoint_event IMMEDIATE")
        .execute(&mut *tx)
        .await
        .err()
        .ok_or("checkpoint without event committed")?;
    assert!(
        error
            .to_string()
            .contains("house disposition lacks own physical HouseOperation event")
    );
    tx.rollback().await?;
    Ok(())
}
