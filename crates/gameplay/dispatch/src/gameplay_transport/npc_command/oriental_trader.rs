//! Yasir's source `ariki` trade keyword uses the existing window and trade owner.
use super::super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;

pub(super) fn open_alias(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
    mut outcome: NpcTalkOutcome,
) -> NpcTalkOutcome {
    // The ordinary talk owner has checked the rate, actor, generation and range.
    // Require its current conversation too: `ariki` is not a global trade alias.
    if outcome.disposition == NpcIntentDisposition::Ok
        && oteryn_gameplay_npc::greeting::words(&intent.text)
            .iter()
            .any(|word| word == "ariki")
        && admission
            .states
            .npc_conversations
            .current_npc(
                &admission.states.chat,
                runtime,
                actor,
                command.game_session_id,
                intent,
            )
            .is_some_and(|context| context.2 == "oteryn:npc.yasir")
    {
        outcome.window = admission.states.npc_conversations.open_window(
            &admission.states.chat,
            runtime,
            command.game_session_id,
            intent.npc_actor,
            super::super::super::item_ref_admission::pinned_index(
                runtime,
                admission.active_generation,
            ),
        );
    }
    outcome
}

/// Yasir's source placement when `outcome` opened the trade window on the intent's NPC.
pub(super) fn opened_source(
    runtime: &ChannelRuntimeV1,
    intent: &NpcTalkIntent,
    outcome: &NpcTalkOutcome,
) -> Option<(u16, u16, i8)> {
    let (shown, _) = outcome.window.as_ref()?.open?;
    if shown != intent.npc_actor {
        return None;
    }
    let visible = runtime.visible_entities();
    let npc = visible.npcs.iter().find(|npc| {
        npc.actor.placement_identity() == shown.identity && npc.generation == shown.generation
    })?;
    Some((
        u16::try_from(npc.position.x).ok()?,
        u16::try_from(npc.position.y).ok()?,
        i8::try_from(npc.position.floor).ok()?,
    ))
}

/// Crystal `201189f8` `yasir.lua:820,826`: the open grants Si, Ariki!, after the runtime lock is
/// released. Best effort: the window stays open on a failed grant, and the next open or a
/// committed trade grants the key the account still lacks.
pub(super) async fn grant_on_open(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    source: (u16, u16, i8),
) -> bool {
    use crate::foundation::{CommandId, CommandRef};
    let (Some(fence), Ok(command_id)) = (command.item_fence, CommandId::new(command.command_id))
    else {
        return false;
    };
    admission
        .root
        .grant_yasir_trade_open(
            admission.character,
            admission.holder,
            fence,
            CommandRef::new(command.game_session_id, command_id),
            source,
        )
        .await
        .is_ok()
}
