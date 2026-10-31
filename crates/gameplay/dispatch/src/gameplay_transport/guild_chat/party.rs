//! Party uses the existing World relay and the Party owner's durable membership.
use super::super::ComposedFreshAdmission;
use super::*;
use crate::chat::{ChatText, relay::RelayDestination};
use oteryn_protocol_oteryn::{chat::ChatLine, chat::party_room::CAPABILITY_CHAT_PARTY_ROOM_V1};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in super::super) async fn party_chat_send(
        &self,
        admitted: AdmittedSession,
        text: String,
    ) -> ChatIntentResult {
        if !self.states.chat.party.matches(&admitted)
            || !admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_CHAT_PARTY_ROOM_V1)
        {
            return result(ChatDisposition::Rejected);
        }
        if let Some(wait_seconds) = self.staff_mute_wait(admitted.game_session_id).await {
            return ChatIntentResult {
                disposition: ChatDisposition::Muted,
                wait_seconds,
            };
        }
        if let Some(refused) =
            self.states
                .chat
                .pre_gate_relay(admitted.game_session_id, None, &text, self.owner_now())
        {
            return refused;
        }
        let Ok(text) = ChatText::parse(&text) else {
            return result(ChatDisposition::Rejected);
        };
        let Some(relay) = self.states.chat.relay() else {
            return result(ChatDisposition::ChatUnavailable);
        };
        let accepted = {
            let runtime = self.runtime.lock().await;
            let Some(fence) = qualified_chat_fence(&admitted, &runtime) else {
                return result(ChatDisposition::Rejected);
            };
            self.root
                .with_party_chat_custody(self.character, self.holder, fence, None, |party, name| {
                    party.map(|party_id| (party_id, name))
                })
                .await
        };
        let (party_id, name) = match accepted {
            Ok(Some(data)) => data,
            Ok(None) => return result(ChatDisposition::NotAllowed),
            Err(_) => return result(ChatDisposition::ChatUnavailable),
        };
        match relay
            .send(RelayDestination::Party { party_id }, name, text)
            .await
        {
            Ok(()) => result(ChatDisposition::Ok),
            Err(disposition) => result(disposition),
        }
    }

    pub(super) async fn party_chat_accept(
        &self,
        admitted: AdmittedSession,
        candidate: &ChatLine,
        destination: crate::chat::egress::PartyChatDestination,
    ) -> Option<ChatLine> {
        let accepted = if self.states.chat.party.matches(&admitted)
            && admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_CHAT_PARTY_ROOM_V1)
        {
            let runtime = self.runtime.lock().await;
            if let Some(fence) = qualified_chat_fence(&admitted, &runtime) {
                self.root
                    .with_party_chat_custody(
                        self.character,
                        self.holder,
                        fence,
                        Some(destination.sent_at_micros),
                        |party, _| {
                            self.states.chat.accept_party_line(
                                &admitted,
                                candidate,
                                destination,
                                party == Some(destination.party_id),
                            )
                        },
                    )
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            }
        } else {
            None
        };
        if accepted.is_none() {
            self.states
                .chat
                .accept_party_line(&admitted, candidate, destination, false);
        }
        accepted
    }
}
