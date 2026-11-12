use super::*;
use native_admission_source::character_commands::{Decision, Input};
use sha2::{Digest, Sha256};
use sqlx::Row;

const SOURCE: &str = "oteryn:character-authority:primary";
const TRANSFER: &str = "TransferCharacterOwnership";
const BAZAAR_ISSUER: &str = "OTERYN_PLATFORM_BAZAAR_AUTHORITY";
const CHARACTER_ISSUER: &str = "OTERYN_PLATFORM_CHARACTER_AUTHORITY";

#[test]
fn bazaar_transfer_authority_postgres_rejects_missing_character_and_replays() -> TestResult {
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
                        let started = std::time::Instant::now();
                        let database = Database::create(admin, "bazaar_transfer").await?;
                        let outcome = run_transfer(&database).await;
                        database.cleanup().await?;
                        eprintln!(
                            "GPROD_BAZAAR_PG_EXECUTED transfer_authority elapsed_ms={} outcome={}",
                            started.elapsed().as_millis(),
                            if outcome.is_ok() { "PASS" } else { "FAIL" }
                        );
                        outcome
                    })
            })()
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Bazaar PostgreSQL worker failed")?
        .map_err(Into::into)
}

fn transfer_wire(operation: u8, character: &str, from: u8, to: u8, times: (i64, i64)) -> String {
    format!(
        r#"{{"contract_version":1,"command":"TransferCharacterOwnership","variant":"PLATFORM_BAZAAR_TRANSFER","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"{}","source_revision":"{}","operation_id":"{}","character_id":"{character}","expected_from_account_id":"{}","to_account_id":"{}","purpose":"BAZAAR_SETTLEMENT","issued_at_source":"{}","expires_at_source":"{}","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}}"#,
        uuid(id(20)),
        operation,
        uuid(id(operation)),
        uuid(id(from)),
        uuid(id(to)),
        times.0,
        times.1,
    )
}

fn decision_label(decision: &Decision) -> &str {
    match decision {
        Decision::Publish { result_code, .. } => result_code,
        Decision::Conflict => "OPERATION_BINDING_CONFLICT",
        Decision::Transient => "PENDING",
    }
}

fn assert_result_code(decision: &Decision, expected: &str) {
    assert_eq!(decision_label(decision), expected);
}

