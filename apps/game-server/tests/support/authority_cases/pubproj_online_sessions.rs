//! G-8 production transition triggers, retention, ACK races and retained-set resync.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, debug, id, runtime,
};
use crate::native_admission_source::public_projections::v2::{Body, Family};
use sqlx::Row;

fn run_case(
    tag: &'static str,
    case: impl for<'a> FnOnce(
        &'a Harness,
    )
        -> std::pin::Pin<Box<dyn std::future::Future<Output = TestResult> + 'a>>
    + Send
    + 'static,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(async move {
                    let started = std::time::Instant::now();
                    let h = Harness::create(admin, tag, false).await?;
                    let result = case(&h).await;
                    h.cleanup().await?;
                    if result.is_ok() {
                        eprintln!(
                            "PUBPROJ_ONLINE_PG_EXECUTED case={tag} elapsed_ms={}",
                            started.elapsed().as_millis()
                        );
                    }
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "online statistics thread failed")?
        .map_err(Into::into)
}
async fn prepare_fresh_fixture(h: &Harness) -> TestResult {
    // Owning fresh receipt baseline, matching the existing CHAT recipient/world
    // total fixture. The source transition below retains all production triggers.
    sqlx::query("INSERT INTO game_durability_fresh_admission_receipts SELECT \
       '\\x01'::bytea || decode(repeat('00',32),'hex'),game_session_id,account_id,character_id,world_id,runtime_scope_channel_id,character_lease_generation,scope_ownership_generation,1,current_transport_ref,1,'{}',1 \
       FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid ON CONFLICT DO NOTHING")
        .bind(id(SESSION).as_slice()).execute(&h.pool).await?;
    Ok(())
}
const ONLINE: &str = "UPDATE game_durability_reconnect_sessions SET session_state=2,control_loss_epoch=NULL,original_grace_deadline=NULL,predecessor_generation=NULL,fresh_replay_key='\\x01'::bytea || decode(repeat('00',32),'hex') WHERE game_session_id=encode($1,'hex')::uuid";
const OFFLINE: &str = "UPDATE game_durability_reconnect_sessions SET session_state=1,control_loss_epoch=1,original_grace_deadline=999999,predecessor_generation=current_generation WHERE game_session_id=encode($1,'hex')::uuid";

