// RC5: the admission relation fence is one statement and keeps every lock and its mode.

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
fn admission_relation_fence_locks_every_relation_exclusive_in_one_statement() -> TestResult {
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
        let expected: Vec<&str> = super::ADMISSION_RELATIONS_LOCK
            .trim_start_matches("LOCK TABLE ")
            .trim_end_matches(" IN EXCLUSIVE MODE")
            .split(", ")
            .collect();
        assert_eq!(expected.len(), 17);
        assert!(expected.windows(2).all(|pair| pair[0] < pair[1]), "lexical lock order");
        assert_eq!(locked, expected);
        database.cleanup().await
    })
}
