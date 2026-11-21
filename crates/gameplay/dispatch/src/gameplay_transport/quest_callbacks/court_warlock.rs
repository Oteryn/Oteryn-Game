//! Source boss lever USE uses shared encounter admission and Character cooldowns.
use super::*;
use crate::durability::boss_encounter::SourceBossLever;
use oteryn_gameplay_quests::quest::world_callbacks::court_warlock as source;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "the source hook shares the existing lever owner turn"
    )]
    pub(super) async fn execute_source_boss_lever(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        after: crate::map::view::EntryFacts,
        cause: QuestCause,
        lever: SourceBossLever,
    ) -> UseDisposition {
        let QuestCause::Use(command) = cause else {
            return UseDisposition::Rejected;
        };
        let ready = self.execute_quest_encounter(
            runtime, states, actor, session, map, callback, key, revision, after, true, cause,
            None, None,
        );
        if ready != UseDisposition::Committed {
            return ready;
        }
        let Some(encounter) = &callback.effects.encounter else {
            return UseDisposition::Rejected;
        };
        let Ok(census) = runtime.positioned_melee_census() else {
            return UseDisposition::Rejected;
        };
        let mut party = Vec::new();
        for &(member, at, member_session) in &census {
            let Some(member_session) = member_session else {
                continue;
            };
            if !encounter
                .players
                .iter()
                .any(|p| local(p.from) == at.position())
            {
                continue;
            }
            let Ok(Some(fence)) = self.current_quest_fence(member_session).await else {
                return UseDisposition::Rejected;
            };
            if !runtime
                .bound_attacker_lease(member, member_session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
            {
                return UseDisposition::Rejected;
            }
            party.push(fence.item_fence());
        }
        // A command replay belongs to its immutable physical observation and
        // exact party fences. A later lever revision or reconnect must still
        // pay the existing cooldown, even if a client repeats a command number.
        party.sort_by_key(|fence| *fence.character_id.as_bytes());
        let mut identity = match lever {
            SourceBossLever::CourtWarlock => b"cwarlock:lever:v1".to_vec(),
            SourceBossLever::Arbaziloth => b"arbaziloth:lever:v1".to_vec(),
        };
        identity.extend_from_slice(&key.to_be_bytes());
        identity.extend_from_slice(&revision.to_be_bytes());
        identity.extend_from_slice(&self.reset_epoch.to_be_bytes());
        identity.extend_from_slice(&runtime.content_pin().server_artifact_digest());
        identity.extend_from_slice(&runtime.content_pin().frame_binding_digest());
        for fence in &party {
            identity.extend_from_slice(fence.character_id.as_bytes());
            identity.extend_from_slice(fence.game_session_id.as_bytes());
            identity.extend_from_slice(&fence.connection_generation.get().to_be_bytes());
            identity.extend_from_slice(&fence.character_lease_generation.to_be_bytes());
            identity.extend_from_slice(&fence.scope_ownership_generation.get().to_be_bytes());
        }
        let admission = super::super::npc_trade_command::derived(
            &identity,
            session,
            command.command_id().get(),
        );
        match self
            .root
            .source_boss_lever_admission(
                self.character,
                self.holder,
                lever,
                party.clone(),
                admission,
                self.reset_epoch,
                false,
            )
            .await
        {
            Ok(true) => {}
            Ok(false) => return UseDisposition::Rejected,
            Err(error) => {
                super::super::operator_event(&format!(
                    "cwarlock_admission_deferred reason={error:?}"
                ));
                return UseDisposition::Rejected;
            }
        }
        let disposition = self.execute_quest_encounter(
            runtime, states, actor, session, map, callback, key, revision, after, false, cause,
            None, None,
        );
        if disposition != UseDisposition::Committed
            && let Err(error) = self
                .root
                .source_boss_lever_admission(
                    self.character,
                    self.holder,
                    lever,
                    party,
                    admission,
                    self.reset_epoch,
                    true,
                )
                .await
        {
            super::super::operator_event(&format!("cwarlock_refund_deferred reason={error:?}"));
        }
        disposition
    }
}

pub(super) fn source_boss_lever(callback: &Callback) -> Option<SourceBossLever> {
    match callback.key.as_str() {
        source::KEY => Some(SourceBossLever::CourtWarlock),
        "oteryn:quest-callback/nrwicked/arbaziloth" => Some(SourceBossLever::Arbaziloth),
        _ => None,
    }
}
