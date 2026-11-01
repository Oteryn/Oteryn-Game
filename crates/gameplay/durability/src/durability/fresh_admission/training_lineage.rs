//! Preserve the real login clock across internal native House GameSessions.
//! Transfer ancestry is read-only history; the logout owner independently proves
//! current custody and the expected CharacterRevision before this read.
use super::*;
use super::super::character_progression::CurrentCharacterGameplayFence;
use sqlx::{Postgres, Transaction};

impl FreshAdmissionStore {
    /// Exercise the actual transaction-local resolver without exposing a clock
    /// override or manufacturing a settlement marker in integration fixtures.
    #[cfg(any(test, feature = "test-support"))]
    pub async fn read_training_login_clock_for_test(
        &self,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<i64>> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend.run_pass(issued, move |holder, deadline| Box::pin(async move {
            let mut tx = super::super::admission_journal::begin_pass_transaction(holder, deadline).await?;
            super::super::db::lock_admission_relations(&mut tx).await?;
            let clock = store.training_login_clock_locked(&mut tx, fence).await?;
            super::super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            Ok(clock)
        })).await
    }

    pub(in crate::durability) async fn training_login_clock_locked(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<i64>> {
        let house_lineage: bool = sqlx::query_scalar(
            "SELECT runtime_scope_house_key IS NOT NULL OR EXISTS( \
               SELECT 1 FROM game_house_transfer_receipts t \
               WHERE t.destination_game_session_id=s.game_session_id AND t.state=2) \
             FROM game_durability_reconnect_sessions s \
             WHERE s.game_session_id=encode($1,'hex')::uuid",
        ).bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if !house_lineage {
            // Preserve the original rule for ordinary Channel and activity sessions.
            return sqlx::query_scalar(
                "SELECT s.training_created_at_us FROM game_durability_reconnect_sessions s WHERE \
                 s.game_session_id=encode($1,'hex')::uuid AND EXISTS( \
                   SELECT 1 FROM game_character_training_markers m \
                   WHERE m.game_session_id=s.game_session_id AND m.cause='offline_settlement')",
            ).bind(fence.game_session_id.as_bytes().as_slice())
             .fetch_optional(&mut **tx).await.map(Option::flatten).map_err(Into::into);
        }

        let current = self.current_session_locked(tx, fence.game_session_id).await?;
        if current.current_game_session_id() != fence.game_session_id
            || current.session_state() != GameSessionState::Active
            || current.current_character_lease().character_id().as_bytes() != fence.character_id.as_bytes()
            || current.current_character_lease().generation() != fence.character_lease_generation
            || current.current_connection_generation() != fence.connection_generation
            || current.current_runtime_scope() != fence.runtime_scope
            || current.current_scope_generation() != fence.scope_ownership_generation
        {
            return Err(DurabilityError::Unavailable);
        }
        let mut cursor = current;
        let mut visited = std::collections::BTreeSet::new();
        while cursor.current_game_session_id() != current.commit().game_session_id() {
            if !visited.insert(*cursor.current_game_session_id().as_bytes()) {
                return Err(DurabilityError::InvalidStoredState);
            }
            // The same immutable edge proof is used by owning-loss recovery.
            // Its strictly decreasing lease and the existing semantic deadline
            // bound this historical walk without creating another ledger.
            let Some(link) = self.committed_house_predecessor_locked(tx, cursor).await? else {
                return Ok(None);
            };
            let source = self.current_session_locked(tx, link.source).await?;
            if source.commit() != current.commit()
                || source.session_state() != GameSessionState::Terminal
                || source.current_connection_generation() != link.source_generation
                || source.current_character_lease().generation() != link.source_lease
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            cursor = source;
        }
        // Only the genuine login's native settlement can establish online time.
        // Intermediate destination clocks and markers are never copied or minted.
        sqlx::query_scalar(
            "SELECT s.training_created_at_us \
             FROM game_durability_reconnect_sessions s \
             JOIN game_character_training_markers m ON m.game_session_id=s.game_session_id \
               AND m.character_id=s.character_id AND m.lease_generation=s.character_lease_generation \
               AND m.cause='offline_settlement' AND m.marker_version=2 \
             JOIN game_character_build_receipts b USING(build_occurrence_id) \
             WHERE s.game_session_id=encode($1,'hex')::uuid \
               AND s.character_id=encode($2,'hex')::uuid \
               AND s.character_lease_generation=$3::text::numeric \
               AND b.character_id=m.character_id AND b.cause=m.cause \
               AND b.committed_at=m.committed_at_us/1000 \
               AND m.as_of_us=m.committed_at_us",
        ).bind(cursor.current_game_session_id().as_bytes().as_slice())
         .bind(fence.character_id.as_bytes().as_slice())
         .bind(current.commit().character_lease_generation().to_string())
         .fetch_optional(&mut **tx).await.map(Option::flatten).map_err(Into::into)
    }
}
