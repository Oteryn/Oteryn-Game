//! Exercise the real outbox size guard without an unrelated unique-event constraint
//! hiding the historical-proof refusal. The temporary probe copies the actual
//! column checks and invokes the production stamp and deferred size guard.
use super::*;

async fn outbox(h: &Harness) -> TestResult<Vec<serde_json::Value>> {
    let rows = sqlx::query_scalar("SELECT to_jsonb(a) FROM game_item_audit_outbox a ORDER BY event_id LIMIT 100")
        .fetch_all(&h.pool).await?;
    assert!(rows.len() < 100);
    Ok(rows)
}

pub(super) async fn reject_size_forgery(h: &Harness) -> TestResult {
    let before = scope_footprint(h).await?;
    let before_outbox = outbox(h).await?;
    for probe in 0..3_i32 {
        let mut tx = h.pool.begin().await?;
        sqlx::raw_sql("CREATE TEMPORARY TABLE atelier_size_probe (LIKE game_item_audit_outbox INCLUDING DEFAULTS INCLUDING CONSTRAINTS) ON COMMIT DROP;
            CREATE TRIGGER atelier_size_probe_stamp BEFORE INSERT ON atelier_size_probe FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
            CREATE CONSTRAINT TRIGGER atelier_size_probe_proven AFTER INSERT ON atelier_size_probe DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_item_audit_envelope_size_proven();
            GRANT INSERT ON atelier_size_probe TO oteryn_game_runtime;")
            .execute(&mut *tx).await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        let envelope = vec![0_u8; if probe == 2 { 262_145 } else { 25_713 }];
        if probe == 1 {
            // A fresh promotion fee has its own smaller registered shape. It must
            // not borrow Atelier's ceiling just because both use the fee owner.
            assert_eq!(sqlx::query("INSERT INTO game_item_fee_burns SELECT (jsonb_populate_record(NULL::game_item_fee_burns,to_jsonb(f)||jsonb_build_object('transaction_id',encode($1,'hex')::uuid,'event_id',encode($2,'hex')::uuid,'cause_occurrence_id',encode($3,'hex')::uuid,'envelope_sha256',sha256($4::bytea)))).* FROM game_item_fee_burns f WHERE f.cause_kind=2 LIMIT 1")
                .bind(id(186).as_slice()).bind(id(187).as_slice()).bind(id(188).as_slice())
                .bind(&envelope).execute(&mut *tx).await?.rows_affected(), 1);
        }
        let inserted = sqlx::query("INSERT INTO atelier_size_probe SELECT (jsonb_populate_record(NULL::game_item_audit_outbox,to_jsonb(a)||CASE WHEN $2=0 THEN '{}'::jsonb ELSE jsonb_build_object('envelope',$3::bytea,'envelope_sha256',sha256($3::bytea)) END||CASE WHEN $2=1 THEN jsonb_build_object('transaction_id',encode($4,'hex')::uuid,'event_id',encode($5,'hex')::uuid) ELSE '{}'::jsonb END)).* FROM game_item_audit_outbox a JOIN game_item_fee_burns f USING(transaction_id) WHERE f.cause_occurrence_id=encode($1,'hex')::uuid")
            .bind(id(180).as_slice()).bind(probe).bind(&envelope)
            .bind(id(186).as_slice()).bind(id(187).as_slice()).execute(&mut *tx).await;
        let error = if probe == 2 {
            inserted.expect_err("the actual stored column rejects an envelope above262144 bytes")
        } else {
            assert_eq!(inserted?.rows_affected(), 1);
            // Other transaction proofs remain deferred; this must fail at the
            // real size guard, not an unrelated absent Character receipt.
            sqlx::query("SET CONSTRAINTS atelier_size_probe_proven IMMEDIATE")
                .execute(&mut *tx).await.expect_err("historical or unrelated proof cannot borrow the Atelier ceiling")
        };
        let database = error.as_database_error().ok_or("expected PostgreSQL guard refusal")?;
        assert_eq!(database.code().as_deref(), Some("23514"));
        if probe == 2 {
            assert_eq!(database.constraint(), Some("game_item_audit_outbox_envelope_check"));
        } else {
            assert!(database.message().contains("registered shape ceiling"));
        }
        tx.rollback().await?;
        assert_eq!(scope_footprint(h).await?, before);
        assert_eq!(outbox(h).await?, before_outbox);
    }
    Ok(())
}
