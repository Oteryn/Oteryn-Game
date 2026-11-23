//! Finalization retires a nonempty portfolio independently of physical erasure policy.
use super::*;
use durability::character_command::account_deletion::{self as deletion, Config};
use native_admission_source::account_closure::{LifecycleObservation, LifecycleRequest};
use native_admission_source::character_commands::{Decision, Input};

const SOURCE: &str = "oteryn:character-authority:primary";

#[test]
fn account_deletion_finalizes_character_without_physical_erasure_policy() -> TestResult {
    postgres_case(false)
}

#[test]
fn restored_closure_replays_receipt_snapshot_and_watermark() -> TestResult {
    postgres_case(true)
}

fn postgres_case(restore: bool) -> TestResult {
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
                    let db = Database::create(admin, "closure_retirement_audit").await?;
                    let result = exercise(&db, restore).await;
                    db.cleanup().await?;
                    result
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "closure retirement thread failed")?
        .map_err(Into::into)
}

async fn exercise(db: &Database, restore: bool) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&db.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&db.url).await?;
    let retained = fence_parent().join(format!("closure-retirement-{}", db.name));
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .map_err(|e| format!("{e:?}"))?;
    root.admit_fresh_character_recovery(&fresh)
        .await
        .map_err(|e| format!("{e:?}"))?;
    drop(fresh);
    let seal = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&seal)
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
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/contracts/fixtures/account-deletion/closure.json"
    ))?;
    let mut close = deletion::decode(
        fixtures["vectors"][0]["wire"]
            .as_str()
            .ok_or("close fixture")?
            .as_bytes(),
        "CloseAccount",
        "0192a000-0000-7000-8000-000000000030",
    )?;
    let now = now_secs()?;
    close.account_id = uuid(id(31));
    close.expected_recovery_generation = "1".into();
    close.scheduled_for = now.to_string();
    close.issued_at_source = now.to_string();
    close.expires_at_source = (now + 300).to_string();
    let config = Config {
        name_release_after_seconds: Some(0),
        ..Config::default()
    };
    assert_eq!(config.physical_erasure_after_seconds, None);
    let mut observation = LifecycleObservation {
        contract_version: 2,
        operation: "ReadAccountClosureLifecycleV1".into(),
        source_authority: "OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),
        account_id: close.account_id.clone(),
        termination_id: close.termination_id.clone(),
        lifecycle_generation: close.lifecycle_generation.clone(),
        source_revision: close.source_revision.clone(),
        phase: "CLOSE".into(),
        financial_obligations_clear: true,
        recovery_generation: "1".into(),
        observed_at: now.to_string(),
        valid_until: (now + 5).to_string(),
    };
    // Exercise the bounded producer until it issues its terminal receipt.
    for finalize in [false, true] {
        let mut command = close.clone();
        if finalize {
            command.command = "ResolveAccountClosure".into();
            command.variant = "PLATFORM_ACCOUNT_CLOSE_RESOLUTION".into();
            command.operation_id = uuid(id(70));
            command.close_operation_id = Some(close.operation_id.clone());
            command.decision = Some("FINALIZE".into());
            command.lifecycle_generation = "9".into();
            command.source_revision = "31".into();
            observation.phase = "FINALIZE".into();
            observation.lifecycle_generation = command.lifecycle_generation.clone();
            observation.source_revision = command.source_revision.clone();
            // The public tombstone has been delivered; no public name handoff is pending.
            sqlx::query("DELETE FROM game_public_character_profile_outbox WHERE character_id=encode($1,'hex')::uuid").bind(created.character_id.as_bytes().as_slice()).execute(&pool).await?;
            sqlx::query("DELETE FROM game_public_name_handoffs WHERE character_id=encode($1,'hex')::uuid").bind(created.character_id.as_bytes().as_slice()).execute(&pool).await?;
        }
        let raw = serde_json::to_vec(&command)?;
        let request = LifecycleRequest::from_intent(SOURCE, &raw, 1)?;
        root.observe_account_closure_lifecycle(
            &authority,
            &node,
            &request,
            &serde_json::to_vec(&observation)?,
        )
        .await?;
        let mut terminal = None;
        for _ in 0..8 {
            let outcome = root
                .decide_character_command_configured(
                    &authority,
                    &node,
                    SOURCE,
                    (&command.command, &command.operation_id),
                    Input::Intent(&raw),
                    &config,
                )
                .await;
            if matches!(outcome, Decision::Publish { .. }) {
                terminal = Some(outcome);
                break;
            }
        }
        assert!(
            matches!(terminal, Some(Decision::Publish { ref result_code, .. }) if result_code == "CHAR_CMD_OK"),
            "finalize={finalize}: {terminal:?}"
        );
    }
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_account_closures WHERE state='FINALIZED'"
        )
        .await?,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_account_closure_members WHERE step=3"
        )
        .await?,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_character_name_reservations"
        )
        .await?,
        0
    );
    assert_eq!(count(&pool, "SELECT count(*) FROM game_account_closure_erasure_queue WHERE state='PENDING_DOMAIN_RETENTION' AND not_before IS NULL").await?, 1);
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_character_roots").await?,
        1
    );
    if restore {
        use native_admission_source::descriptor::Operation;
        use native_admission_source::account_characters::EpochFenceFile;
        let original: (Vec<u8>, Vec<u8>) = sqlx::query_as(
            "SELECT p.body,j.receipt FROM game_account_closure_publications p JOIN game_account_closure_jobs j ON j.operation_id=p.receipt_operation_id",
        )
        .fetch_one(&pool)
        .await?;
        sqlx::query("UPDATE game_account_closure_jobs SET receipt_acknowledged=true WHERE receipt IS NOT NULL")
            .execute(&pool).await?;
        sqlx::query("UPDATE game_account_closure_publications SET acknowledged=true")
            .execute(&pool).await?;
        drop(authority);
        drop(seal);
        let transition = recovery.begin_recovery(1, id(12), 200).map_err(|e| format!("{e:?}"))?;
        root.reconcile_character_recovery(&transition).await.map_err(|e| format!("{e:?}"))?;
        drop(transition);
        let seal = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
        let authority = root.open_character_authority(&seal).await.map_err(|e| format!("{e:?}"))?;
        let epoch_path = retained.join("closure-epoch");
        std::fs::write(&epoch_path, b"1\n")?;
        let mut epoch = EpochFenceFile::new(epoch_path);
        let mut next = async || production_server::node::serve::next_closure_publication_for_node(
            &root, &authority, &node, SOURCE, &mut epoch,
        ).await;
        assert!(next().await.is_none(), "operator must qualify the new generation");
        sqlx::query("SELECT game_account_closure_resync(2)").execute(&pool).await?;
        assert!(next().await.is_none(), "resync must requeue a page before delivery");
        let receipt = next().await.ok_or("restored receipt missing")?;
        assert_eq!(receipt.operation, Operation::PublishAccountClosureCommandReceiptV2);
        assert_eq!(receipt.body.as_bytes(), original.1);
        root.acknowledge_account_closure(&authority, &node, None, Some(&receipt)).await?;
        let snapshot = next().await.ok_or("restored snapshot missing")?;
        assert_eq!(snapshot.operation, Operation::PublishAccountClosureV1);
        let mut expected: serde_json::Value = serde_json::from_slice(&original.0)?;
        expected["projection_epoch"] = "2".into();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&snapshot.body)?, expected);
        root.acknowledge_account_closure(&authority, &node, None, Some(&snapshot)).await?;
        // Simulate the independent projection publishers draining their queues.
        sqlx::raw_sql("DELETE FROM game_character_account_projection_outbox;
            DELETE FROM game_public_character_profile_outbox;
            DELETE FROM game_public_highscore_outbox;").execute(&pool).await?;
        let watermark = next().await.ok_or("restored watermark blocked")?;
        assert_eq!(watermark.operation, Operation::PublishAccountClosureWatermarkV1);
        eprintln!("PLATFORM_PG_EXECUTED: restored closure receipt, snapshot and watermark");
    }
    Ok(())
}