async fn run_transfer(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let directory = fence_parent().join(format!("bazaar-{}", database.name));
    std::fs::create_dir(&directory)?;
    let recovery = CharacterRecoveryStore::open(&directory, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .map_err(|e| format!("{e:?}"))?;
    root.admit_fresh_character_recovery(&fresh)
        .await
        .map_err(|e| format!("{e:?}"))?;
    drop(fresh);
    let fence = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let node = register(&root, 1, None).await?;
    let now = now_secs()?;
    let live = (now - 1, now + 120);
    let missing_character = uuid(id(22));
    let raw = transfer_wire(21, &missing_character, 31, 32, live);
    let decide = async |operation: u8, bytes: &[u8]| {
        root.decide_character_command(
            &authority,
            &node,
            SOURCE,
            TRANSFER,
            &uuid(id(operation)),
            Input::Intent(bytes),
        )
        .await
    };

    // The shipped producer is enabled by the owner's rollout decision. An
    // explicit rollback refuses new effects, including the unchanged v1 leg,
    // while preserving every existing immutable receipt.
    let enabled: bool =
        sqlx::query_scalar("SELECT enabled FROM game_bazaar_configuration WHERE singleton")
            .fetch_one(&pool)
            .await?;
    assert!(enabled);
    let mut unauthorized_policy = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *unauthorized_policy)
        .await?;
    let denied = sqlx::query("UPDATE game_bazaar_configuration SET enabled=false WHERE singleton")
        .execute(&mut *unauthorized_policy)
        .await;
    assert!(
        denied
            .as_ref()
            .err()
            .and_then(sqlx::Error::as_database_error)
            .and_then(|error| error.code())
            .is_some_and(|code| code == "42501"),
        "runtime must not alter deployment policy"
    );
    unauthorized_policy.rollback().await?;
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=false WHERE singleton")
        .execute(&pool)
        .await?;
    assert_eq!(decide(21, raw.as_bytes()).await, Decision::Transient);
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_bazaar_decisions").await?,
        0
    );
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=true WHERE singleton")
        .execute(&pool)
        .await?;
    let first = decide(21, raw.as_bytes()).await;
    assert_result_code(&first, "CHAR_CMD_CHARACTER_NOT_FOUND");
    assert_eq!(decide(21, raw.as_bytes()).await, first);
    assert_eq!(
        decide(
            21,
            raw.replace("\"source_revision\":\"21\"", "\"source_revision\":\"22\"")
                .as_bytes()
        )
        .await,
        Decision::Conflict
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_bazaar_decisions").await?,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_bazaar_outbox WHERE NOT receipt_acknowledged"
        )
        .await?,
        1
    );

    // v1 retains its per-variant source-revision contradiction rule. Ordering
    // hints do not authorize effects, but one revision cannot bind two intents.
    let duplicate_revision = transfer_wire(93, &missing_character, 31, 32, live)
        .replace("\"source_revision\":\"93\"", "\"source_revision\":\"21\"");
    assert_result_code(
        &decide(93, duplicate_revision.as_bytes()).await,
        "CHAR_CMD_OPERATION_CONFLICT",
    );

    // Transport's technical refusals are durable v1 decisions too. A timeout
    // stays transient, but a definite 409 or oversized 200 must not loop forever.
    for (operation, input, expected) in [
        (90, Input::ReadConflict, "CHAR_CMD_OPERATION_CONFLICT"),
        (91, Input::Oversized, "CHAR_CMD_INTENT_INVALID"),
        (92, Input::Unsupported, "CHAR_CMD_UNSUPPORTED"),
    ] {
        let first = root
            .decide_character_command(
                &authority,
                &node,
                SOURCE,
                TRANSFER,
                &uuid(id(operation)),
                input,
            )
            .await;
        assert_result_code(&first, expected);
        assert_eq!(
            root.decide_character_command(
                &authority,
                &node,
                SOURCE,
                TRANSFER,
                &uuid(id(operation)),
                input
            )
            .await,
            first
        );
        let changed = transfer_wire(operation, &missing_character, 31, 32, live);
        assert_eq!(
            decide(operation, changed.as_bytes()).await,
            Decision::Conflict
        );
    }

    // These are the real bootstrap, account-evidence and World owners. No root,
    // session, receipt or bank guard is disabled to arrange the successful leg.
    initialize_s2(&root, &node).await?;
    assign_world(&pool, &root, &node, 90, 95).await?;
    configure(&pool, ["profile-1", "ruleset-1", "content-1", "starter-1"]).await?;
    allow(&root, &node, 31).await?;
    let created = root
        .bootstrap_character(&authority, &node, &intent(50, 31, 1)?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let character = uuid(*created.character_id.as_bytes());
    allow(&root, &node, 32).await?;
    let raw = transfer_wire(62, &character, 31, 32, live);
    // A bootstrap-only root has no typed gameplay interpretation yet. Transfer
    // cannot invent that context; absence stays PENDING with no receipt.
    assert_eq!(decide(62, raw.as_bytes()).await, Decision::Transient);
    let absent_receipt: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_bazaar_decisions WHERE issuer=$1 AND operation_id=$2::uuid",
    )
    .bind(CHARACTER_ISSUER)
    .bind(uuid(id(62)))
    .fetch_one(&pool)
    .await?;
    assert_eq!(absent_receipt, 0);
    // The existing revision-one initialization guard accepts a known complete
    // fixture interpretation. No trigger or authority fence is bypassed.
    sqlx::query("INSERT INTO game_character_progression_state(character_id,character_revision,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES($1::uuid,1,1,0,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
        .bind(&character).execute(&pool).await?;
    let committed = decide(62, raw.as_bytes()).await;
    assert_result_code(&committed, "CHAR_CMD_OK");
    let Decision::Publish { receipt, .. } = &committed else {
        return Err("transfer omitted its terminal receipt".into());
    };
    let parsed: serde_json::Value = serde_json::from_slice(receipt)?;
    assert_eq!(parsed["contract_version"], 1);
    assert_eq!(parsed["from_account_id"], uuid(id(31)));
    assert_eq!(parsed["to_account_id"], uuid(id(32)));
    assert_eq!(parsed["character_id"], character);
    assert_eq!(parsed["character_revision"], "2");
    assert_eq!(parsed["outcome"], "COMMITTED");
    assert!(parsed.get("result_data").is_none());
    let owner: String = sqlx::query_scalar(
        "SELECT account_id::text FROM game_character_roots WHERE character_id=$1::uuid",
    )
    .bind(&character)
    .fetch_one(&pool)
    .await?;
    assert_eq!(owner, uuid(id(32)));
    let persisted: Vec<u8> = sqlx::query_scalar(
        "SELECT receipt FROM game_bazaar_decisions WHERE issuer=$1 AND operation_id=$2::uuid",
    )
    .bind(CHARACTER_ISSUER)
    .bind(uuid(id(62)))
    .fetch_one(&pool)
    .await?;
    assert_eq!(&persisted, receipt);
    assert_audit_envelope(
        &pool,
        CHARACTER_ISSUER,
        &uuid(id(62)),
        raw.as_bytes(),
        &persisted,
    )
    .await?;
    assert_eq!(decide(62, raw.as_bytes()).await, committed);
    // A historical transfer proof cannot authorize a later owner write. The
    // runtime's root guard requires the matching physical transaction proof.
    let mut forged = pool.begin().await?;
    let unproved = sqlx::query("UPDATE game_character_roots SET account_id=$2::uuid,character_revision=character_revision+1 WHERE character_id=$1::uuid")
        .bind(&character).bind(uuid(id(33))).execute(&mut *forged).await;
    assert!(unproved.is_err());
    forged.rollback().await?;
    let still_owner: String = sqlx::query_scalar(
        "SELECT account_id::text FROM game_character_roots WHERE character_id=$1::uuid",
    )
    .bind(&character)
    .fetch_one(&pool)
    .await?;
    assert_eq!(still_owner, uuid(id(32)));
    assert_eq!(
        decide(62, raw.replace(&uuid(id(32)), &uuid(id(33))).as_bytes()).await,
        Decision::Conflict
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_bazaar_character_transitions"
        )
        .await?,
        1
    );
    let stale_owner = transfer_wire(63, &character, 31, 33, live);
    assert_result_code(
        &decide(63, stale_owner.as_bytes()).await,
        "CHAR_CMD_OWNER_MISMATCH",
    );

    // Two fresh ownership legs on one Character serialize on its portfolio and
    // admission roots. Exactly one can consume the original owner binding.
    allow(&root, &node, 31).await?;
    let another = root
        .bootstrap_character(&authority, &node, &intent(51, 31, 2)?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let another_character = uuid(*another.character_id.as_bytes());
    sqlx::query("INSERT INTO game_character_progression_state(character_id,character_revision,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) VALUES($1::uuid,1,1,0,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
        .bind(&another_character).execute(&pool).await?;
    // Independent durable roots give the race independent DB holders. This
    // exercises the PostgreSQL portfolio/admission locks, not only one local
    // executor's busy-holder serialization.
    let concurrent_root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(concurrent_root.maintain_ready_once().await?);
    let concurrent_authority = match concurrent_root.open_character_authority(&fence).await {
        Ok(authority) => authority,
        Err(error) => {
            return Err(format!(
                "concurrent authority: {error:?}; {}",
                integrity_diagnostics(&pool).await?
            )
            .into());
        }
    };
    allow(&root, &node, 32).await?;
    allow(&root, &node, 33).await?;
    let race_now = now_secs()?;
    let left_raw = transfer_wire(
        70,
        &another_character,
        31,
        32,
        (race_now - 1, race_now + 120),
    );
    let right_raw = transfer_wire(
        71,
        &another_character,
        31,
        33,
        (race_now - 1, race_now + 120),
    );
    let right_decide = async || {
        concurrent_root
            .decide_character_command(
                &concurrent_authority,
                &node,
                SOURCE,
                TRANSFER,
                &uuid(id(71)),
                Input::Intent(right_raw.as_bytes()),
            )
            .await
    };
    let (mut left, mut right) = poll_both(decide(70, left_raw.as_bytes()), right_decide()).await?;
    // Both commands contend for the same Character. A busy durable holder or
    // a transient database serialization outcome is retryable with the same key
    // and bytes; only terminal results participate in the one-winner assertion.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    while matches!(left, Decision::Transient) || matches!(right, Decision::Transient) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "ownership race retry deadline: left={} right={}",
                decision_label(&left),
                decision_label(&right)
            )
            .into());
        }
        tokio::task::yield_now().await;
        if matches!(left, Decision::Transient) {
            left = decide(70, left_raw.as_bytes()).await;
        }
        if matches!(right, Decision::Transient) {
            right = right_decide().await;
        }
    }
    let race_codes = [decision_label(&left), decision_label(&right)];
    assert_eq!(
        race_codes
            .iter()
            .filter(|code| **code == "CHAR_CMD_OK")
            .count(),
        1,
        "ownership race terminal codes: {race_codes:?}"
    );
    assert_eq!(
        race_codes
            .iter()
            .filter(|code| **code == "CHAR_CMD_OWNER_MISMATCH")
            .count(),
        1,
        "ownership race terminal codes: {race_codes:?}"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_bazaar_character_transitions"
        )
        .await?,
        2
    );
    drop(concurrent_authority);
    drop(concurrent_root);

    // Recovery must accept the actual ownership/revision chain after transfer;
    // validating creation as though it still represented current owner is wrong.
    let reopened = root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("post-transfer integrity failed: {e:?}"))?;
    assert_eq!(
        root.decide_character_command(
            &reopened,
            &node,
            SOURCE,
            TRANSFER,
            &uuid(id(62)),
            Input::Intent(raw.as_bytes()),
        )
        .await,
        committed
    );
    drop(reopened);

    // Rollback drains existing receipts but does not authorize a new key.
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=false WHERE singleton")
        .execute(&pool)
        .await?;
    assert_eq!(decide(62, raw.as_bytes()).await, committed);
    let off = transfer_wire(64, &character, 32, 33, live);
    assert_eq!(decide(64, off.as_bytes()).await, Decision::Transient);
    let before = count(&pool, "SELECT count(*) FROM game_bazaar_decisions").await?;
    let successor = register(&root, 2, Some(1)).await?;
    assert_eq!(decide(62, raw.as_bytes()).await, Decision::Transient);
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_bazaar_decisions").await?,
        before
    );
    drop(authority);
    drop(fence);
    let transition = recovery
        .begin_recovery(1, id(12), 200)
        .map_err(|e| format!("{e:?}"))?;
    root.reconcile_character_recovery(&transition)
        .await
        .map_err(|e| format!("post-transfer recovery failed: {e:?}"))?;
    drop(transition);
    let current_fence = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let current_authority = root
        .open_character_authority(&current_fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    // A receipt that survives recovery is historical immutable evidence. The
    // newer live generation proves publication authority, not a second effect.
    assert_eq!(
        root.decide_character_command(
            &current_authority,
            &successor,
            SOURCE,
            TRANSFER,
            &uuid(id(62)),
            Input::Intent(raw.as_bytes())
        )
        .await,
        committed
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_bazaar_character_transitions"
        )
        .await?,
        2
    );
    drop(current_authority);
    drop(current_fence);
    drop(recovery);
    std::fs::remove_dir_all(directory)?;
    pool.close().await;
    Ok(())
}

