//! Profile-B fences through real PostgreSQL, including racing answers and lifecycle hints.
use super::TestResult;
use super::fixture::{Harness, debug, join_two, run};
use crate::durability::platform_streams::profile_b_store::Acceptance;
use crate::durability::platform_streams::profile_b_wire::*;
use sqlx::ConnectOptions;
fn fixture(name: &str) -> Result<Vec<u8>, std::io::Error> {
    std::fs::read(format!(
        "{}/../../../docs/contracts/fixtures/profile-b-authority-v1/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
}
type FixtureEvidence = (Request, Answer, Vec<(String, u64)>);
fn evidence() -> Result<FixtureEvidence, Box<dyn std::error::Error>> {
    let Some(Message::Request(r)) = Message::parse(&fixture("authority-request")?) else {
        return Err("request".into());
    };
    let Some(Message::Answer(a)) = Message::parse(&fixture("authority")?) else {
        return Err("answer".into());
    };
    let products = a
        .entitlements
        .iter()
        .map(|e| (e.product_id.clone(), e.product_version))
        .collect();
    Ok((r, a, products))
}
async fn accept(
    h: &Harness,
    r: &Request,
    a: &Answer,
    p: &[(String, u64)],
) -> Result<Acceptance, Box<dyn std::error::Error>> {
    super::maintain_ready(&h.root).await?;
    h.root
        .accept_profile_b_answer(r, p, &Message::Answer(a.clone()).encode().ok_or("encode")?)
        .await
        .map_err(debug)
}
async fn fence(h: &Harness) -> TestResult {
    let (request, mut a, products) = evidence()?;
    assert!(matches!(
        accept(h, &request, &a, &products).await?,
        Acceptance::Accepted(_)
    ));
    assert!(matches!(
        accept(h, &request, &a, &products).await?,
        Acceptance::Ignored(_)
    ));
    let original = h
        .root
        .load_profile_b(&a.account_id)
        .await
        .map_err(debug)?
        .answer
        .ok_or("stored")?;
    let id = a.entitlements[0].entitlement_id.clone();
    a.authority_revision += 1;
    a.entitlements[0].lifecycle_revision += 1;
    a.entitlements[0].effective_until = Some("2024-01-14T00:00:00Z".into());
    assert!(matches!(
        accept(h, &request, &a, &products).await?,
        Acceptance::Accepted(_)
    ));
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_profile_b_products WHERE account_id=$1::uuid AND product_id=$2 AND product_version=1").bind(&a.account_id).bind(&a.entitlements[0].product_id).fetch_one(&h.pool).await?;
    assert_eq!(count, 1, "repeat purchase must merge");
    assert_eq!(
        h.root
            .load_profile_b(&a.account_id)
            .await
            .map_err(debug)?
            .answer
            .ok_or("stored")?
            .entitlements[0]
            .entitlement_id,
        id
    );
    assert!(matches!(
        accept(h, &request, &original, &products).await?,
        Acceptance::Ignored(_)
    ));
    let mut repin = a.clone();
    repin.authority_revision += 1;
    repin.entitlements[0].lifecycle_revision += 1;
    repin.entitlements[0].entitlement_id = Some("019a0000-0000-7000-8000-000000000602".into());
    assert!(matches!(
        accept(h, &request, &repin, &products).await?,
        Acceptance::Conflict(_)
    ));
    let mut contradiction = a.clone();
    contradiction.authority_revision += 1;
    contradiction.entitlements[0].effective_from = Some("2023-11-13T00:00:00Z".into());
    assert!(matches!(
        accept(h, &request, &contradiction, &products).await?,
        Acceptance::Conflict(_)
    ));
    assert_eq!(
        h.root
            .load_profile_b(&a.account_id)
            .await
            .map_err(debug)?
            .answer
            .ok_or("stored")?,
        a
    );
    let hint = Hint {
        account_id: a.account_id.clone(),
        product_id: a.entitlements[0].product_id.clone(),
        product_version: 1,
        lifecycle_revision: 5,
    };
    let page = Message::Page(Page {
        schema: "oteryn.entitlement_change_page.v1".into(),
        items: vec![hint],
        next_cursor: Some("v1:1".into()),
    })
    .encode()
    .ok_or("page")?;
    assert!(
        h.root
            .apply_profile_b_changes(None, &page)
            .await
            .map_err(debug)?
    );
    a.authority_revision += 1;
    assert!(matches!(
        accept(h, &request, &a, &products).await?,
        Acceptance::Conflict(_)
    ));
    let view = h.root.load_profile_b(&a.account_id).await.map_err(debug)?;
    assert_eq!(view.hints[0].lifecycle_revision, 5);
    let captured = h
        .root
        .profile_b_known_hints(&[a.account_id.clone()], &products)
        .await
        .map_err(debug)?;
    assert_eq!(captured[0].lifecycle_revision, 5);
    let options = h
        .root
        .chat_relay_connect_options()
        .ok_or("database options")?;
    let second =
        crate::durability::DurabilityRoot::connect_test_runtime(options.to_url_lossy().as_str())?;
    super::maintain_ready(&second).await?;
    assert_eq!(
        second.profile_b_changes_cursor().await.map_err(debug)?,
        (Some("v1:1".into()), false)
    );
    assert_eq!(
        second
            .load_profile_b(&a.account_id)
            .await
            .map_err(debug)?
            .answer
            .ok_or("restart")?
            .authority_revision,
        8
    );
    assert!(
        !second
            .apply_profile_b_changes(None, &page)
            .await
            .map_err(debug)?
    );
    for sql in [
        "UPDATE game_profile_b_accounts SET authority_revision=0",
        "UPDATE game_profile_b_products SET hint_revision=0 WHERE hint_revision>0",
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        assert!(sqlx::query(sql).execute(&mut *tx).await.is_err());
        tx.rollback().await?;
    }
    Ok(())
}
async fn concurrency(h: &Harness) -> TestResult {
    let (request, a, products) = evidence()?;
    let options = h
        .root
        .chat_relay_connect_options()
        .ok_or("database options")?;
    let second =
        crate::durability::DurabilityRoot::connect_test_runtime(options.to_url_lossy().as_str())?;
    super::maintain_ready(&second).await?;
    let mut newer = a.clone();
    newer.authority_revision += 1;
    let old = Message::Answer(a.clone()).encode().ok_or("old")?;
    let new = Message::Answer(newer).encode().ok_or("new")?;
    let (left, right) = join_two(
        h.root.accept_profile_b_answer(&request, &products, &old),
        second.accept_profile_b_answer(&request, &products, &new),
    )
    .await;
    left.map_err(debug)?;
    right.map_err(debug)?;
    assert_eq!(
        h.root
            .load_profile_b(&a.account_id)
            .await
            .map_err(debug)?
            .answer
            .ok_or("high")?
            .authority_revision,
        8
    );
    let hint = Hint {
        account_id: a.account_id.clone(),
        product_id: a.entitlements[0].product_id.clone(),
        product_version: 1,
        lifecycle_revision: 4,
    };
    let page = Message::Page(Page {
        schema: "oteryn.entitlement_change_page.v1".into(),
        items: vec![hint],
        next_cursor: Some("v1:1".into()),
    })
    .encode()
    .ok_or("page")?;
    let mut raced = a.clone();
    raced.authority_revision = 9;
    let bytes = Message::Answer(raced).encode().ok_or("raced")?;
    let (answer, hint) = join_two(
        h.root.accept_profile_b_answer(&request, &products, &bytes),
        second.apply_profile_b_changes(None, &page),
    )
    .await;
    answer.map_err(debug)?;
    assert!(hint.map_err(debug)?);
    let v = h.root.load_profile_b(&a.account_id).await.map_err(debug)?;
    let current = v.answer.ok_or("answer")?;
    let now = GameNow {
        now_us: time(&a.authority_issued_at).ok_or("time")?,
        uncertainty_us: 0,
    };
    assert_ne!(
        evaluate(
            &current.entitlements[0],
            Some(now),
            v.hints[0].lifecycle_revision
        ),
        Evaluation::Active
    );
    assert_eq!(
        h.root.profile_b_changes_cursor().await.map_err(debug)?.0,
        Some("v1:1".into())
    );
    Ok(())
}

async fn premium_hint(h: &Harness) -> TestResult {
    use crate::premium::{PremiumClass, PremiumConsumer, TrustedNow};
    let (r, a, _) = evidence()?;
    let raw: Vec<u8> = sqlx::query_scalar("SELECT uuid_send($1::uuid)")
        .bind(&r.account_id)
        .fetch_one(&h.pool)
        .await?;
    let account: [u8; 16] = raw.try_into().map_err(|_| "account")?;
    let consumer = PremiumConsumer::default();
    let mut snapshot = serde_json::json!({
        "schema":crate::premium::SNAPSHOT_SCHEMA,"producer_revision":a.producer_revision,"producer_profile":a.producer_profile,
        "nonce":r.nonce,"account_id":r.account_id,"product_id":"oteryn.premium_time","product_version":1,
        "entitlement_id":a.entitlements[0].entitlement_id,"entitlement_state":"ACTIVE","lifecycle_revision":3,"authority_revision":7,
        "effective_from":a.entitlements[0].effective_from,"effective_until":a.entitlements[0].effective_until,
        "authority_issued_at":a.authority_issued_at,"authority_valid_until":a.entitlements[0].authority_valid_until,"refresh_after":a.entitlements[0].refresh_after
    });
    let now = TrustedNow::new(time(&a.authority_issued_at).ok_or("time")?, 0);
    super::maintain_ready(&h.root).await?;
    assert_eq!(
        consumer
            .ingest(&h.root, account, &r.nonce, &serde_json::to_vec(&snapshot)?)
            .await,
        crate::premium::IngestOutcome::Accepted
    );
    assert!(consumer.premium_current(account, now));
    let wake = std::sync::Arc::new(tokio::sync::Notify::new());
    consumer.profile_b_register_wake(account, &wake);
    let page = Message::Page(Page {
        schema: "oteryn.entitlement_change_page.v1".into(),
        items: vec![Hint {
            account_id: r.account_id.clone(),
            product_id: "oteryn.premium_time".into(),
            product_version: 1,
            lifecycle_revision: 4,
        }],
        next_cursor: Some("v1:1".into()),
    })
    .encode()
    .ok_or("page")?;
    assert!(
        h.root
            .apply_profile_b_changes(None, &page)
            .await
            .map_err(debug)?
    );
    consumer.profile_b_hint(account, 4);
    assert_eq!(consumer.class(account, now), PremiumClass::StaleWithinBound);
    assert!(
        tokio::time::timeout(std::time::Duration::ZERO, wake.notified())
            .await
            .is_ok(),
        "Premium snapshot refresh is woken"
    );
    snapshot["authority_revision"] = 8.into();
    assert_eq!(
        consumer
            .ingest(&h.root, account, &r.nonce, &serde_json::to_vec(&snapshot)?)
            .await,
        crate::premium::IngestOutcome::Accepted
    );
    assert!(
        !consumer.premium_current(account, now),
        "in-flight older lifecycle cannot restore Premium"
    );
    let restarted = PremiumConsumer::default();
    restarted.load(&h.root, account).await.map_err(debug)?;
    assert!(!restarted.premium_current(account, now));
    assert_eq!(
        restarted.class(account, now),
        PremiumClass::StaleWithinBound
    );
    snapshot["authority_revision"] = 9.into();
    snapshot["lifecycle_revision"] = 4.into();
    snapshot["entitlement_state"] = "REVOKED".into();
    assert_eq!(
        consumer
            .ingest(&h.root, account, &r.nonce, &serde_json::to_vec(&snapshot)?)
            .await,
        crate::premium::IngestOutcome::Accepted
    );
    assert_eq!(consumer.class(account, now), PremiumClass::Revoked);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let error=sqlx::query("INSERT INTO game_profile_b_products(account_id,product_id,product_version,lifecycle_bytes) VALUES($1::uuid,'oteryn.oversize',1,$2)").bind(&r.account_id).bind(vec![b'a';513]).execute(&mut *tx).await.err().ok_or("unbounded lifecycle bytes")?;
    assert_eq!(
        error.as_database_error().and_then(|e| e.code()).as_deref(),
        Some("23514")
    );
    tx.rollback().await?;
    Ok(())
}
fn scenario(which: u8) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            run(async |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "profile_b", true, false).await?;
                let result = match which {
                    0 => fence(&h).await,
                    1 => concurrency(&h).await,
                    2 => premium_hint(&h).await,
                    _ => Err("unknown scenario".into()),
                };
                h.cleanup().await?;
                result
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Profile-B test thread failed")?
        .map_err(Into::into)
}
#[test]
fn profile_b_durable_fence_repeat_purchase_and_restart() -> TestResult {
    scenario(0)
}
#[test]
fn profile_b_concurrent_answers_and_hints_are_monotonic() -> TestResult {
    scenario(1)
}

#[test]
fn profile_b_premium_hint_fences_late_snapshot_and_restart() -> TestResult {
    scenario(2)
}
