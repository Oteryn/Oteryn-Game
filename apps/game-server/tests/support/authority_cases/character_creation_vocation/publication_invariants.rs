use super::*;

#[test]
fn retained_create_publication_invariants_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            (|| -> TestResult {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(async {
                        let db = Database::create(admin, "create_publication").await?;
                        let result = exercise_invariants(&db).await;
                        db.cleanup().await?;
                        result
                    })
            })()
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "create publication worker panicked")?
        .map_err(Into::into)
}

struct Pending<'a, 'f, 's> {
    root: &'a DurabilityRoot,
    authority: &'a ReconciledCharacterAuthority<'f, 's>,
    node: &'a NodeIncarnationProof,
}

impl Decider for Pending<'_, '_, '_> {
    async fn decide(&mut self, command: &str, operation: &str, input: Input<'_>) -> Decision {
        self.root
            .decide_character_command(self.authority, self.node, SOURCE, command, operation, input)
            .await
    }

    async fn sendable_create(&mut self, operation: &str) -> Result<CreateSendability, SourceError> {
        self.root
            .sendable_create_receipt(self.authority, self.node, SOURCE, operation)
            .await
            .map_err(|_| SourceError::Unavailable)
    }

    async fn stop_retained_create_receipt(&mut self, operation: &str) -> Result<(), SourceError> {
        self.root
            .stop_retained_create_receipt(self.authority, self.node, operation)
            .await
            .map_err(|_| SourceError::Unavailable)
    }
}

async fn pending_poll(
    store: Pending<'_, '_, '_>,
    operation: &str,
    intent: &str,
) -> TestResult<Vec<(Operation, String)>> {
    let calls = Calls::default();
    let decoded: serde_json::Value = serde_json::from_str(intent)?;
    let revision = decoded["source_revision"]
        .as_str()
        .ok_or("source revision")?;
    let page = format!(
        r#"{{"contract_version":1,"pending":[{{"command":"CreateCharacter","operation_id":"{operation}","source_revision":"{revision}"}}]}}"#
    );
    let peer = Peer {
        calls: calls.clone(),
        replies: VecDeque::from([
            Ok(page.into_bytes()),
            Ok(intent.as_bytes().to_vec()),
            Ok(ACK.to_vec()),
        ]),
    };
    drive(1, |clock| {
        Consumer::new(clock, peer, store, SOURCE.into(), |_| {}).run()
    })
    .await?;
    let sent = calls
        .lock()
        .map_err(|_| "publication calls poisoned")?
        .clone();
    assert_eq!(
        sent.first().map(|call| call.0),
        Some(Operation::ListPendingCharacterCommandsV1)
    );
    assert_eq!(
        sent.get(1).map(|call| call.0),
        Some(Operation::ReadCharacterCommandIntentV1)
    );
    Ok(sent)
}

async fn retained_poll(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    reply: Result<Vec<u8>, Failure>,
    event: fn(&str),
) -> TestResult<Vec<(Operation, String)>> {
    let calls = Calls::default();
    let peer = Peer {
        calls: calls.clone(),
        replies: VecDeque::from([reply]),
    };
    let store = RetainedCreatePublisherStore::new(root, authority, node, SOURCE)?;
    drive(1, |clock| {
        run_retained_create_publications(clock, peer, store, event)
    })
    .await?;
    let result = calls
        .lock()
        .map_err(|_| "publication calls poisoned")?
        .clone();
    Ok(result)
}

