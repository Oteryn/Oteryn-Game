//! Lower SQL/codec contract only: no World artifact, creature admission or
//! autonomous runtime return is claimed by these production-function vectors.
use super::TestResult;
use super::fixture::{Harness, run};

#[test]
fn m25_native_step_exit_sql_contract_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("m25-native-step-sql-contract", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "m25_native_step_sql", false, false)
                .await?;
            let installed: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM _sqlx_migrations WHERE version=2171 AND success",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(
                installed, 1,
                "the normal full-schema migrator installs native STEP proof guards"
            );
            let mut tx = h.pool.begin().await?;
            sqlx::raw_sql("SET LOCAL lock_timeout='2s'; SET LOCAL statement_timeout='8s'")
                .execute(&mut *tx)
                .await?;
            sqlx::raw_sql(VECTORS).execute(&mut *tx).await?;
            sqlx::raw_sql(LEGACY_COMMAND_CONTRACT)
                .execute(&mut *tx)
                .await?;
            tx.rollback().await?;
            eprintln!(
                "M25_NATIVE_STEP_SQL_CONTRACT_EXECUTED vectors=15 legacy_positive_predicates=12 full_schema=true no_world_factory=true"
            );
            h.cleanup().await
        })
    })?;
    eprintln!(
        "M25_NATIVE_STEP_SQL_CONTRACT_DURATION {:?}",
        started.elapsed()
    );
    Ok(())
}

// Fixed reference vectors were independently framed with Python struct.pack
// big-endian u64/i32/i16 and hashlib SHA256. They include 2^63 and u64::MAX,
// negative coordinates/floors, pose/time binding and malformed-proof refusal.
const VECTORS: &str = r###"
DO $test$ DECLARE p JSONB := $p${"schema": "native-step-exit.v1", "movement_binding": "354247775e698aafee5394739c5a4487f4c50181054d525544ca8056331921ac", "session": "0402030405067008800a0b0c0d0e0f04", "character": "0502030405067008800a0b0c0d0e0f05", "lease": "9223372036854775809", "scope": "030102030405067008800a0b0c0d0e0f010202030405067008800a0b0c0d0e0f02", "generation": "18446744073709551615", "actor": "0302030405067008800a0b0c0d0e0f03", "incarnation": "9223372036854775808", "before_revision": "9223372036854775807", "after_revision": "9223372036854775808", "before": [-2147483648, 2147483647, -32768], "after": [-5, -6, -15], "observed_us": "18446744073709551615", "frame": "1111111111111111111111111111111111111111111111111111111111111111", "map": "2222222222222222222222222222222222222222222222222222222222222222", "server": "3333333333333333333333333333333333333333333333333333333333333333"}$p$; r JSONB := $r${"handoff_id": "06020304-0506-7008-800a-0b0c0d0e0f06", "character_id": "05020304-0506-7008-800a-0b0c0d0e0f05", "source_game_session_id": "04020304-0506-7008-800a-0b0c0d0e0f04", "source_connection_generation": "9223372036854775808", "source_lease_generation": "9223372036854775809", "reason": 1, "return_id": "07020304-0506-7008-800a-0b0c0d0e0f07", "native_step_exit": {"schema": "native-step-exit.v1", "movement_binding": "354247775e698aafee5394739c5a4487f4c50181054d525544ca8056331921ac", "session": "0402030405067008800a0b0c0d0e0f04", "character": "0502030405067008800a0b0c0d0e0f05", "lease": "9223372036854775809", "scope": "030102030405067008800a0b0c0d0e0f010202030405067008800a0b0c0d0e0f02", "generation": "18446744073709551615", "actor": "0302030405067008800a0b0c0d0e0f03", "incarnation": "9223372036854775808", "before_revision": "9223372036854775807", "after_revision": "9223372036854775808", "before": [-2147483648, 2147483647, -32768], "after": [-5, -6, -15], "observed_us": "18446744073709551615", "frame": "1111111111111111111111111111111111111111111111111111111111111111", "map": "2222222222222222222222222222222222222222222222222222222222222222", "server": "3333333333333333333333333333333333333333333333333333333333333333"}}$r$; BEGIN
IF encode(game_activity_native_step_u64(0),'hex')<>'0000000000000000' THEN RAISE EXCEPTION 'u64 boundary 0'; END IF;
IF encode(game_activity_native_step_u64(9223372036854775807),'hex')<>'7fffffffffffffff' THEN RAISE EXCEPTION 'u64 boundary 9223372036854775807'; END IF;
IF encode(game_activity_native_step_u64(9223372036854775808),'hex')<>'8000000000000000' THEN RAISE EXCEPTION 'u64 boundary 9223372036854775808'; END IF;
IF encode(game_activity_native_step_u64(18446744073709551615),'hex')<>'ffffffffffffffff' THEN RAISE EXCEPTION 'u64 boundary 18446744073709551615'; END IF;
IF NOT game_activity_native_step_exit_shape(p) THEN RAISE EXCEPTION 'valid negative position shape'; END IF;
IF encode(game_activity_native_step_movement_binding(p),'hex')<>'354247775e698aafee5394739c5a4487f4c50181054d525544ca8056331921ac' THEN RAISE EXCEPTION 'movement reference mismatch'; END IF;
IF encode(game_activity_native_step_request_binding(jsonb_populate_record(NULL::game_activity_runtime_returns,r)),'hex')<>'93b03951772c01a1f43f9d6a5b0479e4ef7245da461714fc72f6b2c9a4d9399e' THEN RAISE EXCEPTION 'request reference mismatch'; END IF;
IF game_activity_native_step_exit_shape(p-'actor') THEN RAISE EXCEPTION 'accepted missing field'; END IF;
IF game_activity_native_step_exit_shape(jsonb_set(p,'{before}',p->'after')) THEN RAISE EXCEPTION 'accepted turn only'; END IF;
IF game_activity_native_step_exit_shape(jsonb_set(p,'{before_revision}',p->'after_revision')) THEN RAISE EXCEPTION 'accepted non translation revision'; END IF;
IF game_activity_native_step_exit_shape(jsonb_set(p,'{scope}',to_jsonb('01'||substring(p->>'scope' from 3)))) THEN RAISE EXCEPTION 'accepted wrong scope tag'; END IF;
IF game_activity_native_step_exit_shape(jsonb_set(p,'{after,2}','32768'::jsonb)) THEN RAISE EXCEPTION 'accepted out of range floor'; END IF;
IF game_activity_native_step_exit_shape(jsonb_set(p,'{generation}','"18446744073709551616"'::jsonb)) THEN RAISE EXCEPTION 'accepted u64 overflow'; END IF;
IF game_activity_native_step_request_binding(jsonb_populate_record(NULL::game_activity_runtime_returns,jsonb_set(r,'{native_step_exit,observed_us}','"0"'::jsonb)))=game_activity_native_step_request_binding(jsonb_populate_record(NULL::game_activity_runtime_returns,r)) THEN RAISE EXCEPTION 'execution clock not bound'; END IF;
IF game_activity_native_step_movement_binding(jsonb_set(p,'{after,0}','-4'::jsonb))=decode(p->>'movement_binding','hex') THEN RAISE EXCEPTION 'pose not bound'; END IF;
END $test$;
"###;

