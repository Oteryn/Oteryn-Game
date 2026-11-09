//! Runtime parameter-result replay keeps its row lock without mutable outbox authority.
use crate::bestiary_postgres_harness as fixture;

#[test]
fn spell_parameter_result_runtime_lock_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let h = fixture::Harness::create(admin, "parameter_result_lock", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "parameter result lock thread failed")??;
    Ok(())
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    // A committed refused cast has no cost receipt. The real migration still
    // proves its Character/World and stamps the physical transaction identity.
    sqlx::query(
        "INSERT INTO game_spell_parameter_results(\
         game_session_id,command_id,character_id,world_id,scope_generation,\
         catalog_digest,spell_key,spell_revision,intent,intent_digest,\
         result,result_digest,cast_succeeded) \
         VALUES(encode($1,'hex')::uuid,100,encode($2,'hex')::uuid,\
         encode($3,'hex')::uuid,1,decode(repeat('11',32),'hex'),\
         'oteryn:spell.find-person','r20',\
         decode('0802120408011001','hex'),sha256(decode('0802120408011001','hex')),\
         decode('080212020802','hex'),sha256(decode('080212020802','hex')),false)",
    )
    .bind(fixture::id(fixture::SESSION).as_slice())
    .bind(fixture::id(fixture::CHARACTER).as_slice())
    .bind(fixture::id(fixture::WORLD).as_slice())
    .execute(&h.pool)
    .await?;

    let mut reader = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *reader)
        .await?;
    for _ in 0..2 {
        let result: (String, bool, Option<String>) = sqlx::query_as(
            "SELECT encode(result,'hex'),created_xact_id=pg_current_xact_id(),\
             pg_xact_status(created_xact_id) FROM game_spell_parameter_results \
             WHERE game_session_id=encode($1,'hex')::uuid AND command_id=100 FOR SHARE",
        )
        .bind(fixture::id(fixture::SESSION).as_slice())
        .fetch_one(&mut *reader)
        .await?;
        assert_eq!(
            result,
            ("080212020802".into(), false, Some("committed".into()))
        );
    }
    let privileges: (bool, bool, bool, bool) = sqlx::query_as(
        "SELECT has_table_privilege(current_user,'game_spell_parameter_results','SELECT'),\
         has_table_privilege(current_user,'game_spell_parameter_results','INSERT'),\
         has_table_privilege(current_user,'game_spell_parameter_results','UPDATE'),\
         has_table_privilege(current_user,'game_spell_parameter_results','DELETE')",
    )
    .fetch_one(&mut *reader)
    .await?;
    assert_eq!(privileges, (true, true, false, false));
    let update_columns: Vec<String> = sqlx::query_scalar(
        "SELECT attname::text FROM pg_attribute \
         WHERE attrelid='game_spell_parameter_results'::regclass \
         AND attnum>0 AND NOT attisdropped \
         AND has_column_privilege(current_user,attrelid,attnum,'UPDATE') ORDER BY attnum",
    )
    .fetch_all(&mut *reader)
    .await?;
    assert_eq!(update_columns, ["game_session_id"]);

    for (statement, code) in [
        (
            "UPDATE game_spell_parameter_results SET game_session_id=game_session_id \
             WHERE game_session_id=encode($1,'hex')::uuid AND command_id=100",
            "23514",
        ),
        (
            "UPDATE game_spell_parameter_results SET result=result \
             WHERE game_session_id=encode($1,'hex')::uuid AND command_id=100",
            "42501",
        ),
        (
            "DELETE FROM game_spell_parameter_results \
             WHERE game_session_id=encode($1,'hex')::uuid AND command_id=100",
            "42501",
        ),
    ] {
        sqlx::query("SAVEPOINT rejected_write")
            .execute(&mut *reader)
            .await?;
        let rejected = sqlx::query(statement)
            .bind(fixture::id(fixture::SESSION).as_slice())
            .execute(&mut *reader)
            .await;
        assert_eq!(database_code(rejected.err()).as_deref(), Some(code));
        sqlx::query("ROLLBACK TO SAVEPOINT rejected_write")
            .execute(&mut *reader)
            .await?;
    }

    // The successful replay retains its actual shared lock, not only permission
    // to issue the query. An independent writer cannot obtain the same row.
    let mut writer = h.pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout='100ms'")
        .execute(&mut *writer)
        .await?;
    let blocked = sqlx::query(
        "SELECT 1 FROM game_spell_parameter_results \
         WHERE game_session_id=encode($1,'hex')::uuid AND command_id=100 FOR UPDATE",
    )
    .bind(fixture::id(fixture::SESSION).as_slice())
    .execute(&mut *writer)
    .await;
    assert_eq!(database_code(blocked.err()).as_deref(), Some("55P03"));
    writer.rollback().await?;
    reader.rollback().await?;
    eprintln!(
        "PARAMETER_RESULT_LOCK PG EXECUTED: runtime replay, narrow grant, immutable writes and lock conflict"
    );
    Ok(())
}

fn database_code(error: Option<sqlx::Error>) -> Option<String> {
    error.and_then(|error| {
        error
            .as_database_error()
            .and_then(|error| error.code())
            .map(|code| code.into_owned())
    })
}