async fn exercise_invariants(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let retained = fence_parent().join(format!("oteryn-{}", database.name));
    std::fs::create_dir(&retained)?;
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|error| format!("{error:?}"))?;
    {
        let fresh = recovery
            .authorize_fresh_store(id(11), 100)
            .map_err(|error| format!("{error:?}"))?;
        root.admit_fresh_character_recovery(&fresh)
            .await
            .map_err(|error| format!("{error:?}"))?;
    }
    let sealed = recovery
        .seal_current()
        .map_err(|error| format!("{error:?}"))?;
    let authority = root
        .open_character_authority(&sealed)
        .await
        .map_err(|error| format!("{error:?}"))?;
    let node = register(&root, 1, None).await?;
    closure_while_create_holder_is_busy(&pool, &root, &authority, &node).await?;
    initialize_s2(&root, &node).await?;
    assign_world(&pool, &root, &node, 90, 95).await?;
    configure(
        &pool,
        ["profile-1", "ruleset-1", "content-1", "newhaven-starter-r1"],
    )
    .await?;
    allow(&root, &node, 41).await?;

    // Put the stopped prefix before the live receipts both by operation ID
    // and by insertion key. Only the two live intents need real creation.
    seed_stopped_prefix(&pool).await?;
    let issue = now_secs()?;
    let mut committed = Vec::new();
    for tag in [61, 62, 63] {
        let operation = uuid(id(tag));
        let raw = create_intent(
            tag,
            41,
            90,
            u64::from(tag - 56),
            &name_for(tag),
            (issue - 1, issue + 120),
        );
        let Decision::Publish {
            receipt,
            result_code,
        } = root
            .decide_character_command(
                &authority,
                &node,
                SOURCE,
                CREATE_COMMAND,
                &operation,
                Input::Intent(raw.as_bytes()),
            )
            .await
        else {
            return Err("committed create fixture failed".into());
        };
        assert_eq!(result_code, "CHAR_CMD_OK");
        committed.push((operation, raw, String::from_utf8(receipt)?));
    }
    let [
        (operation, intent, receipt),
        (invalid, invalid_intent, _),
        (valid, valid_intent, valid_receipt),
    ] = committed.as_slice()
    else {
        return Err("missing committed create test rows".into());
    };
    let published = Operation::PublishCharacterCommandReceiptV1;
    let pending = || Pending {
        root: &root,
        authority: &authority,
        node: &node,
    };
    let healthy = pending_poll(pending(), operation, intent).await?;
    assert_eq!(healthy.len(), 3);
    assert_eq!(healthy[2], (published, receipt.clone()));

    let mut index = RetainedCreatePublisherStore::new(&root, &authority, &node, SOURCE)?;
    let page = index
        .scan_retained_creates(0, None)
        .await
        .map_err(|_| "scan after stopped prefix")?;
    assert_eq!(
        page.keys
            .iter()
            .map(|key| key.operation_id.as_str())
            .collect::<Vec<_>>(),
        [operation.as_str(), invalid.as_str()]
    );
    assert_eq!(
        retained_poll(&root, &authority, &node, Err(classify(404, b"")), |_| {}).await?,
        [(published, receipt.clone())]
    );
    assert!(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM game_character_retained_create_receipt_stops WHERE operation_id=$1::uuid)"
    ).bind(operation).fetch_one(&pool).await?);
    for _ in 0..2 {
        assert_eq!(pending_poll(pending(), operation, intent).await?.len(), 2);
    }

    // Mutate only the bytes. The fences, row identity and scan position stay valid.
    let mut mutation = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *mutation)
        .await?;
    sqlx::query("UPDATE game_character_command_receipts SET receipt = convert_to('{}','UTF8') WHERE operation_id=$1::uuid")
        .bind(invalid).execute(&mut *mutation).await?;
    mutation.commit().await?;
    thread_local! {
        static INVALID: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    assert_eq!(
        pending_poll(pending(), invalid, invalid_intent)
            .await?
            .len(),
        2
    );
    INVALID.set(0);
    for _ in 0..2 {
        assert_eq!(
            retained_poll(&root, &authority, &node, Ok(ACK.to_vec()), |event| {
                if event.ends_with("alarm=invalid_retained_receipt") {
                    INVALID.set(INVALID.get() + 1);
                }
            })
            .await?,
            [(published, valid_receipt.clone())]
        );
    }
    assert_eq!(INVALID.get(), 2);
    assert!(!sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM game_character_retained_create_receipt_stops WHERE operation_id=$1::uuid)"
    ).bind(invalid).fetch_one(&pool).await?);

    // Supersede just the node while preserving the existing recovery fence.
    let successor = register(&root, 2, Some(1)).await?;
    assert!(index.sendable_create(valid).await.is_err());
    assert_eq!(pending_poll(pending(), valid, valid_intent).await?.len(), 2);
    assert!(
        retained_poll(&root, &authority, &node, Ok(ACK.to_vec()), |_| {})
            .await?
            .is_empty()
    );
    let mut current = RetainedCreatePublisherStore::new(&root, &authority, &successor, SOURCE)?;
    assert!(matches!(
        current.sendable_create(valid).await,
        Ok(CreateSendability::Ready(_))
    ));

    // Raise just the recovery generation while continuing to use the live node.
    let predecessor = authority
        .record_for(&root)
        .map_err(|error| format!("{error:?}"))?;
    let digest = recovery_record_digest(&predecessor).map_err(|error| format!("{error:?}"))?;
    sqlx::query(
        "INSERT INTO game_character_recovery_admissions(authority_scope_id,recovery_generation,
            recovery_event_id,predecessor_generation,predecessor_digest,issued_at,issuer_identity,reconciled_at)
         VALUES ('character-primary',2,$1::uuid,1,$2,101,'game-ops',0)"
    ).bind(uuid(id(12))).bind(digest.as_slice()).execute(&pool).await?;
    assert!(current.sendable_create(valid).await.is_err());
    assert_eq!(
        pending_poll(
            Pending {
                root: &root,
                authority: &authority,
                node: &successor
            },
            valid,
            valid_intent,
        )
        .await?
        .len(),
        2
    );
    assert!(
        retained_poll(&root, &authority, &successor, Ok(ACK.to_vec()), |_| {})
            .await?
            .is_empty()
    );

    drop(authority);
    drop(sealed);
    drop(recovery);
    pool.close().await;
    std::fs::remove_dir_all(retained)?;
    Ok(())
}

