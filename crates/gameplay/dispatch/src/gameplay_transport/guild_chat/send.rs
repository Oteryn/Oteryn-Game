use super::super::ComposedFreshAdmission;
use super::*;
use crate::chat::{ChatText, relay::RelayDestination};
use crate::domain::guild::GuildId;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in super::super) async fn guild_chat_send(
        &self,
        admitted: AdmittedSession,
        text: String,
    ) -> ChatIntentResult {
        self.guild_chat_send_to(admitted, text, None).await
    }

    pub(super) async fn guild_chat_send_to(
        &self,
        admitted: AdmittedSession,
        text: String,
        guest_guild: Option<GuildId>,
    ) -> ChatIntentResult {
        if let Some(wait_seconds) = self.staff_mute_wait(admitted.game_session_id).await {
            return ChatIntentResult {
                disposition: ChatDisposition::Muted,
                wait_seconds,
            };
        }
        if !self.states.chat.guild.matches(&admitted) {
            return result(ChatDisposition::Rejected);
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
            let Some(fence) = qualified_fence(&admitted, &runtime) else {
                return result(ChatDisposition::Rejected);
            };
            if let Some(guild_id) = guest_guild {
                self.root
                    .with_guild_guest_custody(
                        self.character,
                        self.holder,
                        fence,
                        guild_id,
                        None,
                        |_, guest, name| {
                            guest.then_some((RelayDestination::GuildGuest { guild_id }, name))
                        },
                    )
                    .await
            } else {
                self.root
                    .with_guild_chat_custody(
                        self.character,
                        self.holder,
                        fence,
                        None,
                        |membership, name| {
                            membership.map(|member| {
                                (
                                    RelayDestination::Guild {
                                        guild_id: member.guild_id,
                                        rank: member.rank,
                                    },
                                    name,
                                )
                            })
                        },
                    )
                    .await
            }
        };
        let (destination, name) = match accepted {
            Ok(Some(accepted)) => accepted,
            Ok(None) => return result(ChatDisposition::NotAllowed),
            Err(_) => return result(ChatDisposition::ChatUnavailable),
        };
        // Exactly one autocommit NOTIFY, outside Guild custody. OK means sent only.
        match relay.send(destination, name, text).await {
            Ok(()) => result(ChatDisposition::Ok),
            Err(disposition) => result(disposition),
        }
    }
}
