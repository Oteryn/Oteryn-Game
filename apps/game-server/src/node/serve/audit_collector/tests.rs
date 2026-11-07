use super::*;
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, debug, runtime};
use crate::durability::item_mint_audit::EventEnvelopeV1;
use prost::Message;
use rcgen::PublicKeyData;

#[test]
fn audit_collector_bazaar_postgres_journals_acknowledges_and_retries() -> TestResult {
    postgres_case(Source::Bazaar)
}
#[test]
fn audit_collector_account_closure_postgres_journals_acknowledges_and_retries() -> TestResult {
    postgres_case(Source::AccountClosure)
}
#[test]
fn audit_collector_t94_postgres_journals_acknowledges_and_retries() -> TestResult {
    postgres_case(Source::T94)
}

fn postgres_case(source: Source) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            (|| -> TestResult {
                runtime()?.block_on(async {
                    let started = std::time::Instant::now();
                    let h =
                        Harness::create(admin, &format!("audit_{}", source.name()), false).await?;
                    let result = exercise(&h, source).await;
                    h.cleanup().await?;
                    eprintln!(
                        "GWIRE2_PG_EXECUTED source={} elapsed_ms={} result={}",
                        source.name(),
                        started.elapsed().as_millis(),
                        if result.is_ok() { "PASS" } else { "FAIL" }
                    );
                    result
                })
            })()
            .map_err(debug)
        })?
        .join()
        .map_err(|_| "audit collector test thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &Harness, source: Source) -> TestResult {
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let temporary = TestDirectory::new()?;
    let directory = temporary.path().join("journal");
    let config = AuditCollectorConfig {
        directory: directory.clone(),
        batch_size: 1,
        interval_ms: 10,
        ..Default::default()
    };
    let key_id = if matches!(source, Source::T94) {
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
        let expected = format!(
            "sha256:{}",
            crate::native_admission_source::world_event_outcomes::digest(
                &generated.signing_key.subject_public_key_info()
            )
        );
        let descriptor =
            crate::native_admission_source::world_event_outcomes::publisher::LeagueDescriptor::new(
                ("127.0.0.1".into(), 443),
                "localhost".into(),
                vec![generated.cert.der().clone()],
                vec![generated.cert.der().clone()],
                PrivateKeyDer::Pkcs8(generated.signing_key.serialize_der().into()),
                &[],
            )?;
        let actual = super::super::t94::audit_key_id(&descriptor);
        assert_eq!(actual, expected);
        Some(actual)
    } else {
        None
    };
    let collector = Collector {
        root: &h.root,
        authority: &authority,
        proof: &h.node,
        config: &config,
        t94_key_id: key_id.clone(),
    };
    let mut expected = Vec::new();
    for ordinal in 1..=2 {
        let event = format!("0192a000-0000-7000-8000-{ordinal:012}");
        let payload = vec![ordinal as u8, 0, 255];
        let envelope = EventEnvelopeV1 {
            envelope_revision: 1,
            event_id: {
                let mut id = [0; 16];
                id[6] = 0x70;
                id[8] = 0x80;
                id[15] = ordinal as u8;
                id[0] = 1;
                id[1] = 0x92;
                id[2] = 0xa0;
                id.to_vec()
            },
            event_type_id: 7,
            event_schema_revision: 1,
            payload_sha256: Sha256::digest(&payload).to_vec(),
            payload: payload.clone(),
            ..Default::default()
        }
        .encode_to_vec();
        match source {
            Source::Bazaar => {
                sqlx::query("UPDATE game_bazaar_configuration SET enabled=true WHERE singleton")
                    .execute(&h.pool)
                    .await?;
                let decision = super::super::dispatch_bazaar_intent(
                    &h.root,
                    &authority,
                    &h.node,
                    "oteryn:character-authority:primary",
                    "EscrowBazaarItem",
                    &event,
                    b"{}",
                )
                .await;
                assert!(matches!(decision, Decision::Publish { .. }));
                let saved: (String, Vec<u8>) = sqlx::query_as("SELECT a.event_id::text,a.envelope FROM game_bazaar_audit_outbox a JOIN game_bazaar_decisions d USING(event_id) WHERE d.operation_id=$1::uuid").bind(&event).fetch_one(&h.pool).await?;
                expected.push(saved);
            }
            Source::AccountClosure => {
                sqlx::query("INSERT INTO game_account_closure_audit_outbox(event_id,transaction_id,operation_id,envelope,payload,payload_sha256,occurred_at,expires_at) VALUES($1::uuid,uuidv7(),uuidv7(),$2,$3,sha256($3),1,7776000001)").bind(&event).bind(&envelope).bind(&payload).execute(&h.pool).await?;
                expected.push((event, envelope));
            }
            Source::T94 => {
                sqlx::query("UPDATE game_t94_control SET reconciled=true,recovery_generation=1,source_authority='oteryn:game:primary' WHERE singleton").execute(&h.pool).await?;
                sqlx::query("INSERT INTO game_t94_subjects(family,world_id,subject_id,projection_revision,record,record_sha256,final_state) VALUES('world_event_outcome',$1::uuid,$2::uuid,1,$3,sha256($3),false)").bind(super::super::super::uuid_text(&crate::bestiary_postgres_harness::id(42))).bind(&event).bind(b"{}".as_slice()).execute(&h.pool).await?;
                sqlx::query("INSERT INTO game_t94_publications(family,world_id,subject_id,projection_epoch,projection_revision,source_observed_at,wire,wire_sha256,audit_event_id,occurred_at_ms,audit) VALUES('world_event_outcome',$1::uuid,$2::uuid,1,1,1,$3,sha256($3),$2::uuid,floor(extract(epoch FROM clock_timestamp())*1000)::bigint,$4)").bind(super::super::super::uuid_text(&crate::bestiary_postgres_harness::id(42))).bind(&event).bind(b"{}".as_slice()).bind(&envelope).execute(&h.pool).await?;
                expected.push((event, envelope));
            }
        }
    }
    // Deterministic journal open failure, twice: the production iteration must retain both rows.
    std::fs::write(&directory, b"not a directory")?;
    for _ in 0..2 {
        assert!(collector.tick(source).await.is_err());
        assert_eq!(pending(h, source).await?, 2);
    }
    std::fs::remove_file(&directory)?;
    // One bounded tick writes and acknowledges exactly one row, leaving the other pending.
    if matches!(source, Source::Bazaar) {
        // Correcting the generated-column trigger must still reject envelope mutation.
        let mut transaction = h.pool.begin().await?;
        assert!(
            sqlx::query(
                "UPDATE game_bazaar_audit_outbox SET acknowledged=true,envelope='changed'::bytea"
            )
            .execute(&mut *transaction)
            .await
            .is_err()
        );
        transaction.rollback().await?;
    }
    collector.tick(source).await?;
    assert_eq!(pending(h, source).await?, 1);
    let stop = CancellationToken::new();
    let observe = async {
        loop {
            if pending(h, source).await? == 0 {
                stop.cancel();
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut observed = Ok(());
        both(collector.run(&stop), async {
            observed = observe.await;
        })
        .await;
        observed
    })
    .await??;
    let contents = std::fs::read_to_string(directory.join(format!("{}.jsonl", source.name())))?;
    let records = contents
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(records.len(), expected.len());
    for ((id, envelope), record) in expected.iter().zip(&records) {
        assert_eq!(record["source"], source.name());
        assert_eq!(record["event_id"], *id);
        assert_eq!(record["format_version"], 1);
        assert!(
            record["recorded_at"]
                .as_str()
                .ok_or("recorded_at")?
                .parse::<u64>()?
                > 0
        );
        assert_eq!(
            STANDARD.decode(record["envelope_base64"].as_str().ok_or("envelope")?)?,
            *envelope
        );
        if matches!(source, Source::T94) {
            assert_eq!(
                record["producer_key_id"],
                key_id.as_deref().ok_or("T94 fingerprint")?
            );
            assert_eq!(record["sequence"]["projection_epoch"], "1");
            assert_eq!(record["sequence"]["projection_revision"], "1");
            assert!(record["sequence"]["publication_id"].as_str().is_some());
        } else {
            assert!(record["producer_key_id"].is_null());
            assert!(record["sequence"].is_null());
        }
    }
    // A restarted collector sees no pending records and does not append new journal rows.
    collector.tick(source).await?;
    assert_eq!(
        std::fs::read_to_string(directory.join(format!("{}.jsonl", source.name())))?,
        contents
    );
    match source {
        Source::Bazaar => assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_bazaar_outbox WHERE receipt_acknowledged"
            )
            .fetch_one(&h.pool)
            .await?,
            0
        ),
        Source::T94 => assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_t94_publications WHERE acknowledged"
            )
            .fetch_one(&h.pool)
            .await?,
            0
        ),
        Source::AccountClosure => (),
    }
    Ok(())
}

