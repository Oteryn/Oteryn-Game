//! Placed Viola uses command 7; current Wiki retires her spell-teaching service.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::tutorial_viola::{ViolaAction, ViolaReply};
use crate::foundation::{CommandId, CommandRef};

type Context = (u64, u64, String);
#[derive(Clone)]
pub(super) enum Pending {
    Offer {
        context: Context,
        npc: EntityRef,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        opened: u64,
    },
    Commit {
        character: crate::domain::CharacterId,
        command: CommandRef,
        intent: NpcTalkIntent,
    },
}
pub(super) struct Step {
    context: Context,
    action: Option<ViolaAction>,
    reply: Option<&'static str>,
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
    if context.2 != "oteryn:npc.viola" {
        return None;
    }
    let text = intent.text.trim().to_ascii_lowercase();
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    if matches!(state.viola, Some(Pending::Commit { .. })) {
        return Some(Step {
            context,
            action: None,
            reply: None,
        });
    }
    let pending = state.viola.take();
    let mut reply = None;
    let action = match text.as_str() {
        "mission" | "task" | "book" => Some(ViolaAction::Mission),
        "yes" => match pending {
            Some(Pending::Offer {
                context: prior,
                npc,
                fence,
                opened,
            }) if prior == context
                && npc == intent.npc_actor
                && Some(fence) == command.item_fence
                && admission
                    .owner_now()
                    .get()
                    .checked_sub(opened)
                    .is_some_and(|age| age < 30_000_000) =>
            {
                Some(ViolaAction::Accept)
            }
            _ => {
                reply = Some("Ask me about a mission first.");
                None
            }
        },
        "no" => {
            reply = Some("Please come back if you change your mind.");
            None
        }
        "learn" | "spells" | "teach" => {
            reply = Some("I have retired from teaching spells and now devote my time to reading.");
            None
        }
        _ => return None,
    };
    Some(Step {
        context,
        action,
        reply,
    })
}
fn line(reply: ViolaReply) -> &'static str {
    match reply {
        ViolaReply::ReturnOffer => "Glad to see you are back. Did you find my book? Say yes or no.",
        ViolaReply::Offer => {
            "A ghost took my Book of Wisdom into the mines to the northeast. Would you retrieve it for me? Say yes or no."
        }
        ViolaReply::Accepted => {
            "Thank you! The book should be in a chest in the corrupted mines to the northeast. Please be careful."
        }
        ViolaReply::MissingBook => {
            "Have you found my Book of Wisdom? Look in the mines to the northeast."
        }
        ViolaReply::Completed => {
            "You found my Book of Wisdom! Please accept this scarf and 800 experience as thanks."
        }
        ViolaReply::AlreadyCompleted => {
            "You have already helped me recover my book. Thank you again!"
        }
        ViolaReply::NoRoom => {
            "I cannot give you the scarf. Please make room and return with my book."
        }
    }
}
fn complete(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    intent: &NpcTalkIntent,
    context: Option<Context>,
    reply: ViolaReply,
) -> NpcTalkOutcome {
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
        && let Some(state) = sessions.get_mut(&command.game_session_id)
    {
        state.viola = if matches!(reply, ViolaReply::Offer | ViolaReply::ReturnOffer) {
            context
                .zip(command.item_fence)
                .map(|(context, fence)| Pending::Offer {
                    context,
                    npc: intent.npc_actor,
                    fence,
                    opened: admission.owner_now().get(),
                })
        } else {
            None
        };
    }
    if reply == ViolaReply::Completed
        && let Some(fence) = command.item_fence
    {
        admission.states.level_refresh.mark([fence.character_id]);
    }
    NpcTalkOutcome {
        inventory_changed: reply == ViolaReply::Completed,
        ..NpcConversations::bank_reply(intent, line(reply).into())
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
    let Some(action) = step.action else {
        return step.reply.map_or_else(unknown, |line| {
            NpcConversations::bank_reply(&intent, line.into())
        });
    };
    let (Some(item_fence), Ok(id), Some(binding), Some(native)) = (
        command.item_fence,
        CommandId::new(command.command_id),
        admission.progression_binding(command.game_session_id),
        admission
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay),
    ) else {
        return rejected();
    };
    let Some(revision) = native
        .item_index()
        .and_then(|index| index.revision("oteryn:item.tibia.i3572"))
    else {
        return rejected();
    };
    let output = crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i3572".into(),
        revision_ref: revision.into(),
    };
    let mut slot = admission
        .revision_sequencer
        .acquire(item_fence.character_id)
        .await;
    let Ok(Some(mut fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if fence.item_fence() != item_fence {
        return rejected();
    }
    {
        let runtime = admission.runtime.lock().await;
        let Ok(states) = admission.states.spell_states.try_lock() else {
            return rejected();
        };
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
    let Ok(cursor) = slot.cursor(admission.root, admission.character).await else {
        return unknown();
    };
    fence.expected_character_revision = cursor;
    let reference = CommandRef::new(command.game_session_id, id);
    if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock() {
        sessions.entry(command.game_session_id).or_default().viola = Some(Pending::Commit {
            character: fence.character_id,
            command: reference,
            intent: intent.clone(),
        });
    } else {
        return unknown();
    }
    let result = admission
        .root
        .commit_viola(
            admission.character,
            admission.holder,
            fence,
            reference,
            action,
            binding.policy.clone(),
            &super::super::item_ref_admission::NativeItemWeights(native),
            output,
            native.source_digest(),
        )
        .await;
    slot.observe_endgame_progression();
    match result {
        Ok(reply) => complete(admission, command, &intent, Some(step.context), reply),
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        Err(_) => {
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
            {
                state.viola = None;
            }
            rejected()
        }
    }
}
pub(super) async fn resume(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<NpcTalkOutcome> {
    let (character, reference, intent) = {
        let sessions = admission.states.npc_conversations.sessions.lock().ok()?;
        match sessions.get(&command.game_session_id)?.viola.as_ref()? {
            Pending::Commit {
                character,
                command: reference,
                intent,
            } if reference.command_id().get() == command.command_id => {
                (*character, *reference, intent.clone())
            }
            _ => return None,
        }
    };
    match admission
        .root
        .reconcile_viola(admission.character, character, reference)
        .await
    {
        Ok(Some(reply)) => Some(complete(admission, command, &intent, None, reply)),
        Ok(None) => {
            if let Ok(mut sessions) = admission.states.npc_conversations.sessions.lock()
                && let Some(state) = sessions.get_mut(&command.game_session_id)
            {
                state.viola = None;
            }
            Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected))
        }
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}
