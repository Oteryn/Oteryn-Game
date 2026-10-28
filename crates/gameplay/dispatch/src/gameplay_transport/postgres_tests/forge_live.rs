//! SQL prototype qualification only. These stores are not native gameplay truth;
//! forge_dispatch.rs proves the actual native DustLimit route separately.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, id, runtime};
use serde_json::{Value, json};

#[test]
fn forge_live_atomic_postgres_cases() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "forge_live", false).await?;
        let character = id(41);
        sqlx::query("INSERT INTO game_character_forge_wallets(character_id,gold,dust,slivers,cores) VALUES(encode($1,'hex')::uuid,100000000,1000,50,100)")
            .bind(character.as_slice()).execute(&h.pool).await?;
        for item in [81,82,83,84] {
            sqlx::query("INSERT INTO game_character_forge_items VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,4,0,true)")
                .bind(id(item).as_slice()).bind(character.as_slice()).execute(&h.pool).await?;
        }
        let fusion = json!({"operation":"convergence_fusion","first_item":uuid(id(81)),"second_item":uuid(id(82))});
        let first = commit(&h, 91, &fusion).await?;
        assert_eq!(first["succeeded"], true);
        let after_first = wallet(&h).await?;
        assert_eq!(commit(&h, 91, &fusion).await?, first);
        assert_eq!(wallet(&h).await?, after_first, "exact replay spends nothing twice");
        assert!(commit(&h, 91, &json!({"operation":"dust_to_slivers"})).await.is_err(), "conflicting replay");
        assert_eq!(wallet(&h).await?, after_first);
        // Preserve the draft's deterministic-failure qualification, without calling it
        // a gameplay RNG: the predictable occurrence hash is staged and inaccessible.
        let mut failed = false;
        for seed in 100..=200 {
            let roll: i64 = sqlx::query_scalar("SELECT ((hashtextextended((encode($1,'hex')::uuid)::text||'fusion',0)%100)+100)%100")
                .bind(id(seed).as_slice()).fetch_one(&h.pool).await?;
            if roll >= 50 {
                let request = json!({"operation":"fusion","first_item":uuid(id(83)),"second_item":uuid(id(84))});
                let result = commit(&h, seed, &request).await?;
                assert_eq!(result["succeeded"], false);
                let donor_active: bool = sqlx::query_scalar("SELECT active FROM game_character_forge_items WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(id(84).as_slice()).fetch_one(&h.pool).await?;
                assert!(!donor_active);
                assert_eq!(wallet(&h).await?, (37_000_000, 770, 50, 100, 2));
                failed = true;
                break;
            }
        }
        assert!(failed);
        let conversion = json!({"operation":"dust_to_slivers"});
        commit(&h, 93, &conversion).await?;
        sqlx::query("UPDATE game_character_forge_wallets SET dust=0 WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_slice()).execute(&h.pool).await?;
        let stable = wallet(&h).await?;
        assert!(commit(&h, 94, &conversion).await.is_err());
        assert_eq!(wallet(&h).await?, stable, "refusal changes no resources");
        let history: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_forge_history WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(history, 3, "replay, conflict and refusal add no history");
        // Serializing before receipt lookup makes concurrent same-occurrence calls replay.
        let cores = json!({"operation":"slivers_to_cores"});
        let (a,b) = together(commit(&h, 95, &cores), commit(&h, 95, &cores)).await;
        assert_eq!(a?, b?);
        assert_eq!(wallet(&h).await?, (37_000_000, 0, 3, 101, 4));
        h.cleanup().await
    })
}
async fn commit(h: &Harness, occurrence: u8, request: &Value) -> Result<Value, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT game_commit_character_forge(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3)",
    )
    .bind(id(41).as_slice())
    .bind(id(occurrence).as_slice())
    .bind(request)
    .fetch_one(&h.pool)
    .await
}
async fn wallet(h: &Harness) -> TestResult<(i64, i32, i32, i32, i64)> {
    Ok(sqlx::query_as("SELECT gold,dust,slivers,cores,revision FROM game_character_forge_wallets WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
fn uuid(value: [u8; 16]) -> String {
    let hex = value.map(|b| format!("{b:02x}")).concat();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

async fn together<A: std::future::Future, B: std::future::Future>(
    a: A,
    b: B,
) -> (A::Output, B::Output) {
    use std::task::Poll;
    let mut a = std::pin::pin!(a);
    let mut b = std::pin::pin!(b);
    let mut first = None;
    let mut second = None;
    std::future::poll_fn(|cx| {
        if first.is_none()
            && let Poll::Ready(value) = a.as_mut().poll(cx)
        {
            first = Some(value);
        }
        if second.is_none()
            && let Poll::Ready(value) = b.as_mut().poll(cx)
        {
            second = Some(value);
        }
        match (first.take(), second.take()) {
            (Some(first), Some(second)) => Poll::Ready((first, second)),
            (a, b) => {
                first = a;
                second = b;
                Poll::Pending
            }
        }
    })
    .await
}
