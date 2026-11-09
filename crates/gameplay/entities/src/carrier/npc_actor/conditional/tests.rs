use super::*;
use crate::MovementFacing;
use crate::carrier::tests::runtime;

fn placement(x: u16) -> NpcPlacement {
    NpcPlacement {
        x,
        y: 100,
        floor: -7,
        direction: MovementFacing::South,
    }
}

#[test]
fn conditional_npc_retirement_preserves_other_same_key_occurrences() -> Result<(), NpcPlacementError>
{
    let mut runtime = runtime(8);
    let key = "oteryn:npc.rascalio";
    let unrelated = runtime.place_dynamic_npc(key, placement(102))?;
    let mut projection = ConditionalNpcActor::new(key.into(), placement(101))?;
    let owned = projection
        .reconcile(&mut runtime, true)?
        .ok_or(NpcPlacementError::NoPlacements)?;
    assert_eq!(projection.reconcile(&mut runtime, true)?, Some(owned));
    assert_eq!(projection.reconcile(&mut runtime, false)?, None);
    assert_eq!(
        runtime.npc_key(unrelated),
        Some(key),
        "another quest's copy must survive"
    );
    assert_eq!(
        runtime.npc_key(owned),
        None,
        "only this conditional occurrence is retired"
    );
    Ok(())
}

#[test]
fn conditional_npc_stale_cleanup_preserves_a_reused_slot() -> Result<(), NpcPlacementError> {
    let mut runtime = runtime(8);
    let mut projection = ConditionalNpcActor::new("oteryn:npc.minzy".into(), placement(101))?;
    let old = projection
        .reconcile(&mut runtime, true)?
        .ok_or(NpcPlacementError::NoPlacements)?;
    runtime.remove_dynamic_npc_actor(old)?;
    let replacement = runtime.place_dynamic_npc("oteryn:npc.hoaxette", placement(101))?;
    projection.reconcile(&mut runtime, false)?;
    assert_eq!(runtime.npc_key(replacement), Some("oteryn:npc.hoaxette"));
    assert_eq!(
        projection.reconcile(&mut runtime, true),
        Err(NpcPlacementError::CellOccupied)
    );
    runtime.remove_dynamic_npc_actor(replacement)?;
    let fresh = projection
        .reconcile(&mut runtime, true)?
        .ok_or(NpcPlacementError::NoPlacements)?;
    assert_ne!(fresh, old);
    Ok(())
}

#[test]
fn conditional_npc_return_waits_for_its_anchor_without_replacing_the_occupant()
-> Result<(), NpcPlacementError> {
    let mut runtime = runtime(8);
    let blocker = runtime.place_dynamic_npc("oteryn:npc.rascalio", placement(101))?;
    let mut projection = ConditionalNpcActor::new("oteryn:npc.hoaxette".into(), placement(101))?;
    assert_eq!(projection.reconcile_or_defer(&mut runtime, true)?, None);
    assert_eq!(runtime.npc_key(blocker), Some("oteryn:npc.rascalio"));
    runtime.remove_dynamic_npc_actor(blocker)?;
    let returned = projection
        .reconcile_or_defer(&mut runtime, true)?
        .ok_or(NpcPlacementError::NoPlacements)?;
    assert_eq!(
        projection.reconcile_or_defer(&mut runtime, true)?,
        Some(returned)
    );
    projection.reconcile_or_defer(&mut runtime, false)?;
    assert_eq!(runtime.npc_key(returned), None);
    Ok(())
}

#[test]
fn conditional_npc_deferred_return_does_not_hide_capacity_failure() -> Result<(), NpcPlacementError>
{
    let mut runtime = runtime(1);
    runtime.place_dynamic_npc("oteryn:npc.rascalio", placement(102))?;
    let mut projection = ConditionalNpcActor::new("oteryn:npc.hoaxette".into(), placement(101))?;
    assert_eq!(
        projection.reconcile_or_defer(&mut runtime, true),
        Err(NpcPlacementError::Carrier(
            super::super::CarrierError::CapacityExceeded
        ))
    );
    Ok(())
}

