use crate::play::{
    Outbox, Stepper,
    mailbox::{Mailbox, lock},
};
impl Outbox {
    pub(in crate::play) fn guild_chat_publish(&self, session: &impl Stepper) {
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::chat_guild_room::replace_state(
            &mut mailbox.chat,
            session.feature_selected(36),
            session.guild_chat_state(),
        );
        refresh_membership(&mut mailbox);
    }
}

/// Guild Own responses take precedence over the join-time chat snapshot when
/// guild controls are negotiated. Their refresh after a guild action also
/// handles joining, leaving and self-removal without a new chat snapshot.
pub(in crate::play) fn refresh_membership(mailbox: &mut Mailbox) {
    if mailbox.guild.selected {
        mailbox.chat.guild =
            mailbox
                .guild
                .own
                .as_ref()
                .map_or(oteryn_session::GuildRoomState::Absent, |own| {
                    oteryn_session::GuildRoomState::Member {
                        guild_id: own.guild,
                        rank: own.own_rank,
                        motd: own.message.clone(),
                    }
                });
    }
}
