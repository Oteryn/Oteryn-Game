use super::super::ComposedFreshAdmission;
use super::*;
use crate::domain::guild::GuildId;
use oteryn_protocol_oteryn::chat::{CAPABILITY_CHAT_SYSTEM_MESSAGE_V1, ChatLine};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in super::super) async fn guild_chat_next(
        &self,
        admitted: AdmittedSession,
        remaining: &mut usize,
    ) -> Option<ChatLine> {
        while *remaining > 0 {
            let candidate = self.states.chat.peek_line(admitted.game_session_id)?;
            if let ChatLine::Room { .. } = &candidate {
                *remaining -= 1;
                if let Some(line) = self
                    .states
                    .chat
                    .accept_public_room_line(&admitted, &candidate)
                {
                    return Some(line);
                }
                continue;
            }
            if let ChatLine::PrivateChannel(_) = &candidate {
                let destination = self
                    .states
                    .chat
                    .private_channel_destination(admitted.game_session_id)?;
                *remaining -= 1;
                if let Some(line) = self
                    .private_channel_accept(admitted, &candidate, destination)
                    .await
                {
                    return Some(line);
                }
                continue;
            }
            if let ChatLine::Party { .. } = &candidate {
                let destination = self
                    .states
                    .chat
                    .party_destination(admitted.game_session_id)?;
                *remaining -= 1;
                if let Some(line) = self
                    .party_chat_accept(admitted, &candidate, destination)
                    .await
                {
                    return Some(line);
                }
                continue;
            }
            let ChatLine::Guild(line) = &candidate else {
                return self.states.chat.next_line(
                    admitted.game_session_id,
                    admitted
                        .continuity
                        .selected_capabilities
                        .contains(CAPABILITY_CHAT_SYSTEM_MESSAGE_V1),
                );
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

    pub(in super::super) async fn guild_war_chat_notice(
        &self,
        admitted: AdmittedSession,
    ) -> Option<(ChatLine, i64)> {
        if !self.states.chat.guild.poll_war_notices(&admitted) {
            return None;
        }
        let fence = {
            let runtime = self.runtime.lock().await;
            qualified_fence(&admitted, &runtime)?
        };
        let notice = self
            .root
            .guild_war_notice(self.character, self.holder, fence, None)
            .await
            .ok()??;
        let line = ChatLine::Guild(oteryn_protocol_oteryn::chat::guild_room::GuildChatLine {
            guild_id: notice.guild,
            rank: 1,
            speaker_name: "[Guild War]".into(),
            text: notice.message,
        });
        Some((line, notice.id))
    }

    pub(in super::super) async fn ack_guild_war_chat_notice(
        &self,
        admitted: AdmittedSession,
        notice: i64,
    ) {
        let fence = {
            let runtime = self.runtime.lock().await;
            qualified_fence(&admitted, &runtime)
        };
        if let Some(fence) = fence {
            // If writing the cursor fails, the next connection can replay the notice.
            let _ = self
                .root
                .guild_war_notice(self.character, self.holder, fence, Some(notice))
                .await;
        }
    }
}
