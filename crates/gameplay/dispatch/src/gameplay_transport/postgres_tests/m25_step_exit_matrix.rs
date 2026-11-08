//! SQL negative controls reuse an actual committed native STEP return and the
//! exact installed proof trigger. No control changes canonical gameplay rows.
use super::TestResult;
use super::fixture::Harness;
use crate::foundation::GameSessionId;
use serde_json::{Value, json};

pub(super) async fn assert_native_step_exit_matrix(
    h: &Harness,
    source: GameSessionId,
) -> TestResult {
    let original:Value=sqlx::query_scalar("SELECT to_jsonb(r) FROM game_activity_runtime_returns r WHERE source_game_session_id=encode($1,'hex')::uuid")
        .bind(source.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::raw_sql("CREATE TEMP TABLE native_step_return_controls(LIKE game_activity_runtime_returns INCLUDING ALL) ON COMMIT DROP;
        CREATE CONSTRAINT TRIGGER native_step_exit_matrix_proven AFTER INSERT ON native_step_return_controls
        DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_activity_native_step_exit_proven();")
        .execute(&mut *tx).await?;
    let mut cases = vec![("original canonical movement", original.clone(), true)];
    let mut add = |label: &'static str, field: &str, value: Value| {
        let mut changed = original.clone();
        changed["native_step_exit"][field] = value;
        cases.push((label, changed, false));
    };
    add("missing movement binding", "movement_binding", json!(""));
    add(
        "forged valid-width movement binding",
        "movement_binding",
        json!("44".repeat(32)),
    );
    add(
        "wrong source session",
        "session",
        json!("0102030405067008800a0b0c0d0e0f01"),
    );
    add(
        "wrong source character",
        "character",
        json!("0202030405067008800a0b0c0d0e0f02"),
    );
    add(
        "Channel namespace cannot stand for Instance",
        "scope",
        json!(format!(
            "01{}",
            &original["native_step_exit"]["scope"]
                .as_str()
                .ok_or("scope")?[2..]
        )),
    );
    add(
        "wrong owner generation",
        "generation",
        json!("18446744073709551615"),
    );
    add("wrong source lease", "lease", json!("18446744073709551615"));
    add("wrong source map", "map", json!("11".repeat(32)));
    add("wrong source frame", "frame", json!("22".repeat(32)));
    add("wrong source artifact", "server", json!("33".repeat(32)));
    add(
        "turn is not a translation",
        "before",
        original["native_step_exit"]["after"].clone(),
    );
    add(
        "no new movement revision",
        "before_revision",
        original["native_step_exit"]["after_revision"].clone(),
    );
    add(
        "original execution time is bound",
        "observed_us",
        json!("18446744073709551615"),
    );
    let mut changed = original.clone();
    changed["request_binding"] = json!(format!(r"\x{}", "55".repeat(32)));
    cases.push((
        "native event cannot borrow a different request binding",
        changed,
        false,
    ));
    let mut changed = original.clone();
    changed["native_step_exit"] = Value::Null;
    cases.push(("ordinary NULL Exit remains rejected", changed, false));
    let mut changed = original.clone();
    changed["command_id"] = json!(1);
    cases.push((
        "native STEP does not accept a client command",
        changed,
        false,
    ));
    let mut changed = original.clone();
    changed["reason"] = json!(2);
    cases.push(("STEP proof cannot alter timeout authority", changed, false));
    let mut changed = original.clone();
    changed["reason"] = json!(52);
    cases.push((
        "STEP proof cannot alter Soulpit terminal authority",
        changed,
        false,
    ));
    for (label, row, accepted) in cases {
        sqlx::query("SAVEPOINT native_step_control")
            .execute(&mut *tx)
            .await?;
        let inserted=sqlx::query("INSERT INTO native_step_return_controls SELECT (jsonb_populate_record(NULL::native_step_return_controls,$1)).*")
            .bind(row).execute(&mut *tx).await;
        let result = match inserted {
            Ok(_) => sqlx::query("SET CONSTRAINTS native_step_exit_matrix_proven IMMEDIATE")
                .execute(&mut *tx)
                .await
                .map(|_| ()),
            Err(error) => Err(error),
        };
        if accepted {
            result.map_err(|error| format!("{label}: {error}"))?;
        } else {
            assert!(
                matches!(&result,Err(sqlx::Error::Database(error)) if error.code().as_deref()==Some("23514")),
                "{label}: {result:?}"
            );
        }
        sqlx::query("ROLLBACK TO SAVEPOINT native_step_control")
            .execute(&mut *tx)
            .await?;
        sqlx::query("RELEASE SAVEPOINT native_step_control")
            .execute(&mut *tx)
            .await?;
    }
    tx.rollback().await?;
    Ok(())
}