async fn pending(h: &Harness, source: Source) -> TestResult<i64> {
    let query = match source {
        Source::Bazaar => "SELECT count(*) FROM game_bazaar_audit_outbox WHERE NOT acknowledged",
        Source::AccountClosure => {
            "SELECT count(*) FROM game_account_closure_audit_outbox WHERE publication_state=1"
        }
        Source::T94 => "SELECT count(*) FROM game_t94_publications WHERE NOT audit_acknowledged",
    };
    Ok(sqlx::query_scalar(query).fetch_one(&h.pool).await?)
}

#[test]
fn audit_collector_rotation_is_bounded_and_repairs_partial_tail() -> TestResult {
    let temporary = TestDirectory::new()?;
    let config = AuditCollectorConfig {
        directory: temporary.path().to_path_buf(),
        rotation_bytes: 8,
        rotation_count: 3,
        ..Default::default()
    };
    for line in [
        b"111\n", b"222\n", b"333\n", b"444\n", b"555\n", b"666\n", b"777\n",
    ] {
        append_synced(&config, "bazaar", line)?;
    }
    assert_eq!(
        std::fs::read(config.directory.join("bazaar.jsonl"))?,
        b"777\n"
    );
    assert_eq!(
        std::fs::read(config.directory.join("bazaar.1.jsonl"))?,
        b"555\n666\n"
    );
    assert_eq!(
        std::fs::read(config.directory.join("bazaar.2.jsonl"))?,
        b"333\n444\n"
    );
    assert_eq!(std::fs::read_dir(&config.directory)?.count(), 3);
    use std::io::Write;
    std::fs::OpenOptions::new()
        .append(true)
        .open(config.directory.join("bazaar.jsonl"))?
        .write_all(b"BAD")?;
    append_synced(&config, "bazaar", b"888\n")?;
    assert_eq!(
        std::fs::read(config.directory.join("bazaar.jsonl"))?,
        b"777\n888\n"
    );
    assert!(append_synced(&config, "bazaar", b"oversized\n").is_err());
    assert_eq!(
        std::fs::read(config.directory.join("bazaar.jsonl"))?,
        b"777\n888\n"
    );
    let single = AuditCollectorConfig {
        rotation_count: 1,
        ..config
    };
    append_synced(&single, "t94", b"111\n222\n")?;
    append_synced(&single, "t94", b"333\n")?;
    assert_eq!(std::fs::read(single.directory.join("t94.jsonl"))?, b"333\n");
    assert!(!single.directory.join("t94.1.jsonl").exists());
    append_synced(&single, "bazaar", b"999\n")?;
    assert!(!single.directory.join("bazaar.1.jsonl").exists());
    assert!(!single.directory.join("bazaar.2.jsonl").exists());
    use std::os::unix::fs::{PermissionsExt, symlink};
    assert_eq!(
        std::fs::metadata(single.directory.join("bazaar.jsonl"))?
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let sentinel = single.directory.join("sentinel");
    std::fs::write(&sentinel, b"unchanged")?;
    symlink(&sentinel, single.directory.join("account_closure.jsonl"))?;
    assert!(append_synced(&single, "account_closure", b"123\n").is_err());
    assert_eq!(std::fs::read(sentinel)?, b"unchanged");
    let directory = std::fs::File::open(&single.directory)?;
    rustix::fs::flock(
        &directory,
        rustix::fs::FlockOperation::NonBlockingLockExclusive,
    )?;
    assert!(append_synced(&single, "t94", b"444\n").is_err());
    drop(directory);
    append_synced(&single, "t94", b"444\n")?;
    Ok(())
}

struct TestDirectory(std::path::PathBuf);
impl TestDirectory {
    fn new() -> io::Result<Self> {
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            ".audit-test-{}",
            super::super::super::uuid_text(&super::super::uuid_v7()?)
        ));
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
        Ok(Self(directory))
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
