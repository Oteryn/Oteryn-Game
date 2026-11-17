//! Publish a committed NPC receipt through the ordinary scoped presentation journal.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use oteryn_gameplay_quests::quest::npc_magic::{Cue, Target};

pub(super) fn publish(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    context: &(u64, u64, String),
    cues: &[Cue],
) {
    if cues.is_empty()
        || admission
            .states
            .npc_conversations
            .current_npc(
                &admission.states.chat,
                runtime,
                actor,
                command.game_session_id,
                intent,
            )
            .as_ref()
            != Some(context)
    {
        return;
    }
    let Some(fence) = command.item_fence else {
        return;
    };
    if !runtime
        .bound_attacker_lease(actor, command.game_session_id)
        .is_ok_and(|lease| {
            lease.character_id().as_bytes() == fence.character_id.as_bytes()
                && lease.generation() == fence.character_lease_generation
        })
    {
        return;
    }
    let Some(player) = service_context::speaker_position(runtime, actor, command.game_session_id)
    else {
        return;
    };
    let Some(npc) =
        service_context::live_npc(runtime, actor, command.game_session_id, intent.npc_actor)
    else {
        return;
    };
    // A visual is best effort after the durable receipt; it cannot refuse a committed payment.
    // Never hold the runtime guard across a second lock's await.
    let Ok(mut states) = admission.states.spell_states.try_lock() else {
        return;
    };
    if states.is_dead(actor) {
        return;
    }
    for cue in cues {
        let Some(effect) = cue.effect_id() else {
            continue;
        };
        let at = match cue.target {
            Target::Player => player,
            Target::Npc => npc.position,
        };
        let position = crate::foundation::MovementLocalPosition {
            x: at.x,
            y: at.y,
            floor: at.floor,
        };
        if runtime
            .house_actor_state(actor, command.game_session_id)
            .is_ok()
        {
            states.publish_house_decoration_visuals(
                runtime,
                actor,
                command.game_session_id,
                position,
                &[effect],
            );
        } else {
            states
                .sounds
                .record_item_effect(runtime, actor, position, effect, false);
        }
    }
}
