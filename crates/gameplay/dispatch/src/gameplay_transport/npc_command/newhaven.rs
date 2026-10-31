//! Newhaven tasks use the existing NPC conversation, quest writer and replay receipt.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use super::super::item_ref_admission::{NativeItemWeights, generation_item_facts, pinned_index};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{RewardClaimMintRequest, RewardClaimMintError, RewardClaimRefusal};
use crate::domain::CharacterId;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::newhaven as quest;

pub(super) struct Step {
    context: (u64, u64, String),
    transition: Option<&'static str>,
    reply: String,
}

#[derive(Clone)]
pub(super) struct Pending {
    character: CharacterId,
    request: QuestTransitionRequest,
    intent: NpcTalkIntent,
    reply: String,
}

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    let context = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    )?;
    let gustavo = context.2 == "oteryn:npc.gustavo_the_guard";
    if !gustavo && context.2 != "oteryn:npc.viola" {
        return None;
    }
    let text = intent.text.trim().to_ascii_lowercase();
    let sessions = admission.states.quest_sessions.lock().ok()?;
    let tracks = sessions.get(&command.game_session_id)?.copy.as_ref()?.tracks();
    let (transition, reply) = if gustavo {
        if !["goblins", "goblin", "muglex", "quest", "mission"].iter().any(|word| text.split_whitespace().any(|token| token == *word)) { return None; }
        if tracks.get(quest::DONE) == Some(&1) {
            (None, "You rendered a great favour to this village. Thank you!".into())
        } else if tracks.get(quest::START) != Some(&1) {
            (Some(quest::BEGIN), "They call themselves the Muglex Clan. Please take down twenty of these Muglex goblins. That should make life in the village safer again.".into())
        } else {
            let kills = tracks.get(quest::KILLS).copied().unwrap_or(0).max(0);
            if kills >= 20 {
                (Some(quest::FINISH), "You rendered a great favour to this village. Thank you! Take this ring as a form of my gratitude. Viola might also have a small task for you.".into())
            } else {
                (None, format!("You still need to kill {} more Muglex goblins. Keep hunting!", 20-kills))
            }
        }
    } else {
        let mut conversations = admission.states.npc_conversations.sessions.lock().ok()?;
        let state = conversations.get_mut(&command.game_session_id)?;
        if text == "yes" && state.newhaven_free_spell.as_ref().is_some_and(|(bound,_)| bound == &context) {
            let (_, transition) = state.newhaven_free_spell.take()?;
            (Some(transition), "Here you are. Look in your spellbook for the pronunciation of this spell.".into())
        } else if text == "light" || text == "wound cleansing" {
            let transition = if text == "light" { "oteryn:quest-transition/newhaven/viola-light" } else { "oteryn:quest-transition/newhaven/viola-wound-cleansing" };
            state.newhaven_free_spell = Some((context.clone(), transition));
            (None, format!("Do you want to learn {text} for free?"))
        } else if text == "spells" || text == "learn" || text == "healing" || text == "healing spells" || text == "support" || text == "support spells" || text == "instant spells" {
            (None, "I teach Wound Cleansing and Light for free, Cure Poison for 150 gold, and Magic Rope for 200 gold. Which spell interests you?".into())
        } else if text == "yes" && state.newhaven_book_prompt.as_ref() == Some(&context) {
            state.newhaven_book_prompt = None;
            (Some(quest::BOOK_BEGIN), "Thank you! Search the mines to the northeast for my Book of Wisdom.".into())
        } else if ["mission", "task", "book"].iter().any(|word| text.split_whitespace().any(|token| token == *word)) {
            if tracks.get(quest::BOOK) == Some(&1) {
                (None, "My Book of Wisdom was taken into the mines. I cannot accept a book at present.".into())
            } else {
                state.newhaven_book_prompt = Some(context.clone());
                (None, "A ghost took my precious Book of Wisdom into the mines to the northeast of the island. Would you retrieve it for me?".into())
            }
        } else {
            if text == "no" { state.newhaven_book_prompt = None; state.newhaven_free_spell = None; }
            return None;
        }
    };
    drop(sessions);
    Some(Step {
        context,
        transition,
        reply,
    })
}

fn clear(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.newhaven = None;
    }
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let Some(transition) = step.transition else {
        let mut outcome = NpcConversations::bank_reply(&intent, step.reply);
        if step.context.2 == "oteryn:npc.gustavo_the_guard" {
            outcome.inventory_changed = deliver_reward(admission, actor, command).await;
        }
        return outcome;
    };
    let (Some(item_fence), Ok(id), Some(catalogue)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission.quest_catalogue.as_ref(),
    ) else {
        return rejected();
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(item_fence.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if fence.item_fence() != item_fence {
        return rejected();
    }
    {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                &intent,
            ) != Some(step.context)
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return rejected();
        }
    }
    let request = QuestTransitionRequest {
        transition_key: transition.into(),
        cause: QuestCause::Command(CommandRef::new(command.game_session_id, id)),
    };
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return rejected();
        };
        let state = sessions.entry(command.game_session_id).or_default();
        if state.newhaven.is_some() {
            return unknown();
        }
        state.newhaven = Some(Pending {
            character: fence.character_id,
            request: request.clone(),
            intent: intent.clone(),
            reply: step.reply.clone(),
        });
    }
    let outcome = slot
        .commit_quest_transition(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request,
            std::sync::Arc::clone(catalogue),
        )
        .await;
    drop(slot);
    match outcome {
        Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => {
            clear(admission, command.game_session_id);
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            let mut outcome = NpcConversations::bank_reply(&intent, step.reply);
            if transition == quest::FINISH { outcome.inventory_changed = deliver_reward(admission, actor, command).await; }
            outcome
        }
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        _ => {
            clear(admission, command.game_session_id);
            rejected()
        }
    }
}

