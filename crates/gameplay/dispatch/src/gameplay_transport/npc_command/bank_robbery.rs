//! Native NPC dialogue for the World-owned Bank Robbery.
use super::*;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::world_event::bank_robbery::{
    Conversation, HandIn, Phase, RecognizedBag, ReturnReceipt, TOWNCRYER, TOWNCRYER_LINE,
    banker_city,
};
use crate::foundation::{CommandId, CommandRef};

pub(super) fn interested(key: &str) -> bool {
    key == TOWNCRYER || banker_city(key).is_some()
}

pub(super) fn clue(key: &str) -> &'static str {
    match banker_city(key) {
        Some("thais") => {
            "HELP! We have been robbed! I can't give you any gold until the robber has been brought to justice or we have received a compensation from the king. I think the robbers ran towards the Ancient Temple."
        }
        Some("carlin") => {
            "I'm really sorry, but I can't serve you right now. Maybe you can help find the robber? He ran off into the direction of the ghostlands."
        }
        Some("venore") => {
            "Those Shadowthorn elves have gone too far this time! Unless someone finds and kills those elves I can't do anything for you."
        }
        Some("ab_dendriel") => {
            "HELP! We have been robbed! I fear they hauled it into the Kazordoon mines. I can't serve you until we have gold again."
        }
        _ => "Sorry, not possible.",
    }
}

fn recognition(key: &str) -> &'static str {
    match banker_city(key) {
        Some("thais") => {
            "I'm sorry, but we have been robb- hey, what's that bag? Have you hunted down the bank robbers?"
        }
        Some("carlin") => {
            "*Sob*... *sniff*... Ohhhh!! All that shiny glittering gold! Have you caught the bank robbers?"
        }
        Some("venore") => "Those Shadowthorn elves have - wait a minute. Is that the missing gold?",
        _ => {
            "I'm sorry, but we still can't... wait. What's that big bag over your shoulder? Have you caught the bank robbers?"
        }
    }
}

fn reopened(key: &str) -> &'static str {
    match banker_city(key) {
        Some("thais") => {
            "The filthy bank robber has been brought to justice and almost all gold has been returned. We are back in business!"
        }
        Some("carlin") => {
            "I can't believe it - the bank robber has actually been caught and the gold been returned. I'm so relieved. How may I serve you?"
        }
        Some("venore") => {
            "Ha, those lousy elves got what they deserved. All the robbed money has been returned to the bank. What can I do for you?"
        }
        _ => {
            "Those dwarven scumbags have been caught and the gold from the bank robbery was returned. I can fulfil your bank business as usual."
        }
    }
}

fn paid(key: &str) -> &'static str {
    match banker_city(key) {
        Some("thais") => {
            "Hey, awesome. Good job. Here's some gold for all your work. Thanks to you we can open the bank again."
        }
        Some("carlin") => {
            "Thank you so much! I feel much safer now knowing those robbers have been caught. Here's a little something for your effort. We can open the bank again!"
        }
        _ => {
            "Thank you! Maybe you humans aren't all too bad. Here's a little something for your effort. We can open the bank again!"
        }
    }
}

