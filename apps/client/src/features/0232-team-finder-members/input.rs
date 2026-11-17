use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn team_finder_browse(&self, applicants: bool, next: bool) -> bool {
        let command = oteryn_client_ui::team_finder::query(
            &mut lock(&self.mailbox).discovery.teams,
            applicants,
            next,
        );
        command.is_some_and(|command| self.discovery_send(command))
    }
    pub fn team_finder_select(&self, row: usize) {
        lock(&self.mailbox).discovery.teams.selected_row = row;
    }
    pub fn team_finder_member(&self, cancel: bool) -> bool {
        let command =
            oteryn_client_ui::team_finder::member(&mut lock(&self.mailbox).discovery.teams, cancel);
        command.is_some_and(|command| self.discovery_send(command))
    }
    pub fn team_finder_decide(&self, action: u8) -> bool {
        let command =
            oteryn_client_ui::team_finder::leader(&mut lock(&self.mailbox).discovery.teams, action);
        command.is_some_and(|command| self.discovery_send(command))
    }
}

impl PlayLink {
    pub fn team_finder_control(&self, action: usize, fields: &[String], row: usize) -> bool {
        self.team_finder_select(row);
        match action {
            1 => self.team_finder_browse(false, false),
            2 => self.team_finder_browse(true, false),
            3 => self.team_finder_browse(self.discovery_panel().teams.applicants, true),
            4 | 5 => self.team_finder_member(action == 5),
            6..=8 => self.team_finder_decide((action - 4) as u8),
            9 => self.team_finder_publish_fields(fields),
            10 => self.team_finder_advertise(None),
            11 => {
                let parse = |index: usize| fields.get(index)?.trim().parse::<u16>().ok();
                let Some((level, mask, target, open)) = parse(11).zip(parse(12)).zip(parse(13)).zip(parse(14)).map(|(((l,m),t),o)| (l,m,t,o)) else { return false; };
                if mask > 255 || target > 3 || open > 1 { return false; }
                let mut mailbox = lock(&self.mailbox);
                mailbox.discovery.teams.filter = oteryn_client_ui::team_finder::TeamFilter {
                    name: fields.get(10).cloned().unwrap_or_default(), level: (level != 0).then_some(level), vocation_mask: mask as u8, target: target as u8, only_open: open == 1
                };
                mailbox.discovery.teams.selected_row = 0;
                true
            },
            12 => {
                let mut mailbox = lock(&self.mailbox);
                mailbox.discovery.teams.filter = Default::default();
                mailbox.discovery.teams.selected_row = 0;
                true
            },
            _ => false,
        }
    }
    fn team_finder_publish_fields(&self, fields: &[String]) -> bool {
        use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget};
        let parse = |index: usize| fields.get(index)?.parse::<u16>().ok();
        let build = || {
            Some(LeaderListing {
                min_level: parse(0)?,
                max_level: parse(1)?,
                vocation_mask: u8::try_from(parse(2)?).ok()?,
                slots: parse(3)?,
                free_slots: parse(4)?,
                party_members_join: match parse(5)? {
                    0 => false,
                    1 => true,
                    _ => return None,
                },
                timestamp: fields.get(9).map_or(Some(0), |value| value.parse().ok())?,
                target: match parse(6)? {
                    1 => TeamTarget::Boss(parse(7)?),
                    2 => TeamTarget::Hunt {
                        kind: parse(7)?,
                        area: parse(8)?,
                    },
                    3 => TeamTarget::Quest(parse(7)?),
                    _ => return None,
                },
            })
        };
        build().is_some_and(|listing| self.team_finder_advertise(Some(listing)))
    }
}