#[derive(serde::Deserialize)]
struct FixtureVectors {
    vectors: Vec<FixtureVector>,
}

#[derive(serde::Deserialize)]
struct FixtureVector {
    name: String,
    wire: String,
    sha256: String,
}

fn fixture_vectors() -> TestResult<Vec<FixtureVector>> {
    Ok(serde_json::from_str::<FixtureVectors>(include_str!(
        "../../../../../docs/contracts/fixtures/bazaar/decisions.json"
    ))?
    .vectors)
}

fn fixture<'a>(vectors: &'a [FixtureVector], name: &str) -> TestResult<&'a FixtureVector> {
    vectors
        .iter()
        .find(|vector| vector.name == name)
        .ok_or_else(|| format!("missing Bazaar fixture {name}").into())
}

#[test]
fn bazaar_fixture_postgres_retained_receipts_replay_exactly_while_disabled() -> TestResult {
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
                        let started = std::time::Instant::now();
                        let database = Database::create(admin, "bazaar_fixtures").await?;
                        let outcome = run_fixture_replay(&database).await;
                        database.cleanup().await?;
                        eprintln!(
                            "GPROD_BAZAAR_PG_EXECUTED retained_fixture_replay elapsed_ms={} outcome={}",
                            started.elapsed().as_millis(),
                            if outcome.is_ok() { "PASS" } else { "FAIL" }
                        );
                        outcome
                    })
            })()
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Bazaar fixture PostgreSQL worker failed")?
        .map_err(Into::into)
}

