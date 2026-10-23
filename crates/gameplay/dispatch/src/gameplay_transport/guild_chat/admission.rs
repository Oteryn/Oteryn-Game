use super::super::ComposedFreshAdmission;
use super::*;
use oteryn_protocol_oteryn::chat::guild_room::GuildRoomState;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in super::super) async fn guild_chat_admit(
        &self,
        admitted: AdmittedSession,
    ) -> GuildRoomState {
        let runtime = self.runtime.lock().await;
        let Some(fence) = qualified_fence(&admitted, &runtime) else {
            return GuildRoomState::Unavailable;
        };
        self.root
            .with_guild_chat_custody(self.character, self.holder, fence, None, |membership, _| {
                self.chat.guild.bind(admitted);
                membership.map_or(GuildRoomState::Absent, |member| GuildRoomState::Member {
                    guild_id: *member.guild_id.as_bytes(),
                    rank: member.rank.level(),
                    motd: member.motd,
                })
            })
            .await
            .unwrap_or(GuildRoomState::Unavailable)
    }
}
