//! MAIL cases share the canonical target's routed PostgreSQL 17.6 database.
//! Without routing, the harness reports PRE-ROUTING / NONCANONICAL; that path
//! does not qualify PostgreSQL behavior. A configured run executes every body.

use crate::Database;
use crate::bestiary_postgres_harness::{TestResult, configured_admin, runtime};

fn run(tag: &'static str, case: impl AsyncFnOnce(sqlx::PgPool) -> TestResult) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let database = Database::create(admin, tag).await?;
        let result = async {
            let pool = sqlx::PgPool::connect(&database.url).await?;
            let result = case(pool.clone()).await;
            pool.close().await;
            result
        }
        .await;
        database.cleanup().await?;
        result
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
