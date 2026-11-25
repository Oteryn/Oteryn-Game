use crate::play::{
    Outbox, Stepper,
    mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::guild::{GuildCommand, GuildReply, guild};
use oteryn_client_ui::chat_guild_room::{replace_state, show_motd_notice};
use oteryn_session::{GuildRoomState, SessionError};
impl Outbox {
    pub(in crate::play) async fn guild_motd_run(
        &self,
        session: &mut impl Stepper,
        command: &GuildCommand,
    ) -> Result<(), SessionError> {
        let reply = session.guild(command).await?;
        let code = match &reply {
            GuildReply::Intent {
                result: Some(result),
                ..
            } => Some(result.code),
            _ => None,
        };
        let query = if matches!(code, Some(1 | 13)) {
            session.guild(&GuildCommand::ChatOwn).await?
        } else {
            reply
        };
        let mut mailbox = lock(&self.0);
        let read = if let GuildReply::Query {
            result: Some(guild::GuildQueryResult::Own(own)),
            ..
        } = &query
        {
            let pending = mailbox.guild.pending;
            let changed_guild = matches!(mailbox.chat.guild, GuildRoomState::Member { guild_id, .. }
                if own.as_ref().is_some_and(|summary| summary.guild != guild_id));
            oteryn_client_ui::guild::apply_result(&mut mailbox.guild, &query);
            mailbox.guild.pending = pending;
            if own.is_none() || changed_guild {
                let selected = mailbox.chat.guild_selected;
                replace_state(&mut mailbox.chat, selected, GuildRoomState::Absent);
            }
            refresh_membership(&mut mailbox);
            show_motd_notice(&mut mailbox.chat);
            true
        } else {
            false
        };
        if matches!(command, GuildCommand::SetMessage(_)) {
            mailbox.chat.pending = false;
            if code == Some(1) {
                mailbox.chat.text.clear();
            }
            mailbox.chat.feedback = match code {
                Some(1) if read => None,
                Some(13) if read => {
                    Some(tr!("The Guild changed. Send !setmotd again to retry").into())
                }
                Some(1 | 13) => {
                    Some(tr!("The current view is unavailable; refresh before editing").into())
                }
                _ => Some(tr!("Guild request refused by the server").into()),
            };
        } else if !read {
            mailbox.chat.feedback = Some(tr!("Guild request refused by the server").into());
        }
        Ok(())
    }

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
