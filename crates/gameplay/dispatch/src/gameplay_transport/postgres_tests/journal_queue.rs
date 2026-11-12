//! Queue ownership, rescan cadence and the two private erasure lifecycles.
use super::{
    TestResult,
    fixture::{Harness, debug, id, run},
};
use crate::durability::{
    character_authority::ReconciledCharacterAuthority, journal::ProducerStore,
};
use crate::native_admission_source::{
    descriptor::Operation,
    journal::{
        ItemsBegin, ItemsEnd, ItemsFamily, ItemsPayload, PublishGameJournalRecordV1,
        ReadGameJournalCursorV1, ReadGameJournalCursorV1Response,
        publisher::{Failure, JournalExportConfig, JournalPeer, JournalRateGate, Publisher},
    },
};
use sqlx::Row;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
const SOURCE: &str = "OTERYN_GAME_JOURNAL";
fn case(mode: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::journal_watchdog::run("journal-queue", move |stage| {
        run(async move |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                if mode {
                    "journal_queue"
                } else {
                    "journal_privacy"
                },
                true,
                false,
            )
            .await?;
            super::prepare_current_session(&h).await?;
            let streams = seed(&h, if mode { 31 } else { 1 }).await?;
            super::journal_inventory::runtime_role(&h.root).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let store = ProducerStore {
                root: &h.root,
                authority: &authority,
            };
            if mode {
                stage.set("31-stream rescan and concurrent claims");
                queue(&h, &authority, &store, &streams).await?;
            } else {
                stage.set("expiry and account closure erasure");
                privacy(&h, &authority, &store, &streams[0]).await?;
            }
            drop(authority);
            drop(seal);
            stage.set("cleanup");
            h.cleanup().await?;
            Ok(())
        })
    })
}
#[test]
fn game_journal_queue_claims_rescan_and_persistent_quarantine() -> TestResult {
    case(true)
}
#[test]
fn game_journal_account_closure_and_ordinary_expiry_have_distinct_fences() -> TestResult {
    case(false)
}
async fn seed(h: &Harness, count: u8) -> TestResult<Vec<ReadGameJournalCursorV1>> {
    let row=sqlx::query("SELECT r.account_id::text,r.world_id::text,r.character_revision::text,p.projection_revision::text,e.projection_epoch::text,(SELECT recovery_generation::text FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1) AS generation FROM game_character_roots r JOIN game_character_account_projections p ON p.account_id=r.account_id CROSS JOIN game_character_account_projection_epoch e WHERE r.character_id=encode($1,'hex')::uuid AND e.epoch_scope=1").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let characters = characters(h, count).await?;
    let mut streams = Vec::new();
    for (n, character) in (0..count).zip(characters) {
        let stream = ReadGameJournalCursorV1 {
            contract_version: 1,
            operation: "ReadGameJournalCursorV1".into(),
            source_authority: SOURCE.into(),
            account_id: row.try_get("account_id")?,
            world_id: row.try_get("world_id")?,
            character_id: Some(character),
            character_revision: Some(row.try_get("character_revision")?),
            from_projection_revision: Some(row.try_get("projection_revision")?),
            to_projection_revision: Some(row.try_get("projection_revision")?),
            projection_epoch: Some(row.try_get("projection_epoch")?),
            family: ItemsFamily::Items,
            recovery_generation: row.try_get("generation")?,
        };
        let key = stream.encode()?;
        sqlx::query("INSERT INTO game_journal_streams(stream_key,source_authority,account_id,recovery_generation) VALUES($1,$2,$3::text::uuid,$4::text::numeric)").bind(&key).bind(SOURCE).bind(&stream.account_id).bind(&stream.recovery_generation).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_journal_items_baselines(stream_key,approval_ref) VALUES($1,'test:accepted-queue-cut')").bind(&key).execute(&h.pool).await?;
        append(h, &stream, 1, 112 + n, 143 + n, 174 + n, true).await?;
        streams.push(stream);
    }
    sqlx::query("UPDATE game_journal_export_settings SET game_journal_items_export_enabled=true,source_authority=$1 WHERE singleton").bind(SOURCE).execute(&h.pool).await?;
    Ok(streams)
}
// Every queued Character has real birth evidence, so a restarted node can
// reconcile the full Character Authority before resuming these streams.
async fn characters(h: &Harness, count: u8) -> TestResult<Vec<String>> {
    let mut characters = vec![crate::gameplay_transport::canonical_uuid(&id(41))];
    if count == 1 {
        return Ok(characters);
    }
    use crate::durability::native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
        SourceObservation,
    };
    let provenance = FreshStoreProvenance {
        namespace: "journal-bootstrap".into(),
        authorization: "test-owner".into(),
        source_authority: "platform".into(),
        initialized_at: 10,
    };
    let descriptor = DescriptorRegistration {
        revision: 1,
        facts: vec![1],
        installed_at: 10,
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
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let account = crate::gameplay_transport::canonical_uuid(&id(40));
    for n in 1..count {
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let revision = u64::from(n) + 1;
        let facts = serde_json::json!({"version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform","source_revision":revision.to_string(),"decision_identity":revision.to_string(),"source_observed_at":now.to_string(),"clock_uncertainty_seconds":"0","account_id":account,"purpose":"platform_security","scope":"fresh_admission","allowed":true,"minimum_valid_generation":"1"});
        h.root
            .accept_native_source_observation(
                &h.node,
                SourceObservation {
                    source_authority: "platform".into(),
                    operation: NativeSourceOperation::ReadAccountSecurityV1,
                    subject: NativeSourceSubject::account_security(account.clone())
                        .map_err(debug)?,
                    source_revision: revision,
                    decision_identity: revision.to_string(),
                    observed_at: now,
                    semantic_facts: serde_json::to_vec(&facts)?,
                },
            )
            .await
            .map_err(debug)?;
        let operation = id(81 + n);
        let wire = serde_json::json!({"contract_version":3,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":crate::gameplay_transport::canonical_uuid(&id(210+n)),"source_revision":revision.to_string(),"operation_id":crate::gameplay_transport::canonical_uuid(&operation),"operation":"INITIAL_CHARACTER_BOOTSTRAP","account_id":account,"target_world_id":crate::gameplay_transport::canonical_uuid(&id(42)),"requested_sex":"Male","requested_name":format!("Journal {}{}",char::from(b'A'+n/26),char::from(b'a'+n%26)),"interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},"issued_at_source":now.to_string(),"expires_at_source":(now+120).to_string(),"audience":"OTERYN_GAME_CHARACTER_AUTHORITY"});
        let intent = crate::character_bootstrap_intent::decode_producer_response(
            &serde_json::to_vec(&wire)?,
            operation,
        )
        .map_err(debug)?;
        let created = h
            .root
            .bootstrap_character(&authority, &h.node, &intent)
            .await
            .map_err(debug)?;
        assert_eq!(created.revision.get(), 1);
        characters.push(crate::gameplay_transport::canonical_uuid(
            created.character_id.as_bytes(),
        ));
    }
    Ok(characters)
}
async fn append(
    h: &Harness,
    s: &ReadGameJournalCursorV1,
    start: u64,
    begin: u8,
    end: u8,
    snapshot: u8,
    acked: bool,
) -> TestResult {
    let now: String =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint::text")
            .fetch_one(&h.pool)
            .await?;
    let mut record = PublishGameJournalRecordV1::decode(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../native-admission-source/src/journal/publisher_fixture.json"
    )))?;
    record.account_id = s.account_id.clone();
    record.world_id = s.world_id.clone();
    record.character_id = s.character_id.clone();
    record.character_revision = s.character_revision.clone();
    record.from_projection_revision = s.from_projection_revision.clone();
    record.to_projection_revision = s.to_projection_revision.clone();
    record.projection_epoch = s.projection_epoch.clone();
    record.family = ItemsFamily::Items;
    record.recovery_generation = s.recovery_generation.clone();
    record.occurred_at = now.clone();
    record.coverage_start_at = now;
    for (offset, kind, identity) in [(0, "ITEMS_BEGIN", begin), (1, "ITEMS_END", end)] {
        record.source_revision = (start + offset).to_string();
        record.kind = kind.into();
        record.record_id = crate::gameplay_transport::canonical_uuid(&id(identity));
        let snapshot = crate::gameplay_transport::canonical_uuid(&id(snapshot));
        record.payload = if offset == 0 {
            ItemsPayload::Begin(ItemsBegin {
                snapshot_id: snapshot,
                character_revision: s.character_revision.clone().ok_or("cut")?,
                page_count: 0,
                item_count: 0,
                depots: vec![],
                houses: vec![],
            })
        } else {
            ItemsPayload::End(ItemsEnd {
                snapshot_id: snapshot,
                page_count: 0,
                item_count: 0,
                pages_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .into(),
            })
        };
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT game_journal_enqueue($1,$2)")
            .bind(s.encode()?)
            .bind(record.encode()?)
            .execute(&mut *tx)
            .await?;
        if acked {
            sqlx::query("SELECT game_journal_ack($1,$2::text::numeric,$3::text::uuid)")
                .bind(s.encode()?)
                .bind(&record.source_revision)
                .bind(&record.record_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
    }
    Ok(())
}
struct Remote {
    seen: Vec<String>,
    ends: BTreeMap<String, String>,
}
struct Peer {
    state: Arc<Mutex<Remote>>,
    gate: JournalRateGate,
    inflight: Arc<AtomicUsize>,
    denied: bool,
}
impl JournalPeer for Peer {
    async fn exchange(&mut self, op: Operation, bytes: &[u8]) -> Result<Vec<u8>, Failure> {
        assert_eq!(
            self.inflight.fetch_add(1, Ordering::SeqCst),
            0,
            "one exchange per consumer"
        );
        self.gate.wait().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        self.inflight.fetch_sub(1, Ordering::SeqCst);
        if self.denied {
            return Err(Failure::Quarantine);
        }
        assert_eq!(op, Operation::ReadGameJournalCursorV1);
        let s = ReadGameJournalCursorV1::decode(bytes).map_err(|_| Failure::Quarantine)?;
        let character = s.character_id.ok_or(Failure::Quarantine)?;
        let mut state = self.state.lock().map_err(|_| Failure::Quarantine)?;
        let last = state
            .ends
            .get(&character)
            .cloned()
            .ok_or(Failure::Quarantine)?;
        state.seen.push(character);
        ReadGameJournalCursorV1Response {
            contract_version: 1,
            after_source_revision: "2".into(),
            last_record_id: Some(last),
        }
        .encode()
        .map_err(|_| Failure::Quarantine)
    }
}
type PeerPool = (Vec<Publisher<Peer>>, Arc<Mutex<Remote>>);
fn pool(streams: &[ReadGameJournalCursorV1], denied: bool) -> TestResult<PeerPool> {
    let ends = streams
        .iter()
        .enumerate()
        .map(|(n, s)| -> TestResult<_> {
            Ok((
                s.character_id.clone().ok_or("character")?,
                crate::gameplay_transport::canonical_uuid(&id(143 + u8::try_from(n)?)),
            ))
        })
        .collect::<TestResult<BTreeMap<_, _>>>()?;
    let state = Arc::new(Mutex::new(Remote { seen: vec![], ends }));
    let gate = JournalRateGate::default();
    let publishers = (0..4)
        .map(|_| {
            Publisher::new(
                JournalExportConfig {
                    game_journal_items_export_enabled: true,
                    ..Default::default()
                },
                Peer {
                    state: state.clone(),
                    gate: gate.clone(),
                    inflight: Arc::new(AtomicUsize::new(0)),
                    denied,
                },
                SOURCE.into(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((publishers, state))
}
async fn due_now(h: &Harness, s: &ReadGameJournalCursorV1) -> TestResult {
    sqlx::query(
        "UPDATE game_journal_streams SET next_delivery_at=clock_timestamp() WHERE stream_key=$1",
    )
    .bind(s.encode()?)
    .execute(&h.pool)
    .await?;
    Ok(())
}
async fn queue(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    store: &ProducerStore<'_, '_, '_>,
    streams: &[ReadGameJournalCursorV1],
) -> TestResult {
    let (mut publishers, remote) = pool(streams, false)?;
    let start = Instant::now();
    assert_eq!(
        Box::pin(store.deliver_due(SOURCE, &mut publishers)).await?,
        31
    );
    assert!(
        start.elapsed() < Duration::from_secs(30),
        "31-stream rescan must fit within 30 seconds"
    );
    {
        let seen = remote.lock().map_err(|_| "remote")?;
        assert_eq!(seen.seen.len(), 31);
        assert_eq!(
            seen.seen
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            31
        );
    }
    let bounded:bool=sqlx::query_scalar("SELECT bool_and(next_delivery_at<=last_progress_at+interval '28 seconds') FROM game_journal_streams").fetch_one(&h.pool).await?;
    assert!(bounded);
    sqlx::query(
        "UPDATE game_journal_streams SET next_delivery_at=clock_timestamp()+interval '1 hour'",
    )
    .execute(&h.pool)
    .await?;
    let other = h.independent_root()?;
    assert!(other.maintain_ready_once().await?);
    super::journal_inventory::runtime_role(&other).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let second = other.open_character_authority(&seal).await.map_err(debug)?;
    let second = ProducerStore {
        root: &other,
        authority: &second,
    };
    let s = &streams[0];
    due_now(h, s).await?;
    let first = store
        .claim_due(SOURCE, 1)
        .await?
        .pop()
        .ok_or("first claim")?;
    assert_eq!(&first.stream, s);
    assert!(
        second.claim_due(SOURCE, 1).await?.is_empty(),
        "another node cannot take a live claim"
    );
    sqlx::query("UPDATE game_journal_streams SET claim_until=clock_timestamp()-interval '1 second' WHERE stream_key=$1").bind(s.encode()?).execute(&h.pool).await?;
    let successor = second
        .claim_due(SOURCE, 1)
        .await?
        .pop()
        .ok_or("successor claim")?;
    assert_ne!(first.token, successor.token);
    assert!(!store.claim_current(&first).await?);
    assert!(
        second
            .finish(&successor, 2, Duration::from_secs(999), false, false)
            .await?
    );
    let saved: String = sqlx::query_scalar(
        "SELECT next_delivery_at::text FROM game_journal_streams WHERE stream_key=$1",
    )
    .bind(s.encode()?)
    .fetch_one(&h.pool)
    .await?;
    assert!(
        !store.finish(&first, 0, Duration::ZERO, true, false).await?,
        "stale completion must be fenced"
    );
    let after: String = sqlx::query_scalar(
        "SELECT next_delivery_at::text FROM game_journal_streams WHERE stream_key=$1",
    )
    .bind(s.encode()?)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(saved, after);
    assert!(store.claim_due(SOURCE, 1).await?.is_empty());
    due_now(h, s).await?;
    let (mut denied, _) = pool(streams, true)?;
    assert_eq!(Box::pin(store.deliver_due(SOURCE, &mut denied)).await?, 1);
    assert!(
        store.claim_due(SOURCE, 1).await?.is_empty(),
        "quarantine must survive repeated worker scans"
    );
    let mut repair = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_control")
        .execute(&mut *repair)
        .await?;
    sqlx::query("SELECT game_journal_repair($1)")
        .bind(s.encode()?)
        .execute(&mut *repair)
        .await?;
    repair.commit().await?;
    let repaired = store
        .claim_due(SOURCE, 1)
        .await?
        .pop()
        .ok_or("explicit repaired claim")?;
    sqlx::query("UPDATE game_journal_streams SET next_delivery_at=clock_timestamp()+interval '999 seconds' WHERE stream_key=$1").bind(s.encode()?).execute(&h.pool).await?;
    let protected: String = sqlx::query_scalar(
        "SELECT next_delivery_at::text FROM game_journal_streams WHERE stream_key=$1",
    )
    .bind(s.encode()?)
    .fetch_one(&h.pool)
    .await?;
    assert!(
        store
            .finish(&repaired, 0, Duration::ZERO, true, false)
            .await?
    );
    let unchanged: String = sqlx::query_scalar(
        "SELECT next_delivery_at::text FROM game_journal_streams WHERE stream_key=$1",
    )
    .bind(s.encode()?)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        protected, unchanged,
        "even a current completion cannot shorten an existing later deadline"
    );
    let _ = authority;
    Ok(())
}
async fn privacy(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    store: &ProducerStore<'_, '_, '_>,
    s: &ReadGameJournalCursorV1,
) -> TestResult {
    let key = s.encode()?;
    append(h, s, 3, 220, 221, 222, false).await?;
    sqlx::query("UPDATE game_journal_outbox SET retain_until=clock_timestamp()-interval '1 second' WHERE stream_key=$1 AND source_revision<=2").bind(&key).execute(&h.pool).await?;
    store.maintenance().await?;
    store.maintenance().await?;
    let state:(bool,String,i64)=sqlx::query_as("SELECT active,retained_floor::text,(SELECT count(export_bytes) FROM game_journal_outbox WHERE stream_key=$1) FROM game_journal_streams WHERE stream_key=$1").bind(&key).fetch_one(&h.pool).await?;
    assert_eq!(
        state,
        (true, "2".into(), 2),
        "expiry may not erase newer valid payloads or close the stream"
    );
    let ordinary:bool=sqlx::query_scalar("SELECT erasure_reason='EXPIRY' AND tombstone_until=erased_at+interval '31 days' FROM game_journal_outbox WHERE stream_key=$1 AND source_revision=2").bind(&key).fetch_one(&h.pool).await?;
    assert!(ordinary);
    sqlx::query("UPDATE game_journal_outbox SET tombstone_until=clock_timestamp()-interval '1 second' WHERE stream_key=$1 AND source_revision=1").bind(&key).execute(&h.pool).await?;
    store.maintenance().await?;
    let removed:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_journal_outbox WHERE stream_key=$1 AND source_revision=1)").bind(&key).fetch_one(&h.pool).await?;
    assert!(removed, "ordinary tombstone expires on its 31-day deadline");
    append(h, s, 5, 223, 224, 225, false).await?;
    store.maintenance().await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(export_bytes) FROM game_journal_outbox WHERE stream_key=$1",
    )
    .bind(&key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 4, "new exports remain usable after expiry");
    let index:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_indexes WHERE indexname='game_journal_tombstone_expiry' AND indexdef LIKE '%tombstone_until%')").fetch_one(&h.pool).await?;
    assert!(index);
    close(h, authority, &s.account_id).await?;
    let fenced:bool=sqlx::query_scalar("SELECT NOT active AND privacy_fenced_at IS NOT NULL FROM game_journal_streams WHERE stream_key=$1").bind(&key).fetch_one(&h.pool).await?;
    assert!(fenced);
    use crate::native_admission_source::journal::publisher::JournalOutbox;
    assert!(!store.current(s).await?);
    store.maintenance().await?;
    let erased:(i64,bool)=sqlx::query_as("SELECT count(export_bytes),bool_and(erasure_reason='DELETION' AND tombstone_until>=erased_at+interval '119 days') FROM game_journal_outbox WHERE stream_key=$1").bind(&key).fetch_one(&h.pool).await?;
    assert_eq!(erased, (0, true));
    // Distinct ordinary/deletion GC schedules, with a permanent stream floor.
    sqlx::query("UPDATE game_journal_outbox SET tombstone_until=clock_timestamp()-interval '1 second' WHERE stream_key=$1").bind(&key).execute(&h.pool).await?;
    store.maintenance().await?;
    let retained: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_journal_outbox WHERE stream_key=$1")
            .bind(&key)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        retained, 5,
        "deletion fence extends an earlier ordinary tombstone"
    );
    sqlx::query("UPDATE game_journal_streams SET privacy_fenced_at=clock_timestamp()-interval '121 days' WHERE stream_key=$1").bind(&key).execute(&h.pool).await?;
    sqlx::query("UPDATE game_journal_outbox SET erased_at=clock_timestamp()-interval '121 days',tombstone_until=clock_timestamp()-interval '1 second' WHERE stream_key=$1").bind(&key).execute(&h.pool).await?;
    store.maintenance().await?;
    let remaining: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_journal_outbox WHERE stream_key=$1")
            .bind(&key)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(remaining, 0);
    let floor: String = sqlx::query_scalar(
        "SELECT retained_floor::text FROM game_journal_streams WHERE stream_key=$1",
    )
    .bind(&key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(floor, "6");
    Ok(())
}

async fn close(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    account: &str,
) -> TestResult {
    use crate::durability::character_command::account_deletion::{Config, Intent};
    use crate::durability::native_admission_source::{
        DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
        SourceObservation,
    };
    use crate::native_admission_source::{
        account_closure::{LifecycleObservation, LifecycleRequest},
        character_commands::{Decision, Input},
    };
    // Control evidence is prepared through its real API, then the Game runtime
    // processes CloseAccount. No root lifecycle value is fabricated.
    {
        let mut c = h.root.try_acquire_ready()?;
        sqlx::query("SET ROLE NONE").execute(&mut *c).await?;
        let returned = c
            .return_to_pool_observed_until(Instant::now() + Duration::from_secs(5))
            .await;
        assert!(matches!(
            returned,
            sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
        ));
    }
    let now: i64 = sqlx::query_scalar("SELECT game_account_closure_now()")
        .fetch_one(&h.pool)
        .await?;
    let provenance = FreshStoreProvenance {
        namespace: "closure-dispatch".into(),
        authorization: "test-owner".into(),
        source_authority: "platform".into(),
        initialized_at: 10,
    };
    let descriptor = DescriptorRegistration {
        revision: 1,
        facts: vec![1],
        installed_at: 10,
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
    let facts = format!(
        r#"{{"version":1,"operation":"ReadAccountSecurityV1","result":"observed","source_authority":"platform","source_revision":"1","decision_identity":"1","source_observed_at":"{now}","clock_uncertainty_seconds":"0","account_id":"{account}","purpose":"platform_security","scope":"fresh_admission","allowed":false,"minimum_valid_generation":"2"}}"#
    );
    h.root
        .accept_native_source_observation(
            &h.node,
            SourceObservation {
                source_authority: "platform".into(),
                operation: NativeSourceOperation::ReadAccountSecurityV1,
                subject: NativeSourceSubject::account_security(account).map_err(debug)?,
                source_revision: 1,
                decision_identity: "1".into(),
                observed_at: now,
                semantic_facts: facts.into_bytes(),
            },
        )
        .await
        .map_err(debug)?;
    let command = Intent {
        contract_version: 2,
        command: "CloseAccount".into(),
        variant: "PLATFORM_ACCOUNT_CLOSE".into(),
        issuer_authority: "OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),
        issuer_decision_id: crate::gameplay_transport::canonical_uuid(&id(180)),
        source_revision: "1".into(),
        operation_id: crate::gameplay_transport::canonical_uuid(&id(181)),
        account_id: account.to_owned(),
        termination_id: crate::gameplay_transport::canonical_uuid(&id(182)),
        lifecycle_generation: "8".into(),
        expected_recovery_generation: "1".into(),
        close_operation_id: None,
        decision: None,
        scheduled_for: (now - 1).to_string(),
        issued_at_source: (now - 1).to_string(),
        expires_at_source: (now + 120).to_string(),
        audience: "OTERYN_GAME_CHARACTER_AUTHORITY".into(),
    };
    let raw = serde_json::to_vec(&command)?;
    let config = Config::default();
    assert!(config.enabled);
    let request = LifecycleRequest::from_intent("oteryn:character-authority:primary", &raw, 1)?;
    let observation = LifecycleObservation {
        contract_version: 2,
        operation: "ReadAccountClosureLifecycleV1".into(),
        source_authority: "OTERYN_PLATFORM_CHARACTER_AUTHORITY".into(),
        account_id: account.to_owned(),
        termination_id: command.termination_id.clone(),
        lifecycle_generation: "8".into(),
        source_revision: "1".into(),
        phase: "CLOSE".into(),
        financial_obligations_clear: true,
        recovery_generation: "1".into(),
        observed_at: now.to_string(),
        valid_until: (now + 5).to_string(),
    };
    h.root
        .observe_account_closure_lifecycle(
            authority,
            &h.node,
            &request,
            &serde_json::to_vec(&observation)?,
        )
        .await
        .map_err(debug)?;

    super::journal_inventory::runtime_role(&h.root).await?;
    let result = h
        .root
        .decide_character_command_configured(
            authority,
            &h.node,
            "oteryn:character-authority:primary",
            (&command.command, &command.operation_id),
            Input::Intent(&raw),
            &config,
        )
        .await;
    assert!(
        matches!(result, Decision::Transient | Decision::Publish { .. }),
        "CloseAccount must enter its real lifecycle"
    );
    let closed: bool = sqlx::query_scalar("SELECT game_account_closure_closed($1::text::uuid)")
        .bind(account)
        .fetch_one(&h.pool)
        .await?;
    assert!(closed);
    let unchanged:bool=sqlx::query_scalar("SELECT bool_and(lifecycle IN(1,2)) FROM game_character_roots WHERE account_id=$1::text::uuid").bind(account).fetch_one(&h.pool).await?;
    assert!(unchanged);
    Ok(())
}
