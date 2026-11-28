use super::super::ComposedFreshAdmission;
use super::*;
use crate::domain::guild::GuildId;
use oteryn_protocol_oteryn::chat::{
    CAPABILITY_CHAT_SYSTEM_MESSAGE_V1, ChatLine,
    guild_guest::{CAPABILITY_CHAT_GUILD_GUEST_V1, GUILD_GUEST_RANK},
};

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
            let sent_at = self
                .states
                .chat
                .guild_sent_at_micros(admitted.game_session_id, &candidate);
            let guest_capable = admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_CHAT_GUILD_GUEST_V1);
            if line.rank == GUILD_GUEST_RANK && !guest_capable {
                self.states
                    .chat
                    .discard_guild_line(&admitted, &candidate, sent_at);
                continue;
            }
            let accepted = if let (true, Some(sent_at)) = (guest_capable, sent_at) {
                self.root
                    .with_guild_guest_custody(
                        self.character,
                        self.holder,
                        fence,
                        guild,
                        Some(sent_at),
                        |membership, guest, _| {
                            self.states.chat.accept_guild_line(
                                &admitted,
                                &candidate,
                                Some(sent_at),
                                membership.is_some() || guest,
                            )
                        },
                    )
                    .await
            } else {
                self.root
                    .with_guild_chat_custody(
                        self.character,
                        self.holder,
                        fence,
                        Some(guild),
                        |membership, _| {
                            self.states.chat.accept_guild_line(
                                &admitted,
                                &candidate,
                                sent_at,
                                membership.is_some(),
                            )
                        },
                    )
                    .await
            };
            match accepted {
                Ok(Some(line)) => return Some(line),
                Ok(None) => self.states.chat.guild.dropped(),
                Err(_) => {
                    self.states
                        .chat
                        .discard_guild_line(&admitted, &candidate, sent_at);
                    self.states.chat.guild.dropped();
                }
            }
        }
        None
    }
}
