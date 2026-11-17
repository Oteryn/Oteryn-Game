use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(in crate::play) fn guild_initialize(&self, selected: bool) {
        lock(&self.0).guild.selected = selected;
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
