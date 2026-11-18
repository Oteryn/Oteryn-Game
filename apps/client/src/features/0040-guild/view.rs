use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn guild_panel(&self) -> oteryn_client_ui::guild::GuildPanel {
        lock(&self.mailbox).guild.clone()
    }
}

/// Guild controls for the shared frontend: the native list selection is a row number here.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    use crate::combat_input::PlayCommand;
    use oteryn_client_ui::guild::{Intent, panel_text, request};
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Guild"),
        labels: &[
            "Guild or character name",
            "Rank level",
            "Row number (members, then invitations)",
        ],
        buttons: &[
            "Refresh",
            "Invitations",
            "Roster",
            "More",
            "Join selected",
            "Leave",
            "Kick selected",
            "Revoke selected",
            "Found",
            "Invite",
            "Set rank",
            "Guild chat",
        ],
        snapshot: |mailbox| {
            let panel = &mailbox.guild;
            let mut text = String::new();
            let rows = panel
                .roster
                .iter()
                .map(|member| match member.channel {
                    Some(_) => oteryn_client_ui::social_channel::guild_member_text(
                        &panel.channel_names,
                        member,
                    ),
                    None => member.name.clone(),
                })
                .chain(panel.invitations.iter().map(|invitation| invitation.name.clone()));
            for (index, name) in rows.enumerate() {
                text.push_str(&format!("{}. {name}\n", index + 1));
            }
            text.push_str(&panel_text(panel));
            (panel.open, panel.pending, vec![String::new(); 3], text)
        },
        set_open: |mailbox, value| mailbox.guild.open = value,
        submit: |mailbox, commands, action, values| {
            use crate::play::feature_0115_mail::view::form::field;
            if action == 11 {
                if !crate::play::feature_0036_chat_guild_room::input::open_locked(mailbox) {
                    mailbox.guild.feedback =
                        Some(tr!("Guild chat unavailable or waiting for the server").into());
                }
                return;
            }
            let row = field(&values, 2)
                .parse::<usize>()
                .ok()
                .and_then(|row| row.checked_sub(1));
            let guild = &mailbox.guild;
            let intent = match action {
                0 => Some(Intent::RefreshOwn),
                1 => Some(Intent::RefreshInvitations),
                2 => Some(Intent::RefreshRoster),
                3 => Some(Intent::MoreRoster),
                4 => row
                    .and_then(|row| row.checked_sub(guild.roster.len()))
                    .and_then(|row| guild.invitations.get(row))
                    .map(|invitation| Intent::Join(invitation.guild)),
                5 => Some(Intent::Leave),
                6 => row
                    .and_then(|row| guild.roster.get(row))
                    .map(|member| Intent::Kick(member.character)),
                7 => row
                    .and_then(|row| guild.roster.get(row))
                    .map(|member| Intent::RevokeInvite(member.character)),
                8 => Some(Intent::Found(field(&values, 0).to_owned())),
                9 => Some(Intent::Invite(field(&values, 0).to_owned())),
                10 => row
                    .and_then(|row| guild.roster.get(row))
                    .and_then(|member| field(&values, 1).parse().ok().map(|level| (member, level)))
                    .map(|(member, level)| Intent::SetRank {
                        character: member.character,
                        level,
                    }),
                _ => None,
            };
            let Some(intent) = intent else {
                mailbox.guild.feedback = Some(
                    if action == 10 {
                        tr!("Enter a member row and a rank level")
                    } else {
                        tr!("Enter a row number from the list")
                    }
                    .into(),
                );
                return;
            };
            if let Some(command) = request(&mut mailbox.guild, intent)
                && commands.send(PlayCommand::Guild(command)).is_err()
            {
                mailbox.guild.pending = false;
                mailbox.guild.feedback = Some(tr!("The session has ended").into());
            }
        },
    }
}
