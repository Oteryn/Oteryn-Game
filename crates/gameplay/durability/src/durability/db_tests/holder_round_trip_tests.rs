// RC5: the admission relation fence keeps the required lock set and EXCLUSIVE mode.

#[test]
fn inventory_capacity_observation_rejects_mutation_locks_postgres() -> TestResult {
    run_postgres_test(async {
        let (database, url) = migrated_database("capacity_observation_read_only").await?;
        let root = ready_root(&url).await?;
        let code = root.try_issue_semantic_pass()?.run(|holder, deadline| {
            Box::pin(async move {
                let mut tx = super::begin_observation_transaction(holder, deadline).await?;
                let result = sqlx::query("SELECT character_id FROM game_character_roots FOR UPDATE")
                    .fetch_all(&mut *tx).await;
                let code = result.err().and_then(|error| {
                    error.as_database_error().and_then(|error| error.code()).map(|code| code.into_owned())
                });
                tx.rollback().await?;
                Ok(code)
            })
        }).await?;
        assert_eq!(code.as_deref(), Some("25006"), "observation must reject mutation locks even on an empty table");
        database.cleanup().await
    })
}

#[test]
fn admission_relation_fence_locks_the_required_relations_exclusively() -> TestResult {
    run_postgres_test(async {
        let (database, url) = migrated_database("holder_fence_one_stmt").await?;
        let root = ready_root(&url).await?;
        let locked: Vec<String> = root
            .try_issue_semantic_pass()?
            .run(|holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::begin_semantic_transaction(holder, deadline).await?;
                    super::lock_admission_relations(&mut tx).await?;
                    let locked = sqlx::query_scalar(
                        "SELECT c.relname::text FROM pg_locks l JOIN pg_class c ON c.oid = l.relation \
                         WHERE l.pid = pg_backend_pid() AND l.locktype = 'relation' \
                           AND l.mode = 'ExclusiveLock' AND l.granted \
                           AND c.relname LIKE 'game_durability_%' ORDER BY c.relname COLLATE \"C\"",
                    )
                    .fetch_all(&mut *tx)
                    .await?;
                    super::commit_semantic_transaction(tx, deadline).await?;
                    Ok(locked)
                })
            })
            .await?;
        // The shared native/SQL fence also appears in migrations 0095 (House shapes)
        // and 0148 (World reset). Keep this expectation independent of the executed SQL.
        let expected = [
            "game_durability_admission_account_guards",
            "game_durability_admission_character_guards",
            "game_durability_admission_guard_history",
            "game_durability_admission_lifecycle_receipts",
            "game_durability_admission_runtime_guards",
            "game_durability_admission_signing_trust_guards",
            "game_durability_control_loss_continuity",
            "game_durability_executor_custody",
            "game_durability_fresh_admission_receipts",
            "game_durability_reconnect_attempts",
            "game_durability_reconnect_pending_commands",
            "game_durability_reconnect_sessions",
            "game_durability_recovery_grant_consumptions",
            "game_durability_session_replacements",
            "game_durability_session_use_ledgers",
            "game_durability_session_use_memberships",
            "game_durability_transport_ref_reservations",
        ];
        let declared: Vec<&str> = super::ADMISSION_RELATIONS_LOCK
            .trim_start_matches("LOCK TABLE ")
            .trim_end_matches(" IN EXCLUSIVE MODE")
            .split(", ")
            .collect();
        assert_eq!(declared, expected, "required relations in lexical lock order");
        assert_eq!(locked, expected, "the holder owns every required EXCLUSIVE lock");
        database.cleanup().await
    })
}
