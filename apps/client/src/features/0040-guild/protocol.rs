use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn guild_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.guild = oteryn_client_ui::guild::GuildPanel { selected, ..Default::default() };
        mailbox.chat.guild_selected = false;
        mailbox.chat.guild = oteryn_session::GuildRoomState::Unavailable;
    }

    /// Refresh membership views after opening or an accepted membership change.
    pub(in crate::play) async fn guild_run(
        &self,
        session: &mut impl crate::play::Stepper,
        command: &oteryn_client_runtime::guild::GuildCommand,
    ) -> Result<(), oteryn_session::SessionError> {
        use oteryn_client_runtime::guild::{GuildCommand, GuildReply, guild};
        let reply = session.guild(command).await?;
        let refresh =
            matches!(&reply, GuildReply::Intent { result: Some(result), .. } if result.code == 1);
        let own = matches!(command, GuildCommand::Query(guild::GuildQuery::Own));
        let mut replies = vec![reply];
        if refresh {
            replies.push(
                session
                    .guild(&GuildCommand::Query(guild::GuildQuery::Own))
                    .await?,
            );
        }
        if refresh || own {
            let guild_id = replies.iter().find_map(|reply| match reply {
                GuildReply::Query {
                    result: Some(guild::GuildQueryResult::Own(Some(summary))),
                    ..
                } => Some(summary.guild),
                _ => None,
            });
            replies.push(
                session
                    .guild(&GuildCommand::Query(guild::GuildQuery::Invitations))
                    .await?,
            );
            if let Some(guild) = guild_id {
                replies.push(
                    session
                        .guild(&GuildCommand::Query(guild::GuildQuery::Roster {
                            guild,
                            after: None,
                        }))
                        .await?,
                );
            }
        }
        let mut mailbox = lock(&self.0);
        let mut refusal = None;
        for reply in &replies {
            oteryn_client_ui::guild::apply_result(&mut mailbox.guild, reply);
            if matches!(reply, GuildReply::Query { result: Some(guild::GuildQueryResult::Own(None)), .. }) {
                let selected = mailbox.chat.guild_selected;
                oteryn_client_ui::chat_guild_room::replace_state(&mut mailbox.chat, selected, oteryn_session::GuildRoomState::Absent);
            }
            if matches!(reply, GuildReply::Query { result: None, .. }) {
                refusal.clone_from(&mailbox.guild.feedback);
            }
        }
        crate::play::feature_0036_chat_guild_room::protocol::refresh_membership(&mut mailbox);
        if refusal.is_some() {
            mailbox.guild.feedback = refusal;
        } else if refresh && mailbox.guild.feedback.is_none() {
            mailbox.guild.feedback = Some(tr!("Guild action accepted").into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;
    #[test]
    fn social_initializers_clear_inflight_session_state() {
        let (link, mut incoming, outbox) = crate::play::play_channel();
        outbox.guild_initialize(true);
        outbox.party_initialize(true, true);
        {
            let mut mailbox = lock(&link.mailbox);
            mailbox.guild.pending = true;
            mailbox.guild.open = true;
            mailbox.guild.own = Some(oteryn_client_ui::guild::guild::GuildSummary {
                guild: [1;16], name: "Old guild".into(), state: 1, revision: 3,
                own_rank: 1, title: String::new(), message: String::new(), ranks: vec!["Leader".into()],
            });
            mailbox.guild.next = Some(oteryn_client_ui::guild::guild::GuildRosterCursor { guild: [1;16], level: 1, character: [1;16] });
            mailbox.guild.roster.push(oteryn_client_ui::guild::guild::GuildMember {
                character: [1;16], name: "Old member".into(), level: 1, title: String::new(),
            });
            mailbox.guild.invitations.push(oteryn_client_ui::guild::guild::GuildInvitation {
                guild: [1;16], name: "Old guild".into(), expires_at_unix_ms: 10,
            });
            mailbox.party.pending = Some(oteryn_client_runtime::party::PartyCommand::query());
            mailbox.party.open = true;
            mailbox.party.state = Some(oteryn_client_ui::party::party::PartyResult::refused(oteryn_client_ui::party::party::PartyDisposition::Ok));
            mailbox.party.selected_member = Some(1);
            mailbox.party.selected_invitation = Some(2);
            mailbox.party.awaiting_shared_feedback = true;
        }
        outbox.guild_initialize(true);
        outbox.party_initialize(true, false);
        {
            let mailbox = lock(&link.mailbox);
            assert!(!mailbox.guild.pending && !mailbox.guild.open);
            assert!(mailbox.guild.roster.is_empty() && mailbox.guild.invitations.is_empty());
            assert!(mailbox.guild.own.is_none() && mailbox.guild.next.is_none());
            assert!(mailbox.party.pending.is_none() && !mailbox.party.open);
            assert!(mailbox.party.state.is_none() && mailbox.party.received_at.is_none());
            assert!(mailbox.party.selected_member.is_none() && mailbox.party.selected_invitation.is_none());
            assert!(!mailbox.party.awaiting_shared_feedback && !mailbox.party.chat_selected);
        }
        link.guild_request(oteryn_client_ui::guild::Intent::RefreshOwn);
        link.party_request(oteryn_client_ui::party::Intent::Refresh);
        assert!(incoming.try_recv().is_ok());
        assert!(incoming.try_recv().is_ok());
    }
}