const LEGACY_COMMAND_CONTRACT: &str = r###"
CREATE TEMP TABLE native_step_server_contract(command_id NUMERIC(20,0),reason SMALLINT,native_step_exit JSONB) ON COMMIT DROP;
DO $legacy$
DECLARE r RECORD; count_guards INTEGER:=0; reason_value SMALLINT;
BEGIN
 FOR r IN SELECT conname,pg_get_constraintdef(oid) AS definition FROM pg_constraint
   WHERE conrelid='game_activity_runtime_returns'::regclass
   AND conname IN('nx1_return_server_command','soulpit_terminal_server_only','native_step_exit_only') LOOP
  EXECUTE format('ALTER TABLE pg_temp.native_step_server_contract ADD CONSTRAINT %I %s',r.conname,r.definition);
  count_guards:=count_guards+1;
 END LOOP;
 IF count_guards<>3 THEN RAISE EXCEPTION 'complete installed native/legacy return guards required'; END IF;
 -- These are predicate controls, not accepted gameplay returns.
 INSERT INTO native_step_server_contract VALUES(1,1,NULL);
 FOREACH reason_value IN ARRAY ARRAY[2,3,4,5,6,7,8,9,10,11,52]::smallint[] LOOP
  INSERT INTO native_step_server_contract VALUES(NULL,reason_value,NULL);
 END LOOP;
 BEGIN
  INSERT INTO native_step_server_contract VALUES(NULL,1,NULL);
  RAISE EXCEPTION 'ordinary NULL Exit was accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
  INSERT INTO native_step_server_contract VALUES(NULL,1,'{}'::jsonb);
  RAISE EXCEPTION 'unsealed STEP Exit was accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
  INSERT INTO native_step_server_contract VALUES(1,52,NULL);
  RAISE EXCEPTION 'Soulpit terminal was changed into a client command';
 EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN
  INSERT INTO native_step_server_contract VALUES(NULL,12,NULL);
  RAISE EXCEPTION 'unallocated server reason was accepted';
 EXCEPTION WHEN check_violation THEN NULL; END;
END $legacy$;
"###;
