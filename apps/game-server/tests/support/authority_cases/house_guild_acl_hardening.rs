use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, WORLD, debug, id, runtime,
};
use sqlx::{Acquire, PgConnection};

const MIGRATION: &str = include_str!("../../../migrations/2986_house_guild_acl_hardening.sql");
const HOUSE: &str = "oteryn:content.house.guild_acl_hardening";

fn run_case(
    admin: String,
    tag: &'static str,
    check: impl AsyncFnOnce(&Harness) -> TestResult + Send + 'static,
) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(async {
                    let h = Harness::create(admin, tag, false).await?;
                    let result = check(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "House guild ACL PostgreSQL case panicked")?
        .map_err(Into::into)
}

async fn seed_house(tx: &mut PgConnection) -> TestResult {
    // The shared fixture opens a reconnectable session. Seed a current holder for
    // the real ACL port, then re-enable the normal proof triggers before calling it.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2 WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(id(SESSION).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await?;
    sqlx::query("WITH property AS (
        INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest,
         owner_character_id,owner_account_id,acquisition_receipt_ref,paid_until)
        SELECT world_id,$1,'private_house',3,100,decode(repeat('01',32),'hex'),
         character_id,account_id,game_character_uuid_v7(),statement_timestamp()+interval '30 days'
        FROM game_character_roots WHERE character_id=encode($2,'hex')::uuid RETURNING *
    ), slot AS (
        INSERT INTO game_account_housing_slots(account_id,world_id,house_key)
        SELECT owner_account_id,world_id,house_key FROM property
    )
    INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref)
    SELECT world_id,house_key,owner_character_id,ownership_generation,acquisition_receipt_ref FROM property")
        .bind(HOUSE)
        .bind(id(CHARACTER).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(tx)
        .await?;
    Ok(())
}

async fn advance_acl(tx: &mut PgConnection, list_key: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO game_house_management_receipts
        (game_session_id,command_id,character_id,world_id,house_key,request_binding,action_kind,list_key,result_revision)
        VALUES(encode($1,'hex')::uuid,2,encode($2,'hex')::uuid,encode($3,'hex')::uuid,
         $4,decode(repeat('01',32),'hex'),1,$5,2)")
        .bind(id(SESSION).as_slice())
        .bind(id(CHARACTER).as_slice())
        .bind(id(WORLD).as_slice())
        .bind(HOUSE)
        .bind(list_key)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "SELECT game_house_management_guild_acl_advance(encode($1,'hex')::uuid,2,1,'[]'::jsonb)",
    )
    .bind(id(SESSION).as_slice())
    .execute(tx)
    .await?;
    Ok(())
}

#[test]
fn house_guild_acl_hardening_reapplies_with_legacy_door_rows() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "house_acl_reapply", async |h| {
        let mut tx = h.pool.begin().await?;
        // Recreate the predecessor schema inside the transaction, then use its
        // normal ACL writer to model a retained row beyond the new wire range.
        sqlx::query("ALTER TABLE game_house_acl_guild_ranks DROP CONSTRAINT game_house_acl_guild_ranks_door_wire_range")
            .execute(&mut *tx).await?;
        seed_house(&mut tx).await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        advance_acl(&mut tx, "door:65536:0:0").await?;
        sqlx::query("RESET ROLE").execute(&mut *tx).await?;

        let constraint = "SELECT oid::text, convalidated FROM pg_constraint
            WHERE conrelid='game_house_acl_guild_ranks'::regclass
              AND conname='game_house_acl_guild_ranks_door_wire_range'";
        sqlx::raw_sql(MIGRATION).execute(&mut *tx).await?;
        let installed: (String, bool) = sqlx::query_as(constraint).fetch_one(&mut *tx).await?;
        assert!(!installed.1);
        sqlx::raw_sql(MIGRATION).execute(&mut *tx).await?;
        let repeated: (String, bool) = sqlx::query_as(constraint).fetch_one(&mut *tx).await?;
        assert_eq!(repeated, installed);
        let list_key: String = sqlx::query_scalar(
            "SELECT list_key FROM game_house_acl_guild_ranks WHERE house_key=$1",
        )
        .bind(HOUSE)
        .fetch_one(&mut *tx)
        .await?;
        assert_eq!(list_key, "door:65536:0:0");
        tx.rollback().await?;
        Ok(())
    })
}

