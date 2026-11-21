use crate::play::{
    Outbox, Stepper,
    mailbox::{Mailbox, lock},
};
impl Outbox {
    pub(in crate::play) fn guild_chat_publish(&self, session: &impl Stepper) {
        let mut mailbox = lock(&self.0);
        if mailbox.guild.selected && mailbox.chat.guild_selected {
            refresh_membership(&mut mailbox);
            return;
        }
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
    if mailbox.guild.selected && let Some(own) = &mailbox.guild.own {
        mailbox.chat.guild = oteryn_session::GuildRoomState::Member {
            guild_id: own.guild, rank: own.own_rank, motd: own.message.clone(),
        };
    }
}

#[cfg(test)]
mod membership_tests {
    use super::*;
    #[test]
    fn unknown_guild_own_preserves_login_membership() {
        let (link, _, _) = crate::play::play_channel();
        let mut mailbox = lock(&link.mailbox);
        mailbox.guild.selected = true;
        let login = oteryn_session::GuildRoomState::Member {
            guild_id: [1; 16], rank: 1, motd: "Welcome".into(),
        };
        mailbox.chat.guild = login.clone();
        refresh_membership(&mut mailbox);
        assert_eq!(mailbox.chat.guild, login);
    }
}
