//! Yasir's source reward for opening the existing trade window.
use super::super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;

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
