//! Reconcile retained spell and travel work before control loss.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Complete only retained original occurrences while their current active
    /// session still owns the actor. A loss must not detach an uncertain payment.
    pub(super) async fn reconcile_spells_before_control_loss(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let access = self.refresh_spell_access(actor, session).await;
        if matches!(
            self.reconcile_pending_native_for_control_loss(actor, session, &access)
                .await,
            Some(actor_spell::NativeCastDispatch::Pending)
        ) {
            return false;
        }
        if matches!(
            self.reconcile_pending_familiar_for_control_loss(actor, session, &access)
                .await,
            Some(actor_spell::NativeCastDispatch::Pending)
        ) {
            return false;
        }
        if matches!(
            self.reconcile_pending_world_items(actor, session, &access)
                .await,
            actor_spell::NativeCastDispatch::Pending
        ) {
            return false;
        }
        if self
            .reconcile_pending_parameters_for_control_loss(actor, session, &access)
            .await
            .is_some_and(|result| matches!(result.cast, actor_spell::NativeCastDispatch::Pending))
        {
            return false;
        }
        let _ = self
            .reconcile_pending_stance_for_control_loss(actor, session)
            .await;
        let cleanup_access = self.refresh_spell_access(actor, session).await;
        let _ = self
            .drain_familiar_deaths(actor, session, &cleanup_access)
            .await;
        let _ = self.save_spell_training(actor, session, true).await;
        let runtime = self.runtime.lock().await;
        let states = self.spell_states.lock().await;
        let settled = !states.has_pending_spell_commit(actor, session)
            && !runtime.actor_spell_reserved(actor);
        drop(states);
        drop(runtime);
        settled
            && (!self.newhaven_travel.held(session)
                || self
                    .reconcile_newhaven_travel(actor, session)
                    .await
                    .is_some())
    }
}
