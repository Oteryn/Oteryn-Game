//! Main's filled rewards retain exact child equality through questpc8's proof.
use super::TestResult;
use super::fixture::{Harness, run};

#[test]
fn questpc8_fluid_outputs_preserve_exact_subtype_and_legacy_plan_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(
        "questpc8-fluid-output-proof",
        || {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "questpc8_fluid_outputs",
                    false,
                    false,
                )
                .await?;
                eprintln!(
                    "POSTGRES_CASE_STARTED questpc8_fluid_outputs after PostgreSQL fixture initialization"
                );
                sqlx::query(r#"
DO $probe$
DECLARE body text; expr text; projected jsonb; sample jsonb;
BEGIN
 body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
 expr:=substring(body from 'AND \(SELECT (COALESCE\(jsonb_agg[\s\S]+?)\s+FROM jsonb_array_elements\(m.contents_plan\) e\)');
 IF expr IS NULL THEN RAISE EXCEPTION 'installed exact-output projection missing'; END IF;
 FOR sample IN SELECT value FROM jsonb_array_elements('[
  {"input":{"production_key":"oteryn:item.tibia.i2901","quantity":1,"ordinal":0,"fluid_subtype":16},"expected":[{"item":"oteryn:item.tibia.i2901","count":1,"fluid_subtype":16}],"matches":true},
  {"input":{"production_key":"oteryn:item.tibia.i2901","quantity":1,"ordinal":0,"fluid_subtype":17},"expected":[{"item":"oteryn:item.tibia.i2901","count":1,"fluid_subtype":17}],"matches":false},
  {"input":{"production_key":"oteryn:item.tibia.i2901","quantity":2,"ordinal":0,"fluid_subtype":16},"expected":[{"item":"oteryn:item.tibia.i2901","count":2,"fluid_subtype":16}],"matches":false},
  {"input":{"production_key":"oteryn:item.tibia.i2901","quantity":1,"ordinal":0},"expected":[{"item":"oteryn:item.tibia.i2901","count":1}],"matches":false}
 ]'::jsonb) LOOP
  EXECUTE 'SELECT '||expr||' FROM (VALUES ('||quote_literal(jsonb_build_array(sample->'input')::text)||'::jsonb)) m(contents_plan) CROSS JOIN LATERAL jsonb_array_elements(m.contents_plan) e' INTO projected;
  IF projected IS DISTINCT FROM sample->'expected' THEN
   RAISE EXCEPTION 'installed projection lost an exact authored output: %',projected;
  END IF;
  IF (projected='[{"item":"oteryn:item.tibia.i2901","count":1,"fluid_subtype":16}]'::jsonb)
      IS DISTINCT FROM (sample->>'matches')::boolean THEN
   RAISE EXCEPTION 'wrong fluid, quantity or empty vessel matched authored mead';
  END IF;
 END LOOP;
END $probe$;
"#).execute(&h.pool).await?;
                eprintln!("POSTGRES_EXACT_FLUID_OUTPUTS_LEGACY_AND_REFUSALS_PASS");
                Ok(())
            })
        },
    );
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_fluid_outputs duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}
