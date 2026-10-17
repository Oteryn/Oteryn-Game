//! The unchanged native Bestiary owner fixture supplies recovery, a live
//! session, node registration and published runtime readiness. The actual SQLx
//! migrator installs the HUNT schema before the fixture opens recovery authority.
//! Only retained DAILY claim/line data below are synthetic; no sink, fence,
//! sequencer, receipt, commit or audit implementation is replaced by a mock.

use super::*;
use crate::foundation::{ConnectionGeneration, ScopeOwnershipGeneration};
use std::future::Future;
use std::task::Poll;

#[allow(dead_code)]
#[path = "../../tests/support/bestiary_postgres_harness.rs"]
mod native;

use native::{CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id, runtime};

async fn setup(admin: String, tag: &str) -> TestResult<Harness> {
    // The actual embedded SQLx migrator must install the complete HUNT
    // migration before the native fixture opens recovery or writes a root.
    let migrator = sqlx::migrate!("./migrations");
    let mut matches = migrator
        .iter()
        .filter(|migration| migration.description == "prey daily wildcard");
    let migration = matches
        .next()
        .ok_or("HUNT requires its numbered native migration")?;
    if matches.next().is_some() {
        return Err("ambiguous HUNT migration identity".into());
    }
    let version = migration.version;
    let checksum = migration.checksum.to_vec();
    let harness = Harness::create(admin, tag, true).await?;
    let installed: Option<(Vec<u8>, bool)> =
        sqlx::query_as("SELECT checksum,success FROM _sqlx_migrations WHERE version=$1")
            .bind(version)
            .fetch_optional(&harness.pool)
            .await?;
    if installed != Some((checksum, true)) {
        return Err("HUNT migration checksum or installation mismatch".into());
    }
    // Consume the byte-exact real DAILY owner module in the explicit native
    // qualification assembly. No SQL table/guard is reproduced by this test.
    // This source candidate is not evidence of an installed DAILY migration.
    sqlx::raw_sql(crate::durability::daily_reward::DAILY_REWARD_SCHEMA_CANDIDATE)
        .execute(&harness.pool)
        .await?;
    Ok(harness)
}

