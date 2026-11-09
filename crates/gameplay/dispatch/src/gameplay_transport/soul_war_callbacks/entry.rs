//! Read the original committed lever-use time under the retained destination lane.
use super::*;
use crate::durability::quest_state::soul_war::CurrentSoulWarEntryClock;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::GameSessionId;

impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    /// The native handoff already owns this lane. Reacquiring it here would
    /// deadlock and would discard the canonical transfer's ordering evidence.
    pub(in crate::gameplay_transport) async fn current_soul_war_entry_clock_with_permit(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        expected_handoff: [u8; 16],
    ) -> Result<Option<(CurrentSoulWarEntryClock, u64)>, ()> {
        let runtime = self.runtime.lock().await;
        let room = self.effective_rooms().gate_monster_ai().ok_or(())?;
        if !room
            .pinned_activity_source_room()
            .is_some_and(taints::boss_room)
        {
            return Ok(None);
        }
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        if !qualifies_source(&runtime, native)
            || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
            || runtime.map_revision_digest() != room.map_revision_digest()
        {
            return Err(());
        }
        let observed=crate::durability::admission_journal::party_target_binding::observe_current_party_actor_for_source_read(&runtime,actor,session).ok_or(())?;
        drop(runtime);
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let mut context = (self, &observed, permit);
        pass.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            use crate::durability::spell_item_transaction::{assert_spell_item_scope_in_transaction, begin_spell_owner_transaction, check_scope_transaction};
            let (owner, observed, permit) = context;
            let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
            let authority = assert_spell_item_scope_in_transaction(&mut tx, permit, owner.root, owner.character, owner.holder,
                observed.runtime_scope(),observed.generation()).await
                .map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            let clock = crate::durability::quest_state::soul_war::read_current_entry_clock_for_observation_in_transaction(
                &mut tx, owner.root, &authority, observed, expected_handoff).await?;
            let observed_owner_us=owner.owner_now().get();
            check_scope_transaction(&mut tx,&authority).await
                .map_err(|_|crate::durability::DurabilityError::Unavailable)?;
            tx.rollback().await?;
            let runtime=owner.runtime.lock().await;
            if !observed.matches(&runtime) {return Err(crate::durability::DurabilityError::Unavailable);}
            Ok(clock.map(|clock|(clock,observed_owner_us)))
        })).await.map_err(|_|())
    }
}