#[test]
fn pubproj_online_interval_transition_atomicity_ack_race_no_overlap_and_resync() -> TestResult {
    run_case("pubproj_online_replay", |h| {
        Box::pin(async move {
            prepare_fresh_fixture(h).await?;
            assert_eq!(h.count("game_character_online_intervals").await?, 0);
            let mut rolled_back = h.pool.begin().await?;
            sqlx::query(ONLINE)
                .bind(id(SESSION).as_slice())
                .execute(&mut *rolled_back)
                .await?;
            sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
                .execute(&mut *rolled_back)
                .await?;
            let pending: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_online_intervals")
                    .fetch_one(&mut *rolled_back)
                    .await?;
            assert_eq!(pending, 1);
            rolled_back.rollback().await?;
            assert_eq!(h.count("game_character_online_intervals").await?, 0);
            let outbox:i64=sqlx::query_scalar("SELECT count(*) FROM game_public_v2_outbox WHERE family='character_online_sessions'").fetch_one(&h.pool).await?;
            assert_eq!(
                outbox, 0,
                "rolled-back source creates no interval revision/outbox"
            );
            sqlx::query(ONLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let first = h
                .root
                .next_public_v2(&authority, Family::CharacterOnlineSessions, &[])
                .await
                .map_err(debug)?
                .ok_or("open interval")?;
            assert_ne!(
                first.subject, "32020304-0506-7008-800a-0b0c0d0e0f32",
                "statistics id is not session id"
            );
            let Body::OnlineSessions(rows) = &first.body else {
                return Err("sessions body".into());
            };
            assert_eq!(rows.len(), 1);
            assert!(rows[0].ended_at.is_empty());
            // A transient internal gap in one transaction is not a committed gap.
            let mut handoff = h.pool.begin().await?;
            sqlx::query(OFFLINE)
                .bind(id(SESSION).as_slice())
                .execute(&mut *handoff)
                .await?;
            sqlx::query(ONLINE)
                .bind(id(SESSION).as_slice())
                .execute(&mut *handoff)
                .await?;
            handoff.commit().await?;
            assert_eq!(h.count("game_character_online_intervals").await?, 1);
            sqlx::query(OFFLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            h.root
                .ack_public_v2(&authority, &first)
                .await
                .map_err(debug)?;
            let closed = h
                .root
                .next_public_v2(&authority, Family::CharacterOnlineSessions, &[])
                .await
                .map_err(debug)?
                .ok_or("closure survives older ACK")?;
            assert_eq!(closed.subject, first.subject);
            assert!(closed.projection_revision > first.projection_revision);
            let Body::OnlineSessions(rows) = &closed.body else {
                return Err("closed sessions body".into());
            };
            assert_eq!(rows[0].ended_at.len(), 1);
            assert!(rows[0].ended_at[0].parse::<u64>()? >= rows[0].started_at.parse::<u64>()?);
            h.root
                .ack_public_v2(&authority, &closed)
                .await
                .map_err(debug)?;
            sqlx::query(ONLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            let overlap:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_online_intervals a JOIN game_character_online_intervals b ON a.character_id=b.character_id AND a.statistics_id<b.statistics_id WHERE a.state<>3 AND b.state<>3 AND a.started_ms<coalesce(b.ended_ms,9223372036854775807) AND b.started_ms<coalesce(a.ended_ms,9223372036854775807))").fetch_one(&h.pool).await?;
            assert!(!overlap);
            let second = h
                .root
                .next_public_v2(&authority, Family::CharacterOnlineSessions, &[])
                .await
                .map_err(debug)?
                .ok_or("new interval")?;
            assert_ne!(
                second.subject, first.subject,
                "a reconnect does not reopen its old statistics subject"
            );
            h.root
                .ack_public_v2(&authority, &second)
                .await
                .map_err(debug)?;
            sqlx::query("SELECT game_public_projection_resync('character_online_sessions',false)")
                .execute(&h.pool)
                .await?;
            let replayed:i64=sqlx::query_scalar("SELECT count(*) FROM game_public_v2_outbox WHERE family='character_online_sessions'").fetch_one(&h.pool).await?;
            assert_eq!(replayed, 2, "resync covers both retained interval subjects");
            Ok(())
        })
    })
}

#[test]
fn pubproj_online_retention_tombstones_erase_payload_and_unsafe_clock_stalls() -> TestResult {
    run_case("pubproj_online_retention", |h| {
        Box::pin(async move {
            prepare_fresh_fixture(h).await?;
            sqlx::query(ONLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            sqlx::query(OFFLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            // A valid old closed source fixture crosses the 30-day retention cut.
            let mut historical = h.pool.begin().await?;
            sqlx::query("UPDATE game_character_online_intervals SET started_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-2592002000,ended_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-2592001000,expires_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-1000 WHERE state=2").execute(&mut *historical).await?;
            let statistics: String = sqlx::query_scalar(
                "SELECT statistics_id::text FROM game_character_online_intervals WHERE state=2",
            )
            .fetch_one(&mut *historical)
            .await?;
            sqlx::query("SELECT game_public_online_publish($1::uuid)")
                .bind(&statistics)
                .execute(&mut *historical)
                .await?;
            historical.commit().await?;
            let changed: i32 = sqlx::query_scalar("SELECT game_public_online_maintain(32)")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(changed, 1);
            let erased:bool=sqlx::query_scalar("SELECT state=3 AND started_ms IS NULL AND ended_ms IS NULL AND owner_account_id IS NULL AND world_id IS NULL AND expires_ms IS NULL FROM game_character_online_intervals WHERE statistics_id=$1::uuid").bind(&statistics).fetch_one(&h.pool).await?;
            assert!(
                erased,
                "payload is erased, only the opaque subject fence survives"
            );
            sqlx::query("SELECT game_public_projection_resync('character_online_sessions',false)")
                .execute(&h.pool)
                .await?;
            let body:serde_json::Value=sqlx::query_scalar("SELECT payload FROM game_public_v2_heads WHERE family='character_online_sessions' AND subject=$1").bind(&statistics).fetch_one(&h.pool).await?;
            assert_eq!(
                body,
                serde_json::json!([]),
                "retention resync cannot resurrect payload"
            );
            sqlx::query(ONLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            // An independently injected unsafe-clock source fixture must not make
            // the production OFFLINE transition clamp or invent a duration.
            sqlx::query("UPDATE game_character_online_intervals SET started_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint+60000 WHERE state=1").execute(&h.pool).await?;
            sqlx::query(OFFLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            let pending: Option<i64> = sqlx::query_scalar(
                "SELECT game_public_v2_source_pending('character_online_sessions')",
            )
            .fetch_one(&h.pool)
            .await?;
            assert!(pending.is_some());
            let row=sqlx::query("SELECT i.state,i.ended_ms,h.payload FROM game_character_online_intervals i JOIN game_public_v2_heads h ON h.family='character_online_sessions' AND h.subject=i.statistics_id::text WHERE i.state=1").fetch_one(&h.pool).await?;
            assert_eq!(row.try_get::<i16, _>("state")?, 1);
            assert_eq!(row.try_get::<Option<i64>, _>("ended_ms")?, None);
            assert_eq!(
                row.try_get::<serde_json::Value, _>("payload")?,
                serde_json::Value::Null
            );
            Ok(())
        })
    })
}

#[test]
fn pubproj_online_unknown_start_and_open_retention_never_invent_session_duration() -> TestResult {
    run_case("pubproj_online_unknown", |h| {
        Box::pin(async move {
            prepare_fresh_fixture(h).await?;
            // The source installation baseline for an already-ONLINE session has
            // an unknown historical start. No interval is fabricated at install.
            sqlx::query("INSERT INTO game_character_online_pending(character_id,observed_ms,reason,world_id,owner_account_id) SELECT character_id,floor(extract(epoch FROM clock_timestamp())*1000)::bigint,'INITIAL_START_UNKNOWN',world_id,account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
            sqlx::query(ONLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            assert_eq!(h.count("game_character_online_intervals").await?, 0);
            let pending: Option<i64> = sqlx::query_scalar(
                "SELECT game_public_v2_source_pending('character_online_sessions')",
            )
            .fetch_one(&h.pool)
            .await?;
            assert!(pending.is_some());
            sqlx::query(OFFLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            sqlx::query(ONLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            let statistics: String = sqlx::query_scalar(
                "SELECT statistics_id::text FROM game_character_online_intervals WHERE state=1",
            )
            .fetch_one(&h.pool)
            .await?;
            sqlx::query("UPDATE game_character_online_intervals SET started_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-2592001000,expires_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-1000 WHERE state=1").execute(&h.pool).await?;
            assert_eq!(
                sqlx::query_scalar::<_, i32>("SELECT game_public_online_maintain(32)")
                    .fetch_one(&h.pool)
                    .await?,
                1
            );
            let reason:String=sqlx::query_scalar("SELECT reason FROM game_character_online_pending WHERE character_id=encode($1,'hex')::uuid").bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(reason, "OPEN_RETENTION");
            assert_eq!(sqlx::query_scalar::<_,Option<i64>>("SELECT ended_ms FROM game_character_online_intervals WHERE statistics_id=$1::uuid").bind(&statistics).fetch_one(&h.pool).await?,None);
            sqlx::query(OFFLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            assert_eq!(h.count("game_character_online_pending").await?, 0);
            sqlx::query(ONLINE)
                .bind(id(SESSION).as_slice())
                .execute(&h.pool)
                .await?;
            let next: String = sqlx::query_scalar(
                "SELECT statistics_id::text FROM game_character_online_intervals WHERE state=1",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_ne!(next, statistics);
            Ok(())
        })
    })
}

#[test]
fn pubproj_online_retained_set_resync_with_one_thousand_intervals_completes_under_operator_bound()
-> TestResult {
    run_case("pubproj_online_resync", |h| {
        Box::pin(async move {
            // Finite old source baseline, not generated session/admission identities.
            // The private table still enforces each interval's shape and UUID purpose.
            sqlx::query("INSERT INTO game_character_online_intervals(character_id,owner_account_id,world_id,started_ms,ended_ms,expires_ms,state) SELECT r.character_id,r.account_id,r.world_id,n.cut-86400000+g.n*1000,n.cut-86400000+g.n*1000+500,n.cut-86400000+g.n*1000+500+2592000000,2 FROM game_character_roots r CROSS JOIN (SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS cut) n CROSS JOIN generate_series(1,1000) g(n) WHERE r.character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
            let start = std::time::Instant::now();
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL transaction_timeout='60s'")
                .execute(&mut *tx)
                .await?;
            sqlx::query("SELECT game_public_projection_resync('character_online_sessions',false)")
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            assert!(start.elapsed() < std::time::Duration::from_secs(60));
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_public_v2_heads WHERE family='character_online_sessions'").fetch_one(&h.pool).await?;
            let queued:i64=sqlx::query_scalar("SELECT count(*) FROM game_public_v2_outbox WHERE family='character_online_sessions'").fetch_one(&h.pool).await?;
            assert_eq!(count, 1000);
            assert_eq!(queued, 1000);
            eprintln!(
                "PUBPROJ_ONLINE_RESYNC_PG_EXECUTED subjects=1000 elapsed_ms={}",
                start.elapsed().as_millis()
            );
            Ok(())
        })
    })
}

#[test]
fn pubproj_online_closed_previous_owner_cohort_is_withdrawn_even_during_clock_reconciliation()
-> TestResult {
    run_case("pubproj_online_owner", |h| {
        Box::pin(async move {
            // Private retained source baseline belongs to an earlier owner. Current
            // authoritative root belongs to account40, so no pre-transfer history
            // may be sent to either owner, even if another interval has a bad clock.
            sqlx::query("INSERT INTO game_character_online_intervals(character_id,owner_account_id,world_id,started_ms,ended_ms,expires_ms,state) SELECT r.character_id,encode($2,'hex')::uuid,r.world_id,n.cut-10000-g.n*1000,n.cut-9500-g.n*1000,n.cut-9500-g.n*1000+2592000000,2 FROM game_character_roots r CROSS JOIN (SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS cut) n CROSS JOIN generate_series(1,3) g(n) WHERE r.character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_character_online_pending(character_id,observed_ms,reason) VALUES(encode($1,'hex')::uuid,1,'UNSAFE_CLOCK')")
            .bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_public_online_refresh_character(encode($1,'hex')::uuid)")
                .bind(id(CHARACTER).as_slice())
                .execute(&h.pool)
                .await?;
            let retained: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_online_intervals WHERE withheld",
            )
            .fetch_one(&h.pool)
            .await?;
            let suppressed:i64=sqlx::query_scalar("SELECT count(*) FROM game_public_v2_heads WHERE family='character_online_sessions' AND payload='[]'::jsonb").fetch_one(&h.pool).await?;
            assert_eq!(retained, 3);
            assert_eq!(suppressed, 3);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let withdrawn = h
                .root
                .next_public_v2(&authority, Family::CharacterOnlineSessions, &[])
                .await
                .map_err(debug)?
                .ok_or("ownership withdrawal")?;
            assert!(matches!(withdrawn.body,Body::OnlineSessions(ref rows) if rows.is_empty()));
            assert!(
                sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT game_public_v2_source_pending('character_online_sessions')"
                )
                .fetch_one(&h.pool)
                .await?
                .is_some()
            );
            Ok(())
        })
    })
}