/// This is data staging in the real, source-pinned DAILY relation, not a
/// substitute claim/grant writer or an assertion of current reset authority.
async fn pending(harness: &Harness, claim: u8, line: u32, quantity: i64, kind: &str) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("INSERT INTO game_character_daily_reward_claims(character_id,world_id,reset_epoch,transaction_id,request_binding,lane_day,streak_before,streak_after,jokers_spent,premium) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,$5,1,0,1,0,false) ON CONFLICT(character_id,world_id,reset_epoch) DO NOTHING")
        .bind(id(CHARACTER).as_slice()).bind(id(WORLD).as_slice()).bind(claim.to_string())
        .bind(id(claim).as_slice()).bind([claim;32].as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_daily_reward_lines(character_id,world_id,reset_epoch,line,kind,quantity,status) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,'PENDING')")
        .bind(id(CHARACTER).as_slice()).bind(id(WORLD).as_slice()).bind(claim.to_string())
        .bind(i32::try_from(line)?).bind(kind).bind(quantity).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn pending_other_world(harness: &Harness) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("INSERT INTO game_character_daily_reward_claims SELECT character_id,encode($1,'hex')::uuid,reset_epoch,encode($2,'hex')::uuid,request_binding,lane_day,streak_before,streak_after,jokers_spent,premium FROM game_character_daily_reward_claims WHERE transaction_id=encode($3,'hex')::uuid")
        .bind(id(44).as_slice()).bind(id(62).as_slice()).bind(id(60).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_daily_reward_lines(character_id,world_id,reset_epoch,line,kind,quantity,status) SELECT character_id,world_id,reset_epoch,0,'PREY_WILDCARD',7,'PENDING' FROM game_character_daily_reward_claims WHERE transaction_id=encode($1,'hex')::uuid")
        .bind(id(62).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

fn request(claim: u8, line: u32, planned: u8) -> PendingDailyWildcardRequest {
    PendingDailyWildcardRequest {
        claim_transaction_id: id(claim),
        line,
        delivery_transaction_id: id(planned),
        event_id: id(planned + 1),
        server_build_id: "prey-native-fixture-1".into(),
    }
}

fn committed(outcome: DailyWildcardOutcome) -> TestResult<CommittedDailyWildcardDelivery> {
    match outcome {
        DailyWildcardOutcome::Committed(receipt) => Ok(receipt),
        other => Err(format!("expected a new committed delivery, got {other:?}").into()),
    }
}

async fn dimensions(harness: &Harness) -> TestResult<(String, i64, i64, i64, i64)> {
    Ok((
        harness.root_revision().await?,
        harness.count("game_character_prey_receipts").await?,
        harness.count("game_account_prey_wildcard_entries").await?,
        harness
            .count("game_account_prey_wildcard_audit_outbox")
            .await?,
        sqlx::query_scalar(
            "SELECT count(*) FROM game_character_daily_reward_lines WHERE status='DELIVERED'",
        )
        .fetch_one(&harness.pool)
        .await?,
    ))
}

fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map(|code| code.into_owned())
}

async fn join_two<A: Future, B: Future>(first: A, second: B) -> (A::Output, B::Output) {
    let mut first = std::pin::pin!(first);
    let mut second = std::pin::pin!(second);
    let mut a = None;
    let mut b = None;
    std::future::poll_fn(move |cx| {
        if a.is_none()
            && let Poll::Ready(value) = first.as_mut().poll(cx)
        {
            a = Some(value);
        }
        if b.is_none()
            && let Poll::Ready(value) = second.as_mut().poll(cx)
        {
            b = Some(value);
        }
        match (a.take(), b.take()) {
            (Some(a), Some(b)) => Poll::Ready((a, b)),
            (left, right) => {
                a = left;
                b = right;
                Poll::Pending
            }
        }
    })
    .await
}

#[test]
fn prey_daily_delivery_replays_from_receipt_caps_and_records_zero_credit() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(Box::pin(async move {
        let h=setup(admin,"prey_cap").await?;
        pending(&h,60,0,50,"PREY_WILDCARD").await?;
        pending(&h,60,1,i64::MAX,"PREY_WILDCARD").await?;
        pending(&h,60,2,9,"PREY_WILDCARD").await?;
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let first=committed(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(1)?,request(60,0,80)).await.map_err(debug)?)?;
        assert_eq!((first.quantity,first.credited,first.uncredited_excess,first.balance_before,first.balance_after),(50,50,0,0,50));
        assert_eq!((first.original_character_revision.get(),first.committed_character_revision.get()),(1,2));
        let before=dimensions(&h).await?;
        for revision in [1,2,99] {
            // Different planned IDs do not create another effect for (claim,line).
            let result=h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(revision)?,request(60,0,90)).await.map_err(debug)?;
            assert_eq!(result,DailyWildcardOutcome::AlreadyCommitted(first.clone()));
        }
        assert_eq!(dimensions(&h).await?,before);
        let second=committed(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(2)?,request(60,1,82)).await.map_err(debug)?)?;
        assert_eq!((second.credited,second.balance_before,second.balance_after),(2,50,52));
        assert_eq!(second.uncredited_excess,u64::try_from(i64::MAX)?-2);
        let third=committed(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(3)?,request(60,2,84)).await.map_err(debug)?)?;
        assert_eq!((third.credited,third.uncredited_excess,third.balance_before,third.balance_after),(0,9,52,52));
        assert_eq!((third.entry_id,third.event_id),(None,None));
        assert_eq!(dimensions(&h).await?,("4".into(),3,2,2,3));
        let wires:Vec<Vec<u8>>=sqlx::query_scalar("SELECT envelope FROM game_account_prey_wildcard_audit_outbox ORDER BY occurred_at,event_id").fetch_all(&h.pool).await?;
        for wire in wires {
            let (_,payload)=prey_audit::decode_wildcard_envelope(&wire).map_err(debug)?;
            assert_eq!(payload.daily_claim_transaction_id,id(60));
            assert_eq!(payload.account_id,id(40));
            assert_eq!(payload.world_id,id(WORLD));
            assert_eq!(payload.kind,"DAILY_REWARD");
        }
        // Native reopen validates the entire mixed receipt chain. This catches
        // omission of PREY from Rust recovery integrity, not only its SQL trigger.
        drop(authority);
        let reopened=h.root.open_character_authority(&seal).await.map_err(debug)?;
        assert_eq!(h.root.deliver_pending_daily_wildcard(&reopened,&h.node,fence(4)?,request(60,2,94)).await.map_err(debug)?,DailyWildcardOutcome::AlreadyCommitted(third));
        drop(reopened);drop(seal);h.cleanup().await
    }))
}