fn take(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> Option<RecognizedBag> {
    admission
        .states
        .npc_conversations
        .sessions
        .lock()
        .ok()?
        .get_mut(&session)?
        .robbery
        .take()
}

pub(super) fn clear(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) {
    take(admission, session);
}

pub(super) async fn talk(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
    context: (u64, u64, String),
) -> Option<NpcTalkOutcome> {
    let (_, generation, key) = context;
    let pending = take(admission, command.game_session_id);
    let fence = command.item_fence?;
    let conversation = Conversation {
        generation,
        key,
        npc: intent.npc_actor,
    };
    let reply = |line: &str| Some(NpcConversations::bank_reply(intent, line.into()));
    let greeting = intent.text.trim_matches(' ').eq_ignore_ascii_case("hi")
        || intent.text.trim_matches(' ').eq_ignore_ascii_case("hello")
        || intent.text.trim_matches(' ').eq_ignore_ascii_case("hail");
    if intent.text.trim_matches(' ').eq_ignore_ascii_case("yes")
        && let Some(bag) = pending
    {
        let reference = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        );
        let Some(native) = admission
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return reply("Sorry, not possible.");
        };
        let weights = super::super::item_ref_admission::NativeItemWeights(native)
            .with_wheel(admission.premium_coordinator);
        let result = admission
            .root
            .return_bank_robbery_bag(
                admission.character,
                admission.holder,
                fence,
                HandIn {
                    command: reference,
                    conversation,
                    text: intent.text.clone(),
                    bag,
                },
                &weights,
            )
            .await;
        return match result {
            Ok(Some(receipt)) => {
                #[cfg(test)]
                if admission
                    .states
                    .npc_conversations
                    .sessions
                    .lock()
                    .is_ok_and(|mut sessions| {
                        sessions
                            .get_mut(&command.game_session_id)
                            .is_some_and(|state| {
                                state.interrupt_robbery_after_commit.take()
                                    == Some(command.command_id)
                            })
                    })
                {
                    return Some(held_bank_outcome());
                }
                Some(completion(intent, &receipt))
            }
            Ok(None) => reply("Sorry, not possible."),
            Err(CharacterProgressionError::Unavailable(_)) => Some(held_bank_outcome()),
            Err(_) => Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected)),
        };
    }

    let key = conversation.key.clone();
    let state = match admission
        .root
        .bank_robbery_dialogue(
            admission.character,
            admission.holder,
            fence,
            conversation,
            greeting,
        )
        .await
    {
        Ok(state) => state?,
        Err(CharacterProgressionError::Unavailable(_)) => return Some(held_bank_outcome()),
        Err(_) => return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected)),
    };
    if key == TOWNCRYER {
        return greeting.then(|| NpcConversations::bank_reply(intent, TOWNCRYER_LINE.into()));
    }
    if state.phase.suspended() {
        admission
            .states
            .npc_conversations
            .cancel_bank(command.game_session_id);
        if let Some(bag) = state.bag
            && let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        {
            sessions.entry(command.game_session_id).or_default().robbery = Some(bag);
            return reply(recognition(&key));
        }
        return reply(clue(&key));
    }
    (greeting && state.phase == Phase::Returned)
        .then(|| NpcConversations::bank_reply(intent, reopened(&key).into()))
}

fn completion(intent: &NpcTalkIntent, receipt: &ReturnReceipt) -> NpcTalkOutcome {
    let mut outcome = NpcConversations::bank_reply(intent, paid(&receipt.npc_key).into());
    outcome.inventory_changed = true;
    outcome
}

pub(super) async fn replay(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: Option<&NpcTalkIntent>,
) -> Option<NpcTalkOutcome> {
    // Fresh talk can only match a committed confirmation. The reconnect caller
    // has the exact retained CommandRef and need not supply the original payload.
    if intent.is_some_and(|intent| !intent.text.trim_matches(' ').eq_ignore_ascii_case("yes")) {
        return None;
    }
    let (fence, reference) = retained_bank_key(&command).ok().flatten()?;
    match admission
        .root
        .reconcile_bank_robbery_return(
            admission.character,
            admission.holder,
            fence,
            reference,
            intent.cloned(),
        )
        .await
    {
        Ok(Some(receipt)) => {
            let intent = NpcTalkIntent {
                npc_actor: receipt.npc,
                text: String::new(),
            };
            let mut outcome = completion(&intent, &receipt);
            let runtime = admission.runtime.lock().await;
            if admission
                .states
                .npc_conversations
                .current_npc(
                    &admission.states.chat,
                    &runtime,
                    actor,
                    command.game_session_id,
                    &intent,
                )
                .is_none()
                && let Some(conversation) = outcome.conversation.as_mut()
            {
                conversation.open = None;
            }
            Some(outcome)
        }
        Ok(None) => None,
        Err(CharacterProgressionError::Unavailable(_)) => Some(held_bank_outcome()),
        Err(_) => Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected)),
    }
}
