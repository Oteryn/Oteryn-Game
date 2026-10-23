use super::super::ComposedFreshAdmission;
use super::*;
use crate::domain::guild::GuildId;
use oteryn_protocol_oteryn::chat::ChatLine;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in super::super) async fn guild_chat_next(
        &self,
        admitted: AdmittedSession,
        remaining: &mut usize,
    ) -> Option<ChatLine> {
        while *remaining > 0 {
            let candidate = self.states.chat.peek_line(admitted.game_session_id)?;
            let ChatLine::Guild(line) = &candidate else {
                return self.states.chat.next(admitted.game_session_id);
            };
            if !self.states.chat.guild.matches(&admitted) {
                return None;
            }
            let guild = GuildId::from_bytes(line.guild_id).ok()?;
            let runtime = self.runtime.lock().await;
            let fence = qualified_fence(&admitted, &runtime)?;
            *remaining -= 1;
            let accepted = self
                .root
                .with_guild_chat_custody(
                    self.character,
                    self.holder,
                    fence,
                    Some(guild),
                    |membership, _| {
                        self.states.chat.accept_guild_line(
                            &admitted,
                            &candidate,
                            membership.is_some(),
                        )
                    },
                )
                .await;
            match accepted {
                Ok(Some(line)) => return Some(line),
                Ok(None) => self.states.chat.guild.dropped(),
                Err(_) => {
                    self.states.chat.discard_guild_line(&admitted, &candidate);
                    self.states.chat.guild.dropped();
                }
            }
        }
        None
    }
}
