//! Confirmed non-quest Lynda service. Pending prompts confer no durable authority.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::npc_marriage::{
    self, MarriageAction, MarriageOutcome, MarriageReceipt, MarriageRequest,
};
use crate::foundation::{CommandId, CommandRef};

#[derive(Clone, Copy)]
enum Prompt {
    Marry,
    Name,
    Celebrate,
    Begin,
    Cancel,
    Divorce,
}
pub(super) struct Pending {
    prompt: Prompt,
    context: (u64, u64, String),
    npc: EntityRef,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    node: ([u8; 16], u64),
    opened: u64,
}
pub(super) enum Step {
    Ordinary,
    Reply(String),
    Execute(MarriageAction, Box<Pending>),
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
    if context.2 != npc_marriage::NPC_KEY {
        return None;
    }
    let mut sessions = admission.states.npc_conversations.sessions.lock().ok()?;
    let state = sessions.get_mut(&command.game_session_id)?;
    let pending = state.marriage.take();
    let had_pending = pending.is_some();
    let text = intent.text.trim_matches(' ');
    let now = admission.owner_now().get();
    if ["hi", "hello", "greetings", "no"]
        .iter()
        .any(|word| text.eq_ignore_ascii_case(word))
    {
        return Some(Step::Ordinary);
    }
    let Some(fence) = command.item_fence else {
        return Some(Step::Reply(
            "Your current character authority is unavailable.".into(),
        ));
    };
    let node = (
        *admission.holder.fact().node_id().as_bytes(),
        admission.holder.fact().registration_revision(),
    );
    let prompt = if text.eq_ignore_ascii_case("marry") {
        Some((
            Prompt::Marry,
            "Do you wish to marry? You will each need a wedding ring and a wedding outfit box.",
        ))
    } else if text.eq_ignore_ascii_case("celebration") {
        Some((
            Prompt::Celebrate,
            "Are you ready for your marriage celebration?",
        ))
    } else if text.eq_ignore_ascii_case("remove") || text.eq_ignore_ascii_case("remove proposal") {
        Some((
            Prompt::Cancel,
            "Do you wish to remove your pending proposal and recover its wedding items?",
        ))
    } else if text.eq_ignore_ascii_case("divorce") {
        Some((Prompt::Divorce, "Are you sure you wish to divorce?"))
    } else {
        None
    };
    if let Some((prompt, line)) = prompt {
        state.marriage = Some(Pending {
            prompt,
            context,
            npc: intent.npc_actor,
            fence,
            node,
            opened: now,
        });
        state.bank = None;
        state.premium = None;
        return Some(Step::Reply(line.into()));
    }
    if let Some(mut pending) = pending {
        if pending.context != context
            || pending.npc != intent.npc_actor
            || pending.fence != fence
            || pending.node != node
            || now < pending.opened
            || now - pending.opened >= 30_000_000
        {
            return Some(Step::Reply("The marriage offer has expired.".into()));
        }
        let action = match pending.prompt {
            Prompt::Marry if text.eq_ignore_ascii_case("yes") => {
                pending.prompt = Prompt::Name;
                pending.opened = now;
                state.marriage = Some(pending);
                return Some(Step::Reply(
                    "Who do you wish to marry? Tell me the character's name.".into(),
                ));
            }
            Prompt::Celebrate if text.eq_ignore_ascii_case("yes") => {
                pending.prompt = Prompt::Begin;
                pending.opened = now;
                state.marriage = Some(pending);
                return Some(Step::Reply(
                    "Say begin when you are ready for the ceremony.".into(),
                ));
            }
            Prompt::Name => Some(MarriageAction::Marry(text.into())),
            Prompt::Begin if text.eq_ignore_ascii_case("begin") => Some(MarriageAction::Celebrate),
            Prompt::Cancel if text.eq_ignore_ascii_case("yes") => Some(MarriageAction::Cancel),
            Prompt::Divorce if text.eq_ignore_ascii_case("yes") => Some(MarriageAction::Divorce),
            _ => None,
        };
        if let Some(action) = action {
            return Some(Step::Execute(action, Box::new(pending)));
        }
    }
    // Only an active marriage conversation owns unrelated replies. Without one,
    // Lynda's independently admitted quest and addon handlers must see the input.
    had_pending.then(|| {
        Step::Reply("Ask me about marry, celebration, remove proposal, or divorce.".into())
    })
}

