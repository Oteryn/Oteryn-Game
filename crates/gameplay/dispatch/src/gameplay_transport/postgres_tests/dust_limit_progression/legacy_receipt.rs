//! Insert the original paid ledger and receipt before 3331 with the real guards enabled.
use super::TestResult;
use crate::domain::CharacterId;
use crate::durability::character_forge_dust::read_forge_dust_limit_history;
use crate::durability::character_forge_dust_audit::{
    OneForgeDustBalanceV1, OneItemForgeDustLimitV1, audit_forge_dust_ledger,
    decode_forge_dust_limit_envelope, encode_forge_dust_limit_event,
};
use crate::durability::item_mint_audit::{MintEventIdentity, Type2EventTuple};
use oteryn_gameplay_durability::GAME_MIGRATOR;
use sqlx::Connection;

use super::super::fixture::{debug, run};

#[test]
fn dust_limit_3331_preserves_a_committed_3323_receipt_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("forge-dust-legacy", || {
        run(async |admin| {
            let options: sqlx::postgres::PgConnectOptions = admin.parse()?;
            if options.get_host() != "127.0.0.1"
                || options.get_username() != "oteryn_test_admin"
                || options.get_database() != Some("postgres")
            {
                return Err(
                    "Forge migration fixture requires the local PostgreSQL test admin".into(),
                );
            }
            let mut catalog = sqlx::PgConnection::connect_with(&options).await?;
            let suffix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            let name = format!("forge_dust_legacy_{suffix}");
            sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
                .execute(&mut catalog)
                .await?;
            let result = async {
                let mut db = sqlx::PgConnection::connect_with(&options.database(&name)).await?;
                check_migration(&mut db).await
            }
            .await;
            sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {name} WITH (FORCE)"
            )))
            .execute(&mut catalog)
            .await?;
            result
        })
    })
}

async fn check_migration(db: &mut sqlx::PgConnection) -> TestResult {
    let before = sqlx::migrate::Migrator::with_migrations(
        GAME_MIGRATOR
            .iter()
            .filter(|migration| migration.version < 3331)
            .cloned()
            .collect(),
    );
    before.run(&mut *db).await?;
    let envelope = historical_envelope()?;
    let mut tx = db.begin().await?;
    sqlx::raw_sql(include_str!("legacy_receipt.sql"))
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,
         event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,
         envelope,envelope_sha256,publication_state)
         SELECT event_id,transaction_id,1,1,2,1,'DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1',
         NULL,occurred_at,occurred_at+7776000000,$1,sha256($1),1
         FROM game_forge_dust_limit_reservations",
    ).bind(&envelope).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_forge_dust_limit_receipts(character_id,occurrence_id,transaction_id,event_id,
         entry_id,outcome,previous_entry_id,amount,balance_before,balance_after,dust_limit_before,
         dust_limit_after,envelope_sha256)
         SELECT character_id,occurrence_id,transaction_id,event_id,entry_id,
         1,'00000000-0000-7000-8000-000000000009',25,100,75,100,101,sha256($1)
         FROM game_forge_dust_limit_reservations",
    ).bind(&envelope).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_forge_dust_limit_history(character_id,occurrence_id,history_ordinal)
         SELECT character_id,occurrence_id,1 FROM game_forge_dust_limit_receipts",
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let stored_before: String =
        sqlx::query_scalar("SELECT to_jsonb(r)::text FROM game_forge_dust_limit_receipts r")
            .fetch_one(&mut *db)
            .await?;
    GAME_MIGRATOR.run(&mut *db).await?;
    let applied: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version=3331 AND success)",
    )
    .fetch_one(&mut *db)
    .await?;
    assert!(applied, "the fixture must cross migration 3331");
    // Revalidate the actual CHECKs over the historical row and its ledger.
    sqlx::raw_sql("ALTER TABLE game_forge_dust_limit_receipts VALIDATE CONSTRAINT game_forge_dust_limit_receipt_closed;
                   ALTER TABLE game_character_forge_dust_entries VALIDATE CONSTRAINT game_character_forge_dust_entry_arithmetic")
        .execute(&mut *db).await?;
    let stored_after: String =
        sqlx::query_scalar("SELECT to_jsonb(r)::text FROM game_forge_dust_limit_receipts r")
            .fetch_one(&mut *db)
            .await?;
    assert_eq!(stored_before, stored_after);
    let character = CharacterId::from_bytes(legacy_id(1)).map_err(debug)?;
    let history = read_forge_dust_limit_history(db, character, None, 5)
        .await
        .map_err(debug)?;
    assert_eq!(history.entries.len(), 1);
    let receipt = &history.entries[0].receipt;
    assert_eq!(receipt.bound_ruleset_revision, "global-forge-v2");
    let entry = receipt
        .entry
        .as_ref()
        .ok_or("historical Dust Limit entry")?;
    assert_eq!(
        (
            entry.before.limit().get(),
            entry.after.limit().get(),
            entry.amount
        ),
        (100, 101, 25)
    );
    let state = audit_forge_dust_ledger(db, character, 2)
        .await
        .map_err(debug)?;
    assert_eq!(
        (
            state.balance.balance(),
            state.balance.limit().get(),
            state.spent
        ),
        (75, 101, 25)
    );
    let stored_envelope: Vec<u8> =
        sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox")
            .fetch_one(&mut *db)
            .await?;
    assert_eq!(stored_envelope, envelope);
    let (_, event) = decode_forge_dust_limit_envelope(&stored_envelope).map_err(debug)?;
    assert_eq!(event.ruleset_revision, "global-forge-v2");
    assert_eq!(
        event
            .after
            .ok_or("historical audit after-state")?
            .dust_limit,
        101
    );
    Ok(())
}

fn legacy_id(last: u8) -> [u8; 16] {
    [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, last]
}

fn historical_envelope() -> TestResult<Vec<u8>> {
    encode_forge_dust_limit_event(
        MintEventIdentity {
            event_id: legacy_id(7),
            transaction_id: legacy_id(6),
            occurred_at_unix_ms: 1_800_000_000_000,
            server_build_id: "forge-dust-legacy-test",
        },
        OneItemForgeDustLimitV1 {
            character_id: legacy_id(1).to_vec(),
            world_id: legacy_id(3).to_vec(),
            channel_id: legacy_id(4).to_vec(),
            occurrence_id: legacy_id(5).to_vec(),
            entry_id: legacy_id(8).to_vec(),
            previous_entry_id: Some(legacy_id(9).to_vec()),
            before: Some(OneForgeDustBalanceV1 {
                balance: 100,
                dust_limit: 100,
            }),
            after: Some(OneForgeDustBalanceV1 {
                balance: 75,
                dust_limit: 101,
            }),
            dust_burned: 25,
            content_digest: vec![7; 32],
            ruleset_revision: "global-forge-v2".into(),
            simulation_profile_revision: 1,
            game_session_id: legacy_id(12).to_vec(),
            command_id: 1,
            connection_generation: 1,
            character_lease_generation: 1,
            scope_ownership_generation: 1,
            character_revision: 1,
        },
        Type2EventTuple::V1,
    )
    .map_err(debug)
}
