//! PostgreSQL-backed, journal-only reconnect durability boundary.
//!
//! This module deliberately owns persistence/classification only. Foundation
//! constructs and revalidates reconnect authority; the runtime must submit the
//! resulting request asynchronously and consume its completion as new input.

mod admission_journal;
mod db;
mod schema;

pub use admission_journal::AdmissionReconnectJournal;
pub use schema::{MigrationExecutor, SchemaCompatibility};

use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum DurabilityError {
    Database(sqlx::Error),
    Migration(sqlx::migrate::MigrateError),
    SchemaIncompatible(SchemaCompatibility),
    InvalidStoredState,
}

impl Display for DurabilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => {
                write!(formatter, "PostgreSQL durability operation failed: {error}")
            }
            Self::Migration(error) => write!(formatter, "game migration operation failed: {error}"),
            Self::SchemaIncompatible(state) => {
                write!(
                    formatter,
                    "game durability schema is not runtime-compatible: {state:?}"
                )
            }
            Self::InvalidStoredState => {
                formatter.write_str("durability journal contains invalid state")
            }
        }
    }
}

impl std::error::Error for DurabilityError {}

impl From<sqlx::Error> for DurabilityError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<sqlx::migrate::MigrateError> for DurabilityError {
    fn from(error: sqlx::migrate::MigrateError) -> Self {
        Self::Migration(error)
    }
}

#[cfg(test)]
mod terminal_replacement_schema_red_tests {
    const MIGRATION: &str = include_str!("../../migrations/0001_admission_reconnect_journal.sql");

    #[test]
    fn terminal_replacement_forward_syncs_lagging_scope_fence_atomically() {
        assert!(
            MIGRATION.contains("session_state SMALLINT NOT NULL DEFAULT 1 CHECK (session_state BETWEEN 1 AND 3)"),
            "terminal predecessor replacement requires an explicit durable TERMINAL session state before any forward scope synchronization can be committed"
        );
        assert!(
            MIGRATION.contains("predecessor_scope_ownership_generation NUMERIC(20, 0) NOT NULL"),
            "replacement receipt must retain the exact Foundation-authorized predecessor scope fence"
        );
    }

    #[test]
    fn terminal_replacement_rejects_scope_fence_ahead_of_foundation_authority() {
        assert!(
            MIGRATION.contains("game_durability_session_replacements"),
            "scope comparison must be bound to a durable predecessor-to-candidate replacement receipt"
        );
    }

    #[test]
    fn terminal_replacement_rejects_live_or_mismatched_predecessor_without_mutation() {
        assert!(
            MIGRATION.contains("game_durability_one_nonterminal_session_per_character"),
            "one-live-session-per-character must remain database-enforced while terminal history is retained"
        );
        assert!(
            !MIGRATION.contains("    UNIQUE (character_id),"),
            "unconditional actor uniqueness permanently binds a character to its first historical GameSession"
        );
    }

    #[test]
    fn terminal_replacement_lost_response_replays_only_exact_receipt_binding() {
        for required in [
            "character_id UUID NOT NULL",
            "predecessor_game_session_id UUID NOT NULL",
            "candidate_game_session_id UUID NOT NULL",
            "PRIMARY KEY (character_id, predecessor_game_session_id, candidate_game_session_id)",
        ] {
            assert!(
                MIGRATION.contains(required),
                "lost-response replacement replay requires exact receipt binding field: {required}"
            );
        }
    }

    #[test]
    fn terminal_replacement_conflicting_receipt_binding_fails_closed() {
        assert!(
            MIGRATION.contains("UNIQUE (character_id, candidate_game_session_id)"),
            "a candidate cannot be replay-equivalent to multiple predecessor bindings"
        );
    }

    #[test]
    fn terminal_replacement_fences_predecessor_prepared_attempt_against_late_commit() {
        assert!(
            MIGRATION.contains("session_state SMALLINT NOT NULL DEFAULT 1 CHECK (session_state BETWEEN 1 AND 3)"),
            "the predecessor needs a durable terminal state that late COMMIT validation can fail closed against"
        );
    }

    #[test]
    fn terminal_replacement_mid_transaction_failure_rolls_back_predecessor_and_candidate() {
        assert!(
            MIGRATION.contains("game_durability_session_replacements"),
            "replacement transaction rollback cannot be proven until receipt persistence exists in the same ledger"
        );
    }

    #[test]
    fn collision_existing_terminal_replay_preserves_typed_collision_reason() {
        assert!(
            MIGRATION.contains("game_durability_reconnect_attempts"),
            "typed terminal replay must continue to derive from the durable attempt ledger"
        );
        assert!(
            MIGRATION.contains("state SMALLINT NOT NULL CHECK (state BETWEEN 1 AND 5)"),
            "existing terminal attempt classes must remain durably distinguishable"
        );
    }

    #[test]
    fn v2_reconciliation_round_trips_collision_concurrent_and_stale_distinctly() {
        assert!(
            MIGRATION.contains("state SMALLINT NOT NULL CHECK (state BETWEEN 1 AND 5)"),
            "V2 reconciliation requires the existing distinct durable terminal attempt states"
        );
    }

    #[test]
    fn concurrent_terminal_replacement_has_exactly_one_candidate_winner() {
        assert!(
            MIGRATION.contains(
                "CREATE UNIQUE INDEX game_durability_one_nonterminal_session_per_character"
            ),
            "concurrent terminal replacement requires a database-enforced unique nonterminal actor anchor"
        );
        assert!(
            MIGRATION.contains("WHERE session_state IN (1, 2)"),
            "the unique actor anchor must exclude terminal historical rows"
        );
    }
}
