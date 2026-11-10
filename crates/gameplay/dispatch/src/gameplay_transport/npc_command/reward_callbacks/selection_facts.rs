//! Read carried facts only for source branches that actually need them.
use super::*;

pub(in crate::gameplay_transport::npc_command) fn needs_facts(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    runtime: &ChannelRuntimeV1,
) -> bool {
    let Some(context) = admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        command.game_session_id,
        intent,
    ) else {
        return false;
    };
    let text = intent.text.trim().to_ascii_lowercase();
    let Some(catalogue) = admission.quest_catalogue.as_deref() else {
        return false;
    };
    let callbacks = reward_callbacks::offered_for(catalogue).unwrap_or_default();
    #[cfg(test)]
    let callbacks = admission
        .states
        .npc_conversations
        .fixture_callback_rows
        .get()
        .unwrap_or(&callbacks);
    callbacks.iter().any(|callback| {
        callback.npc == context.2
            && callback.keywords.contains(&text)
            && (!callback.inventory.is_empty() || callback.minimum_level.is_some())
    }) || offers::load(admission).is_some_and(|document| {
        document.offers.iter().any(|offer| {
            offer.npc == context.2
                && offer.keywords.contains(&text)
                && offer.minimum_level.is_some()
        })
    })
}
