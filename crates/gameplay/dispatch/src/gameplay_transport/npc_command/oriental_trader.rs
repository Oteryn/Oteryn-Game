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
