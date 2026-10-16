//! MAIL native PostgreSQL cases run only in the canonical PG target. Each real sqlx
//! wrapper provides the migrated pool to its unchanged test-only native exercise.

#[sqlx::test(migrations = "./migrations")]
async fn mail_recipient_same_world_self_and_unknown_are_privacy_safe(
    pool: sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::durability::mail::tests::mail_recipient_same_world_self_and_unknown_are_privacy_safe(
        pool,
    )
    .await
}

#[sqlx::test(migrations = "./migrations")]
async fn mail_stale_hint_rechecks_rename_lifecycle_and_world_independently(
    pool: sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::durability::mail::tests::mail_stale_hint_rechecks_rename_lifecycle_and_world_independently(pool).await
}

#[sqlx::test(migrations = "./migrations")]
async fn mail_recipient_root_share_lock_blocks_terminal_change_until_transaction_ends(
    pool: sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::durability::mail::tests::mail_recipient_root_share_lock_blocks_terminal_change_until_transaction_ends(pool).await
}

#[sqlx::test(migrations = "./migrations")]
async fn text_storage_delete_retains_tombstone_and_rewrite_advances(
    pool: sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::durability::item_text::tests::text_storage_delete_retains_tombstone_and_rewrite_advances(
        pool,
    )
    .await
}

#[sqlx::test(migrations = "./migrations")]
async fn text_storage_rate_refusal_preserves_text_and_revision(
    pool: sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::durability::item_text::tests::text_storage_rate_refusal_preserves_text_and_revision(pool)
        .await
}

#[sqlx::test(migrations = "./migrations")]
async fn text_storage_rollback_loses_neither_atomicity_nor_command_retry(
    pool: sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::durability::item_text::tests::text_storage_rollback_loses_neither_atomicity_nor_command_retry(pool).await
}
