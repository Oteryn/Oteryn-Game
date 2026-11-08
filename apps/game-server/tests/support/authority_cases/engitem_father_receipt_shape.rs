//! The generic sixteen-track capacity must preserve Father's exact historical exception.
use crate::bestiary_postgres_harness as f;

#[test]
fn engitem_father_receipt_shape_postgres() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "engitem_father_shape", true).await?;
                    let result = exercise(&h.pool).await;
                    h.cleanup().await?;
                    eprintln!(
                        "ENGITEM_PG_EXECUTED father_receipt_shape duration={:?} success={}",
                        started.elapsed(),
                        result.is_ok()
                    );
                    result
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "Father receipt shape PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(pool: &sqlx::PgPool) -> f::TestResult {
    let expression: String = sqlx::query_scalar("SELECT string_agg('('||pg_get_expr(conbin,conrelid)||')',' AND ' ORDER BY conname) FROM pg_constraint WHERE conrelid='game_character_quest_receipts'::regclass AND conname IN ('game_character_quest_receipt_effects_shape','game_character_quest_father_receipt_shape')").fetch_one(pool).await?;
    // Only the installed CHECK from the isolated fixture is interpolated. All receipt data is bound.
    let query = format!(
        "SELECT {expression} FROM (SELECT $1::text AS quest_key,$2::text AS transition_key,'command'::text AS cause_kind,false AS completes,$3::text[] AS track_keys,$4::bigint[] AS values_before,$5::bigint[] AS values_after) AS receipt"
    );
    let father = "oteryn:quest.a_father_s_burden_quest";
    let accept = "oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept";
    let tracks: Vec<_> = [
        "cloth",
        "crystal",
        "iron",
        "progress",
        "quest_log",
        "root",
        "scale",
        "silk",
        "sinew",
        "status",
        "wood",
    ]
    .into_iter()
    .map(|key| format!("oteryn:quest-progress/quest/u8_6/afathers_burden/{key}"))
    .collect();
    let after = [1_i64, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1];
    let check = |quest: &str, transition: &str, keys: Vec<String>, values: Vec<i64>| {
        let count = keys.len();
        sqlx::query_scalar::<_, bool>(sqlx::AssertSqlSafe(query.as_str()))
            .bind(quest.to_owned())
            .bind(transition.to_owned())
            .bind(keys)
            .bind(vec![0_i64; count])
            .bind(values)
    };
    assert!(
        check(father, accept, tracks.clone(), after.to_vec())
            .fetch_one(pool)
            .await?
    );
    assert!(
        !check(father, accept, tracks[..9].to_vec(), vec![1; 9])
            .fetch_one(pool)
            .await?,
        "generic capacity must not widen Father's exact exception"
    );
    assert!(
        !check(father, accept, tracks.clone(), vec![1; 11])
            .fetch_one(pool)
            .await?,
        "the exact after-values remain mandatory"
    );
    assert!(
        !check(
            father,
            "oteryn:quest-transition/alias",
            tracks,
            after.to_vec()
        )
        .fetch_one(pool)
        .await?,
        "the exception remains bound to mission-accept"
    );
    for (count, expected) in [(8, true), (16, true), (17, false)] {
        let keys = (0..count)
            .map(|i| format!("oteryn:quest-progress/generic/{i}"))
            .collect();
        assert_eq!(
            check(
                "oteryn:quest.generic",
                "oteryn:quest-transition/generic",
                keys,
                vec![1; count]
            )
            .fetch_one(pool)
            .await?,
            expected
        );
    }
    Ok(())
}