async fn run_fixture_replay(database: &Database) -> TestResult {
    let vectors = fixture_vectors()?;
    for vector in &vectors {
        let actual: String = Sha256::digest(vector.wire.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(actual, vector.sha256, "{} fixture digest", vector.name);
    }
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let directory = fence_parent().join(format!("bazaar-fixture-{}", database.name));
    std::fs::create_dir(&directory)?;
    let recovery = CharacterRecoveryStore::open(&directory, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .map_err(|e| format!("{e:?}"))?;
    root.admit_fresh_character_recovery(&fresh)
        .await
        .map_err(|e| format!("{e:?}"))?;
    drop(fresh);
    // The normative receipts carry generation 3. Reach that real nonrollback
    // floor before importing their historical decisions; never lower the floor.
    for (predecessor, event, issued) in [(1, 12, 200), (2, 13, 300)] {
        let transition = recovery
            .begin_recovery(predecessor, id(event), issued)
            .map_err(|e| format!("{e:?}"))?;
        root.reconcile_character_recovery(&transition)
            .await
            .map_err(|e| format!("{e:?}"))?;
        drop(transition);
    }
    let fence = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let node = register(&root, 1, None).await?;
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=false WHERE singleton")
        .execute(&pool)
        .await?;

    // These imports model a retained store restored before lost publication.
    // Fresh successful effects are tested separately; this case proves that a
    // late retry never rechecks expired TTL or rewrites receipt/signature bytes.
    let stale_receipt = fixture(&vectors, "stale_fence_rejection")?;
    let stale_intent = fixture(&vectors, "delivery_intent")?
        .wire
        .replace(
            "\"operation_id\":\"0192a000-0000-7000-8000-000000000004\"",
            "\"operation_id\":\"0192a000-0000-7000-8000-000000000005\"",
        )
        .replace(
            "\"issuer_decision_id\":\"0192a000-0000-7000-8000-0000000000a1\"",
            "\"issuer_decision_id\":\"0192a000-0000-7000-8000-0000000000a2\"",
        )
        .replace("\"source_revision\":\"20\"", "\"source_revision\":\"21\"");
    for (intent_name, intent_wire, receipt, issuer, result_code) in [
        (
            "escrow_intent",
            fixture(&vectors, "escrow_intent")?.wire.as_str(),
            fixture(&vectors, "escrow_receipt")?,
            BAZAAR_ISSUER,
            "CHAR_CMD_OK",
        ),
        (
            "delivery_intent",
            fixture(&vectors, "delivery_intent")?.wire.as_str(),
            fixture(&vectors, "delivery_receipt")?,
            BAZAAR_ISSUER,
            "CHAR_CMD_OK",
        ),
        (
            "character_settlement_intent",
            fixture(&vectors, "character_settlement_intent")?
                .wire
                .as_str(),
            fixture(&vectors, "character_settlement_receipt")?,
            CHARACTER_ISSUER,
            "CHAR_CMD_OK",
        ),
        (
            "stale_fence_rejection",
            stale_intent.as_str(),
            stale_receipt,
            BAZAAR_ISSUER,
            "CHAR_CMD_BAZAAR_STALE_REVISION",
        ),
    ] {
        let parsed: serde_json::Value = serde_json::from_str(intent_wire)?;
        let operation = parsed["operation_id"].as_str().ok_or("fixture operation")?;
        let command = parsed["command"].as_str().ok_or("fixture command")?;
        let mut payload = vec![0x0a, 32];
        payload.extend_from_slice(&Sha256::digest(intent_wire.as_bytes()));
        payload.extend_from_slice(&[0x12, 32]);
        payload.extend_from_slice(&Sha256::digest(receipt.wire.as_bytes()));
        payload.extend_from_slice(&[0x18, 3]);
        sqlx::query("INSERT INTO game_bazaar_decisions(issuer,operation_id,command,intent,receipt,result_code,recovery_generation,decided_at,event_id,transaction_id,world_id,payload) VALUES($1,$2::uuid,$3,$4,$5,$6,3,1790000002,game_character_uuid_v7(),game_character_uuid_v7(),'0192a000-0000-7000-8000-000000000012'::uuid,$7)")
            .bind(issuer).bind(operation).bind(command).bind(intent_wire.as_bytes()).bind(receipt.wire.as_bytes()).bind(result_code).bind(&payload).execute(&pool).await?;
        let decision = if issuer == CHARACTER_ISSUER {
            root.decide_character_command(
                &authority,
                &node,
                SOURCE,
                command,
                operation,
                Input::Intent(intent_wire.as_bytes()),
            )
            .await
        } else {
            root.decide_bazaar_command(
                &authority,
                &node,
                SOURCE,
                command,
                operation,
                intent_wire.as_bytes(),
            )
            .await
        };
        assert_eq!(
            decision,
            Decision::Publish {
                receipt: receipt.wire.as_bytes().to_vec(),
                result_code: result_code.to_owned()
            },
            "{intent_name}"
        );
        assert_audit_envelope(
            &pool,
            issuer,
            operation,
            intent_wire.as_bytes(),
            receipt.wire.as_bytes(),
        )
        .await?;
    }
    let changed = fixture(&vectors, "changed_retry")?;
    let changed_value: serde_json::Value = serde_json::from_str(&changed.wire)?;
    assert_eq!(
        root.decide_bazaar_command(
            &authority,
            &node,
            SOURCE,
            "EscrowBazaarItem",
            changed_value["operation_id"]
                .as_str()
                .ok_or("changed operation")?,
            changed.wire.as_bytes()
        )
        .await,
        Decision::Conflict
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_bazaar_decisions").await?,
        4
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_bazaar_outbox WHERE NOT receipt_acknowledged"
        )
        .await?,
        4
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_bazaar_character_transitions"
        )
        .await?,
        0
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_bazaar_item_locations").await?,
        0
    );
    drop(authority);
    drop(fence);
    drop(recovery);
    std::fs::remove_dir_all(directory)?;
    pool.close().await;
    Ok(())
}