async fn closure_while_create_holder_is_busy(
    pool: &sqlx::PgPool,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
) -> TestResult {
    let mut barrier = pool.begin().await?;
    sqlx::query("LOCK TABLE game_character_retained_create_scan IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *barrier)
        .await?;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *barrier)
        .await?;
    let mut store = RetainedCreatePublisherStore::new(root, authority, node, SOURCE)?;
    let scan = store.scan_retained_creates(0, None);
    tokio::pin!(scan);
    let independently_serve = async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid)))"
            ).bind(pid).fetch_one(&mut *barrier).await?;
            if waiting {
                break;
            }
            tokio::task::yield_now().await;
        }
        let closure = root
            .next_account_closure_publication(authority, node, SOURCE)
            .await?
            .ok_or("due closure heartbeat missing while publisher holds its slot")?;
        assert_eq!(
            closure.operation,
            Operation::PublishAccountClosureWatermarkV1
        );
        barrier.commit().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::select! {
        result = independently_serve => result?,
        _ = &mut scan => return Err("retained scan did not wait for the test barrier".into()),
        _ = tokio::time::sleep(Duration::from_secs(10)) => return Err("retained scan never acquired its holder".into()),
    }
    assert!(
        scan.await
            .map_err(|_| "scan after barrier")?
            .keys
            .is_empty()
    );
    Ok(())
}

async fn seed_stopped_prefix(pool: &sqlx::PgPool) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO game_character_account_guards (account_id) VALUES ('00000001-0000-7000-8000-000000000001')"
    ).execute(&mut *tx).await?;
    // These roots only satisfy the receipt foreign keys. Their gameplay
    // projections and progression chains are outside the scan fixture.
    sqlx::query("ALTER TABLE game_character_roots DISABLE TRIGGER USER")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, character_revision,
             profile_revision, ruleset_revision, content_revision, starter_template_revision, name)
         SELECT ('00000002-0000-7000-8000-' || lpad(to_hex(n),12,'0'))::uuid,
             '00000001-0000-7000-8000-000000000001', $1::uuid, 1, 1,
             'profile-1', 'ruleset-1', 'content-1', 'newhaven-starter-r1',
             'Stopped' || chr(65+n/676) || chr(65+(n/26)%26) || chr(65+n%26)
         FROM generate_series(1,2048) AS n"
    ).bind(uuid(id(90))).execute(&mut *tx).await?;
    sqlx::query("ALTER TABLE game_character_roots ENABLE TRIGGER USER")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_command_receipts(operation_id,command,variant,source_revision,command_binding,
             outcome,result_code,recovery_generation,decided_at,receipt,character_id,account_id,world_id,event_id,
             transaction_id,server_build_id,occurred_at)
         SELECT ('00000003-0000-7000-8000-' || lpad(to_hex(n),12,'0'))::uuid,
             'CreateCharacter',1,n+1000,decode('00','hex'),1,'CHAR_CMD_OK',1,1,decode('00','hex'),
             ('00000002-0000-7000-8000-' || lpad(to_hex(n),12,'0'))::uuid,
             '00000001-0000-7000-8000-000000000001',$1::uuid,
             ('00000004-0000-7000-8000-' || lpad(to_hex(n),12,'0'))::uuid,
             ('00000005-0000-7000-8000-' || lpad(to_hex(n),12,'0'))::uuid,'fixture',0
         FROM generate_series(1,2048) AS n"
    ).bind(uuid(id(90))).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_character_retained_create_receipt_stops(operation_id)
         SELECT operation_id FROM game_character_command_receipts WHERE source_revision > 1000",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
