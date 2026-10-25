//! Leader shared-XP setting through the existing CHAT command envelope and PARTY-1 writer.
//! `/sharedxp on` and `/sharedxp off` retain command 13's genuine session/CommandId receipt.
use super::ComposedFreshAdmission;
use super::combat_lock_fact::PvpLockSource;
use crate::durability::{DurabilityError, spell_item_transaction as item_tx, world_party};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, GameSessionId, PartyAction};
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntent, ChatIntentResult, ChatSpeechMode};
use sha2::{Digest, Sha256};

/// Whether a shared-XP toggle is refused for a fight. Canary's
/// `Game::playerEnableSharedPartyExperience` (`game.cpp:9786`) refuses `CONDITION_INFIGHT` only
/// outside `TILESTATE_PROTECTIONZONE`. An unknown cell keeps the refusal.
fn shared_xp_refused_in_fight(in_fight: bool, in_protection_zone: Option<bool>) -> bool {
    in_fight && in_protection_zone != Some(true)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn party_shared_xp_chat_command(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: &ChatIntent,
    ) -> Option<ChatIntentResult> {
        let ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text,
        } = intent
        else {
            return None;
        };
        let enabled = match text.trim() {
            "/sharedxp on" => true,
            "/sharedxp off" => false,
            _ => return None,
        };
        let result = |disposition| ChatIntentResult {
            disposition,
            wait_seconds: 0,
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Some(result(ChatDisposition::ChatUnavailable));
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return Some(result(ChatDisposition::Rejected));
        };
        let Ok(command) = CommandId::new(command_id) else {
            return Some(result(ChatDisposition::Rejected));
        };
        let item_fence = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: fence.character_id,
            game_session_id: session,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
        };
        let mut runtime = self.runtime.lock().await;
        if runtime.assert_actor_spell_unreserved(actor).is_err()
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|p| p.control_loss.is_none())
            || !runtime.bound_attacker_lease(actor, session).is_ok_and(|l| {
                l.character_id().as_bytes() == fence.character_id.as_bytes()
                    && l.generation() == fence.character_lease_generation
            })
        {
            return Some(result(ChatDisposition::Rejected));
        }
        let in_protection_zone =
            super::actor_spell::qualified_protection_zone(&mut runtime, self.movement_cells, actor);
        let fight = self.combat_lock_fact(actor, session).await;
        if shared_xp_refused_in_fight(
            fight.pve_in_fight_until.is_some() || fight.pvp == PvpLockSource::LiveBlocked,
            in_protection_zone,
        ) {
            self.states.chat.push_party_spell_feedback(
                session,
                "You cannot change shared experience while in a fight.",
            );
            return Some(result(ChatDisposition::Rejected));
        }
        let mut bytes = b"oteryn.party.shared-xp.v1".to_vec();
        bytes.push(u8::from(enabled));
        let binding: [u8; 32] = Sha256::digest(bytes).into();
        let mut ctx = (self, &permit, &item_fence);
        let pass = match self.root.try_issue_semantic_pass() {
            Ok(pass) => pass,
            Err(_) => return Some(result(ChatDisposition::ChatUnavailable)),
        };
        let applied = pass
            .run_with_context(&mut ctx, move |holder, deadline, ctx| {
                Box::pin(async move {
                    let (owner, permit, fence) = ctx;
                    let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                    let authority = item_tx::assert_spell_item_authority_in_transaction(
                        &mut tx,
                        permit,
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        CommandRef::new(session, command),
                        binding,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let receipt = world_party::apply_world_party_command_in_transaction(
                        &mut tx,
                        &authority,
                        PartyAction::SetSharedExperience { enabled },
                        None,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let source = world_party::read_world_party_in_transaction(&mut tx, &authority)
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    crate::durability::party_kill_xp::commit_party_kill_transaction(tx, deadline)
                        .await?;
                    Ok((receipt.disposition, source))
                })
            })
            .await;
        match applied {
            Ok((disposition, source)) => {
                if runtime.project_world_party(actor, session, source).is_err() {
                    return Some(result(ChatDisposition::ChatUnavailable));
                }
                let (disposition, text) = if disposition == "APPLIED" {
                    (
                        ChatDisposition::Ok,
                        if enabled {
                            "Shared experience enabled."
                        } else {
                            "Shared experience disabled."
                        },
                    )
                } else {
                    (
                        ChatDisposition::Rejected,
                        "Only the party leader can change shared experience.",
                    )
                };
                self.states.chat.push_party_spell_feedback(session, text);
                Some(result(disposition))
            }
            Err(_) => Some(result(ChatDisposition::ChatUnavailable)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::shared_xp_refused_in_fight;

    #[test]
    fn a_fight_refuses_the_shared_xp_toggle_only_outside_a_protection_zone() {
        assert!(shared_xp_refused_in_fight(true, Some(false)));
        assert!(shared_xp_refused_in_fight(true, None));
        assert!(!shared_xp_refused_in_fight(true, Some(true)));
        assert!(!shared_xp_refused_in_fight(false, Some(false)));
        assert!(!shared_xp_refused_in_fight(false, None));
    }
}