#[test]
fn bazaar_fixture_postgres_unseen_expired_attempts_are_terminal_without_custody() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        (|| -> TestResult {
            tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
                let started=std::time::Instant::now();
                let database=Database::create(admin,"bazaar_expiry").await?;
                let result=run_expired_fixtures(&database).await;
                database.cleanup().await?;
                eprintln!("GPROD_BAZAAR_PG_EXECUTED unseen_fixture_expiry elapsed_ms={} outcome={}",started.elapsed().as_millis(),if result.is_ok(){"PASS"}else{"FAIL"});
                result
            })
        })().map_err(|error|error.to_string())
    })?.join().map_err(|_|"Bazaar expiry PostgreSQL worker failed")?.map_err(Into::into)
}

async fn run_expired_fixtures(database: &Database) -> TestResult {
    let root = DurabilityRoot::connect_test_runtime(&database.url)?;
    assert!(root.maintain_ready_once().await?);
    let pool = sqlx::PgPool::connect(&database.url).await?;
    let directory = fence_parent().join(format!("bazaar-expiry-{}", database.name));
    std::fs::create_dir(&directory)?;
    let recovery = CharacterRecoveryStore::open(&directory, "character-primary", "game-ops")
        .map_err(|e| format!("{e:?}"))?;
    let fresh = recovery
        .authorize_fresh_store(id(11), 100)
        .map_err(|e| format!("{e:?}"))?;
    root.admit_fresh_character_recovery(&fresh)
        .await
        .map_err(|e| format!("{e:?}"))?;
    drop(fresh);
    let fence = recovery.seal_current().map_err(|e| format!("{e:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let node = register(&root, 1, None).await?;
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=true WHERE singleton")
        .execute(&pool)
        .await?;
    let vectors = fixture_vectors()?;
    for name in [
        "escrow_intent",
        "delivery_intent",
        "character_settlement_intent",
    ] {
        let vector = fixture(&vectors, name)?;
        let parsed: serde_json::Value = serde_json::from_str(&vector.wire)?;
        let command = parsed["command"].as_str().ok_or("expiry fixture command")?;
        let operation = parsed["operation_id"]
            .as_str()
            .ok_or("expiry fixture operation")?;
        let decide = async || {
            if command == TRANSFER {
                root.decide_character_command(
                    &authority,
                    &node,
                    SOURCE,
                    command,
                    operation,
                    Input::Intent(vector.wire.as_bytes()),
                )
                .await
            } else {
                root.decide_bazaar_command(
                    &authority,
                    &node,
                    SOURCE,
                    command,
                    operation,
                    vector.wire.as_bytes(),
                )
                .await
            }
        };
        let first = decide().await;
        assert_result_code(&first, "CHAR_CMD_INTENT_EXPIRED");
        assert_eq!(decide().await, first);
        let Decision::Publish { receipt, .. } = first else {
            return Err("expiry missing receipt".into());
        };
        let decoded: serde_json::Value = serde_json::from_slice(&receipt)?;
        assert_eq!(decoded["outcome"], "REJECTED");
        if command != TRANSFER {
            assert!(
                decoded
                    .get("result_data")
                    .is_some_and(serde_json::Value::is_null)
            );
        } else {
            assert!(decoded.get("result_data").is_none());
        }
    }
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_bazaar_decisions").await?,
        3
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_bazaar_item_locations").await?,
        0
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM game_bazaar_gold_holds").await?,
        0
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM game_bazaar_character_transitions"
        )
        .await?,
        0
    );
    drop(authority);
    drop(fence);
    drop(recovery);
    std::fs::remove_dir_all(directory)?;
    pool.close().await;
    Ok(())
}

