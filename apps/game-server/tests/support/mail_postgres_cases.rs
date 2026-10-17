//! MAIL cases share the canonical target's routed PostgreSQL 17.6 database.
//! Without routing, the harness reports PRE-ROUTING / NONCANONICAL; that path
//! does not qualify PostgreSQL behavior. A configured run executes every body.

use crate::Database;
use crate::bestiary_postgres_harness::{TestResult, configured_admin, runtime};

/// Prove that this database ran the exact standing migration through the real
/// native migrator. No private schema copy can satisfy this checksum match.
async fn require_mail_migration(pool: &sqlx::PgPool) -> TestResult {
    let expected_checksum = sqlx::migrate!("./migrations")
        .iter()
        .find(|migration| migration.version == 101)
        .map(|migration| migration.checksum.to_vec())
        .ok_or("MAIL requires standing migration 0101 in the actual SQLx migrator")?;
    let installed: Option<(Vec<u8>, bool)> =
        sqlx::query_as("SELECT checksum, success FROM _sqlx_migrations WHERE version = 101")
            .fetch_optional(pool)
            .await?;
    if installed != Some((expected_checksum, true)) {
        return Err("MAIL requires installed migration 0101 with its exact checksum".into());
    }
    Ok(())
}

fn run(tag: &'static str, case: impl AsyncFnOnce(sqlx::PgPool) -> TestResult) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let database = Database::create(admin, tag).await?;
        let result = async {
            let pool = sqlx::PgPool::connect(&database.url).await?;
            let result = async {
                require_mail_migration(&pool).await?;
                case(pool.clone()).await
            }
            .await;
            pool.close().await;
            result
        }
        .await;
        database.cleanup().await?;
        result?;
        eprintln!("native MAIL recipient/text migration0101 PASS: {tag}");
        Ok(())
    })
}

#[test]
fn mail_recipient_same_world_self_and_unknown_are_privacy_safe() -> TestResult {
    run(
        "mail_world",
        crate::durability::mail::tests::mail_recipient_same_world_self_and_unknown_are_privacy_safe,
    )
}

#[test]
fn mail_stale_hint_rechecks_rename_lifecycle_and_world_independently() -> TestResult {
    run(
        "mail_hint",
        crate::durability::mail::tests::mail_stale_hint_rechecks_rename_lifecycle_and_world_independently,
    )
}

#[test]
fn mail_recipient_root_share_lock_blocks_terminal_change_until_transaction_ends() -> TestResult {
    run(
        "mail_lock",
        crate::durability::mail::tests::mail_recipient_root_share_lock_blocks_terminal_change_until_transaction_ends,
    )
}

#[test]
fn text_storage_delete_retains_tombstone_and_rewrite_advances() -> TestResult {
    run(
        "mail_text_delete",
        crate::durability::item_text::tests::text_storage_delete_retains_tombstone_and_rewrite_advances,
    )
}

#[test]
fn text_storage_rate_refusal_preserves_text_and_revision() -> TestResult {
    run(
        "mail_text_rate",
        crate::durability::item_text::tests::text_storage_rate_refusal_preserves_text_and_revision,
    )
}

#[test]
fn text_storage_rollback_loses_neither_atomicity_nor_command_retry() -> TestResult {
    run(
        "mail_text_rollback",
        crate::durability::item_text::tests::text_storage_rollback_loses_neither_atomicity_nor_command_retry,
    )
}

#[path = "character_inbox_reward_claim_postgres_cases.rs"]
mod reward_claim_inbox;

#[path = "character_inbox_reward_claim_mint_postgres_cases.rs"]
mod reward_claim_inbox_mint;