use sqlx::{AssertSqlSafe, Executor};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn conditional_npc_postgres_committed_availability_and_independent_occurrences() -> TestResult {
    let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("NPCFIX_M38_POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    };
    let started = Instant::now();
    let result = tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let pool = sqlx::postgres::PgPoolOptions::new().max_connections(2).connect(&url).await?;
        // Identifier contains only this fixed prefix and decimal process/time values.
        let schema = format!("npcfix_m38_{}_{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos());
        pool.execute(sqlx::raw_sql(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))).await?;
        eprintln!("POSTGRES_STARTED NPCFIX_M38 conditional NPC occurrence projection");
        let result: TestResult = async {
            pool.execute(sqlx::raw_sql(AssertSqlSafe(format!("CREATE TABLE {schema}.availability (singleton boolean PRIMARY KEY CHECK(singleton), present boolean NOT NULL, epoch bigint NOT NULL CHECK(epoch>=0))")))).await?;
            pool.execute(sqlx::raw_sql(AssertSqlSafe(format!("INSERT INTO {schema}.availability VALUES(true,false,0)")))).await?;
            let mut runtime = runtime(16);
            let key = "oteryn:npc.rascalio";
            let unrelated = runtime.place_dynamic_npc(key, placement(110)).map_err(|e| format!("{e:?}"))?;
            let mut projections = (101..105).map(|x| ConditionalNpcActor::new(key.into(), placement(x)))
                .collect::<Result<Vec<_>, _>>().map_err(|e| format!("{e:?}"))?;
            let select = format!("SELECT present FROM {schema}.availability WHERE singleton");
            let update = format!("UPDATE {schema}.availability SET present=$1,epoch=epoch+1 WHERE singleton");
            let mut previous = false;
            for epoch in 0..8 {
                let present = epoch % 2 == 0;
                let mut transaction = pool.begin().await?;
                sqlx::query(AssertSqlSafe(update.as_str())).bind(present).execute(&mut *transaction).await?;
                let uncommitted: bool = sqlx::query_scalar(AssertSqlSafe(select.as_str())).fetch_one(&pool).await?;
                if uncommitted != previous { return Err("uncommitted availability escaped the PostgreSQL owner".into()); }
                transaction.commit().await?;
                let committed: bool = sqlx::query_scalar(AssertSqlSafe(select.as_str())).fetch_one(&pool).await?;
                if committed != present { return Err("committed availability snapshot missing".into()); }
                if epoch == 0 {
                    let blocker = runtime.place_dynamic_npc("oteryn:npc.hoaxette", placement(101))
                        .map_err(|e| format!("{e:?}"))?;
                    assert_eq!(projections[0].reconcile_or_defer(&mut runtime, committed)
                        .map_err(|e| format!("{e:?}"))?, None);
                    assert_eq!(runtime.npc_key(blocker), Some("oteryn:npc.hoaxette"));
                    // One blocked return must not stop the other committed occurrences.
                    for projection in &mut projections[1..] {
                        assert!(projection.reconcile_or_defer(&mut runtime, committed)
                            .map_err(|e| format!("{e:?}"))?.is_some());
                    }
                    runtime.remove_dynamic_npc_actor(blocker).map_err(|e| format!("{e:?}"))?;
                }
                let mut refs = Vec::new();
                for projection in &mut projections {
                    refs.push(projection.reconcile(&mut runtime, committed).map_err(|e| format!("{e:?}"))?);
                }
                // Replayed committed snapshots must neither duplicate nor replace actors.
                for _ in 0..32 {
                    let replayed: bool = sqlx::query_scalar(AssertSqlSafe(select.as_str())).fetch_one(&pool).await?;
                    for (projection, actor) in projections.iter_mut().zip(&refs) {
                        if projection.reconcile(&mut runtime, replayed).map_err(|e| format!("{e:?}"))? != *actor {
                            return Err("snapshot replay replaced a conditional actor".into());
                        }
                    }
                }
                if runtime.npc_key(unrelated) != Some(key) {
                    return Err("conditional retirement removed another quest's same-key NPC occurrence".into());
                }
                let expected = if present { 5 } else { 1 };
                if runtime.visible_entities().npcs.len() != expected {
                    return Err("conditional NPC actor count does not match committed availability".into());
                }
                previous = present;
            }
            Ok(())
        }.await;
        let cleanup = pool.execute(sqlx::raw_sql(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))).await;
        pool.close().await;
        cleanup?;
        result
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38 conditional NPC occurrence projection duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
