//! Advisory provenance is loaded before the talk handler takes its runtime lock.
use super::*;
use crate::durability::stag_beaks::{HARTFORD, STAGE, StagBeakPreview};

pub(super) async fn preview(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: super::super::connection::UseCommand,
    intent: &NpcTalkIntent,
) -> Result<Option<StagBeakPreview>, ()> {
    let active = admission
        .states
        .quest_sessions
        .lock()
        .map_err(|_| ())?
        .get(&command.game_session_id)
        .and_then(|s| s.copy.as_ref())
        .is_some_and(|copy| copy.tracks().get(STAGE) == Some(&6));
    if !active {
        return Ok(None);
    }
    let Some(fence) = command.item_fence else {
        return Ok(None);
    };
    let hartford = {
        let runtime = admission.runtime.lock().await;
        admission
            .states
            .npc_conversations
            .current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                command.game_session_id,
                intent,
            )
            .is_some_and(|context| context.2 == HARTFORD)
    };
    if !hartford {
        return Ok(None);
    }
    admission
        .root
        .read_stag_beak_preview(admission.character, fence.character_id)
        .await
        .map(Some)
        .map_err(|_| ())
}