#[test]
fn prey_daily_delivery_rejects_current_fence_and_source_mismatches_without_writes() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(Box::pin(async move {
        let h=setup(admin,"prey_fence").await?;
        pending(&h,60,0,3,"PREY_WILDCARD").await?;
        pending(&h,60,1,3,"XP_BOOST").await?;
        pending(&h,60,2,3,"PREY_WILDCARD").await?;
        pending_other_world(&h).await?;
        sqlx::query("UPDATE game_character_daily_reward_lines SET status='DELIVERED',delivering_transaction_id=encode($1,'hex')::uuid WHERE line=2")
            .bind(id(99).as_slice()).execute(&h.pool).await?;
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        let baseline=dimensions(&h).await?;
        let mut changed=fence(1)?;changed.connection_generation=ConnectionGeneration::new(2).map_err(debug)?;
        let mut lease=fence(1)?;lease.character_lease_generation=2;
        let mut scope=fence(1)?;scope.scope_ownership_generation=ScopeOwnershipGeneration::new(2).map_err(debug)?;
        let mut session=fence(1)?;session.game_session_id=crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
        for bad in [changed,lease,scope,session] {
            assert!(matches!(h.root.deliver_pending_daily_wildcard(&authority,&h.node,bad,request(60,0,80)).await,Err(PreyWildcardError::AuthorityRejected)));
            assert_eq!(dimensions(&h).await?,baseline);
        }
        assert!(matches!(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(2)?,request(60,0,80)).await,Err(PreyWildcardError::CharacterRevisionMismatch)));
        assert!(matches!(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(1)?,request(60,1,80)).await,Err(PreyWildcardError::NotWildcardLine)));
        assert!(matches!(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(1)?,request(61,0,80)).await,Err(PreyWildcardError::MissingDailyLine)));
        assert!(matches!(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(1)?,request(62,0,80)).await,Err(PreyWildcardError::SourceBindingMismatch)));
        assert!(matches!(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(1)?,request(60,2,80)).await,Err(PreyWildcardError::Unavailable(DurabilityError::InvalidStoredState))));
        assert_eq!(dimensions(&h).await?,baseline);
        assert_eq!(h.count("game_account_prey_wildcards").await?,0);
        drop(authority);drop(seal);h.cleanup().await
    }))
}

#[test]
fn prey_daily_delivery_rolls_back_all_effects_when_terminal_source_update_fails() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(Box::pin(async move {
        let h=setup(admin,"prey_rollback").await?;
        pending(&h,60,0,3,"PREY_WILDCARD").await?;
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        // A database failure at the final mutation occurs after the writer has
        // attempted root/state, receipt, balance, entry and event statements.
        sqlx::raw_sql("CREATE FUNCTION prey_test_reject_terminal() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic final write failure' USING ERRCODE='23514'; END $$; CREATE TRIGGER prey_test_reject_terminal BEFORE UPDATE ON game_character_daily_reward_lines FOR EACH ROW EXECUTE FUNCTION prey_test_reject_terminal();").execute(&h.pool).await?;
        assert!(matches!(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(1)?,request(60,0,80)).await,Err(PreyWildcardError::Unavailable(_))));
        assert_eq!(dimensions(&h).await?,("1".into(),0,0,0,0));
        assert_eq!(h.count("game_account_prey_wildcards").await?,0);
        sqlx::raw_sql("DROP TRIGGER prey_test_reject_terminal ON game_character_daily_reward_lines; DROP FUNCTION prey_test_reject_terminal();").execute(&h.pool).await?;
        // Swallow the actual event INSERT. The writer reaches commit, where
        // its real deferred receipt guard must reject the incomplete effect.
        sqlx::raw_sql("CREATE FUNCTION prey_test_omit_event() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$; CREATE TRIGGER prey_test_omit_event BEFORE INSERT ON game_account_prey_wildcard_audit_outbox FOR EACH ROW EXECUTE FUNCTION prey_test_omit_event();").execute(&h.pool).await?;
        assert!(matches!(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(1)?,request(60,0,80)).await,Err(PreyWildcardError::Unavailable(_))));
        assert_eq!(dimensions(&h).await?,("1".into(),0,0,0,0));
        assert_eq!(h.count("game_account_prey_wildcards").await?,0);
        sqlx::raw_sql("DROP TRIGGER prey_test_omit_event ON game_account_prey_wildcard_audit_outbox; DROP FUNCTION prey_test_omit_event();").execute(&h.pool).await?;
        let receipt=committed(h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(1)?,request(60,0,80)).await.map_err(debug)?)?;
        assert_eq!(receipt.credited,3);
        assert_eq!(dimensions(&h).await?,("2".into(),1,1,1,1));
        drop(authority);drop(seal);h.cleanup().await
    }))
}

