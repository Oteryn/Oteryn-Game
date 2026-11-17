//! The production terminal-release API captures the source cut and outbox atomically.
use super::fixture::{Harness, run};
use super::{TestResult, prepare_current_session};
use crate::durability::{
    fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore},
    journal::ProducerStore,
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
#[derive(Clone, Copy, PartialEq, Eq)]
enum Change {
    None,
    Epoch,
    OwnershipCut,
    Receipt,
    AfterCursor,
}
#[test]
fn game_journal_logout_changed_epoch_has_no_capture_or_delivery() -> TestResult {
    run_case(true, true, Change::Epoch)
}
#[test]
fn game_journal_logout_obsolete_cut_has_no_capture_or_delivery() -> TestResult {
    run_case(true, true, Change::OwnershipCut)
}
#[test]
fn game_journal_logout_mismatched_receipt_has_no_capture_or_delivery() -> TestResult {
    run_case(true, true, Change::Receipt)
}
#[test]
fn game_journal_logout_authority_lost_after_cursor_selection() -> TestResult {
    run_case(true, true, Change::AfterCursor)
}
fn case(enabled: bool, approved: bool) -> TestResult {
    run_case(enabled, approved, Change::None)
}
fn run_case(enabled: bool, approved: bool, change: Change) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::journal_watchdog::run("journal-logout", move |stage| {
        scenario(enabled, approved, change, stage)
    })
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
fn scenario(
    enabled: bool,
    approved: bool,
    change: Change,
    stage: super::journal_watchdog::Stage,
) -> TestResult {
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
        super::journal_inventory::seed(&h).await?;
        super::journal_inventory::assert_runtime_helpers(&h).await?;
        super::journal_inventory::runtime_role(&h.root).await?;
        stage.set("authority and custody setup");
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
            assert!(
                ProducerStore {
                    root: &h.root,
                    authority: &authority
                }
                .claim_due(&stream.source_authority, 1)
                .await?
                .is_empty(),
                "registration alone is not export history"
            );
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
            super::journal_authority::change(
                &h,
                &stream,
                match change {
                    Change::None | Change::AfterCursor => "none",
                    Change::Epoch => "epoch",
                    Change::OwnershipCut => "cut",
                    Change::Receipt => "receipt",
                },
            )
            .await?;
            stage.set("composed logout capture");
            let channel = crate::foundation::ChannelId::decode(&super::fixture::id(43))
                .map_err(super::debug)?;
            let map = super::journal_inventory::world(world, channel)?;
            let base = crate::foundation::ChannelContentPin::test(world);
            let pin = crate::foundation::ChannelContentPin::from_activation(
                world,
                base.activation_sequence(),
                base.server_artifact_digest(),
                base.client_artifact_digest(),
                base.frame_binding_digest(),
                map.map_revision_digest(),
                (2, 0, 7),
            );
            let deps = super::Dependencies::with_content(
                pin,
                crate::content::qualify_native_entry_room(world).map_err(super::debug)?,
                crate::spell::cast::v1_spell_book().map_err(super::debug)?,
            )?;
            let cells = map
                .movement_cells(deps.room.movement_cells())
                .map_err(super::debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.movement_cells = (&cells).into();
            // Track this actual admitted actor/session through release.rs. It
            // receives the catalogue only from the admitted bundle's depot facts.
            deps.initialize(&owner).await;
            super::track(&owner, &deps)?;
            {
                let mut runtime = owner.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(super::debug)?;
                runtime
                    .bind_attacker_lease(deps.actor, session, current.current_character_lease())
                    .map_err(super::debug)?;
            }
            if matches!(
                change,
                Change::Epoch | Change::OwnershipCut | Change::Receipt
            ) {
                // These adversarial retained authority observations are tested
                // at the atomic capture boundary, without unrelated actor saves.
                let releasing = owner.journal_logout_store(store.clone(), world);
                assert!(matches!(
                    releasing
                        .release_logout_session(session, &stream.account_id, transport)
                        .await?,
                    ExpiredLossReleaseV1::Released { .. }
                ));
            } else {
                assert_eq!(
                    Box::pin(owner.logout_session(deps.admitted)).await,
                    crate::gameplay_transport::connection::LogoutReply::Result(
                        oteryn_protocol_oteryn::logout::LogoutResult::Accepted
                    )
                );
            }
            let records:Vec<Vec<u8>>=sqlx::query_scalar("SELECT export_bytes FROM game_journal_outbox WHERE stream_key=$1 ORDER BY source_revision LIMIT 16386").bind(&key).fetch_all(&h.pool).await?;
            if enabled && approved && matches!(change, Change::None | Change::AfterCursor) {
                super::journal_inventory::assert_export(&records)?;
                assert!(matches!(
                    store
                        .release_logout_session(session, &stream.account_id, transport)
                        .await?,
                    ExpiredLossReleaseV1::Terminal
                ));
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_journal_logout_cuts WHERE game_session_id=encode($1,'hex')::uuid").bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(count, 1);
                let captured:String=sqlx::query_scalar("SELECT character_revision_at_logout::text FROM game_journal_logout_cuts WHERE game_session_id=encode($1,'hex')::uuid").bind(session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
                let actual:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=$1::text::uuid").bind(&stream.character_id).fetch_one(&h.pool).await?;
                assert_eq!(captured, actual);
                assert!(captured.parse::<u64>()? > before);
                let record = PublishGameJournalRecordV1::decode(&records[0])?;
                assert_eq!(record.character_revision, stream.character_revision);
                let crate::native_admission_source::journal::ItemsPayload::Begin(begin) =
                    record.payload
                else {
                    return Err("begin".into());
                };
                assert_eq!(begin.character_revision, captured);
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
            stage.set("publisher cursor and replay");
            let outbox = ProducerStore {
                root: &h.root,
                authority: &authority,
            };
            if change == Change::AfterCursor {
                state.lock().map_err(|_| "peer state")?.unregistered = false;
                let losing = super::journal_authority::Losing {
                    store: ProducerStore {
                        root: &h.root,
                        authority: &authority,
                    },
                    h: &h,
                };
                assert_eq!(
                    Box::pin(publisher.step(&stream, &losing)).await,
                    Step::Quarantined
                );
                {
                    let peer = state.lock().map_err(|_| "peer state")?;
                    assert_eq!(peer.calls, 1);
                    assert!(peer.sent.is_empty());
                }
                let acked:i64=sqlx::query_scalar("SELECT count(*) FROM game_journal_outbox WHERE stream_key=$1 AND acknowledged_at IS NOT NULL").bind(&key).fetch_one(&h.pool).await?;
                assert_eq!(acked, 0);
            } else if enabled && approved && change == Change::None {
                publish_to_idle(&mut publisher, &stream, &outbox, &h.root).await?;
                let acked:i64=sqlx::query_scalar("SELECT count(*) FROM game_journal_outbox WHERE stream_key=$1 AND acknowledged_at IS NOT NULL").bind(&key).fetch_one(&h.pool).await?;
                assert_eq!(acked, 3);
                state.lock().map_err(|_| "peer state")?.accepted.clear();
                publish_to_idle(&mut publisher, &stream, &outbox, &h.root).await?;
                {
                    let sent = state.lock().map_err(|_| "peer state")?;
                    assert_eq!(sent.sent.len(), 6);
                    assert_eq!(&sent.sent[..3], &sent.sent[3..]);
                }
                stage.set("retention and restart scheduling");
                let claim = outbox
                    .claim_due(&stream.source_authority, 1)
                    .await?
                    .pop()
                    .ok_or("claim")?;
                assert!(
                    outbox
                        .finish(&claim, 4, std::time::Duration::from_secs(999), false, false)
                        .await?
                );
                assert!(
                    outbox
                        .claim_due(&stream.source_authority, 1)
                        .await?
                        .is_empty()
                );
                // Simulate that persisted deadline having elapsed, not an early
                // schedule override (which is now forbidden).
                sqlx::query("UPDATE game_journal_streams SET next_delivery_at=clock_timestamp() WHERE stream_key=$1").bind(&key).execute(&h.pool).await?;
                let restarted_root = h.independent_root()?;
                assert!(restarted_root.maintain_ready_once().await?);
                super::journal_inventory::runtime_role(&restarted_root).await?;
                let restarted_authority = restarted_root
                    .open_character_authority(&seal)
                    .await
                    .map_err(super::debug)?;
                let restarted = ProducerStore {
                    root: &restarted_root,
                    authority: &restarted_authority,
                };
                let due = restarted
                    .claim_due(&stream.source_authority, 1)
                    .await?
                    .pop()
                    .ok_or("restart claim")?;
                assert_eq!(due.stream, stream);
                assert_eq!(due.attempt, 4);
                restarted.release_claim(&due).await?;
                // Expiry blocks replay even before the periodic erasure sweep.
                sqlx::query("UPDATE game_journal_outbox SET payload_expires_at=clock_timestamp()-interval '1 second' WHERE stream_key=$1").bind(&key).execute(&h.pool).await?;
                use crate::native_admission_source::journal::publisher::JournalOutbox;
                assert!(restarted.first_for_replay(&stream).await.is_err());
                // ACK evidence remains for 30 days, but replay does not extend
                // the private payload's occurred-at lifetime.
                let days:bool=sqlx::query_scalar("SELECT bool_and(retain_until>=acknowledged_at+interval '30 days') FROM game_journal_outbox WHERE stream_key=$1").bind(&key).fetch_one(&h.pool).await?;
                assert!(days);
                sqlx::query("UPDATE game_journal_outbox SET retain_until=clock_timestamp()-interval '1 second' WHERE stream_key=$1").bind(&key).execute(&h.pool).await?;
                restarted.maintenance().await?;
                let retained:(i64,i64,bool)=sqlx::query_as("SELECT count(*),count(export_bytes),bool_and(octet_length(export_sha256)=32) FROM game_journal_outbox WHERE stream_key=$1").bind(&key).fetch_one(&h.pool).await?;
                assert_eq!(retained, (3, 0, true));
                let floor: String = sqlx::query_scalar(
                    "SELECT retained_floor::text FROM game_journal_streams WHERE stream_key=$1",
                )
                .bind(&key)
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(floor, "3");
                assert!(
                    restarted.current(&stream).await?,
                    "ordinary expiry must not fence a healthy stream"
                );
            } else {
                assert_eq!(
                    Box::pin(publisher.step(&stream, &outbox)).await,
                    Step::Quarantined
                );
                assert_eq!(state.lock().map_err(|_| "peer state")?.calls, 0);
            }
        }
        stage.set("cleanup");
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
