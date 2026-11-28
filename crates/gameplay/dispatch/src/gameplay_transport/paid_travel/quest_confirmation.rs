//! A pending paid quote retains its existing fenced confirmation owner.
use super::*;
use oteryn_protocol_oteryn::npc_service::TRAVEL_CONFIRMATION_TIMEOUT_SECONDS;

impl Prompt {
    pub(super) fn matches_source_confirmation(&self, lowercase: &str) -> bool {
        super::contains_whole_keyword(lowercase, "yes")
            && (self.route.postman_tour_passage().is_some()
                || (self.route.service == "oteryn:service.travel.eremo"
                    && self.route.route.key == "cormaya"))
    }

    pub(super) fn is_live(&self, now: u64) -> bool {
        now.checked_sub(self.opened)
            .is_some_and(|age| age <= u64::from(TRAVEL_CONFIRMATION_TIMEOUT_SECONDS) * 1_000_000)
    }

    pub(super) fn matches_confirmation(
        &self,
        npc: EntityRef,
        context: &(u64, u64, String),
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        now: u64,
    ) -> bool {
        self.npc == npc && self.context == *context && self.fence == fence && self.is_live(now)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn has_pending_paid_confirmation(
        &self,
        command: connection::UseCommand,
        intent: &NpcTalkIntent,
        context: &(u64, u64, String),
        received_at: SemanticTimeMicros,
    ) -> bool {
        let text = intent.text.trim().to_ascii_lowercase();
        let Some(fence) = command.item_fence else {
            return false;
        };
        self.states
            .newhaven_travel
            .paid_prompts
            .lock()
            .is_ok_and(|prompts| {
                prompts.get(&command.game_session_id).is_some_and(|prompt| {
                    (text == "yes" || prompt.matches_source_confirmation(&text))
                        && prompt.matches_confirmation(
                            intent.npc_actor,
                            context,
                            fence,
                            received_at.get(),
                        )
                })
            })
    }
}
