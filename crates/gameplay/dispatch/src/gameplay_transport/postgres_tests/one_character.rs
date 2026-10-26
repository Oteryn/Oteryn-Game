//! ONE-CHAR-1: qualify the existing account-global admission fence, without
//! adding a second lease, migration or admission authority.
use super::fixture::{Harness, debug, id, join_two, run};
use super::fresh_fixture::fresh::Source;
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::durability::admission_authority_guards::{
    AdmissionGuardStore, GuardPublicationDisposition,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::{MAX_ADMISSION_GUARD_BYTES, MAX_FRESH_OPERATION_BYTES};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
};
use crate::foundation::fresh_admission_durability::{
    FreshAdmissionCommitRequestV1, FreshAdmissionDurableOutcomeV1,
};
use crate::foundation::{CharacterId, GameSessionState};
use crate::gameplay_transport::canonical_uuid;
use crate::gameplay_transport::connection::LogoutReply;
use oteryn_protocol_oteryn::logout::LogoutResult;
use sqlx::ConnectOptions;

async fn clock(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn publish(h: &Harness, source: &Source) -> TestResult {
    let publication =
        AdmissionAuthorityPublicationV1::prepare(source, source.now).map_err(debug)?;
    assert!(matches!(
        AdmissionGuardStore::from_root(h.root.clone())
            .publish(&publication)
            .await?,
        GuardPublicationDisposition::Applied | GuardPublicationDisposition::Existing
    ));
    Ok(())
}

async fn candidates(
    h: &Harness,
    same_account: bool,
) -> TestResult<(FreshAdmissionCommitRequestV1, FreshAdmissionCommitRequestV1)> {
    let now = clock(h).await?;
    let mut first = Source::new(now)?;
    let mut second = Source::appearance_observer(
        now,
        canonical_uuid(&id(if same_account { 40 } else { 100 })),
        CharacterId::decode(&id(101)).map_err(debug)?,
    )?;
    // Both candidates are prepared from the same empty account claim, before
    // either transaction decides. Their nonces, transports and characters differ.
    let requests = (
        first.request()?,
        second.appearance_observer_request(&canonical_uuid(&id(101)), now)?,
    );
    // Runtime readiness must pass through its node-fenced owning publisher.
    // The generic admission publisher cannot establish runtime authority.
    let mut runtime = Source::new(now)?;
    runtime
        .rows
        .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication = AdmissionAuthorityPublicationV1::prepare(&runtime, now).map_err(debug)?;
    h.root
        .publish_runtime_readiness(&h.node, &publication)
        .await
        .map_err(debug)?;
    first
        .rows
        .retain(|row| !matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    publish(h, &first).await?;
    second.rows.retain(|row| {
        matches!(row.key, AdmissionAuthorityGuardKeyV1::Character(_))
            || (!same_account && matches!(row.key, AdmissionAuthorityGuardKeyV1::Account { .. }))
    });
    publish(h, &second).await?;
    Ok(requests)
}

async fn active_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_durability_reconnect_sessions WHERE session_state IN (1,2)",
    )
    .fetch_one(&h.pool)
    .await?)
}

#[test]
fn one_character_postgres_commit_refuses_second_and_preserves_incumbent() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "onechar_refusal", true, false).await?;
        eprintln!("POSTGRES_STARTED ONE-CHAR-1 commit");
        let (first, second) = candidates(&h, true).await?;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        assert!(matches!(
            store.commit(&first).await?,
            FreshAdmissionDurableOutcomeV1::Committed(_)
        ));
        let session = first.operation().authorization.candidate_session;
        let before = store.current_session(session).await?;
        assert_eq!(
            store.commit(&second).await?,
            FreshAdmissionDurableOutcomeV1::RejectedIncumbent
        );
        assert_eq!(
            store.current_session(session).await?,
            before,
            "no kick or generation change"
        );
        assert_eq!(active_count(&h).await?, 1);
        let effects: (i64, i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_durability_fresh_admission_receipts), \
             (SELECT count(*) FROM game_durability_session_use_memberships), \
             (SELECT count(*) FROM game_durability_transport_ref_reservations)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            effects,
            (1, 1, 1),
            "refusal consumes no nonce, session or transport"
        );
        assert!(matches!(
            store.commit(&first).await?,
            FreshAdmissionDurableOutcomeV1::ExistingCommitted(_)
        ));
        h.cleanup().await
    })
}

#[test]
fn one_character_postgres_two_nodes_choose_exactly_one() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "onechar_race", true, false).await?;
        eprintln!("POSTGRES_STARTED ONE-CHAR-1 race");
        let (first, second) = candidates(&h, true).await?;
        let a = FreshAdmissionStore::from_root(h.root.clone());
        // Independent ready holder, as on another node; sharing a root could
        // refuse on its local busy flag without ever testing the database race.
        let b = FreshAdmissionStore::connect_runtime(
            h.pool.connect_options().to_url_lossy().as_str(),
            MAX_FRESH_OPERATION_BYTES,
            MAX_ADMISSION_GUARD_BYTES,
        )
        .await?;
        let (a, b) = join_two(a.commit(&first), b.commit(&second)).await;
        let outcomes = [a?, b?];
        assert_eq!(
            outcomes
                .iter()
                .filter(|o| matches!(o, FreshAdmissionDurableOutcomeV1::Committed(_)))
                .count(),
            1
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|o| matches!(o, FreshAdmissionDurableOutcomeV1::RejectedIncumbent))
                .count(),
            1
        );
        assert_eq!(active_count(&h).await?, 1);
        h.cleanup().await
    })
}