#[test]
fn prey_daily_delivery_once_key_serializes_independent_native_connections() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(Box::pin(async move {
        let h = setup(admin, "prey_once").await?;
        pending(&h, 60, 0, 7, "PREY_WILDCARD").await?;
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let a = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let b = other.open_character_authority(&seal).await.map_err(debug)?;
        let (left, right) = join_two(
            h.root
                .deliver_pending_daily_wildcard(&a, &h.node, fence(1)?, request(60, 0, 80)),
            other.deliver_pending_daily_wildcard(&b, &h.node, fence(1)?, request(60, 0, 90)),
        )
        .await;
        let left = left.map_err(debug)?;
        let right = right.map_err(debug)?;
        match (left, right) {
            (
                DailyWildcardOutcome::Committed(fresh),
                DailyWildcardOutcome::AlreadyCommitted(replay),
            )
            | (
                DailyWildcardOutcome::AlreadyCommitted(replay),
                DailyWildcardOutcome::Committed(fresh),
            ) => assert_eq!(fresh, replay),
            other => {
                return Err(format!("one new and one retained result required: {other:?}").into());
            }
        }
        assert_eq!(dimensions(&h).await?, ("2".into(), 1, 1, 1, 1));
        drop(a);
        drop(b);
        drop(other);
        drop(seal);
        h.cleanup().await
    }))
}

#[test]
fn prey_daily_delivery_authoritative_history_and_event_are_immutable() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(Box::pin(async move {
        let h=setup(admin,"prey_immutable").await?;
        pending(&h,60,0,7,"PREY_WILDCARD").await?;
        let seal=h.recovery.seal_current().map_err(debug)?;
        let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
        h.root.deliver_pending_daily_wildcard(&authority,&h.node,fence(1)?,request(60,0,80)).await.map_err(debug)?;
        let snapshot=dimensions(&h).await?;
        for sql in [
            "UPDATE game_character_prey_receipts SET command_binding=repeat('x',32)::bytea",
            "DELETE FROM game_character_prey_receipts",
            "UPDATE game_account_prey_wildcard_entries SET amount=amount",
            "DELETE FROM game_account_prey_wildcard_entries",
            "TRUNCATE game_account_prey_wildcards CASCADE",
            "UPDATE game_account_prey_wildcards SET balance=balance+1",
            "UPDATE game_account_prey_wildcards SET balance=balance,last_entry_id=NULL",
            "UPDATE game_account_prey_wildcard_audit_outbox SET expires_at=expires_at+1",
            "DELETE FROM game_account_prey_wildcard_audit_outbox",
        ] {
            let error=sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&h.pool).await.err().ok_or("authoritative mutation must fail")?;
            assert_eq!(sqlstate(&error).as_deref(),Some("23514"));
            assert_eq!(dimensions(&h).await?,snapshot);
        }
        for batch in [None,Some(-1_i32),Some(0),Some(65)] {
            let error=sqlx::query("SELECT game_prey_expire_audit($1)").bind(batch)
                .execute(&h.pool).await.err().ok_or("invalid expiry batch must fail")?;
            assert_eq!(sqlstate(&error).as_deref(),Some("22023"));
        }
        let early:i64=sqlx::query_scalar("SELECT game_prey_expire_audit(64)").fetch_one(&h.pool).await?;
        assert_eq!(early,0);
        let expiry:i64=sqlx::query_scalar("SELECT expires_at FROM game_account_prey_wildcard_audit_outbox").fetch_one(&h.pool).await?;
        // These are owner-role storage checks, not a fabricated application
        // case-authority token. Runtime/control cannot place or release holds.
        let hold:String=sqlx::query_scalar("SELECT game_prey_place_legal_hold(encode($1,'hex')::uuid,'case:prey-regression','test-admin')::text")
            .bind(id(81).as_slice()).fetch_one(&h.pool).await?;
        let retained:String=sqlx::query_scalar("SELECT game_prey_place_legal_hold(encode($1,'hex')::uuid,'case:prey-regression','test-admin')::text")
            .bind(id(81).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(hold,retained);
        sqlx::query("SELECT game_prey_release_legal_hold($1::text::uuid,'test-admin')").bind(&hold).execute(&h.pool).await?;
        let released:bool=sqlx::query_scalar("SELECT released_at>=started_at AND released_by='test-admin' FROM game_account_prey_wildcard_audit_legal_holds WHERE hold_id=$1::text::uuid")
            .bind(&hold).fetch_one(&h.pool).await?;
        assert!(released);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT expires_at FROM game_account_prey_wildcard_audit_outbox").fetch_one(&h.pool).await?,expiry);
        for role in ["oteryn_game_runtime","oteryn_game_control"] {
            let allowed:bool=sqlx::query_scalar("SELECT has_function_privilege($1::name,'game_prey_place_legal_hold(uuid,text,text)','EXECUTE') OR has_function_privilege($1::name,'game_prey_release_legal_hold(uuid,text)','EXECUTE')")
                .bind(role).fetch_one(&h.pool).await?;
            assert!(!allowed);
        }
        assert_eq!(dimensions(&h).await?,snapshot);
        drop(authority);drop(seal);h.cleanup().await
    }))
}
