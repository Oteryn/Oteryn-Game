//! Focused reward migration qualification against a fresh PostgreSQL database.
//! Uses the standard embedded migrator and removes the temporary database.
use sqlx::{Connection, Executor};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = std::time::Instant::now();
    assert!(
        oteryn_gameplay_durability::GAME_MIGRATOR
            .iter()
            .any(|m| m.version == 1127)
    );
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let admin_url=std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
        let name=format!("wevents_migration_{}",std::process::id());
        let mut admin=sqlx::PgConnection::connect(&admin_url).await?;
        admin.execute(sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))).await?;
        let url=format!("{}/{}",admin_url.strip_suffix("/postgres").ok_or("admin database")?,name);
        let result: Result<(),Box<dyn std::error::Error>>=async {
            let mut db=sqlx::PgConnection::connect(&url).await?;
            oteryn_gameplay_durability::GAME_MIGRATOR.run(&mut db).await?;
            let body:String=sqlx::query_scalar("SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure)").fetch_one(&mut db).await?;
            assert!(body.contains("r.cause_kind IN ('command','use','step')"));
            db.execute(sqlx::raw_sql("CREATE TEMP TABLE wevents_reward_probe (LIKE game_quest_reward_sources INCLUDING ALL); CREATE CONSTRAINT TRIGGER probe_complete AFTER INSERT ON wevents_reward_probe DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_complete();")).await?;
            db.execute("BEGIN").await?;
            db.execute("INSERT INTO wevents_reward_probe(character_id,cause_id,cause_ordinal,transition_key,plan,grant_keys) VALUES('00000000-0000-7000-8000-000000000001','00000000-0000-7000-8000-000000000002',1,'oteryn:test.reward','{\"key\":\"oteryn:test.reward\",\"quest\":\"oteryn:quest.test\",\"take\":[],\"give\":null,\"outfit\":null}','{}')").await?;
            let Err(error) = db.execute("COMMIT").await else {
                return Err("reward admitted without a quest receipt".into());
            };
            if !error.to_string().contains("quest reward lacks same-transaction quest receipt") {
                return Err(error.into());
            }
            db.close().await?;
            Ok(())
        }.await;
        admin.execute(sqlx::query(sqlx::AssertSqlSafe(format!("DROP DATABASE {name} WITH(FORCE)")))).await?;
        result
    })?;
    println!(
        "WEVENTS PG EXECUTED: migration 1127 and missing-receipt guard duration={:?}",
        started.elapsed()
    );
    Ok(())
}
