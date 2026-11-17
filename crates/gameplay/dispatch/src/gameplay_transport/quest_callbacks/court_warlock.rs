//! Real lever USE -> shared encounter admission -> existing Character boss cooldowns.
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::court_warlock as source;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments, reason = "the source hook shares the existing lever owner turn")]
    pub(super) async fn execute_court_warlock(
        &self, runtime: &mut ChannelRuntimeV1, states: &mut ChannelSpellStates,
        actor: ExactActorRef, session: GameSessionId, map: &BundleMap, callback: &Callback,
        key: u64, revision: u64, after: crate::map::view::EntryFacts, cause: QuestCause,
    ) -> UseDisposition {
        let QuestCause::Use(command) = cause else { return UseDisposition::Rejected; };
        let ready = self.execute_quest_encounter(runtime, states, actor, session, map, callback,
            key, revision, after, true, cause, None);
        if ready != UseDisposition::Committed { return ready; }
        let Some(encounter) = &callback.effects.encounter else { return UseDisposition::Rejected; };
        let Ok(census) = runtime.positioned_melee_census() else { return UseDisposition::Rejected; };
        let mut party = Vec::new();
        for &(member, at, member_session) in &census {
            let Some(member_session) = member_session else { continue; };
            if !encounter.players.iter().any(|p| local(p.from) == at.position()) { continue; }
            let Ok(Some(fence)) = self.current_quest_fence(member_session).await else { return UseDisposition::Rejected; };
            if !runtime.bound_attacker_lease(member, member_session).is_ok_and(|lease|
                lease.character_id().as_bytes() == fence.character_id.as_bytes() && lease.generation() == fence.character_lease_generation)
            { return UseDisposition::Rejected; }
            party.push(fence.item_fence());
        }
        let admission = super::super::npc_trade_command::derived(b"cwarlock:lever", session, command.command_id().get());
        match self.root.court_warlock_admission(self.character, self.holder, party.clone(), admission, false).await {
            Ok(true) => {},
            Ok(false) => return UseDisposition::Rejected,
            Err(error) => { super::super::operator_event(&format!("cwarlock_admission_deferred reason={error:?}")); return UseDisposition::Rejected; }
        }
        let disposition = self.execute_quest_encounter(runtime, states, actor, session, map, callback,
            key, revision, after, false, cause, None);
        if disposition != UseDisposition::Committed
            && let Err(error) = self.root.court_warlock_admission(self.character, self.holder, party, admission, true).await
        { super::super::operator_event(&format!("cwarlock_refund_deferred reason={error:?}")); }
        disposition
    }
}

pub(super) fn owns(callback: &Callback) -> bool { callback.key == source::KEY }
