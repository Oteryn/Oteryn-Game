//! Predicate boundary only: historical unexpected contents cannot be orphaned.
//! This does not claim that a fixture imported its corpse through a native kill.
use super::TestResult;
use super::fixture::{Harness, id, run};

#[test]
fn srx3_elder_generic_terminal_refuses_direct_and_nested_contents_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-elder-terminal", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "srx3_elder_terminal", false, false)
                .await?;
            let mut prior = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *prior)
                .await?;
            for seed in 200..=204_u8 {
                let definition = if seed == 202 || seed == 204 {
                    "oteryn:item.tibia.i3031"
                } else {
                    "oteryn:item.tibia.i43667"
                };
                sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
                    .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(definition).bind(id(210).as_slice()).execute(&mut *prior).await?;
            }
            sqlx::query("INSERT INTO game_item_corpse_container_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                .bind(id(202).as_slice()).bind(id(42).as_slice()).bind(id(201).as_slice()).bind(id(211).as_slice()).execute(&mut *prior).await?;
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                .bind(id(204).as_slice()).bind(id(42).as_slice()).bind(id(203).as_slice()).bind(id(212).as_slice()).execute(&mut *prior).await?;
            prior.commit().await?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            for (seed, target, expected) in [
                (200, "oteryn:item.tibia.i43666", true),
                (201, "oteryn:item.tibia.i43666", false),
                (203, "oteryn:item.tibia.i43666", false),
                (201, "oteryn:item.tibia.i43668", true),
            ] {
                let actual: bool = sqlx::query_scalar(
                    "SELECT game_source_corpse_terminal_contents_v1(encode($1,'hex')::uuid,$2)",
                )
                .bind(id(seed).as_slice())
                .bind(target)
                .fetch_one(&mut *tx)
                .await?;
                assert_eq!(
                    actual, expected,
                    "actual production predicate seed{seed} target{target}"
                );
            }
            tx.rollback().await?;
            assert!(sqlx::query_scalar::<_,bool>("SELECT position('game_source_corpse_terminal_contents_v1' IN pg_get_functiondef('game_spell_field_temporal_receipt_proven()'::regprocedure))>0").fetch_one(&h.pool).await?,"the real existing timer writer must consume this predicate");
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND quantity=1"
                )
                .fetch_one(&h.pool)
                .await?,
                5
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT (SELECT count(*) FROM game_item_corpse_container_entries)+(SELECT count(*) FROM game_item_nested_entries)").fetch_one(&h.pool).await?,2);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_ELDER_TERMINAL_PREDICATE_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
