use super::super::ComposedFreshAdmission;
use super::*;
use crate::chat::{ChatText, relay::RelayDestination};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in super::super) async fn guild_chat_send(
        &self,
        admitted: AdmittedSession,
        text: String,
    ) -> ChatIntentResult {
        if !self.chat.guild.matches(&admitted) {
            return result(ChatDisposition::Rejected);
        }
        if let Some(refused) =
            self.chat
                .pre_gate_relay(admitted.game_session_id, None, &text, self.owner_now())
        {
            return refused;
        }
        let Ok(text) = ChatText::parse(&text) else {
            return result(ChatDisposition::Rejected);
        };
        let Some(relay) = self.chat.relay() else {
            return result(ChatDisposition::ChatUnavailable);
        };
        let accepted = {
            let runtime = self.runtime.lock().await;
            let Some(fence) = qualified_fence(&admitted, &runtime) else {
                return result(ChatDisposition::Rejected);
            };
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
