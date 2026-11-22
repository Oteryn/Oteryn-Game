//! A committed source greeting may create a hostile wave and decline focus.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;

pub(super) fn publish(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    outcome: &mut NpcTalkOutcome,
    callback: &RewardCallback,
) -> bool {
    let session = command.game_session_id;
    let Some(npc) = service_context::live_npc(runtime, actor, session, intent.npc_actor) else {
        return false;
    };
    if npc.key != callback.npc
        || admission.states.chat.engaged(session) != Some(npc.actor)
        || !command.item_fence.is_some_and(|fence| {
            runtime
                .bound_attacker_lease(actor, session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        })
    {
        return false;
    }
    if admission.states.chat.end_engagement(session, npc.actor)
        && let Some(conversation) = &mut outcome.conversation
    {
        conversation.open = None;
    }
    let at = crate::foundation::MovementLocalPosition {
        x: npc.position.x,
        y: npc.position.y,
        floor: npc.position.floor,
    };
    let guards: Vec<_> = callback
        .greeting_encounter
        .iter()
        .map(String::as_str)
        .collect();
    let _ = admission.spawn_quest_creatures(runtime, at, &guards);
    true
}
