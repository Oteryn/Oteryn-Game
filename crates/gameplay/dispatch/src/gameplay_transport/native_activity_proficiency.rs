//! Session-keyed proficiency deltas must commit before a shared Instance handoff.
//! The existing Character writer keeps its exact native scope/lease/revision proofs.
use crate::foundation::{ActivityTransferMember, ChannelRuntimeV1, GameSessionId};
use crate::gameplay_transport::{ComposedFreshAdmission, actor_spell::TrainingSave};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Run before either scope lane or any Character revision slot is retained.
    pub(in crate::gameplay_transport) async fn flush_activity_proficiency(
        &self,
        session: GameSessionId,
    ) -> Result<(), ()> {
        let actor = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            runtime
                .positioned_actor_census()
                .map_err(|_| ())?
                .into_iter()
                .find_map(|(actor, _, current)| (current == Some(session)).then_some(actor))
                .ok_or(())?
        };
        match self.save_weapon_proficiency(actor, session, true).await {
            TrainingSave::Saved | TrainingSave::NotApplicable => Ok(()),
            TrainingSave::FencedOut | TrainingSave::Unknown => Err(()),
        }
    }

    /// The final lane-protected freeze closes a credit arriving after the flush.
    /// A retained checkpoint counts as pending even if its response was lost.
    pub(in crate::gameplay_transport) fn activity_proficiency_is_clean(
        &self,
        runtime: &ChannelRuntimeV1,
        member: &ActivityTransferMember,
    ) -> bool {
        if runtime.owner_fence().is_err()
            || runtime
                .player_control_facts(member.source_actor, member.source_session)
                .is_err()
            || !runtime
                .bound_attacker_lease(member.source_actor, member.source_session)
                .is_ok_and(|lease| lease == member.source_lease)
        {
            return false;
        }
        self.states.proficiencies.lock().is_ok_and(|states| {
            states
                .get(&member.source_session)
                .is_none_or(|state| !state.checkpoint_due(self.owner_now().get(), true))
        })
    }
}
