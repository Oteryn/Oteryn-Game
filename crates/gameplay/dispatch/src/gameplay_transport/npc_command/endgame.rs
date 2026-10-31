//! W11 uses the existing NPC conversation and command 7. The wire carries text and
//! an exact visible NPC reference; it never carries a boss death, taint value or reward.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::quest_state::rotten_blood::RottenBloodOutcome;
use crate::durability::quest_state::rotten_blood_access::RottenBloodAccessPayment;
use crate::durability::quest_state::soul_war::{SoulWarCommand, SoulWarCommandOutcome};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::rotten_blood::{CleanseMode, RottenBloodRefusal};
mod payment;

type Context = (u64, u64, String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Topic {
    SoulTask,
    RottenCleanse,
    ConfirmCleanse(CleanseMode),
    TorkadaTime,
    TorkadaMission,
    SacrificeQuest,
    SacrificeDetour,
    SacrificeConfirm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mutation {
    Soul(SoulWarCommand),
    RottenCleanse(CleanseMode),
    RottenMission,
    RottenRestoreDoor,
    RottenAcknowledge,
    RottenPurchase,
}

#[derive(Clone)]
pub(super) enum Pending {
    Topic {
        context: Context,
        npc: EntityRef,
        topic: Topic,
    },
    Commit {
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        mutation: Mutation,
        intent: NpcTalkIntent,
        payment: Option<Box<RottenBloodAccessPayment>>,
    },
}

enum Action {
    Reply(&'static str),
    SoulTask,
    Commit(Mutation),
    Held,
    Torkada(Option<Topic>),
}

pub(super) struct Step {
    context: Context,
    action: Action,
}

fn topic_line(topic: Topic) -> &'static str {
    match topic {
        Topic::SoulTask => "Will you face my fettered vices? Say yes to accept this task.",
        Topic::RottenCleanse => {
            "Choose regular to remove regular taints, or all to remove every taint."
        }
        Topic::ConfirmCleanse(CleanseMode::Regular) => {
            "Remove your regular taints and retain your final taint? Say yes or no."
        }
        Topic::ConfirmCleanse(CleanseMode::All) => {
            "Remove all your Rotten Blood taints? Say yes or no."
        }
        Topic::TorkadaTime => {
            "Our expedition has an important mission for the inquisition. Ask about the mission."
        }
        Topic::TorkadaMission => "Will you carry out our mission? Say yes or no.",
        Topic::SacrificeQuest => {
            "Offer bloody tears at the sacrificial altar, or ask me about a detour."
        }
        Topic::SacrificeDetour => "The detour costs 5000000 gold pieces. Are you willing to pay?",
        Topic::SacrificeConfirm => "Confirm the payment of 5000000 gold pieces. Say yes or no.",
    }
}

fn choose(npc: &str, text: &str, topic: Option<Topic>) -> Option<(Option<Topic>, Action)> {
    if matches!(text, "no" | "bye") && topic.is_some() {
        return Some((None, Action::Reply("Very well.")));
    }
    match npc {
        "oteryn:npc.flickering_soul" => match text {
            "task" => Some((None, Action::SoulTask)),
            "yes" if topic == Some(Topic::SoulTask) => {
                Some((None, Action::Commit(Mutation::Soul(SoulWarCommand::Start))))
            }
            "taints" | "penalties" => Some((
                None,
                Action::Commit(Mutation::Soul(SoulWarCommand::Cleanse)),
            )),
            _ => None,
        },
        "oteryn:npc.bloodshade_rotten" => match text {
            "quest" => Some((None, Action::Commit(Mutation::RottenAcknowledge))),
            "taints" | "cleanse" | "purify" => Some((
                Some(Topic::RottenCleanse),
                Action::Reply(topic_line(Topic::RottenCleanse)),
            )),
            "regular" | "all" if topic == Some(Topic::RottenCleanse) => {
                let mode = if text == "all" {
                    CleanseMode::All
                } else {
                    CleanseMode::Regular
                };
                let next = Topic::ConfirmCleanse(mode);
                Some((Some(next), Action::Reply(topic_line(next))))
            }
            "yes" => match topic {
                Some(Topic::ConfirmCleanse(mode)) => {
                    Some((None, Action::Commit(Mutation::RottenCleanse(mode))))
                }
                _ => None,
            },
            _ => None,
        },
        "oteryn:npc.torkada" => match text {
            "time" => Some((
                Some(Topic::TorkadaTime),
                Action::Torkada(Some(Topic::TorkadaTime)),
            )),
            "mission" if topic == Some(Topic::TorkadaTime) => Some((
                Some(Topic::TorkadaMission),
                Action::Torkada(Some(Topic::TorkadaMission)),
            )),
            "yes" if topic == Some(Topic::TorkadaMission) => {
                Some((None, Action::Commit(Mutation::RottenMission)))
            }
            "mission" | "access" => Some((None, Action::Torkada(None))),
            _ => None,
        },
        "oteryn:npc.bloodshade_sacrifice" => match text {
            "quest" => Some((
                Some(Topic::SacrificeQuest),
                Action::Reply(topic_line(Topic::SacrificeQuest)),
            )),
            "detour" if topic == Some(Topic::SacrificeQuest) => Some((
                Some(Topic::SacrificeDetour),
                Action::Reply(topic_line(Topic::SacrificeDetour)),
            )),
            "yes" if topic == Some(Topic::SacrificeDetour) => Some((
                Some(Topic::SacrificeConfirm),
                Action::Reply(topic_line(Topic::SacrificeConfirm)),
            )),
            "yes" if topic == Some(Topic::SacrificeConfirm) => {
                Some((None, Action::Commit(Mutation::RottenPurchase)))
            }
            _ => None,
        },
        _ => None,
    }
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
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.entry(command.game_session_id).or_default();
    let topic = match &state.endgame {
        Some(Pending::Commit { .. }) => {
            return Some(Step {
                context,
                action: Action::Held,
            });
        }
        Some(Pending::Topic {
            context: held,
            npc,
            topic,
        }) if held == &context && npc == &intent.npc_actor => Some(*topic),
        _ => None,
    };
    let text = intent.text.trim().to_ascii_lowercase();
    let (next, action) = choose(&context.2, &text, topic)?;
    state.endgame = next.map(|topic| Pending::Topic {
        context: context.clone(),
        npc: intent.npc_actor,
        topic,
    });
    Some(Step { context, action })
}

fn clear(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&session)
    {
        state.endgame = None;
    }
}

fn unknown() -> NpcTalkOutcome {
    NpcTalkOutcome {
        unknown: true,
        ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
    }
}

fn soul_reply(
    intent: &NpcTalkIntent,
    command: SoulWarCommand,
    result: &SoulWarCommandOutcome,
) -> NpcTalkOutcome {
    let line = if result.result.is_err() {
        "You have not yet completed the task required for this reward."
    } else {
        match command {
            SoulWarCommand::Start => "Thank you for accepting this burden.",
            SoulWarCommand::Cleanse => "Your Soul War taints have been cleansed.",
            SoulWarCommand::ReportCompletion => {
                "You have freed my soul. You have earned the Revenant outfit."
            }
        }
    };
    NpcConversations::bank_reply(intent, line.into())
}

fn rotten_reply(intent: &NpcTalkIntent, result: &RottenBloodOutcome) -> NpcTalkOutcome {
    let line = match result {
        RottenBloodOutcome::Committed(_) | RottenBloodOutcome::AlreadyCommitted(_) => {
            "Your chosen Rotten Blood taints have been cleansed."
        }
        RottenBloodOutcome::Refused(RottenBloodRefusal::CleanseCooldown { .. }) => {
            "Your next cleansing is still on cooldown."
        }
        RottenBloodOutcome::Refused(RottenBloodRefusal::NothingToCleanse) => {
            "You have no taints of that kind to cleanse."
        }
        RottenBloodOutcome::Refused(_) => "You do not meet the requirements for this cleansing.",
    };
    NpcConversations::bank_reply(intent, line.into())
}

fn mission_request(command: CommandRef, restore: bool) -> QuestTransitionRequest {
    QuestTransitionRequest {
        transition_key: if restore {
            oteryn_gameplay_quests::quest::rotten_blood::RESTORE_DOOR
        } else {
            oteryn_gameplay_quests::quest::rotten_blood::START
        }
        .into(),
        cause: QuestCause::Command(command),
    }
}

fn acknowledgement_request(command: CommandRef) -> QuestTransitionRequest {
    QuestTransitionRequest {
        transition_key: oteryn_gameplay_quests::quest::rotten_blood::ACKNOWLEDGE_CHAIN.into(),
        cause: QuestCause::Command(command),
    }
}

fn acknowledgement_reply(
    intent: &NpcTalkIntent,
    result: &QuestTransitionOutcome,
) -> NpcTalkOutcome {
    let line = match result {
        QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_) => {
            "Defeat all four children of the rotten blood to open the path to Bakragore."
        }
        QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed => {
            "Complete your offerings before undertaking this quest."
        }
    };
    NpcConversations::bank_reply(intent, line.into())
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
    permit: Option<&SpellLanePermit>,
) -> NpcTalkOutcome {
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let action = match step.action {
        Action::Reply(line) => return NpcConversations::bank_reply(&intent, line.into()),
        Action::Held => return unknown(),
        action => action,
    };
    let _owned_permit = if permit.is_none() {
        let Some(permit) = admission.spell_lane_permit().await else {
            return unknown();
        };
        Some(permit)
    } else {
        None
    };
    let (Some(item_fence), Ok(id)) = (command.item_fence, CommandId::new(command.command_id))
    else {
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
            ) != Some(step.context.clone())
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
    let mutation = match action {
        Action::Torkada(topic) => {
            let level = match admission
                .root
                .read_character_progression(admission.character, fence.character_id)
                .await
            {
                Ok(Some(state))
                    if state.character_revision == fence.expected_character_revision =>
                {
                    state.level
                }
                _ => return unknown(),
            };
            if level < oteryn_gameplay_quests::quest::rotten_blood::REQUIRED_LEVEL {
                clear(admission, command.game_session_id);
                return NpcConversations::bank_reply(
                    &intent,
                    "You need at least level 250 to start our mission.".into(),
                );
            }
            let state = match admission
                .root
                .read_rotten_blood_state(admission.character, admission.holder, fence)
                .await
            {
                Ok(state) => state,
                Err(_) => return unknown(),
            };
            if state.access_stage() > 0 {
                Mutation::RottenRestoreDoor
            } else {
                return NpcConversations::bank_reply(
                    &intent,
                    topic
                        .map_or("Ask about our time here, then our mission.", topic_line)
                        .into(),
                );
            }
        }
        Action::SoulTask => {
            let state = match admission
                .root
                .read_soul_war_state(admission.character, admission.holder, fence)
                .await
            {
                Ok(state) => state,
                Err(_) => return unknown(),
            };
            if state.completed() {
                Mutation::Soul(SoulWarCommand::ReportCompletion)
            } else {
                if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() {
                    sessions.entry(command.game_session_id).or_default().endgame =
                        Some(Pending::Topic {
                            context: step.context,
                            npc: intent.npc_actor,
                            topic: Topic::SoulTask,
                        });
                } else {
                    return unknown();
                }
                return NpcConversations::bank_reply(&intent, topic_line(Topic::SoulTask).into());
            }
        }
        Action::Commit(Mutation::RottenAcknowledge) => {
            // Crystal bloodshade_rotten.lua:129-142. The generic Quest writer
            // records only the already-admitted access4 -> acknowledged5 edge.
            let state = match admission
                .root
                .read_rotten_blood_state(admission.character, admission.holder, fence)
                .await
            {
                Ok(state) => state,
                Err(_) => return unknown(),
            };
            match state.access_stage() {
                4 => Mutation::RottenAcknowledge,
                5 => {
                    clear(admission, command.game_session_id);
                    return NpcConversations::bank_reply(&intent,
                        "You have already started this quest. Ask me about taints if you seek cleansing.".into());
                }
                _ => {
                    clear(admission, command.game_session_id);
                    return NpcConversations::bank_reply(
                        &intent,
                        "Complete your offerings before undertaking this quest.".into(),
                    );
                }
            }
        }
        Action::Commit(Mutation::RottenMission) => {
            let state = match admission
                .root
                .read_rotten_blood_state(admission.character, admission.holder, fence)
                .await
            {
                Ok(state) => state,
                Err(_) => return unknown(),
            };
            if state.access_stage() > 0 {
                Mutation::RottenRestoreDoor
            } else {
                Mutation::RottenMission
            }
        }
        Action::Commit(mutation) => mutation,
        Action::Reply(_) | Action::Held => return rejected(),
    };
    let reference = CommandRef::new(command.game_session_id, id);
    let payment = if mutation == Mutation::RottenPurchase {
        match payment::prepare(admission, command, fence).await {
            Ok(payment) => Some(Box::new(payment)),
            Err(outcome) => return outcome,
        }
    } else {
        None
    };
    {
        let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() else {
            return unknown();
        };
        let state = sessions.entry(command.game_session_id).or_default();
        if matches!(state.endgame, Some(Pending::Commit { .. })) {
            return unknown();
        }
        state.endgame = Some(Pending::Commit {
            fence,
            command: reference,
            mutation,
            intent: intent.clone(),
            payment: payment.clone(),
        });
    }
    // A cancelled durable call may still commit. Discard the cached revision
    // before awaiting it so the next slot holder must read the Character root.
    slot.observe_endgame_progression();
    let outcome = match mutation {
        Mutation::Soul(action) => admission
            .root
            .commit_soul_war_command(
                admission.character,
                admission.holder,
                fence,
                reference,
                action,
            )
            .await
            .map(|outcome| soul_reply(&intent, action, &outcome)),
        Mutation::RottenCleanse(mode) => admission
            .root
            .commit_rotten_blood_cleanse(
                admission.character,
                admission.holder,
                fence,
                command.command_id,
                mode,
            )
            .await
            .map(|outcome| rotten_reply(&intent, &outcome)),
        Mutation::RottenMission | Mutation::RottenRestoreDoor => {
            if let Some(catalogue) = admission.quest_catalogue.as_ref() {
                slot.commit_quest_transition(
                    admission.root,
                    admission.character,
                    admission.holder,
                    fence,
                    mission_request(reference, mutation == Mutation::RottenRestoreDoor),
                    std::sync::Arc::clone(catalogue),
                )
                .await
                .map(|outcome| match outcome {
                    QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_) => NpcConversations::bank_reply(
                        &intent,
                        "Your mission has begun. Seek the ancient temple in the Drefian ruins."
                            .into(),
                    ),
                    QuestTransitionOutcome::Refused(_)
                        if mutation == Mutation::RottenRestoreDoor =>
                    {
                        NpcConversations::bank_reply(
                            &intent,
                            "You have already accepted this mission.".into(),
                        )
                    }
                    QuestTransitionOutcome::Refused(_)
                    | QuestTransitionOutcome::ObligationClosed => rejected(),
                })
            } else {
                Err(CharacterProgressionError::AuthorityRejected)
            }
        }
        Mutation::RottenAcknowledge => {
            if let Some(catalogue) = admission.quest_catalogue.as_ref() {
                slot.commit_quest_transition(
                    admission.root,
                    admission.character,
                    admission.holder,
                    fence,
                    acknowledgement_request(reference),
                    std::sync::Arc::clone(catalogue),
                )
                .await
                .map(|outcome| acknowledgement_reply(&intent, &outcome))
            } else {
                Err(CharacterProgressionError::AuthorityRejected)
            }
        }
        Mutation::RottenPurchase => match (admission.quest_catalogue.as_ref(), payment) {
            (Some(catalogue), Some(payment)) => admission
                .root
                .commit_rotten_blood_access_purchase(
                    admission.character,
                    admission.holder,
                    fence,
                    *payment,
                    std::sync::Arc::clone(catalogue),
                )
                .await
                .map(|outcome| payment::reply(&intent, &outcome)),
            _ => Err(CharacterProgressionError::AuthorityRejected),
        },
    };
    slot.observe_endgame_progression();
    drop(slot);
    match outcome {
        Ok(outcome) => {
            clear(admission, command.game_session_id);
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            outcome
        }
        Err(CharacterProgressionError::Unavailable(_)) => unknown(),
        Err(_) => {
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
        .endgame
        .clone()?;
    let Pending::Commit {
        fence,
        command: reference,
        mutation,
        intent,
        payment,
    } = pending
    else {
        return None;
    };
    if reference.command_id().get() != command.command_id
        || reference.game_session_id() != command.game_session_id
    {
        return None;
    }
    let mut slot = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    // This also covers a second cancellation during receipt reconciliation.
    slot.observe_endgame_progression();
    let result = match mutation {
        Mutation::Soul(action) => admission
            .root
            .reconcile_soul_war_command(admission.character, fence, reference, action)
            .await
            .map(|result| result.map(|result| soul_reply(&intent, action, &result))),
        Mutation::RottenCleanse(mode) => admission
            .root
            .reconcile_rotten_blood_cleanse(admission.character, fence, command.command_id, mode)
            .await
            .map(|result| result.map(|result| rotten_reply(&intent, &result))),
        Mutation::RottenMission | Mutation::RottenRestoreDoor => admission
            .root
            .reconcile_character_quest_transition(
                admission.character,
                fence.character_id,
                mission_request(reference, mutation == Mutation::RottenRestoreDoor),
            )
            .await
            .map(|result| {
                result.map(|_| {
                    NpcConversations::bank_reply(
                        &intent,
                        "Your mission has begun. Seek the ancient temple in the Drefian ruins."
                            .into(),
                    )
                })
            }),
        Mutation::RottenAcknowledge => admission
            .root
            .reconcile_character_quest_transition(
                admission.character,
                fence.character_id,
                acknowledgement_request(reference),
            )
            .await
            .map(|result| {
                result.map(|_| {
                    NpcConversations::bank_reply(&intent,
            "Defeat all four children of the rotten blood to open the path to Bakragore.".into())
                })
            }),
        Mutation::RottenPurchase => match payment {
            Some(payment) => admission
                .root
                .reconcile_rotten_blood_access_purchase(admission.character, fence, *payment)
                .await
                .map(|result| result.map(|outcome| payment::reply(&intent, &outcome))),
            None => Err(CharacterProgressionError::AuthorityRejected),
        },
    };
    drop(slot);
    match result {
        Ok(result) => {
            clear(admission, command.game_session_id);
            admission
                .refresh_quest_session(command.game_session_id, false)
                .await;
            Some(result.unwrap_or_else(|| NpcTalkOutcome::only(NpcIntentDisposition::Rejected)))
        }
        Err(_) => Some(unknown()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endgame_npc_requires_its_exact_confirmed_topic_and_npc() {
        assert!(choose("oteryn:npc.flickering_soul", "yes", None).is_none());
        assert!(choose("oteryn:npc.bloodshade_rotten", "yes", Some(Topic::SoulTask)).is_none());
        assert!(matches!(
            choose("oteryn:npc.torkada", "mission", None),
            Some((None, Action::Torkada(None)))
        ));
        assert!(choose("oteryn:npc.bloodshade_sacrifice", "yes", None).is_none());
        assert!(matches!(
            choose(
                "oteryn:npc.bloodshade_sacrifice",
                "yes",
                Some(Topic::SacrificeDetour)
            ),
            Some((Some(Topic::SacrificeConfirm), Action::Reply(_)))
        ));
        assert!(matches!(
            choose(
                "oteryn:npc.bloodshade_sacrifice",
                "yes",
                Some(Topic::SacrificeConfirm)
            ),
            Some((None, Action::Commit(Mutation::RottenPurchase)))
        ));
        assert!(choose("oteryn:npc.flickering_soul_fake", "taints", None).is_none());
        assert!(matches!(
            choose("oteryn:npc.flickering_soul", "yes", Some(Topic::SoulTask)),
            Some((None, Action::Commit(Mutation::Soul(SoulWarCommand::Start))))
        ));
        for mode in [CleanseMode::Regular, CleanseMode::All] {
            assert!(
                matches!(choose("oteryn:npc.bloodshade_rotten", "yes", Some(Topic::ConfirmCleanse(mode))),
                Some((None, Action::Commit(Mutation::RottenCleanse(found)))) if found == mode)
            );
        }
        assert!(matches!(
            choose(
                "oteryn:npc.bloodshade_rotten",
                "no",
                Some(Topic::ConfirmCleanse(CleanseMode::All))
            ),
            Some((None, Action::Reply(_)))
        ));
    }

    #[test]
    fn rotten_chain_acknowledgement_is_bound_to_the_rotten_bloodshade() {
        assert!(matches!(
            choose("oteryn:npc.bloodshade_rotten", "quest", None),
            Some((None, Action::Commit(Mutation::RottenAcknowledge)))
        ));
        for npc in [
            "oteryn:npc.bloodshade_sacrifice",
            "oteryn:npc.torkada",
            "oteryn:npc.bloodshade_rotten_fake",
        ] {
            assert!(!matches!(
                choose(npc, "quest", None),
                Some((_, Action::Commit(Mutation::RottenAcknowledge)))
            ));
        }
        assert!(choose("oteryn:npc.bloodshade_rotten", "yes", None).is_none());
    }
}
