//! MARKET-1: installed migration and real depot-admitted command route.
use super::market_fixtures::{scenario, snapshot};

use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};

use crate::foundation::{CommandId, CommandRef};

use oteryn_protocol_oteryn::market::*;

#[test]
fn market_postgres_settlement_replay_and_conservation() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "market scenario panicked")??;
    Ok(())
}
#[test]
fn market_postgres_placement_projection_bounds_and_thirty_day_expiry() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    eprintln!("FIXE6W5 START market bounds and 30-day expiry");
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "market bounds scenario panicked")??;
    eprintln!(
        "FIXE6W5 FINISH market bounds and 30-day expiry {:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn reaudit_market_population() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, true).map_err(|e| e.to_string()))?
        .join();
    eprintln!(
        "reaudit_market_population duration: {:?}",
        started.elapsed()
    );
    result.map_err(|_| "market population scenario panicked")??;
    Ok(())
}

#[test]
fn market_postgres_migration_diagnostic() -> TestResult {
    run(async |admin| {
        use sqlx::Connection;
        let mut connection = sqlx::PgConnection::connect(&admin)
            .await
            .map_err(|_| "PG connect")?;
        let name = format!(
            "market_diag_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut connection)
            .await?;
        let options: sqlx::postgres::PgConnectOptions = admin.parse().map_err(|_| "PG options")?;
        let mut database = sqlx::PgConnection::connect_with(&options.database(&name))
            .await
            .map_err(|_| "PG database connect")?;
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../apps/game-server/migrations");
        let migrator = sqlx::migrate::Migrator::new(directory.as_path()).await?;
        let result = migrator
            .run(&mut database)
            .await
            .map_err(|e| format!("{e:?}").into());
        database.close().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "DROP DATABASE {name} WITH (FORCE)"
        )))
        .execute(&mut connection)
        .await?;
        result
    })
}

#[test]
fn market_postgres_expiry_recovery_returns_buy_escrow_once() -> TestResult {
    expiry_recovery(false)
}

#[test]
fn market_postgres_expiry_clears_pending_match() -> TestResult {
    expiry_recovery(true)
}

fn expiry_recovery(pending_match: bool) -> TestResult {
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "market_expiry", true, false).await?;

        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        // Retained prefix: an already paid-for buy offer, expired before restart.
        sqlx::query("INSERT INTO game_market_books VALUES(encode($1,'hex')::uuid,'oteryn:item.tibia.i3031')").bind(id(42).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_market_offers(offer_id,world_id,definition_key,definition_revision,side,account_id,character_id,piece_price,amount,remaining,created_at_us,expires_at_us,state,escrow_gold,held_credit_gold) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'oteryn:item.tibia.i3031','definition-r1',1,encode($3,'hex')::uuid,encode($4,'hex')::uuid,100,1,1,1,2592000000001,1,100,0)")
            .bind(id(190).as_slice()).bind(id(42).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
        if pending_match {
            sqlx::query("UPDATE game_market_offers SET matching_pending=true WHERE offer_id=encode($1,'hex')::uuid")
                .bind(id(190).as_slice()).execute(&h.pool).await?;
        }
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;

        h.root
            .advance_market(&authority, &h.node, world)
            .await
            .map_err(|e| format!("Market expiry step: {e:?}"))?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT balance FROM game_account_bank_balances")
                .fetch_one(&h.pool)
                .await?,
            100
        );
        assert_eq!(
            sqlx::query_scalar::<_, i16>("SELECT state FROM game_market_offers")
                .fetch_one(&h.pool)
                .await?,
            4
        );
        let settled = snapshot(&h).await?;

        h.root
            .advance_market(&authority, &h.node, world)
            .await
            .map_err(|e| format!("Market expiry step: {e:?}"))?;
        assert_eq!(settled, snapshot(&h).await?);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_market_operations")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn market_postgres_buy_funds_refusal_replays_without_value_rows() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "market_refusal", true, false).await?;

        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(
                    crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
                )
                .await?
                .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        let command = CommandRef::new(fence.game_session_id, CommandId::new(1).map_err(debug)?);
        let request = MarketRequest {
            action: 5,
            definition: 1,
            offer: [0; 16],
            amount: 1,
            price: 10,
            anonymous: false,
            page: 0,
        };
        let definition = crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i3031".into(),
            revision_ref: "definition-r1".into(),
        };
        let first = h
            .root
            .commit_market(
                &authority,
                &h.node,
                fence,
                command,
                request,
                Some(definition.clone()),
                0,
            )
            .await?;
        assert_eq!(first.code, 2);
        let replay = h
            .root
            .commit_market(
                &authority,
                &h.node,
                fence,
                command,
                request,
                Some(definition),
                0,
            )
            .await?;
        assert_eq!(first, replay);
        for table in [
            "game_market_offers",
            "game_market_settlements",
            "game_market_item_lines",
            "game_market_event_outbox",
            "game_account_bank_balances",
        ] {
            let count: i64 =
                sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(count, 0, "refusal wrote {table}");
        }
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