fn completed(intent: &NpcTalkIntent, receipt: &MarriageReceipt) -> NpcTalkOutcome {
    NpcTalkOutcome {
        inventory_changed: receipt.operation != 5,
        ..NpcConversations::bank_reply(intent, receipt.reply.clone())
    }
}
pub(super) async fn replay(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    intent: Option<&NpcTalkIntent>,
) -> Option<NpcTalkOutcome> {
    let fence = command.item_fence?;
    let reference = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).ok()?,
    );
    match admission
        .root
        .reconcile_npc_marriage(admission.character, fence.character_id, reference)
        .await
    {
        Ok(Some(receipt)) => {
            if let Some(intent) = intent {
                if intent.text.len() > oteryn_protocol_oteryn::npc_service::MAX_TALK_TEXT_BYTES
                    || receipt.wire_binding
                        != npc_marriage::wire_binding(
                            intent.npc_actor.identity,
                            intent.npc_actor.generation,
                            &intent.text,
                        )
                {
                    return Some(NpcTalkOutcome::only(NpcIntentDisposition::Rejected));
                }
                Some(completed(intent, &receipt))
            } else {
                Some(completed(
                    &NpcTalkIntent {
                        npc_actor: EntityRef {
                            identity: receipt.npc_identity,
                            generation: receipt.npc_generation,
                        },
                        text: String::new(),
                    },
                    &receipt,
                ))
            }
        }
        Ok(None) => None,
        Err(_) => Some(NpcTalkOutcome {
            unknown: true,
            ..NpcTalkOutcome::only(NpcIntentDisposition::Rejected)
        }),
    }
}

pub(super) async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: NpcTalkIntent,
    step: Step,
) -> NpcTalkOutcome {
    let (action, pending) = match step {
        Step::Reply(line) => return NpcConversations::bank_reply(&intent, line),
        Step::Execute(action, pending) => (action, *pending),
        Step::Ordinary => return NpcTalkOutcome::only(NpcIntentDisposition::Rejected),
    };
    let rejected = || NpcTalkOutcome::only(NpcIntentDisposition::Rejected);
    let unknown = || NpcTalkOutcome {
        unknown: true,
        ..rejected()
    };
    let Some(_permit) = admission.spell_lane_permit().await else {
        return rejected();
    };
    if !admission
        .states
        .player_trades
        .physical_service(command.game_session_id)
    {
        return rejected();
    }
    let Ok(id) = CommandId::new(command.command_id) else {
        return rejected();
    };
    let reference = CommandRef::new(command.game_session_id, id);
    let mut slot = admission
        .revision_sequencer
        .acquire(pending.fence.character_id)
        .await;
    let Ok(Some(fence)) = admission.current_quest_fence(command.game_session_id).await else {
        return unknown();
    };
    if fence.item_fence() != pending.fence {
        return rejected();
    }
    let runtime = admission.runtime.lock().await;
    if admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        &runtime,
        actor,
        command.game_session_id,
        &intent,
    ) != Some(pending.context)
    {
        return rejected();
    }
    let Some(native) = admission
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    else {
        return rejected();
    };
    let Some(index) =
        super::super::item_ref_admission::pinned_index(&runtime, admission.active_generation)
    else {
        return rejected();
    };
    let definition = |key: &str| {
        index.revision(key).map(
            |revision| crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: revision.into(),
            },
        )
    };
    let (Some(ring), Some(outfit_box), Some(engraved)) = (
        definition("oteryn:item.tibia.i3004"),
        definition("oteryn:item.tibia.i9586"),
        definition("oteryn:item.tibia.i9585"),
    ) else {
        return rejected();
    };
    drop(runtime);
    let derive = |slot: u8| {
        super::super::npc_trade_command::derived(
            format!("oteryn:npc-marriage:v1:{slot}").as_bytes(),
            command.game_session_id,
            command.command_id,
        )
    };
    let request = MarriageRequest {
        command: reference,
        occurrence: derive(0),
        event: derive(1),
        npc_identity: intent.npc_actor.identity,
        npc_generation: intent.npc_actor.generation,
        wire_binding: npc_marriage::wire_binding(
            intent.npc_actor.identity,
            intent.npc_actor.generation,
            &intent.text,
        ),
        action,
        definitions: [ring, outfit_box, engraved],
    };
    let weights = super::super::item_ref_admission::NativeItemWeights(native)
        .with_wheel(admission.premium_coordinator);
    match slot
        .commit_npc_marriage(
            admission.root,
            admission.character,
            admission.holder,
            fence,
            request,
            &weights,
        )
        .await
    {
        Ok(MarriageOutcome::Committed(receipt)) => completed(&intent, &receipt),
        Ok(MarriageOutcome::Refused(line)) => NpcConversations::bank_reply(&intent, line),
        Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(
            _,
        )) => unknown(),
        Err(_) => rejected(),
    }
}
