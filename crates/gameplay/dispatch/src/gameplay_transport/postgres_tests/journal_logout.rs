//! The production terminal-release API captures the source cut and outbox atomically.
use super::fixture::{Harness, run};
use super::{TestResult, prepare_current_session};
use crate::durability::{
    fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore},
    journal::{LogoutCatalogue, ProducerStore},
};
use crate::native_admission_source::{
    descriptor::Operation,
    journal::{
        ItemsFamily, PublishGameJournalRecordV1, PublishGameJournalRecordV1Response,
        ReadGameJournalCursorV1, ReadGameJournalCursorV1Response,
        publisher::{Failure, JournalExportConfig, JournalPeer, Publisher, Step},
    },
};
use sqlx::Row;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
#[derive(Default)]
struct PeerState {
    accepted: BTreeMap<u64, (String, Vec<u8>)>,
    sent: Vec<Vec<u8>>,
    calls: usize,
    drop_ack: bool,
    unregistered: bool,
}
struct Peer(Arc<Mutex<PeerState>>);
impl JournalPeer for Peer {
    async fn exchange(&mut self, op: Operation, bytes: &[u8]) -> Result<Vec<u8>, Failure> {
        let mut state = self.0.lock().map_err(|_| Failure::Quarantine)?;
        state.calls += 1;
        match op {
            Operation::ReadGameJournalCursorV1 => {
                if state.unregistered {
                    return Err(Failure::Conflict);
                }
                let after = state.accepted.keys().next_back().copied().unwrap_or(0);
                ReadGameJournalCursorV1Response {
                    contract_version: 1,
                    after_source_revision: after.to_string(),
                    last_record_id: state.accepted.get(&after).map(|(id, _)| id.clone()),
                }
                .encode()
                .map_err(|_| Failure::Quarantine)
            }
            Operation::PublishGameJournalRecordV1 => {
                let r =
                    PublishGameJournalRecordV1::decode(bytes).map_err(|_| Failure::Quarantine)?;
                let revision = r
                    .source_revision
                    .parse::<u64>()
                    .map_err(|_| Failure::Quarantine)?;
                state.sent.push(bytes.to_vec());
                if let Some((_, old)) = state.accepted.get(&revision) {
                    if old != bytes {
                        return Err(Failure::Conflict);
                    }
                } else {
                    if revision != state.accepted.keys().next_back().copied().unwrap_or(0) + 1 {
                        return Err(Failure::Conflict);
                    }
                    state
                        .accepted
                        .insert(revision, (r.record_id, bytes.to_vec()));
                }
                state.unregistered = false;
                if state.drop_ack {
                    state.drop_ack = false;
                    return Err(Failure::Retry(None));
                }
                Ok(PublishGameJournalRecordV1Response::BYTES.to_vec())
            }
            _ => Err(Failure::Quarantine),
        }
    }
}
fn case(enabled: bool, approved: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(enabled, approved).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "journal logout PG thread failed")?
        .map_err(Into::into)
}
#[test]
fn game_journal_logout_captures_durable_rows_and_publishes_replay() -> TestResult {
    case(true, true)
}
#[test]
fn game_journal_logout_flag_off_has_no_outbox_or_publication() -> TestResult {
    case(false, true)
}
#[test]
fn game_journal_logout_unapproved_cut_never_fabricates_empty_snapshot() -> TestResult {
    case(true, false)
}
fn scenario(enabled: bool, approved: bool) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(
            admin,
            if !approved {
                "journal_logout_unapproved"
            } else if enabled {
                "journal_logout_on"
            } else {
                "journal_logout_off"
            },
            true,
            false,
        )
        .await?;
        prepare_current_session(&h).await?;
        {
            let seal = h.recovery.seal_current().map_err(super::debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(super::debug)?;
            let store = FreshAdmissionStore::from_root(h.root.clone());
            let session = crate::foundation::GameSessionId::decode(&super::fixture::id(50))
                .map_err(super::debug)?;
            let current = store.current_session_at(session).await?.0;
            let transport = current.current_transport().ok_or("active transport")?;
            let world = current.current_runtime_scope().world_id();
            // The harness's genesis ownership is this test's accepted initial baseline.
            // Cut members are independently read from Game, not copied from export bytes.
            let facts=sqlx::query("SELECT r.character_id::text,r.account_id::text,r.world_id::text,r.character_revision::text,e.projection_epoch::text,p.projection_revision::text,(SELECT recovery_generation::text FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1) AS generation FROM game_character_roots r JOIN game_character_account_projections p ON p.account_id=r.account_id CROSS JOIN game_character_account_projection_epoch e WHERE r.character_id=encode($1,'hex')::uuid AND e.epoch_scope=1")
   .bind(current.current_character_lease().character_id().as_bytes().as_slice()).fetch_one(&h.pool).await?;
            let projection: String = facts.try_get("projection_revision")?;
            let stream = ReadGameJournalCursorV1 {
                contract_version: 1,
                operation: "ReadGameJournalCursorV1".into(),
                source_authority: "OTERYN_GAME_JOURNAL".into(),
                account_id: facts.try_get("account_id")?,
                world_id: facts.try_get("world_id")?,
                character_id: Some(facts.try_get("character_id")?),
                character_revision: Some(facts.try_get("character_revision")?),
                from_projection_revision: Some(projection.clone()),
                to_projection_revision: Some(projection),
                projection_epoch: Some(facts.try_get("projection_epoch")?),
                family: ItemsFamily::Items,
                recovery_generation: facts.try_get("generation")?,
            };
            let key = stream.encode()?;
            sqlx::query("INSERT INTO game_journal_streams(stream_key,source_authority,account_id,recovery_generation) VALUES($1,$2,$3::text::uuid,$4::text::numeric)")
   .bind(&key).bind(&stream.source_authority).bind(&stream.account_id).bind(&stream.recovery_generation).execute(&h.pool).await?;
            if approved {
                sqlx::query("INSERT INTO game_journal_items_baselines(stream_key,approval_ref) VALUES($1,'harness:genesis-owner-policy')").bind(&key).execute(&h.pool).await?;
            }
            sqlx::query("UPDATE game_journal_export_settings SET game_journal_items_export_enabled=$1,source_authority=$2 WHERE singleton").bind(enabled).bind(&stream.source_authority).execute(&h.pool).await?;
            // An ordinary gameplay save advances CharacterRevision, not the
            // accepted ownership cut. Snapshot ordering remains source_revision.
            use crate::durability::monk_state::{
                DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome,
                MonkStateSaveRequest,
            };
            let before = stream
                .character_revision
                .as_deref()
                .ok_or("ownership revision")?
                .parse::<u64>()?;
            let dependencies = super::Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            let gameplay = owner
                .current_monk_fence(session)
                .await
                .map_err(|_| "current gameplay fence")?
                .ok_or("active gameplay fence")?;
            assert_eq!(
                gameplay.character_lease_generation,
                current.current_character_lease().generation()
            );
            let save = h
                .root
                .commit_character_monk_state_save(
                    &authority,
                    &h.node,
                    gameplay,
                    MonkStateSaveRequest {
                        occurrence: MonkStateSaveOccurrence::from_bytes(super::fixture::id(180))
                            .map_err(super::debug)?,
                        state: DurableMonkState::new(3, 0).map_err(super::debug)?,
                    },
                )
                .await
                .map_err(|error| format!("journal ordinary save: {error:?}"))?;
            assert!(matches!(save, MonkStateSaveOutcome::Committed(_)));
            // This fixture World has no authored depot placements or houses. Production
            // supplies the complete list from the admitted BundleFacts at release_attempt.
            let releasing = store.with_journal_catalogue(LogoutCatalogue::new(world, Vec::new())?);
            assert!(matches!(
                releasing
                    .release_logout_session(session, &stream.account_id, transport)
                    .await?,
                ExpiredLossReleaseV1::Released { .. }
            ));
            let records:Vec<Vec<u8>>=sqlx::query_scalar("SELECT export_bytes FROM game_journal_outbox WHERE stream_key=$1 ORDER BY source_revision LIMIT 16386").bind(&key).fetch_all(&h.pool).await?;
            if enabled && approved {
                assert_eq!(records.len(), 2);
                assert_eq!(
                    PublishGameJournalRecordV1::decode(&records[0])?.kind,
                    "ITEMS_BEGIN"
                );
                assert_eq!(
                    PublishGameJournalRecordV1::decode(&records[1])?.kind,
                    "ITEMS_END"
                );
                assert!(matches!(
                    releasing
                        .release_logout_session(session, &stream.account_id, transport)
                        .await?,
                    ExpiredLossReleaseV1::Terminal
                ));
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_journal_logout_cuts WHERE game_session_id=encode($1,'hex')::uuid").bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(count, 1);
                let captured:String=sqlx::query_scalar("SELECT character_revision_at_logout::text FROM game_journal_logout_cuts WHERE game_session_id=encode($1,'hex')::uuid").bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(captured.parse::<u64>()?, before + 1);
            } else {
                assert!(records.is_empty());
            }
            let state = Arc::new(Mutex::new(PeerState {
                drop_ack: true,
                unregistered: true,
                ..Default::default()
            }));
            // Client enabled + durable flag off must still make zero peer calls.
            let mut publisher = Publisher::new(
                JournalExportConfig {
                    game_journal_items_export_enabled: true,
                    ..Default::default()
                },
                Peer(state.clone()),
                stream.source_authority.clone(),
            )?;
            let outbox = ProducerStore {
                root: &h.root,
                authority: &authority,
            };
            if enabled && approved {
                publish_to_idle(&mut publisher, &stream, &outbox, &h.root).await?;
                let acked:i64=sqlx::query_scalar("SELECT count(*) FROM game_journal_outbox WHERE stream_key=$1 AND acknowledged_at IS NOT NULL").bind(&key).fetch_one(&h.pool).await?;
                assert_eq!(acked, 2);
                state.lock().map_err(|_| "peer state")?.accepted.clear();
                publish_to_idle(&mut publisher, &stream, &outbox, &h.root).await?;
                let sent = state.lock().map_err(|_| "peer state")?;
                assert_eq!(sent.sent.len(), 4);
                assert_eq!(sent.sent[0], sent.sent[2]);
                assert_eq!(sent.sent[1], sent.sent[3]);
            } else {
                assert_eq!(
                    Box::pin(publisher.step(&stream, &outbox)).await,
                    Step::Quarantined
                );
                assert_eq!(state.lock().map_err(|_| "peer state")?.calls, 0);
            }
        }
        h.cleanup().await?;
        Ok(())
    })
}
async fn publish_to_idle(
    p: &mut Publisher<Peer>,
    s: &ReadGameJournalCursorV1,
    b: &ProducerStore<'_, '_, '_>,
    root: &crate::durability::DurabilityRoot,
) -> TestResult {
    for _ in 0..32 {
        match Box::pin(p.step(s, b)).await {
            Step::Idle => return Ok(()),
            Step::Accepted | Step::RetryAfter(_) => {
                if root.has_ready_demand() {
                    root.maintain_ready_once().await?;
                }
                tokio::task::yield_now().await;
            }
            _ => return Err("journal publication refused".into()),
        }
    }
    Err("journal publication did not settle".into())
}
