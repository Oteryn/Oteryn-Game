//! Native NPC dialogue for the World-owned Bank Robbery.
use super::*;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::world_event::bank_robbery::{
    Conversation, HandIn, Phase, RecognizedBag, banker_city,
};
use crate::foundation::{CommandId, CommandRef};

#[cfg(test)]
impl NpcConversations {
    pub(in crate::gameplay_transport) fn fixture_lose_robbery_ack(
        &self,
        session: GameSessionId,
        command: u64,
    ) -> Result<(), &'static str> {
        let mut sessions = self.sessions.lock().map_err(|_| "sessions")?;
        sessions
            .get_mut(&session)
            .ok_or("session")?
            .interrupt_robbery_after_commit = Some(command);
        Ok(())
    }
}

pub(super) fn interested(key: &str) -> bool {
    banker_city(key).is_some()
}

// Hideouts follow the Bank Robbery walkthrough in the Tibia Fandom archive.
pub(super) fn clue(key: &str) -> &'static str {
    match banker_city(key) {
        Some("thais") => {
            "The bank has closed until we get the gold back. The thieves were last seen heading for the Ancient Temple."
        }
        Some("carlin") => {
            "The robber emptied our vault and fled toward the Ghostlands. I can't take care of any banking until we recover the gold."
        }
        Some("venore") => {
            "The Shadowthorn elves raided the bank. Look for them in Shadowthorn if you can help. Our counters are closed until the gold comes back."
        }
        Some("ab_dendriel") => {
            "The dwarves took our gold into the mines near Kazordoon. We need it back before I can reopen the bank."
        }
        _ => "Sorry, not possible.",
    }
}

const RECOGNITION: &str = "Wait, what's that bag? Did you recover the stolen gold?";
const REOPENED: &str =
    "We've recovered the stolen gold, and the bank is back in business. How can I help?";
const PAID: &str =
    "Good job tracking down the thieves. Please take this reward. The bank can reopen now.";

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
    permit: Option<&crate::durability::spell_owner_commit::SpellLanePermit>,
    native: Option<&crate::content::native_gameplay::NativeGameplayState>,
) -> Option<NpcTalkOutcome> {
    let (_, generation, key) = context;
    let pending = take(admission, command.game_session_id);
    let fence = command.item_fence?;
    let conversation = Conversation {
        generation,
        key,
        npc: intent.npc_actor,
        reset_epoch: admission.reset_epoch,
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
        let (Some(native), Some(permit)) = (native, permit) else {
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
                permit,
                &weights,
            )
            .await;
        return match result {
            Ok(Some(_)) => {
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
                Some(completion(intent))
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
    if state.phase.suspended() {
        admission
            .states
            .npc_conversations
            .cancel_bank(command.game_session_id);
        if let Some(bag) = state.bag
            && let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        {
            sessions.entry(command.game_session_id).or_default().robbery = Some(bag);
            return reply(RECOGNITION);
        }
        return reply(clue(&key));
    }
    (greeting && state.phase == Phase::Returned)
        .then(|| NpcConversations::bank_reply(intent, REOPENED.into()))
}

fn completion(intent: &NpcTalkIntent) -> NpcTalkOutcome {
    let mut outcome = NpcConversations::bank_reply(intent, PAID.into());
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
            let mut outcome = completion(&intent);
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