pub(in crate::gameplay_transport) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let pending = admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get(&command.game_session_id)?
        .newhaven
        .clone()?;
    let Ok(id) = CommandId::new(command.command_id) else {
        return None;
    };
    if pending.request.cause != QuestCause::Command(CommandRef::new(command.game_session_id, id)) {
        return None;
    }
    match admission
        .root
        .reconcile_character_quest_transition(
            admission.character,
            pending.character,
            pending.request,
        )
        .await
    {
        Ok(receipt) => {
            clear(admission, command.game_session_id);
            if receipt.is_some() {
                admission
                    .refresh_quest_session(command.game_session_id, false)
                    .await;
                Some(NpcConversations::bank_reply(&pending.intent, pending.reply))
            } else {
                Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
            }
        }
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}

/// Consume exactly the same settled principal and occurrence used for the Task Board.
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn credit_newhaven_kill(
        &self, fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
        race: &str, occurrence: [u8;16],
    ) {
        let eligible = self.states.quest_sessions.lock().ok().and_then(|sessions| {
            let copy = sessions.get(&fence.game_session_id)?.copy.as_ref()?;
            Some(quest::kill_credit(copy.tracks(), race))
        }).unwrap_or(false);
        if !eligible { return; }
        let Ok(occurrence) = crate::durability::character_progression::ExperienceRewardOccurrence::from_bytes(occurrence) else { return; };
        let Some(catalogue) = self.quest_catalogue.as_ref() else { return; };
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let result = slot.commit_quest_transition(self.root, self.character, self.holder,
            fence, QuestTransitionRequest {transition_key:quest::KILL.into(), cause:QuestCause::CreatureDeath(occurrence)},
            std::sync::Arc::clone(catalogue)).await;
        drop(slot);
        if matches!(result, Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_))) {
            self.refresh_quest_session(fence.game_session_id, false).await;
        } else if result.is_err() { self.schedule_quest_refresh(fence.game_session_id); }
    }
}

    async fn deliver_reward(
        admission: &ComposedFreshAdmission<'_, '_, '_>,
        actor: ExactActorRef,
        command: UseCommand,
    ) -> bool {
        deliver_reward_inner(admission, actor, command).await.unwrap_or(false)
}
async fn deliver_reward_inner(admission: &ComposedFreshAdmission<'_, '_, '_>, actor: ExactActorRef, command: UseCommand) -> Option<bool> {
        let fence = admission.current_quest_fence(command.game_session_id).await.ok()??;
        let pending = match admission
            .root
            .pending_newhaven_reward(admission.character, fence.character_id, quest::FINISH)
            .await
        {
            Ok(Some(pending)) => pending,
            Ok(None) => return None,
            Err(_) => return Some(false),
        };
        let native = admission.active_generation?.native_gameplay()?;
        let item = {
            let runtime = admission.runtime.lock().await;
            let states = admission.states.spell_states.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || !admission
                    .states
                    .player_trades
                    .physical(command.game_session_id, None)
            {
                return Some(false);
            }
            let index = pinned_index(&runtime, admission.active_generation)?;
            let key = format!("oteryn:item.tibia.i{}", pending.item_source_id);
            let revision = index.revision(&key)?;
            generation_item_facts(
                native,
                &TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key,
                    revision_ref: revision.into(),
                },
            )
            .ok()?
        };
        let bag = match admission
            .root
            .read_character_backpack(admission.character, fence.character_id)
            .await
        {
            Ok(Some(bag)) => bag,
            Ok(None) => return Some(false),
            Err(_) => return Some(false),
        };
        let backpack = generation_item_facts(native, &bag.backpack.definition).ok()?;
        let id = CommandId::new(command.command_id).ok()?;
        let request = RewardClaimMintRequest {
            command: CommandRef::new(command.game_session_id, id),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: pending.claim_key,
                revision_ref: "newhaven-gustavo-r1".into(),
            },
            source_placement: quest::FINISH.into(),
            item,
            quantity: 1,
            backpack,
            // The entitlement retains the earning generation as evidence. Delivery
            // resolves the exact stable Item identity in the current pinned generation,
            // so a later deployment cannot strand a previously full backpack.
            content_revision: format!(
                "sha256:{}",
                native
                    .source_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        };
        Some(
            match admission
                .root
                .claim_pending_newhaven_reward(
                    admission.character,
                    admission.holder,
                    fence.item_fence(),
                    quest::FINISH,
                    request,
                    &NativeItemWeights(native),
                )
                .await
            {
                Ok(_) => {
                    admission.states.loop_ground.invalidate();
                    true
                }
                Err(RewardClaimMintError::Refused(
                    RewardClaimRefusal::MainBackpackFull
                    | RewardClaimRefusal::CarryingCapacity
                    | RewardClaimRefusal::NoMainBackpack,
                )) => false,
                Err(_) => false,
            },
        )
    }
