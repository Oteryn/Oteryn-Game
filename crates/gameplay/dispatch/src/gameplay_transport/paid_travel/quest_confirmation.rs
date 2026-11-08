//! A pending paid quote retains its existing fenced confirmation owner.
use super::*;

impl Prompt {
    pub(super) fn matches_confirmation(
        &self,
        npc: EntityRef,
        context: &(u64, u64, String),
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        now: u64,
    ) -> bool {
        self.npc == npc
            && self.context == *context
            && self.fence == fence
            && now >= self.opened
            && now - self.opened < 60_000_000
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn has_pending_paid_confirmation(
        &self,
        command: connection::UseCommand,
        intent: &NpcTalkIntent,
        context: &(u64, u64, String),
    ) -> bool {
        if !intent.text.trim().eq_ignore_ascii_case("yes") {
            return false;
        }
        let Some(fence) = command.item_fence else {
            return false;
        };
        self.states
            .newhaven_travel
            .paid_prompts
            .lock()
            .is_ok_and(|prompts| {
                prompts.get(&command.game_session_id).is_some_and(|prompt| {
                    prompt.matches_confirmation(
                        intent.npc_actor,
                        context,
                        fence,
                        self.owner_now().get(),
                    )
                })
            })
    }
}
