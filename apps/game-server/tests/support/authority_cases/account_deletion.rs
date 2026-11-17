//! GPROD-DELETION: off/unqualified closure must never become a terminal unsupported receipt.
use super::*;
use native_admission_source::character_commands::{Decision, Input};
use sha2::{Digest, Sha256};

const SOURCE: &str = "oteryn:character-authority:primary";

#[test]
fn account_deletion_postgres_off_replays_without_effects() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(async move {
                    let started = std::time::Instant::now();
                    let database = Database::create(admin, "account_deletion_off").await?;
                    let result = pending_flow(&database).await;
                    database.cleanup().await?;
                    result?;
                    eprintln!(
                        "GPROD_DELETION_PG_EXECUTED off_replay elapsed={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "account deletion test thread failed")?
        .map_err(Into::into)
}

async fn pending_flow(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let retained = fence_parent().join(format!("oteryn-account-deletion-{}", database.name));
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    {
        let fresh = recovery
            .authorize_fresh_store(id(11), 100)
            .map_err(|e| format!("{e:?}"))?;
        root.admit_fresh_character_recovery(&fresh)
            .await
            .map_err(|e| format!("{e:?}"))?;
    }
    let fence = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let node = register(&root, 1, None).await?;
    initialize_s2(&root, &node).await?;
    assign_world(&pool, &root, &node, 90, 95).await?;
    configure(&pool, ["profile-1", "ruleset-1", "content-1", "starter-1"]).await?;
    allow(&root, &node, 31).await?;
    let created = root
        .bootstrap_character(&authority, &node, &intent(21, 31, 1)?)
        .await
        .map_err(|e| format!("{e:?}"))?;

    let restarted = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(restarted.maintain_ready_once().await?);
    let reopened = restarted
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/account-deletion/closure.json"
    ))?;
    for vector in fixtures["vectors"]
        .as_array()
        .ok_or("fixture vectors missing")?
    {
        let wire = vector["wire"].as_str().ok_or("fixture wire missing")?;
        let digest: String = Sha256::digest(wire.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(digest, vector["sha256"]);
        let intent: serde_json::Value = serde_json::from_str(wire)?;
        let Some(command) = intent["command"].as_str() else {
            continue;
        };
        // Receipt fixtures also carry command, but are outputs rather than intents.
        if intent.get("operation").is_some() {
            continue;
        }
        let operation = intent["operation_id"].as_str().ok_or("operation missing")?;
        for (owner, current) in [(&root, &authority), (&restarted, &reopened)] {
            for _ in 0..2 {
                assert_eq!(
                    owner
                        .decide_character_command_configured(
                            current,
                            &node,
                            SOURCE,
                            (command, operation),
                            Input::Intent(wire.as_bytes()),
                            &durability::character_command::account_deletion::Config {
                                enabled: false,
                                ..Default::default()
                            }
                        )
                        .await,
                    Decision::Transient
                );
            }
        }
        let targeted = wire.replace(
            intent["account_id"].as_str().ok_or("account missing")?,
            &uuid(id(31)),
        );
        assert_eq!(
            root.decide_character_command_configured(
                &authority,
                &node,
                SOURCE,
                (command, operation),
                Input::Intent(targeted.as_bytes()),
                &durability::character_command::account_deletion::Config {
                    enabled: false,
                    ..Default::default()
                }
            )
            .await,
            Decision::Transient
        );
        // Expiry, malformed attempts and read failures cannot burn an operation while off.
        for input in [Input::Intent(b"{}"), Input::ReadConflict, Input::Oversized] {
            assert_eq!(
                root.decide_character_command_configured(
                    &authority,
                    &node,
                    SOURCE,
                    (command, operation),
                    input,
                    &durability::character_command::account_deletion::Config {
                        enabled: false,
                        ..Default::default()
                    }
                )
                .await,
                Decision::Transient
            );
        }
    }
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_command_receipts"
        )
        .await?,
        0
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_character_roots").await?,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_operation_receipts"
        )
        .await?,
        1
    );
    // Disabled handling cannot bypass the existing operation namespace or node fence.
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            ("CloseAccount", &uuid(id(21))),
            Input::Intent(b"{}"),
            &durability::character_command::account_deletion::Config {
                enabled: false,
                ..Default::default()
            }
        )
        .await,
        Decision::Conflict
    );
    let _successor = register(&root, 2, Some(1)).await?;
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            ("CloseAccount", &uuid(id(21))),
            Input::Intent(b"{}"),
            &durability::character_command::account_deletion::Config {
                enabled: false,
                ..Default::default()
            }
        )
        .await,
        Decision::Transient
    );
    assert_eq!(
        restarted
            .reconcile_character_bootstrap(&reopened, id(21))
            .await
            .map_err(|e| format!("{e:?}"))?,
        Some(created)
    );
    eprintln!(
        "GPROD_DELETION_PG_CHECKS fixture_hashes=4 intents=2 reopened_replay=executed operation_conflict=executed stale_incarnation=executed"
    );
    pool.close().await;
    drop(reopened);
    drop(restarted);
    drop(authority);
    drop(fence);
    drop(recovery);
    drop(root);
    std::fs::remove_dir_all(retained)?;
    Ok(())
}

