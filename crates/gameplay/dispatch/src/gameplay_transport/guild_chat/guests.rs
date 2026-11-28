use super::super::ComposedFreshAdmission;
use super::*;
use crate::domain::{CharacterId, guild::GuildId};
use crate::durability::guild_chat::GuildGuestEdit;
use oteryn_protocol_oteryn::chat::{self, guild_guest::GuildGuestIntent};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in super::super) async fn guild_guest_command(
        &self,
        admitted: AdmittedSession,
        intent: GuildGuestIntent,
    ) -> Option<Vec<u8>> {
        if !self.states.chat.guild.matches(&admitted) {
            return None;
        }
        let accepted = match intent {
            GuildGuestIntent::Send { guild, text } => {
                let guild = GuildId::from_bytes(guild).ok()?;
                self.guild_chat_send_to(admitted, text, Some(guild)).await
            }
            GuildGuestIntent::Invite { character } | GuildGuestIntent::Exclude { character } => {
                let target = CharacterId::from_bytes(character).ok()?;
                let edit = if matches!(intent, GuildGuestIntent::Invite { .. }) {
                    GuildGuestEdit::Invite(target)
                } else {
                    GuildGuestEdit::Exclude(target)
                };
                let runtime = self.runtime.lock().await;
                let fence = qualified_fence(&admitted, &runtime)?;
                match self
                    .root
                    .edit_guild_chat_guest(self.character, self.holder, fence, edit)
                    .await
                {
                    Ok(true) => result(ChatDisposition::Ok),
                    Ok(false) => result(ChatDisposition::NotAllowed),
                    Err(_) => result(ChatDisposition::ChatUnavailable),
                }
            }
        };
        if accepted.disposition == ChatDisposition::Rejected {
            return None;
        }
        chat::encode_chat_intent_result(&accepted).ok()
    }
}