#[test]
fn bazaar_transfer_postgres_live_lease_refusal_preserves_owner_and_generations() -> TestResult {
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
                        let started = std::time::Instant::now();
                        let harness = crate::bestiary_postgres_harness::Harness::create(
                            admin,
                            "bazaar_live_lease",
                            true,
                        )
                        .await?;
                        let result = run_live_lease_refusal(&harness).await;
                        harness.cleanup().await?;
                        eprintln!(
                            "GPROD_BAZAAR_PG_EXECUTED live_lease_refusal elapsed_ms={} outcome={}",
                            started.elapsed().as_millis(),
                            if result.is_ok() { "PASS" } else { "FAIL" }
                        );
                        result
                    })
            })()
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Bazaar live lease PostgreSQL worker failed")?
        .map_err(Into::into)
}

async fn run_live_lease_refusal(harness: &crate::bestiary_postgres_harness::Harness) -> TestResult {
    let fence = harness
        .recovery
        .seal_current()
        .map_err(|e| format!("{e:?}"))?;
    let authority = harness
        .root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("{e:?}"))?;
    sqlx::query("UPDATE game_bazaar_configuration SET enabled=true WHERE singleton")
        .execute(&harness.pool)
        .await?;
    let now = now_secs()?;
    let character = uuid(id(41));
    let raw = transfer_wire(80, &character, 40, 60, (now - 1, now + 120));
    let generations = async || -> TestResult<(String, String, i16)> {
        Ok(sqlx::query_as("SELECT current_generation::text,character_lease_generation::text,session_state FROM game_durability_reconnect_sessions WHERE character_id=$1::uuid AND session_state IN(1,2)")
            .bind(&character).fetch_one(&harness.pool).await?)
    };
    let before = generations().await?;
    let decide = async || {
        harness
            .root
            .decide_character_command(
                &authority,
                &harness.node,
                SOURCE,
                TRANSFER,
                &uuid(id(80)),
                Input::Intent(raw.as_bytes()),
            )
            .await
    };
    let first = decide().await;
    assert_result_code(&first, "CHAR_CMD_CHARACTER_IN_SESSION");
    assert_eq!(decide().await, first);
    assert_eq!(generations().await?, before);
    let root:(String,String,i16)=sqlx::query_as("SELECT account_id::text,character_revision::text,lifecycle FROM game_character_roots WHERE character_id=$1::uuid")
        .bind(&character).fetch_one(&harness.pool).await?;
    assert_eq!(root, (uuid(id(40)), "1".to_owned(), 1));
    assert_eq!(harness.count("game_bazaar_character_transitions").await?, 0);
    Ok(())
}

