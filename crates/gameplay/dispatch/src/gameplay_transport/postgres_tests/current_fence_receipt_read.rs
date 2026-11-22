//! One immutable fresh receipt scan is enough for both current authority readers.
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::durability::DurabilityError;
use crate::durability::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority,
};
use crate::durability::fresh_admission::{FreshAdmissionStore, FreshReconciliation};
use crate::foundation::GameSessionId;

#[test]
fn current_fences_read_one_receipt_and_keep_replay_and_history_checks_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("current-fence-receipt", move || {
        run(async move |_| {
            let h =
                Harness::create_with_admission_seed(admin, "current_fence_receipt", true, false)
                    .await?;
            let fresh = prepare_current_session(&h).await?;
            let session = fresh.operation().authorization.candidate_session;
            let store = FreshAdmissionStore::from_root(h.root.clone());
            let (current, _) = store.current_session_at(session).await?;

            let mut tx = h.pool.begin().await?;
            sqlx::query("LOCK TABLE game_durability_fresh_admission_receipts IN EXCLUSIVE MODE")
                .execute(&mut *tx)
                .await?;
            assert_eq!(
                store
                    .current_session_in_transaction(&mut tx, session)
                    .await?,
                current
            );
            // Both entry points call this fenced read once. Count the underlying
            // heap or index scans in the same transaction, before any replay.
            let reads: i64 = sqlx::query_scalar(
                "SELECT seq_scan + COALESCE(idx_scan, 0) FROM pg_stat_xact_user_tables \
                 WHERE relid = 'game_durability_fresh_admission_receipts'::regclass",
            )
            .fetch_one(&mut *tx)
            .await?;
            assert_eq!(reads, 1, "the original pass scanned this receipt twice");
            tx.commit().await?;

            {
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let fence = h
                    .root
                    .read_current_gameplay_fence(&authority, session)
                    .await
                    .map_err(debug)?
                    .ok_or("current fence")?;
                assert_eq!(fence.game_session_id, session);
                assert_eq!(
                    fence.connection_generation,
                    current.current_connection_generation()
                );

                let replay = store.reconcile(fresh.operation()).await?;
                let FreshReconciliation::Committed(replay) = replay else {
                    return Err(format!("exact fresh replay: {replay:?}").into());
                };
                assert_eq!(replay.current_session, current);

                let absent = GameSessionId::decode(&id(51)).map_err(debug)?;
                assert!(matches!(
                    store.current_session_at(absent).await,
                    Err(DurabilityError::Unavailable)
                ));
                assert!(
                    h.root
                        .read_current_gameplay_fence(&authority, absent)
                        .await
                        .map_err(debug)?
                        .is_none()
                );

                // Model restored storage with a receipt mirror inconsistent with its payload.
                set_receipt_transport(&h, session, [0; 16]).await?;
                assert_invalid(&h, &store, &authority, session).await?;
                assert!(matches!(
                    store.reconcile(fresh.operation()).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
                set_receipt_transport(
                    &h,
                    session,
                    fresh.operation().authorization.transport.to_bytes(),
                )
                .await?;
                assert_eq!(store.current_session_at(session).await?.0, current);

                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role = replica")
                    .execute(&mut *tx)
                    .await?;
                let updated = sqlx::query(
                    "UPDATE game_durability_admission_guard_history SET change_json = '{}' \
                     WHERE guard_key = decode('02','hex') || uuid_send(encode($1,'hex')::uuid)",
                )
                .bind(current.commit().character_id().as_bytes().as_slice())
                .execute(&mut *tx)
                .await?;
                assert!(updated.rows_affected() > 0);
                tx.commit().await?;
                assert_invalid(&h, &store, &authority, session).await?;
                assert!(matches!(
                    store.reconcile(fresh.operation()).await,
                    Err(DurabilityError::InvalidStoredState)
                ));
            }
            h.cleanup().await
        })
    })
}

async fn set_receipt_transport(
    h: &Harness,
    session: GameSessionId,
    transport: [u8; 16],
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let updated = sqlx::query(
        "UPDATE game_durability_fresh_admission_receipts SET transport_ref = $2 \
         WHERE game_session_id = encode($1,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .bind(transport.as_slice())
    .execute(&mut *tx)
    .await?;
    assert_eq!(updated.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

async fn assert_invalid(
    h: &Harness,
    store: &FreshAdmissionStore,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    session: GameSessionId,
) -> TestResult {
    let lifecycle = store.current_session_at(session).await;
    assert!(
        matches!(lifecycle, Err(DurabilityError::InvalidStoredState)),
        "{lifecycle:?}"
    );
    let gameplay = h.root.read_current_gameplay_fence(authority, session).await;
    assert!(
        matches!(
            gameplay,
            Err(CharacterAuthorityError::Unavailable(
                DurabilityError::InvalidStoredState
            ))
        ),
        "{gameplay:?}"
    );
    Ok(())
}
