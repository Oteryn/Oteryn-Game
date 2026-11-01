pub use oteryn_client_runtime::guild::{GuildCommand, GuildReply, guild};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    RefreshOwn,
    RefreshInvitations,
    RefreshRoster,
    MoreRoster,
    Found(String),
    Invite(String),
    Join([u8; 16]),
    Leave,
    Kick([u8; 16]),
    SetRank { character: [u8; 16], level: u8 },
    RevokeInvite([u8; 16]),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuildPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub feedback: Option<String>,
    pub own: Option<guild::GuildSummary>,
    pub invitations: Vec<guild::GuildInvitation>,
    pub roster: Vec<guild::GuildMember>,
    pub next: Option<guild::GuildRosterCursor>,
}

pub fn request(panel: &mut GuildPanel, intent: Intent) -> Option<GuildCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let command = match intent {
        Intent::RefreshOwn => GuildCommand::Query(guild::GuildQuery::Own),
        Intent::RefreshInvitations => GuildCommand::Query(guild::GuildQuery::Invitations),
        Intent::RefreshRoster => GuildCommand::Query(guild::GuildQuery::Roster {
            guild: panel.own.as_ref()?.guild,
            after: None,
        }),
        Intent::MoreRoster => GuildCommand::Query(guild::GuildQuery::Roster {
            guild: panel.own.as_ref()?.guild,
            after: panel.next,
        }),
        Intent::Found(name) => GuildCommand::Intent(guild::GuildIntent::Found { name }),
        Intent::Invite(name) => GuildCommand::Intent(guild::GuildIntent::Invite {
            guild: panel.own.as_ref()?.guild,
            name,
        }),
        Intent::Join(guild) => GuildCommand::Intent(guild::GuildIntent::Join { guild }),
        Intent::Leave => GuildCommand::Intent(guild::GuildIntent::Leave {
            guild: panel.own.as_ref()?.guild,
        }),
        Intent::Kick(character) => GuildCommand::Intent(guild::GuildIntent::Kick {
            guild: panel.own.as_ref()?.guild,
            character,
        }),
        Intent::SetRank { character, level } => {
            let own = panel.own.as_ref()?;
            GuildCommand::Intent(guild::GuildIntent::SetRank {
                guild: own.guild,
                character,
                level,
                expected_revision: own.revision,
            })
        }
        Intent::RevokeInvite(character) => GuildCommand::Intent(guild::GuildIntent::RevokeInvite {
            guild: panel.own.as_ref()?.guild,
            character,
        }),
    };
    panel.pending = true;
    panel.feedback = None;
    Some(command)
}

pub fn apply_result(panel: &mut GuildPanel, reply: &GuildReply) {
    panel.pending = false;
    match reply {
        GuildReply::Intent {
            result: Some(result),
            ..
        } if result.code == 1 => panel.feedback = Some("Guild action accepted".into()),
        GuildReply::Intent {
            result: Some(result),
            ..
        } => panel.feedback = Some(format!("Guild action refused (code {})", result.code)),
        GuildReply::Intent { result: None, .. } | GuildReply::Query { result: None, .. } => {
            panel.feedback = Some("Guild request refused by the server".into())
        }
        GuildReply::Query {
            result: Some(guild::GuildQueryResult::Own(value)),
            ..
        } => {
            panel.own.clone_from(value);
            panel.feedback = None;
        }
        GuildReply::Query {
            result: Some(guild::GuildQueryResult::Invitations(value)),
            ..
        } => {
            panel.invitations.clone_from(value);
            panel.feedback = None;
        }
        GuildReply::Query {
            result: Some(guild::GuildQueryResult::Roster { members, next, .. }),
            ..
        } => {
            panel.roster.clone_from(members);
            panel.next = *next;
            panel.feedback = None;
        }
    }
}

#[must_use]
pub fn panel_text(panel: &GuildPanel) -> String {
    if !panel.selected {
        return "Guild unavailable in this session".into();
    }
    let mut lines = vec![panel.own.as_ref().map_or_else(
        || "No guild".into(),
        |g| {
            format!(
                "{} — {}\r\n{}\r\nRank: {}",
                g.name,
                g.title,
                g.message,
                g.ranks
                    .get(usize::from(g.own_rank))
                    .map_or("unknown", String::as_str)
            )
        },
    )];
    lines.push(format!("Roster ({}):", panel.roster.len()));
    lines.extend(
        panel
            .roster
            .iter()
            .map(|m| format!("{} — {} (rank {})", m.name, m.title, m.level)),
    );
    lines.push(format!("Invitations ({}):", panel.invitations.len()));
    lines.extend(panel.invitations.iter().map(|i| i.name.clone()));
    if let Some(feedback) = &panel.feedback {
        lines.push(feedback.clone());
    }
    lines.join("\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requests_and_refusals_are_visible() {
        let mut panel = GuildPanel {
            selected: true,
            ..Default::default()
        };
        assert!(matches!(
            request(&mut panel, Intent::Found("Knights".into())),
            Some(GuildCommand::Intent(guild::GuildIntent::Found { .. }))
        ));
        apply_result(
            &mut panel,
            &GuildReply::Intent {
                status: oteryn_client_runtime::guild::CommandStatus::Accepted,
                result: Some(guild::GuildIntentResult {
                    code: 5,
                    guild: [1; 16],
                    revision: None,
                }),
            },
        );
        assert!(panel_text(&panel).contains("refused (code 5)"));
    }
}