async fn poll_both<L: std::future::Future, R: std::future::Future>(
    left: L,
    right: R,
) -> TestResult<(L::Output, R::Output)> {
    let mut left = std::pin::pin!(left);
    let mut right = std::pin::pin!(right);
    let mut left_result = None;
    let mut right_result = None;
    std::future::poll_fn(|context| {
        if left_result.is_none()
            && let std::task::Poll::Ready(result) =
                std::future::Future::poll(left.as_mut(), context)
        {
            left_result = Some(result);
        }
        if right_result.is_none()
            && let std::task::Poll::Ready(result) =
                std::future::Future::poll(right.as_mut(), context)
        {
            right_result = Some(result);
        }
        if left_result.is_some() && right_result.is_some() {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    Ok((
        left_result.ok_or("missing concurrent left result")?,
        right_result.ok_or("missing concurrent right result")?,
    ))
}

#[derive(Clone, PartialEq, prost::Message)]
struct BazaarAuditPayload {
    #[prost(bytes = "vec", tag = "1")]
    intent_sha256: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    receipt_sha256: Vec<u8>,
    #[prost(uint64, tag = "3")]
    recovery_generation: u64,
}

async fn assert_audit_envelope(
    pool: &sqlx::PgPool,
    issuer: &str,
    operation: &str,
    intent: &[u8],
    receipt: &[u8],
) -> TestResult {
    use durability::item_mint_audit::EventEnvelopeV1;
    let row=sqlx::query("SELECT a.envelope,uuid_send(d.event_id) AS event_id,uuid_send(d.transaction_id) AS transaction_id,uuid_send(d.operation_id) AS operation_id,uuid_send(d.world_id) AS world_id,d.payload,d.recovery_generation::text,d.server_build_id,a.occurred_at,a.expires_at,a.acknowledged FROM game_bazaar_decisions d JOIN game_bazaar_audit_outbox a USING(event_id) WHERE d.issuer=$1 AND d.operation_id=$2::uuid")
        .bind(issuer).bind(operation).fetch_one(pool).await?;
    let raw: Vec<u8> = row.try_get("envelope")?;
    let envelope = EventEnvelopeV1::decode(raw.as_slice())?;
    assert_eq!(
        envelope.encode_to_vec(),
        raw,
        "canonical registered EventEnvelope"
    );
    assert_eq!(envelope.envelope_revision, 1);
    assert_eq!(envelope.event_type_id, 6);
    assert_eq!(envelope.event_schema_revision, 1);
    assert_eq!(envelope.durability_class, 2);
    assert_eq!(envelope.privacy_class, 3);
    assert_eq!(
        envelope.retention_profile_id,
        "PLAT3_AUTHORITY_AUDIT_RETENTION_V1"
    );
    assert_eq!(envelope.event_id, row.try_get::<Vec<u8>, _>("event_id")?);
    assert_eq!(
        envelope.operation_id,
        Some(row.try_get::<Vec<u8>, _>("operation_id")?)
    );
    assert_eq!(
        envelope.world_id,
        row.try_get::<Option<Vec<u8>>, _>("world_id")?
    );
    assert!(envelope.channel_id.is_none());
    assert!(envelope.game_session_id.is_none());
    assert!(envelope.connection_generation.is_none());
    assert_eq!(
        envelope.server_build_id,
        row.try_get::<String, _>("server_build_id")?
    );
    assert_eq!(
        envelope.occurred_at_unix_ms,
        row.try_get::<i64, _>("occurred_at")?
    );
    assert_eq!(
        row.try_get::<i64, _>("expires_at")?,
        envelope.occurred_at_unix_ms + 7_776_000_000
    );
    assert!(!row.try_get::<bool, _>("acknowledged")?);
    let transaction = envelope
        .transaction_event
        .as_ref()
        .ok_or("missing TransactionEventRef")?;
    assert_eq!(
        transaction.transaction_id,
        row.try_get::<Vec<u8>, _>("transaction_id")?
    );
    assert_eq!((transaction.ordinal, transaction.count), (1, 1));
    assert_eq!(envelope.payload, row.try_get::<Vec<u8>, _>("payload")?);
    assert_eq!(
        envelope.payload_sha256,
        Sha256::digest(&envelope.payload).to_vec()
    );
    let payload = BazaarAuditPayload::decode(envelope.payload.as_slice())?;
    assert_eq!(payload.intent_sha256, Sha256::digest(intent).to_vec());
    assert_eq!(payload.receipt_sha256, Sha256::digest(receipt).to_vec());
    assert_eq!(
        payload.recovery_generation,
        row.try_get::<String, _>("recovery_generation")?
            .parse::<u64>()?
    );
    let altered=sqlx::query("UPDATE game_bazaar_audit_outbox SET envelope=envelope||decode('00','hex') WHERE event_id=$1::uuid")
        .bind(envelope.event_id.iter().map(|byte|format!("{byte:02x}")).collect::<String>()).execute(pool).await;
    assert!(
        altered.is_err(),
        "authoritative audit bytes must be immutable"
    );
    Ok(())
}

/// Counts explain the failing integrity boundary without leaking fixture IDs or
/// dumping raw receipts. They run before the isolated database is cleaned up.
async fn integrity_diagnostics(pool: &sqlx::PgPool) -> TestResult<String> {
    let mismatched_revisions:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_roots r LEFT JOIN game_character_progression_state s USING(character_id) WHERE (r.character_revision<>1 AND s.character_id IS NULL) OR (s.character_id IS NOT NULL AND s.character_revision<>r.character_revision)").fetch_one(pool).await?;
    let rows=sqlx::query("SELECT a.payload,uuid_send(r.account_id) AS current_account,uuid_send(c.account_id) AS creation_account,uuid_send(r.character_id) AS character_id,uuid_send(r.world_id) AS world_id FROM game_character_audit_outbox a JOIN game_character_roots r USING(character_id) JOIN game_character_creation_receipts c ON c.event_id=a.event_id ORDER BY a.event_id LIMIT 32").fetch_all(pool).await?;
    let mut original_matches = 0;
    let mut current_matches = 0;
    for row in &rows {
        let current: [u8; 16] = row
            .try_get::<Vec<u8>, _>("current_account")?
            .try_into()
            .map_err(|_| "invalid current audit UUID")?;
        let original: [u8; 16] = row
            .try_get::<Vec<u8>, _>("creation_account")?
            .try_into()
            .map_err(|_| "invalid original audit UUID")?;
        let character: [u8; 16] = row
            .try_get::<Vec<u8>, _>("character_id")?
            .try_into()
            .map_err(|_| "invalid Character audit UUID")?;
        let world: [u8; 16] = row
            .try_get::<Vec<u8>, _>("world_id")?
            .try_into()
            .map_err(|_| "invalid World audit UUID")?;
        let payload: Vec<u8> = row.try_get("payload")?;
        original_matches +=
            usize::from(payload == audit::encode_bootstrap(&original, &character, &world));
        current_matches +=
            usize::from(payload == audit::encode_bootstrap(&current, &character, &world));
    }
    Ok(format!(
        "typed_revision_mismatches={mismatched_revisions}; bootstrap_original_matches={original_matches}/{}; bootstrap_current_matches={current_matches}/{}",
        rows.len(),
        rows.len()
    ))
}