#[test]
fn house_guild_rank_identity_is_assigned_by_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "house_acl_identity", async |h| {
        let mut tx = h.pool.begin().await?;
        // Isolate the SQL role boundary from Guild command proofs, which have
        // their own cases. Checks, generated columns and grants remain active.
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let guild: String = sqlx::query_scalar("INSERT INTO game_guilds
            (guild_id,world_id,name,state,formation_deadline,revision,created_at)
            VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'ACL Test Guild',2,9999999999999,1,1)
            RETURNING guild_id::text")
            .bind(id(WORLD).as_slice()).fetch_one(&mut *tx).await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;

        let identity: i64 = sqlx::query_scalar(
            "INSERT INTO game_guild_ranks(guild_id,level,name)
            VALUES($1::uuid,1,'Leader') RETURNING house_acl_identity",
        )
        .bind(&guild)
        .fetch_one(&mut *tx)
        .await?;
        assert!(identity > 0);
        let edited: i64 = sqlx::query_scalar("INSERT INTO game_guild_ranks(guild_id,level,name)
            VALUES($1::uuid,1,'Captain') ON CONFLICT(guild_id,level) DO UPDATE SET name=EXCLUDED.name
            RETURNING house_acl_identity")
            .bind(&guild).fetch_one(&mut *tx).await?;
        assert_eq!(edited, identity);

        for statement in [
            "INSERT INTO game_guild_ranks(guild_id,level,name,house_acl_identity)
                OVERRIDING SYSTEM VALUE VALUES($1::uuid,2,'Member',99)",
            "UPDATE game_guild_ranks SET house_acl_identity=DEFAULT WHERE guild_id=$1::uuid AND level=1",
        ] {
            let mut attempt = tx.begin().await?;
            let result = sqlx::query(sqlx::AssertSqlSafe(statement))
                .bind(&guild)
                .execute(&mut *attempt)
                .await;
            attempt.rollback().await?;
            let error = result.err().ok_or("runtime supplied a rank identity")?;
            let error = error
                .as_database_error()
                .ok_or("expected PostgreSQL permission error")?;
            assert_eq!(error.code().as_deref(), Some("42501"));
        }
        tx.rollback().await?;
        Ok(())
    })
}

#[test]
fn house_guild_acl_door_keys_follow_wire_bounds() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "house_acl_door_bounds", async |h| {
        let mut tx = h.pool.begin().await?;
        seed_house(&mut tx).await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        for key in ["guest", "subowner", "door:0:0:0", "door:65535:65535:-15"] {
            let mut attempt = tx.begin().await?;
            advance_acl(&mut attempt, key).await?;
            attempt.rollback().await?;
        }
        for key in [
            "door:65536:0:0",
            "door:0:65536:0",
            "door:0:0:-16",
            "door:0:0:1",
        ] {
            let mut attempt = tx.begin().await?;
            let result = advance_acl(&mut attempt, key).await;
            attempt.rollback().await?;
            let error = result
                .err()
                .ok_or_else(|| format!("out-of-range list key accepted: {key}"))?;
            let error = error
                .as_database_error()
                .ok_or("expected PostgreSQL check error")?;
            assert_eq!(error.code().as_deref(), Some("23514"), "{key}");
            assert_eq!(
                error.constraint(),
                Some("game_house_acl_guild_ranks_door_wire_range"),
                "{key}"
            );
        }
        tx.rollback().await?;
        Ok(())
    })
}