#[test]
fn one_character_postgres_different_accounts_admit_independently() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "onechar_accounts", true, false).await?;
        eprintln!("POSTGRES_STARTED ONE-CHAR-1 accounts");
        let (first, second) = candidates(&h, false).await?;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        for request in [&first, &second] {
            assert!(matches!(
                store.commit(request).await?,
                FreshAdmissionDurableOutcomeV1::Committed(_)
            ));
        }
        assert_eq!(active_count(&h).await?, 2);
        h.cleanup().await
    })
}

#[test]
fn one_character_postgres_dispatch_logout_frees_account() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "onechar_logout", true, false).await?;
        eprintln!("POSTGRES_STARTED ONE-CHAR-1 logout");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        track(&owner, &deps)?;
        assert_eq!(
            owner.logout_session(deps.admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        let store = FreshAdmissionStore::from_root(h.root.clone());
        assert_eq!(
            store
                .current_session(deps.admitted.game_session_id)
                .await?
                .session_state(),
            GameSessionState::Terminal
        );
        assert_eq!(active_count(&h).await?, 0);
        let now = clock(&h).await?;
        let mut next = Source::appearance_observer(
            now,
            canonical_uuid(&id(40)),
            CharacterId::decode(&id(101)).map_err(debug)?,
        )?;
        let mut publication = Source::appearance_observer(
            now,
            canonical_uuid(&id(40)),
            CharacterId::decode(&id(101)).map_err(debug)?,
        )?;
        publication
            .rows
            .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Character(_)));
        publish(&h, &publication).await?;
        // Read the actual released account claim. Reusing the pre-logout
        // publication would test stale evidence instead of a legal admission.
        let keys: Vec<_> = next.rows.iter().map(|row| row.key.clone()).collect();
        next.rows = AdmissionGuardStore::from_root(h.root.clone())
            .load(&keys)
            .await?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or("missing current guard")?;
        let request = next.appearance_observer_request(&canonical_uuid(&id(101)), now)?;
        assert!(matches!(
            store.commit(&request).await?,
            FreshAdmissionDurableOutcomeV1::Committed(_)
        ));
        assert_eq!(active_count(&h).await?, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

// A storage-only challenger deliberately bypasses the application claim and
// relation locks. Only the database's account index can reject this different
// character/world/channel; the existing character index cannot do so.
const CHALLENGER: &str = "INSERT INTO game_durability_reconnect_sessions \
    (game_session_id,account_id,character_id,world_id,runtime_scope_kind, \
     runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch, \
     original_grace_deadline,predecessor_generation,character_lease_generation, \
     scope_ownership_generation,current_generation,session_state) \
    SELECT encode($1,'hex')::uuid,account_id,encode($2,'hex')::uuid,encode($3,'hex')::uuid, \
     1,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,9999999999,1,1,1,1,1 \
    FROM game_durability_reconnect_sessions WHERE game_session_id=encode($5,'hex')::uuid";

#[test]
fn one_character_postgres_index_cross_scope_and_counterfactual() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "onechar_index", true, false).await?;
        eprintln!("POSTGRES_STARTED ONE-CHAR-1 index");
        prepare_current_session(&h).await?;
        for state in [2_i16, 1] {
            if state == 1 {
                // Lost transport still reserves the account, including after
                // its grace deadline; only a fenced terminal release frees it.
                sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1,current_transport_ref=NULL,control_loss_epoch=1,predecessor_generation=current_generation,original_grace_deadline=1 WHERE game_session_id=encode($1,'hex')::uuid")
                    .bind(id(50).as_slice()).execute(&h.pool).await?;
            }
            let error = sqlx::query(CHALLENGER)
                .bind(id(110).as_slice())
                .bind(id(101).as_slice())
                .bind(id(102).as_slice())
                .bind(id(103).as_slice())
                .bind(id(50).as_slice())
                .execute(&h.pool)
                .await
                .expect_err("account-global database fence");
            let database = error.as_database_error().ok_or("database error required")?;
            assert_eq!(database.code().as_deref(), Some("23505"));
            assert_eq!(
                database.constraint(),
                Some("game_durability_one_nonterminal_session_per_account")
            );
            assert_eq!(active_count(&h).await?, 1, "state {state}");
        }
        // Counterfactual in a rolled-back transaction of this disposable DB:
        // removing only the account index admits two different characters.
        let mut tx = h.pool.begin().await?;
        sqlx::query("DROP INDEX game_durability_one_nonterminal_session_per_account")
            .execute(&mut *tx)
            .await?;
        assert_eq!(
            sqlx::query(CHALLENGER)
                .bind(id(110).as_slice())
                .bind(id(101).as_slice())
                .bind(id(102).as_slice())
                .bind(id(103).as_slice())
                .bind(id(50).as_slice())
                .execute(&mut *tx)
                .await?
                .rows_affected(),
            1
        );
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_durability_reconnect_sessions WHERE session_state IN (1,2)",
        )
        .fetch_one(&mut *tx)
        .await?;
        assert_eq!(count, 2);
        tx.rollback().await?;
        assert_eq!(active_count(&h).await?, 1);
        h.cleanup().await
    })
}
