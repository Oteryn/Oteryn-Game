//! Composition of current Premium with the existing durable death settlement.
use super::*;
use crate::durability::character_death::promotion::DeathPremiumOwner;

struct PremiumDeathGate<'a, 'r, 'c, 'n> {
    admission: &'a ComposedFreshAdmission<'r, 'c, 'n>,
    session: GameSessionId,
}
impl DeathPremiumOwner for PremiumDeathGate<'_, '_, '_, '_> {
    fn current<'a>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<bool, crate::durability::DurabilityError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let account = self
                .admission
                .states
                .premium_sessions
                .lock()
                .ok()
                .and_then(|sessions| sessions.account_of(self.session));
            let (Some(account), Some(coordinator)) = (account, self.admission.premium_coordinator)
            else {
                return Ok(false);
            };
            coordinator
                .transaction_permission(tx, account, |status| status.gate().premium_current())
                .await
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The durable half of [`Self::settle_death`]. The death request carries the binding's
    /// policy, the whole `CHARACTER_EXPERIENCE_TABLE_LEVELS` threshold table, by value across
    /// the commit, so `settle_death` keeps this future on the heap: inline, it would grow every
    /// step future that settles a death by several copies of that table.
    pub(super) async fn commit_death(
        &self,
        session: GameSessionId,
        progression: &SessionProgressionBinding,
        death: actor_spell::PlayerDeath,
    ) -> (
        DeathCommit,
        Option<crate::durability::character_build::DurableBuildState>,
    ) {
        let Some(respawn) = self.home_temple(session).await else {
            return (DeathCommit::Unavailable, None);
        };
        let (runtime_scope, map_revision) = {
            let runtime = self.runtime.lock().await;
            (
                runtime.binding().runtime_scope(),
                runtime.map_revision_digest(),
            )
        };
        let request =
            player_death_request(progression, runtime_scope, map_revision, death, respawn);
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return (DeathCommit::Unavailable, None);
        };
        // Validate against the active generation's training formula when available.
        let formula = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula())
            .map(|formula| formula as &dyn crate::durability::character_build::BuildFormula);
        // Settle deployment activation before entering the durable transaction. Its
        // Premium decision is read again in that transaction, never retained here.
        let _ = self.session_premium_gate(session).await;
        let premium = PremiumDeathGate {
            admission: self,
            session,
        };
        match self
            .settle_source_player_death(fence, request, formula, Some(&premium))
            .await
        {
            Ok(committed) => (
                DeathCommit::Settled,
                committed.build_loss.map(|loss| loss.after),
            ),
            Err(error) => {
                operator_event(&format!("player_death_settle_failed reason={error}"));
                (death_commit_refusal(&error), None)
            }
        }
    }
}
