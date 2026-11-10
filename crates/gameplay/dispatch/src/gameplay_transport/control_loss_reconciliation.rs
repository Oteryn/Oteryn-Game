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
        // These sealed native owners retain uncertain Item work, including its
        // original CommandRef and audit identity, until a known durable result.
        if !self.reconcile_pending_item_effects(actor, session).await {
            operator_event("event=control_loss_reconcile_pending stage=item_effects");
            return false;
        }
        if !super::item_use::gourmet::checkpoint_at_actor_end(self, actor, session).await {
            operator_event("event=control_loss_reconcile_pending stage=gourmet_checkpoint");
            return false;
        }
        let access = self.refresh_spell_access(actor, session).await;
        if matches!(
            self.reconcile_pending_native_for_control_loss(actor, session, &access)
                .await,
            Some(actor_spell::NativeCastDispatch::Pending)
        ) {
            operator_event("event=control_loss_reconcile_pending stage=native_cast");
            return false;
        }
        if matches!(
            self.reconcile_pending_familiar_for_control_loss(actor, session, &access)
                .await,
            Some(actor_spell::NativeCastDispatch::Pending)
        ) {
            operator_event("event=control_loss_reconcile_pending stage=familiar_cast");
            return false;
        }
        if matches!(
            self.reconcile_pending_world_items(actor, session, &access)
                .await,
            actor_spell::NativeCastDispatch::Pending
        ) {
            operator_event("event=control_loss_reconcile_pending stage=world_items");
            return false;
        }
        if self
            .reconcile_pending_parameters_for_control_loss(actor, session, &access)
            .await
            .is_some_and(|result| matches!(result.cast, actor_spell::NativeCastDispatch::Pending))
        {
            operator_event("event=control_loss_reconcile_pending stage=parameters");
            return false;
        }
        let _ = self
            .reconcile_pending_stance_for_control_loss(actor, session)
            .await;
        let cleanup_access = self.refresh_spell_access(actor, session).await;
        let _ = self
            .drain_familiar_deaths(actor, session, &cleanup_access)
            .await;
        if self.save_spell_training(actor, session, true).await
            == actor_spell::TrainingSave::Unknown
        {
            operator_event("event=control_loss_reconcile_pending stage=training_save");
            return false;
        }
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let unreserved = if runtime.house_actor_state(actor, session).is_ok() {
            runtime
                .assert_house_actor_spell_unreserved(actor, session)
                .is_ok()
        } else {
            !runtime.actor_spell_reserved(actor)
        };
        let pending = states.has_pending_spell_commit_before_actor_end(actor, session);
        let settled = !pending && unreserved;
        drop(states);
        drop(runtime);
        if !settled {
            if pending {
                operator_event("event=control_loss_reconcile_pending stage=pending_spell_commit");
            } else {
                operator_event("event=control_loss_reconcile_pending stage=actor_unreserved");
            }
            return false;
        }
        if self.states.newhaven_travel.held(session)
            && self
                .reconcile_newhaven_travel(actor, session)
                .await
                .is_none()
        {
            operator_event("event=control_loss_reconcile_pending stage=newhaven_travel");
            return false;
        }
        true
    }
}
