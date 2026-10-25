//! HUNT-1 schema qualification; no unadmitted draw or direct runtime writer.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, TestResult, configured_admin, id, runtime,
};
use sqlx::{Connection, Row};

#[test]
fn prey_storage_roundtrip_defaults_rollback_and_role_grants() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "prey_storage_roundtrip", false).await?;
        let result = roundtrip(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn roundtrip(h: &Harness) -> TestResult {
    eprintln!("HUNT-1 POSTGRES_DATABASE_STARTED PostgreSQL=180006");
    assert_eq!(h.count("game_character_prey_slots").await?, 0);
    sqlx::query("INSERT INTO game_character_prey_slots(character_id,slot,state,hunting_s_left,slot_epoch) VALUES(encode($1,'hex')::uuid,1,'SELECTION',0,1)")
        .bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    let defaults: (Vec<String>, String, i64) = sqlx::query_as(
        "SELECT creature_list,option,free_reroll_at FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid AND slot=1")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(defaults, (vec![], "NONE".into(), 0));
    let list = vec![
        "oteryn:creature.rat".to_owned(),
        "oteryn:creature.dragon".to_owned(),
    ];
    sqlx::query("UPDATE game_character_prey_slots SET creature_list=$1,option='LOCK',free_reroll_at=72005 WHERE character_id=encode($2,'hex')::uuid AND slot=1")
        .bind(&list).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    // A new connection observes exactly the retained per-Character projection.
    let mut connection = sqlx::PgConnection::connect_with(&h.pool.connect_options()).await?;
    let stored: (Vec<String>, String, i64) = sqlx::query_as(
        "SELECT creature_list,option,free_reroll_at FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid AND slot=1")
        .bind(id(CHARACTER).as_slice()).fetch_one(&mut connection).await?;
    assert_eq!(stored, (list, "LOCK".into(), 72005));
    drop(connection);
    let mut tx = h.pool.begin().await?;
    sqlx::query("UPDATE game_character_prey_slots SET option='NONE',free_reroll_at=90000 WHERE character_id=encode($1,'hex')::uuid AND slot=1")
        .bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
    tx.rollback().await?;
    let after: (String, i64) = sqlx::query_as("SELECT option,free_reroll_at FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid AND slot=1")
        .bind(id(CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after, ("LOCK".into(), 72005));
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_one(&mut *tx)
    .await?;
    assert_eq!(count, 1);
    let privileges = sqlx::query("SELECT has_table_privilege(current_user,'game_character_prey_slots','INSERT') AS insert_allowed,has_any_column_privilege(current_user,'game_character_prey_slots','UPDATE') AS update_allowed,has_table_privilege(current_user,'game_account_prey_unlocks','INSERT') AS unlock_allowed")
        .fetch_one(&mut *tx).await?;
    assert!(!privileges.try_get::<bool, _>("insert_allowed")?);
    assert!(!privileges.try_get::<bool, _>("update_allowed")?);
    assert!(!privileges.try_get::<bool, _>("unlock_allowed")?);
    tx.rollback().await?;
    // No state write here advances the Character root or manufactures a receipt.
    assert_eq!(h.root_revision().await?, "1");
    Ok(())
}

#[test]
fn prey_storage_rejects_invalid_bounds_and_preserves_all_bonus_grades() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "prey_storage_bounds", false).await?;
        let result = bounds(&h).await;
        h.cleanup().await?;
        result
    })
}

async fn bounds(h: &Harness) -> TestResult {
    let empty: Vec<String> = vec![];
    for list in [
        empty,
        (0..9).map(|n| format!("oteryn:creature.{n}")).collect(),
        vec!["a".repeat(256)],
    ] {
        let valid: bool = sqlx::query_scalar("SELECT game_prey_creature_list_valid($1)")
            .bind(list)
            .fetch_one(&h.pool)
            .await?;
        assert!(valid);
    }
    for list in [
        vec!["".into()],
        vec!["a".repeat(257)],
        vec!["rat".into(), "rat".into()],
        (0..10).map(|n| format!("oteryn:creature.{n}")).collect(),
    ] {
        let mut tx = h.pool.begin().await?;
        let rejected = sqlx::query("INSERT INTO game_character_prey_slots(character_id,slot,state,hunting_s_left,slot_epoch,creature_list) VALUES(encode($1,'hex')::uuid,1,'SELECTION',0,1,$2)")
            .bind(id(CHARACTER).as_slice()).bind(list).execute(&mut *tx).await;
        assert!(
            matches!(rejected, Err(ref e) if e.as_database_error().and_then(|e| e.code()).as_deref()==Some("23514"))
        );
        tx.rollback().await?;
    }
    for expression in [
        "ARRAY[NULL]::text[]",
        "ARRAY[['rat','dragon']]",
        "'[0:0]={rat}'::text[]",
    ] {
        let valid: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT game_prey_creature_list_valid({expression})"
        )))
        .fetch_one(&h.pool)
        .await?;
        assert!(!valid);
    }
    for (kind, step, base) in [
        ("DAMAGE", 2, 5),
        ("REDUCTION", 2, 10),
        ("XP", 3, 10),
        ("LOOT", 3, 10),
    ] {
        for grade in 1_i16..=10 {
            let mut tx = h.pool.begin().await?;
            sqlx::query("INSERT INTO game_character_prey_slots(character_id,slot,state,creature_key,bonus_type,grade,bonus_percent,hunting_s_left,slot_epoch) VALUES(encode($1,'hex')::uuid,1,'ACTIVE','oteryn:creature.rat',$2,$3,$4,7200,1)")
                .bind(id(CHARACTER).as_slice()).bind(kind).bind(grade).bind(step * grade + base).execute(&mut *tx).await?;
            let wrong = sqlx::query("UPDATE game_character_prey_slots SET bonus_percent=bonus_percent+1 WHERE character_id=encode($1,'hex')::uuid AND slot=1")
                .bind(id(CHARACTER).as_slice()).execute(&mut *tx).await;
            assert!(
                matches!(wrong, Err(ref e) if e.as_database_error().and_then(|e| e.code()).as_deref()==Some("23514"))
            );
            tx.rollback().await?;
        }
    }
    for assignment in [
        "option='UNKNOWN'",
        "free_reroll_at=-1",
        "hunting_s_left=7201",
        "slot=4",
    ] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("INSERT INTO game_character_prey_slots(character_id,slot,state,hunting_s_left,slot_epoch) VALUES(encode($1,'hex')::uuid,1,'SELECTION',0,1)")
            .bind(id(CHARACTER).as_slice()).execute(&mut *tx).await?;
        let rejected = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE game_character_prey_slots SET {assignment} WHERE character_id=encode($1,'hex')::uuid")))
            .bind(id(CHARACTER).as_slice()).execute(&mut *tx).await;
        assert!(
            matches!(rejected, Err(ref e) if e.as_database_error().and_then(|e| e.code()).as_deref()==Some("23514"))
        );
        tx.rollback().await?;
    }
    assert_eq!(h.count("game_character_prey_slots").await?, 0);
    Ok(())
}
