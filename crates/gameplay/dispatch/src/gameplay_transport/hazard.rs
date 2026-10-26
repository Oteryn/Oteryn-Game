//! Primal hazard selection and qualification compose the existing NPC, quest and kill owners.
pub(super) mod combat;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use oteryn_gameplay_quests::quest::hazard;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

use super::{ComposedFreshAdmission, QuestSession, connection, npc_command};
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceRewardOccurrence,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef, GameSessionId, RewardDamageShare};

pub(super) type QuestSessions = Mutex<HashMap<GameSessionId, QuestSession>>;

/// Retained in the existing NPC conversation owner before the first await that can commit.
#[derive(Debug, Clone)]
pub(super) struct PendingChoice {
    command: CommandRef,
    character: crate::domain::CharacterId,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    level: u8,
}

#[derive(Debug)]
pub(super) struct Qualification {
    level: u8,
    recipients: Vec<(GameSessionId, [u8; 16], u64)>,
}

/// Capture once in the turn that records the death. A retry never reinterprets this death
/// using a subsequently selected level, a successor session, or a higher qualification.
pub(super) fn capture(
    runtime: &crate::foundation::ChannelRuntimeV1,
    sessions: Option<&QuestSessions>,
    creature: &str,
    shares: &[RewardDamageShare],
) -> Option<Qualification> {
    if creature != hazard::PRIMAL_MENACE || shares.is_empty() {
        return None;
    }
    let sessions = sessions?.lock().ok()?;
    let mut levels = Vec::with_capacity(shares.len());
    for share in shares {
        let (Some(session), Some(lease)) = (share.session, share.lease_generation) else {
            continue;
        };
        let Ok(Some((player, _))) = runtime.positioned_player_for_session(session) else {
            continue;
        };
        if !runtime
            .bound_attacker_lease(player, session)
            .is_ok_and(|current| {
                current.character_id().as_bytes() == share.character.as_bytes()
                    && current.generation() == lease
            })
        {
            continue;
        }
        let copy = sessions.get(&session)?.copy.as_ref()?;
        if copy.tracks().get(hazard::MAGMA_BUBBLE_KILLED) != Some(&1) {
            return None;
        }
        levels.push((
            session,
            *share.character.as_bytes(),
            lease,
            hazard::level(copy.tracks())?,
        ));
    }
    let level = levels.iter().map(|(_, _, _, state)| state.current).min()?;
    let recipients = levels
        .into_iter()
        .filter_map(|(session, character, lease, state)| {
            (state.current == level && state.unlocked == level && level < hazard::MAX_LEVEL)
                .then_some((session, character, lease))
        })
        .collect();
    Some(Qualification { level, recipients })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn active_hazard_level(&self, session: GameSessionId) -> Option<u8> {
        let permit = self.spell_lane_permit().await?;
        let mut runtime = self.runtime.lock().await;
        let (actor, _) = runtime.positioned_player_for_session(session).ok()??;
        let mut states = self.states.spell_states.lock().await;
        self.with_hazard_level(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            |_, _, level| level,
        )
        .await
    }

    pub(super) async fn hazard_talk(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: NpcTalkIntent,
    ) -> npc_command::NpcTalkOutcome {
        let ordinary = self.vocation_talk(actor, command, intent.clone()).await;
        if ordinary.disposition != NpcIntentDisposition::Ok {
            return ordinary;
        }
        let text = intent.text.trim();
        let desired = text.parse::<u8>().ok();
        if !text.eq_ignore_ascii_case("hazard") && desired.is_none() {
            return ordinary;
        }
        let session = command.game_session_id;
        let Some(item_fence) = command.item_fence else {
            return ordinary;
        };
        {
            let runtime = self.runtime.lock().await;
            let Some(npc) = self.states.chat.engaged(session) else {
                return ordinary;
            };
            if runtime.npc_key(npc) != Some(hazard::GNOMADNESS) {
                return ordinary;
            }
            if npc.placement_identity() != intent.npc_actor.identity
                || !runtime.visible_entities().npcs.iter().any(|placed| {
                    placed.actor == npc && placed.generation == intent.npc_actor.generation
                })
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || self.states.spell_states.lock().await.is_dead(actor)
                || !runtime
                    .bound_attacker_lease(actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == item_fence.character_id.as_bytes()
                            && lease.generation() == item_fence.character_lease_generation
                    })
            {
                return npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
            }
            if runtime.read_actor_position(actor).is_err() {
                return ordinary;
            }
        }
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
        };
        if fence.item_fence() != item_fence {
            // CharacterRevision is intentionally absent from the item fence. The revision
            // slot supplies its cursor; every session/scope generation must still match.
            return npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
        }
        let reply = |text| npc_command::NpcConversations::bank_reply(&intent, text);
        if let Some(desired) = desired {
            if !(1..=hazard::MAX_LEVEL).contains(&desired) {
                return reply(
                    "That hazard level is outside the supported range of 1 to 12.".into(),
                );
            }
            let Ok(id) = CommandId::new(command.command_id) else {
                return ordinary;
            };
            let Ok(copy) = self
                .root
                .read_character_quest_state(self.character, fence.character_id)
                .await
            else {
                return npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
            };
            if copy.tracks().get(hazard::MAGMA_BUBBLE_KILLED) != Some(&1) {
                return reply("Defeat Magma Bubble before changing your hazard level.".into());
            }
            if !self.states.npc_conversations.retain_hazard(
                session,
                PendingChoice {
                    command: CommandRef::new(session, id),
                    character: fence.character_id,
                    npc: intent.npc_actor,
                    level: desired,
                },
            ) {
                return npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
            }
            match self
                .commit_hazard(
                    fence,
                    hazard::select_transition(desired),
                    QuestCause::Command(CommandRef::new(session, id)),
                )
                .await
            {
                Ok(QuestTransitionOutcome::Refused(_)) => {
                    self.states.npc_conversations.clear_hazard(session);
                    return reply(
                        "You cannot select a hazard level above your qualification.".into(),
                    );
                }
                Ok(_) => {}
                Err(()) => {
                    return npc_command::NpcTalkOutcome {
                        unknown: true,
                        ..ordinary
                    };
                }
            }
        }
        // Read the durable owner after the write, rather than trusting an acknowledgement or
        // a possibly stale session copy. The same read API is available to the reward lane.
        let Ok(copy) = self
            .root
            .read_character_quest_state(self.character, fence.character_id)
            .await
        else {
            return npc_command::NpcTalkOutcome {
                unknown: desired.is_some(),
                ..ordinary
            };
        };
        let Some(state) = hazard::level(copy.tracks()) else {
            return npc_command::NpcTalkOutcome {
                unknown: desired.is_some(),
                ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
            };
        };
        self.refresh_quest_session(session, false).await;
        self.states.npc_conversations.clear_hazard(session);
        reply(level_reply(
            state.current,
            state.unlocked,
            self.active_hazard_level(session).await,
        ))
    }

    pub(super) async fn resume_hazard_talk(
        &self,
        command: connection::UseCommand,
    ) -> Option<npc_command::NpcTalkOutcome> {
        let pending = self
            .states
            .npc_conversations
            .pending_hazard(command.game_session_id)?;
        let unknown = || npc_command::NpcTalkOutcome {
            unknown: true,
            ..npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        };
        if pending.command.command_id().get() != command.command_id {
            return Some(unknown());
        }
        let request = QuestTransitionRequest {
            transition_key: hazard::select_transition(pending.level),
            cause: QuestCause::Command(pending.command),
        };
        let receipt = match self
            .root
            .reconcile_character_quest_transition(self.character, pending.character, request)
            .await
        {
            Ok(receipt) => receipt,
            Err(_) => return Some(unknown()),
        };
        let outcome = if let Some(receipt) = receipt {
            let unlocked = receipt
                .changes
                .iter()
                .find(|change| change.track == hazard::UNLOCKED)
                .and_then(|change| u8::try_from(change.after).ok());
            let Some(unlocked) = unlocked else {
                return Some(unknown());
            };
            self.refresh_quest_session(command.game_session_id, false)
                .await;
            npc_command::NpcConversations::bank_reply(
                &NpcTalkIntent {
                    npc_actor: pending.npc,
                    text: String::new(),
                },
                level_reply(
                    pending.level,
                    unlocked,
                    self.active_hazard_level(command.game_session_id).await,
                ),
            )
        } else {
            npc_command::NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        };
        self.states
            .npc_conversations
            .clear_hazard(command.game_session_id);
        Some(outcome)
    }

    async fn commit_hazard(
        &self,
        fence: CurrentCharacterGameplayFence,
        transition_key: String,
        cause: QuestCause,
    ) -> Result<QuestTransitionOutcome, ()> {
        let catalogue = self.quest_catalogue.as_ref().ok_or(())?;
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        slot.commit_quest_transition(
            self.root,
            self.character,
            self.holder,
            fence,
            QuestTransitionRequest {
                transition_key,
                cause,
            },
            Arc::clone(catalogue),
        )
        .await
        .map_err(|_| ())
    }

    /// Unknown commits keep the existing kill queue retryable. The original death and fixed
    /// transition key reconcile an acknowledged or unacknowledged commit exactly once.
    pub(super) async fn qualify_hazard_kill(
        &self,
        qualification: Option<&Qualification>,
        occurrence: [u8; 16],
    ) -> bool {
        let Some(qualification) = qualification else {
            return true;
        };
        let Ok(occurrence) = ExperienceRewardOccurrence::from_bytes(occurrence) else {
            return false;
        };
        for (session, character, lease) in &qualification.recipients {
            let Ok(current) = self.current_quest_fence(*session).await else {
                return false;
            };
            let Some(fence) = current else {
                continue;
            };
            if fence.character_id.as_bytes() != character
                || fence.character_lease_generation != *lease
            {
                continue;
            }
            match self
                .commit_hazard(
                    fence,
                    hazard::advance_transition(qualification.level),
                    QuestCause::CreatureDeath(occurrence),
                )
                .await
            {
                Ok(
                    QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_),
                ) => {}
                Ok(QuestTransitionOutcome::Refused(
                    oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch,
                )) => continue,
                Ok(
                    QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed,
                )
                | Err(()) => return false,
            }
            self.refresh_quest_session(*session, false).await;
        }
        true
    }
}

fn level_reply(current: u8, unlocked: u8, active: Option<u8>) -> String {
    let active = active.map_or_else(|| "unavailable".into(), |level| level.to_string());
    format!(
        "Your hazard level is {current}. Your maximum unlocked level is {unlocked}. Your active area level is {active}. Choose a level from 1 to {unlocked}."
    )
}