#[test]
fn account_deletion_postgres_producer_replays_resolves_and_fences() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?
                .block_on(async move {
                    let started = std::time::Instant::now();
                    let db = Database::create(admin, "account_deletion_producer").await?;
                    let result = producer_flow(&db).await;
                    db.cleanup().await?;
                    result?;
                    eprintln!(
                        "GPROD_DELETION_PG_EXECUTED producer elapsed={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "producer test thread failed")?
        .map_err(Into::into)
}

async fn producer_flow(db: &Database) -> TestResult {
    use durability::character_command::account_deletion::{self as deletion, Config};
    use native_admission_source::account_closure::{LifecycleObservation, LifecycleRequest};
    let config = Config::default();
    assert!(config.enabled);
    let root = DurabilityRoot::connect_test_runtime(&db.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&db.url).await?;
    let retained = fence_parent().join(format!("closure-producer-{}", db.name));
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    {
        let fresh = recovery
            .authorize_fresh_store(id(11), 100)
            .map_err(|e| format!("{e:?}"))?;
        root.admit_fresh_character_recovery(&fresh)
            .await
            .map_err(|e| format!("{e:?}"))?;
    }
    for predecessor in 1..3 {
        let transition = recovery
            .begin_recovery(predecessor, id(11 + predecessor as u8), 100 + predecessor)
            .map_err(|e| format!("{e:?}"))?;
        root.reconcile_character_recovery(&transition)
            .await
            .map_err(|e| format!("{e:?}"))?;
    }
    let fence = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let node = register(&root, 1, None).await?;
    initialize_s2(&root, &node).await?;
    // Isolated database owner fixes the trusted business clock to the normative
    // historical vector. The production function has no configurable clock input.
    sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_account_closure_now() RETURNS BIGINT LANGUAGE sql STABLE SET search_path=pg_catalog AS 'SELECT 1790000002::bigint'").execute(&pool).await?;
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/account-deletion/closure.json"
    ))?;
    let vectors = fixtures["vectors"].as_array().ok_or("vectors missing")?;
    let close = vectors[0]["wire"].as_str().ok_or("close missing")?;
    let cancelled = vectors[3]["wire"].as_str().ok_or("cancel missing")?;
    let intent = deletion::decode(
        close.as_bytes(),
        "CloseAccount",
        "0192a000-0000-7000-8000-000000000030",
    )?;
    observe_security_text(&root, &node, &intent.account_id, 1790000002).await?;
    sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES($1::uuid)")
        .bind(&intent.account_id)
        .execute(&pool)
        .await?;
    // Pin the fixture's LCFA pair using normal monotone revision touches. Recovery
    // raised the epoch to wall time, so only this isolated historical test restores it.
    let mut fixture = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_account_projection_epoch SET projection_epoch=3")
        .execute(&mut *fixture)
        .await?;
    fixture.commit().await?;
    for _ in 0..11 {
        sqlx::query("SELECT game_character_account_projection_touch($1::uuid)")
            .bind(&intent.account_id)
            .execute(&pool)
            .await?;
    }
    sqlx::query("SELECT game_account_closure_resync(3)")
        .execute(&pool)
        .await?;
    assert!(
        root.next_account_closure_publication(&authority, &node, SOURCE)
            .await?
            .is_none()
    );
    let observation = LifecycleObservation {
        contract_version: 2,
        operation: "ReadAccountClosureLifecycleV1".into(),
        source_authority: "OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),
        account_id: intent.account_id.clone(),
        termination_id: intent.termination_id.clone(),
        lifecycle_generation: "8".into(),
        source_revision: "30".into(),
        phase: "CLOSE".into(),
        financial_obligations_clear: true,
        recovery_generation: "3".into(),
        observed_at: "1790000002".into(),
        valid_until: "1790000007".into(),
    };
    let request = LifecycleRequest::from_intent(SOURCE, close.as_bytes(), 3)?;
    let lifecycle_fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/account-deletion/lifecycle.json"
    ))?;
    assert_eq!(
        serde_json::to_string(&request)?,
        lifecycle_fixture["vectors"][0]["wire"]
            .as_str()
            .ok_or("lifecycle request missing")?
    );
    assert_eq!(
        serde_json::to_string(&observation)?,
        lifecycle_fixture["vectors"][1]["wire"]
            .as_str()
            .ok_or("lifecycle response missing")?
    );

    root.observe_account_closure_lifecycle(
        &authority,
        &node,
        &request,
        &serde_json::to_vec(&observation)?,
    )
    .await?;
    root.invalidate_account_closure_lifecycle(
        &authority,
        &node,
        &intent.account_id,
        &intent.termination_id,
    )
    .await?;
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            ("CloseAccount", &intent.operation_id),
            Input::Intent(close.as_bytes()),
            &config
        )
        .await,
        Decision::Transient
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_account_closures").await?,
        0
    );
    assert!(
        root.observe_account_closure_lifecycle(&authority, &node, &request, b"{}")
            .await
            .is_err()
    );
    // A pending closure already owns its Character operation key. The other
    // producer must refuse reuse before decoding or storing a transfer receipt.
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            ("TransferCharacterOwnership", &intent.operation_id),
            Input::Intent(b"{}"),
            &config,
        )
        .await,
        Decision::Conflict
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_bazaar_decisions").await?,
        0
    );
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            ("CloseAccount", &intent.operation_id),
            Input::Intent(close.as_bytes()),
            &config
        )
        .await,
        Decision::Transient
    );
    root.observe_account_closure_lifecycle(
        &authority,
        &node,
        &request,
        &serde_json::to_vec(&observation)?,
    )
    .await?;
    let mut stale = intent.clone();
    stale.operation_id = "0192a000-0000-7000-8000-0000000000b1".into();
    stale.expected_recovery_generation = "2".into();
    stale.source_revision = "1000".into();
    let stale_bytes = serde_json::to_vec(&stale)?;
    let stale_result = root
        .decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            ("CloseAccount", &stale.operation_id),
            Input::Intent(&stale_bytes),
            &config,
        )
        .await;
    assert!(
        matches!(stale_result,Decision::Publish{ref result_code,..} if result_code==deletion::STALE_GENERATION)
    );
    root.acknowledge_account_closure(&authority, &node, Some(&stale.operation_id), None)
        .await?;
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            ("CloseAccount", &intent.operation_id),
            Input::Intent(close.as_bytes()),
            &config
        )
        .await,
        Decision::Transient
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_account_closures WHERE state='CLOSING'"
        )
        .await?,
        1
    );
    // A different root resumes the committed job; no test-only producer shortcut.
    let restarted = DurabilityRoot::connect_test_runtime(&db.url)?;
    assert!(restarted.maintain_ready_once().await?);
    let reopened = restarted
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let receipt = restarted
        .decide_character_command_configured(
            &reopened,
            &node,
            SOURCE,
            ("CloseAccount", &intent.operation_id),
            Input::Intent(close.as_bytes()),
            &config,
        )
        .await;
    let Decision::Publish {
        receipt: bytes,
        result_code,
    } = receipt.clone()
    else {
        return Err(format!("close did not commit: {receipt:?}").into());
    };
    assert_eq!(result_code, "CHAR_CMD_OK");
    assert_eq!(
        bytes,
        vectors[1]["wire"]
            .as_str()
            .ok_or("receipt missing")?
            .as_bytes()
    );
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            ("CloseAccount", &intent.operation_id),
            Input::Intent(close.as_bytes()),
            &config
        )
        .await,
        receipt
    );
    let changed = close.replace("\"source_revision\":\"30\"", "\"source_revision\":\"99\"");
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            ("CloseAccount", &intent.operation_id),
            Input::Intent(changed.as_bytes()),
            &config
        )
        .await,
        Decision::Conflict
    );
    let retained_receipt = root
        .next_account_closure_publication(&authority, &node, SOURCE)
        .await?
        .ok_or("receipt outbox missing")?;
    assert_eq!(
        retained_receipt.operation,
        native_admission_source::descriptor::Operation::PublishAccountClosureCommandReceiptV2
    );
    assert_eq!(
        retained_receipt.body,
        vectors[1]["wire"].as_str().ok_or("receipt missing")?
    );
    root.acknowledge_account_closure(&authority, &node, Some(&intent.operation_id), None)
        .await?;
    let publication = root
        .next_account_closure_publication(&authority, &node, SOURCE)
        .await?
        .ok_or("snapshot missing")?;
    assert_eq!(
        publication.body,
        vectors[2]["wire"].as_str().ok_or("projection missing")?
    );
    root.acknowledge_account_closure(&authority, &node, None, Some(&publication))
        .await?;
    assert!(
        root.next_account_closure_publication(&authority, &node, SOURCE)
            .await?
            .is_none(),
        "LCFA lag must stop the watermark"
    );
    root.clear_account_characters(&authority, &intent.account_id, 3, 12)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let old_watermark = root
        .next_account_closure_publication(&authority, &node, SOURCE)
        .await?
        .ok_or("watermark missing")?;
    assert_eq!(
        old_watermark.operation,
        native_admission_source::descriptor::Operation::PublishAccountClosureWatermarkV1
    );
    let mut shifted = intent.clone();
    shifted.operation_id = uuid(id(92));
    shifted.source_revision = "37".into();
    shifted.scheduled_for = "1789999999".into();
    let shifted_raw = serde_json::to_vec(&shifted)?;
    let shifted_result = root
        .decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            (&shifted.command, &shifted.operation_id),
            Input::Intent(&shifted_raw),
            &config,
        )
        .await;
    assert!(
        matches!(shifted_result,Decision::Publish{ref result_code,..} if result_code=="CHAR_CMD_INTENT_INVALID")
    );
    root.acknowledge_account_closure(&authority, &node, Some(&shifted.operation_id), None)
        .await?;
    let mut cancel_observation = observation.clone();
    cancel_observation.lifecycle_generation = "9".into();
    cancel_observation.source_revision = "31".into();
    cancel_observation.phase = "CANCEL".into();
    root.observe_account_closure_lifecycle(
        &authority,
        &node,
        &request,
        &serde_json::to_vec(&cancel_observation)?,
    )
    .await?;
    let cancel = deletion::decode(
        cancelled.as_bytes(),
        "ResolveAccountClosure",
        "0192a000-0000-7000-8000-000000000032",
    )?;
    let result = root
        .decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            (&cancel.command, &cancel.operation_id),
            Input::Intent(cancelled.as_bytes()),
            &config,
        )
        .await;
    assert!(
        matches!(result,Decision::Publish{ref result_code,..} if result_code=="CHAR_CMD_OK"),
        "{result:?}"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_account_closures WHERE state='CANCELLED'"
        )
        .await?,
        1
    );
    assert_eq!(count(&pool,"SELECT count(*) FROM game_account_closure_audit_outbox WHERE event_type_id=7 AND schema_revision=1").await?,4);
    // A higher-generation new termination has its own policy namespace.
    // A cancelled termination's name-retention selection cannot leak into it.
    let mut subsequent = intent.clone();
    subsequent.termination_id = uuid(id(90));
    subsequent.operation_id = uuid(id(91));
    subsequent.lifecycle_generation = "10".into();
    subsequent.source_revision = "50".into();
    let subsequent_raw = serde_json::to_vec(&subsequent)?;
    let subsequent_request = LifecycleRequest::from_intent(SOURCE, &subsequent_raw, 3)?;
    let mut subsequent_observation = cancel_observation.clone();
    subsequent_observation.termination_id = subsequent.termination_id.clone();
    subsequent_observation.lifecycle_generation = "10".into();
    subsequent_observation.source_revision = "50".into();
    subsequent_observation.phase = "CLOSE".into();
    root.observe_account_closure_lifecycle(
        &authority,
        &node,
        &subsequent_request,
        &serde_json::to_vec(&subsequent_observation)?,
    )
    .await?;
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            (&subsequent.command, &subsequent.operation_id),
            Input::Intent(&subsequent_raw),
            &config
        )
        .await,
        Decision::Transient
    );
    assert!(sqlx::query_scalar::<_,bool>("SELECT name_release_after_seconds IS NULL FROM game_account_closures WHERE account_id=$1::uuid").bind(&subsequent.account_id).fetch_one(&pool).await?);
    // Cancellation tombstone before Close, and the exclusive resolution winner.
    let mut tombstone = cancel.clone();
    tombstone.account_id = uuid(id(50));
    tombstone.termination_id = uuid(id(51));
    tombstone.operation_id = uuid(id(52));
    tombstone.close_operation_id = Some(uuid(id(53)));
    tombstone.source_revision = "32".into();
    let raw = serde_json::to_vec(&tombstone)?;
    observe_security_text(&root, &node, &tombstone.account_id, 1790000002).await?;
    let req = LifecycleRequest::from_intent(SOURCE, &raw, 3)?;
    let mut obs = cancel_observation.clone();
    obs.account_id = tombstone.account_id.clone();
    obs.termination_id = tombstone.termination_id.clone();
    obs.source_revision = "32".into();
    root.observe_account_closure_lifecycle(&authority, &node, &req, &serde_json::to_vec(&obs)?)
        .await?;
    assert!(
        matches!(root.decide_character_command_configured(&authority,&node,SOURCE,(&tombstone.command,&tombstone.operation_id),Input::Intent(&raw),&config).await,Decision::Publish{ref result_code,..} if result_code=="CHAR_CMD_OK")
    );
    let mut late = intent.clone();
    late.account_id = tombstone.account_id.clone();
    late.termination_id = tombstone.termination_id.clone();
    late.operation_id = uuid(id(53));
    late.source_revision = "33".into();
    assert!(
        matches!(root.decide_character_command_configured(&authority,&node,SOURCE,(&late.command,&late.operation_id),Input::Intent(&serde_json::to_vec(&late)?),&config).await,Decision::Publish{ref result_code,..} if result_code==deletion::STALE_GENERATION)
    );
    tombstone.operation_id = uuid(id(54));
    tombstone.source_revision = "34".into();
    tombstone.decision = Some("FINALIZE".into());
    assert!(
        matches!(root.decide_character_command_configured(&authority,&node,SOURCE,(&tombstone.command,&tombstone.operation_id),Input::Intent(&serde_json::to_vec(&tombstone)?),&config).await,Decision::Publish{ref result_code,..} if result_code==deletion::RESOLUTION_CONFLICT)
    );

    // A separate empty portfolio reaches FINALIZED through the same producer.
    let mut final_close = intent.clone();
    final_close.account_id = uuid(id(60));
    final_close.termination_id = uuid(id(61));
    final_close.operation_id = uuid(id(62));
    final_close.lifecycle_generation = "10".into();
    final_close.source_revision = "35".into();
    let final_raw = serde_json::to_vec(&final_close)?;
    observe_security_text(&root, &node, &final_close.account_id, 1790000002).await?;
    let final_request = LifecycleRequest::from_intent(SOURCE, &final_raw, 3)?;
    let mut final_observation = observation.clone();
    final_observation.account_id = final_close.account_id.clone();
    final_observation.termination_id = final_close.termination_id.clone();
    final_observation.lifecycle_generation = "10".into();
    final_observation.source_revision = "35".into();
    root.observe_account_closure_lifecycle(
        &authority,
        &node,
        &final_request,
        &serde_json::to_vec(&final_observation)?,
    )
    .await?;
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            (&final_close.command, &final_close.operation_id),
            Input::Intent(&final_raw),
            &config
        )
        .await,
        Decision::Transient
    );
    assert!(
        matches!(root.decide_character_command_configured(&authority,&node,SOURCE,(&final_close.command,&final_close.operation_id),Input::Intent(&final_raw),&config).await,Decision::Publish{ref result_code,..} if result_code=="CHAR_CMD_OK")
    );
    let mut finalize = cancel.clone();
    finalize.account_id = final_close.account_id.clone();
    finalize.termination_id = final_close.termination_id.clone();
    finalize.operation_id = uuid(id(63));
    finalize.close_operation_id = Some(final_close.operation_id.clone());
    finalize.lifecycle_generation = "11".into();
    finalize.source_revision = "36".into();
    finalize.decision = Some("FINALIZE".into());
    final_observation.lifecycle_generation = "11".into();
    final_observation.source_revision = "36".into();
    final_observation.phase = "FINALIZE".into();
    root.observe_account_closure_lifecycle(
        &authority,
        &node,
        &final_request,
        &serde_json::to_vec(&final_observation)?,
    )
    .await?;
    let final_bytes = serde_json::to_vec(&finalize)?;
    let terminal = root
        .decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            (&finalize.command, &finalize.operation_id),
            Input::Intent(&final_bytes),
            &config,
        )
        .await;
    assert!(
        matches!(terminal,Decision::Publish{ref result_code,..} if result_code=="CHAR_CMD_OK"),
        "{terminal:?}"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_account_closures WHERE state='FINALIZED'"
        )
        .await?,
        1
    );
    assert_eq!(count(&pool,"SELECT count(*) FROM game_account_closure_erasure_queue WHERE state='PENDING_DOMAIN_RETENTION' AND not_before IS NULL").await?,1);
    // FINALIZE may be listed before the matching Close's receipt is visible.
    // It remains resumable PENDING, with no recorded resolution winner.
    let mut unfinished = final_close.clone();
    unfinished.account_id = uuid(id(80));
    unfinished.termination_id = uuid(id(81));
    unfinished.operation_id = uuid(id(82));
    unfinished.source_revision = "40".into();
    let unfinished_raw = serde_json::to_vec(&unfinished)?;
    observe_security_text(&root, &node, &unfinished.account_id, 1790000002).await?;
    let req = LifecycleRequest::from_intent(SOURCE, &unfinished_raw, 3)?;
    let mut obs = final_observation.clone();
    obs.account_id = unfinished.account_id.clone();
    obs.termination_id = unfinished.termination_id.clone();
    obs.lifecycle_generation = unfinished.lifecycle_generation.clone();
    obs.source_revision = "40".into();
    obs.phase = "CLOSE".into();
    root.observe_account_closure_lifecycle(&authority, &node, &req, &serde_json::to_vec(&obs)?)
        .await?;
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            (&unfinished.command, &unfinished.operation_id),
            Input::Intent(&unfinished_raw),
            &config
        )
        .await,
        Decision::Transient
    );
    let mut early = finalize.clone();
    early.account_id = unfinished.account_id.clone();
    early.termination_id = unfinished.termination_id.clone();
    early.operation_id = uuid(id(83));
    early.close_operation_id = Some(unfinished.operation_id.clone());
    early.source_revision = "41".into();
    obs.lifecycle_generation = early.lifecycle_generation.clone();
    obs.source_revision = "41".into();
    obs.phase = "FINALIZE".into();
    root.observe_account_closure_lifecycle(&authority, &node, &req, &serde_json::to_vec(&obs)?)
        .await?;
    let early_raw = serde_json::to_vec(&early)?;
    for _ in 0..2 {
        assert_eq!(
            root.decide_character_command_configured(
                &authority,
                &node,
                SOURCE,
                (&early.command, &early.operation_id),
                Input::Intent(&early_raw),
                &config
            )
            .await,
            Decision::Transient
        );
    }
    assert_eq!(count(&pool,"SELECT count(*) FROM game_account_closure_jobs WHERE source_revision=41 AND receipt IS NULL").await?,1);
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_account_closures WHERE state='CLOSING' AND decision IS NULL"
        )
        .await?,
        2
    );
    let events = root.pending_account_closure_audit(&authority, 64).await?;
    assert!(!events.is_empty());
    for event in &events {
        let envelope =
            durability::item_mint_audit::EventEnvelopeV1::decode(event.envelope.as_slice())?;
        assert_eq!(envelope.event_type_id, 7);
        assert_eq!(envelope.event_schema_revision, 1);
        assert_eq!(envelope.privacy_class, 3);
        let transaction = envelope
            .transaction_event
            .ok_or("transaction ref missing")?;
        assert_eq!((transaction.ordinal, transaction.count), (1, 1));
    }
    // A 409 quarantines the exact snapshot; a monotone resync enumerates every
    // subject and replays linked receipts even if Platform no longer lists them.
    sqlx::query(
        "UPDATE game_account_closure_jobs SET receipt_acknowledged=true WHERE receipt IS NOT NULL",
    )
    .execute(&pool)
    .await?;
    let snapshot = root
        .next_account_closure_publication(&authority, &node, SOURCE)
        .await?
        .ok_or("snapshot missing")?;
    root.quarantine_account_closure_publication(&authority, &node, &snapshot)
        .await?;
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_account_closure_publications WHERE quarantined"
        )
        .await?,
        1
    );
    sqlx::query("SELECT game_account_closure_resync(4)")
        .execute(&pool)
        .await?;
    root.quarantine_account_closure_publication(&authority, &node, &old_watermark)
        .await?;
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT NOT quarantined FROM game_account_closure_publication_clock WHERE singleton"
        )
        .fetch_one(&pool)
        .await?
    );

    assert!(
        root.next_account_closure_publication(&authority, &node, SOURCE)
            .await?
            .is_none()
    );
    let linked = root
        .next_account_closure_publication(&authority, &node, SOURCE)
        .await?
        .ok_or("resync receipt missing")?;
    assert_eq!(
        linked.operation,
        native_admission_source::descriptor::Operation::PublishAccountClosureCommandReceiptV2
    );
    assert_eq!(count(&pool,"SELECT count(*) FROM game_account_closure_publications WHERE projection_epoch=4 AND NOT quarantined").await?,3);
    // Audit expiry does not erase or refresh authoritative terminal receipts.
    let held_event: String = sqlx::query_scalar(
        "SELECT event_id::text FROM game_account_closure_audit_outbox ORDER BY event_id LIMIT 1",
    )
    .fetch_one(&pool)
    .await?;
    let hold: String = sqlx::query_scalar("SELECT game_account_closure_hold_audit($1::uuid,'deletion-test-case','retention conformance')::text").bind(&held_event).fetch_one(&pool).await?;
    sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_account_closure_now() RETURNS BIGINT LANGUAGE sql STABLE SET search_path=pg_catalog AS 'SELECT 1797776003::bigint'").execute(&pool).await?;
    let _ = root.expire_account_closure_audit(&authority).await?;
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_account_closure_audit_outbox"
        )
        .await?,
        1
    );
    sqlx::query("SELECT game_account_closure_release_audit_hold($1::uuid)")
        .bind(&hold)
        .execute(&pool)
        .await?;
    let _ = root
        .next_account_closure_publication(&authority, &node, SOURCE)
        .await?;
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_account_closure_audit_outbox"
        )
        .await?,
        0
    );

    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            (&finalize.command, &finalize.operation_id),
            Input::Intent(&final_bytes),
            &config
        )
        .await,
        terminal
    );
    eprintln!(
        "GPROD_DELETION_PG_CHECKS byte_exact_vectors=6 finalized=executed resync=executed stale_watermark_callback=executed audit_expiry=executed legal_hold=executed lifecycle_invalidation=executed stale_recovery=executed immutable_deadline=executed early_finalize_pending=executed new_termination_policy_reset=executed durable_resume=executed receipt_replay=executed cancellation_tombstone=executed exclusive_winner=executed lcfa_lag=executed audit_family_7=executed"
    );
    drop(reopened);
    drop(restarted);
    drop(authority);
    drop(fence);
    drop(recovery);
    drop(root);
    pool.close().await;
    std::fs::remove_dir_all(retained)?;
    Ok(())
}
async fn observe_security_text(
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    account: &str,
    observed_at: i64,
) -> TestResult {
    let revision = S2_REVISION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let body = format!(
        r#"{{"version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform","source_revision":"{revision}","decision_identity":"{revision}","source_observed_at":"{observed_at}","clock_uncertainty_seconds":"0","account_id":"{account}","purpose":"platform_security","scope":"fresh_admission","allowed":false,"minimum_valid_generation":"1"}}"#
    );
    root.accept_native_source_observation(
        node,
        SourceObservation {
            source_authority: "platform".into(),
            operation: NativeSourceOperation::ReadAccountSecurityV1,
            subject: NativeSourceSubject::account_security(account)
                .map_err(|e| format!("{e:?}"))?,
            source_revision: revision,
            decision_identity: revision.to_string(),
            observed_at,
            semantic_facts: body.into_bytes(),
        },
    )
    .await
    .map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn account_deletion_postgres_reserves_resolution_backlog_capacity() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?
                .block_on(async move {
                    let started = std::time::Instant::now();
                    let db = Database::create(admin, "account_deletion_quota").await?;
                    let result = quota_flow(&db).await;
                    db.cleanup().await?;
                    result?;
                    eprintln!(
                        "GPROD_DELETION_PG_EXECUTED reserved_quota elapsed={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "producer test thread failed")?
        .map_err(Into::into)
}

async fn quota_flow(db: &Database) -> TestResult {
    use durability::character_command::account_deletion::{self as deletion, Config};
    let root = DurabilityRoot::connect_test_runtime(&db.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&db.url).await?;
    let retained = fence_parent().join(format!("closure-quota-{}", db.name));
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    {
        let fresh = recovery
            .authorize_fresh_store(id(201), 100)
            .map_err(|e| format!("{e:?}"))?;
        root.admit_fresh_character_recovery(&fresh)
            .await
            .map_err(|e| format!("{e:?}"))?;
    }
    let fence = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let node = register(&root, 1, None).await?;
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/account-deletion/closure.json"
    ))?;
    let original = fixtures["vectors"][0]["wire"]
        .as_str()
        .ok_or("close missing")?;
    let mut close = deletion::decode(
        original.as_bytes(),
        "CloseAccount",
        "0192a000-0000-7000-8000-000000000030",
    )?;
    let now: i64 = sqlx::query_scalar("SELECT game_account_closure_now()")
        .fetch_one(&pool)
        .await?;
    close.expected_recovery_generation = "1".into();
    close.issued_at_source = now.to_string();
    close.expires_at_source = (now + 300).to_string();
    close.scheduled_for = now.to_string();
    let config = Config::default();
    assert!(config.enabled);
    for n in 100..229u16 {
        close.operation_id = uuid(id(n as u8));
        close.source_revision = n.to_string();
        let raw = serde_json::to_vec(&close)?;
        assert_eq!(
            root.decide_character_command_configured(
                &authority,
                &node,
                SOURCE,
                (&close.command, &close.operation_id),
                Input::Intent(&raw),
                &config
            )
            .await,
            Decision::Transient
        );
    }
    assert_eq!(count(&pool,"SELECT count(*) FROM game_account_closure_jobs WHERE command='CloseAccount' AND receipt IS NULL").await?,128);
    let original = fixtures["vectors"][3]["wire"]
        .as_str()
        .ok_or("cancel missing")?;
    let operation: serde_json::Value = serde_json::from_str(original)?;
    let mut cancel = deletion::decode(
        original.as_bytes(),
        "ResolveAccountClosure",
        operation["operation_id"].as_str().ok_or("key missing")?,
    )?;
    cancel.scheduled_for = close.scheduled_for.clone();
    cancel.operation_id = uuid(id(241));
    cancel.source_revision = "241".into();
    cancel.expected_recovery_generation = "1".into();
    cancel.issued_at_source = now.to_string();
    cancel.expires_at_source = (now + 300).to_string();
    let raw = serde_json::to_vec(&cancel)?;
    assert_eq!(
        root.decide_character_command_configured(
            &authority,
            &node,
            SOURCE,
            (&cancel.command, &cancel.operation_id),
            Input::Intent(&raw),
            &config
        )
        .await,
        Decision::Transient
    );
    assert_eq!(count(&pool,"SELECT count(*) FROM game_account_closure_jobs WHERE command='ResolveAccountClosure' AND receipt IS NULL").await?,1);
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_account_closures").await?,
        0
    );
    drop(authority);
    drop(fence);
    drop(recovery);
    drop(root);
    pool.close().await;
    std::fs::remove_dir_all(retained)?;
    Ok(())
}
