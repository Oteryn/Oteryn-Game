//! Pure World-change dialogue selection uses the common conversation/Quest owner.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::quest_dialogue::{self, Choice, Step};
use super::*;
use crate::durability::quest_state::NativeWorldChangeNpc;
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::world_change_npcs as rules;

pub(super) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> Option<Step> {
    quest_dialogue::prepare(
        admission,
        actor,
        command,
        intent,
        runtime,
        |context, _, _, tracks| {
            let action = rules::select(&context.2, &intent.text, tracks)?;
            Some(Choice {
                transition: Some(action.transition().into()),
                next_topic: None,
                reply: vec![action.reply().into()],
            })
        },
    )
}

/// The shared dialogue/trade executors call this while retaining their checked
/// conversation and current runtime owner, after their existing dead/idle check.
/// It does not reacquire spell_states while those executors hold that guard.
/// Source keys/coordinates are resolved
/// inside the private context constructor, never accepted from USE payloads.
pub(in crate::gameplay_transport) fn source(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    npc_identity: [u8; 16],
    npc_generation: u64,
    runtime: &ChannelRuntimeV1,
) -> Option<NativeWorldChangeNpc> {
    let fence = command.item_fence?;
    NativeWorldChangeNpc::from_current_runtime(
        runtime,
        actor,
        CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        ),
        npc_identity,
        npc_generation,
        admission.reset_epoch,
        fence,
    )
    .ok()
}
