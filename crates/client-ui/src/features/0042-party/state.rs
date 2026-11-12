use super::{EntityRef, PartyCommand, party};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Invite(EntityRef),
    Join,
    // PARTY_V1 has no decline intent; incoming invitations expire on the server.
    Leave,
    PassLeadership,
    Kick,
    ToggleSharedExperience,
    Refresh,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartyPanel {
    pub list_title: String,
    pub list_hidden: [bool; 7],
    pub list_sort: ListSort,
    pub list_reverse: bool,
    pub list_order: Vec<EntityRef>,
    pub selected: bool,
    pub own_character_name: String,
    pub open: bool,
    pub chat_selected: bool,
    pub state: Option<party::PartyResult>,
    pub received_at: Option<std::time::Instant>,
    pub selected_member: Option<u64>,
    pub selected_invitation: Option<u64>,
    pub pending: Option<PartyCommand>,
    pub awaiting_shared_feedback: bool,
    pub server_feedback: Option<String>,
    pub feedback: Option<&'static str>,
}

impl PartyPanel {
    #[must_use]
    pub fn is_leader(&self) -> bool {
        !self.own_character_name.is_empty()
            && self.state.as_ref().is_some_and(|state| {
                state.members.iter().any(|member| {
                    member.leader && member.name.eq_ignore_ascii_case(&self.own_character_name)
                })
            })
    }

    #[must_use]
    pub fn can_manage_member(&self) -> bool {
        self.is_leader()
            && self.pending.is_none()
            && self.selected_member.is_some_and(|handle| {
                self.state.as_ref().is_some_and(|state| {
                    state
                        .members
                        .iter()
                        .any(|member| member.handle == handle && !member.leader)
                })
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListSort {
    #[default]
    Time,
    Distance,
    Health,
    Name,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListAction {
    Rename(String),
    /// 0 players, 1..5 vocations, 6 summons.
    ToggleGroup(usize),
    Sort(ListSort),
}
impl PartyPanel {
    pub fn list_action(&mut self, action: ListAction) {
        match action {
            ListAction::Rename(title) => self.list_title = title.trim().chars().take(64).collect(),
            ListAction::ToggleGroup(group) => {
                if let Some(hidden) = self.list_hidden.get_mut(group) {
                    *hidden = !*hidden;
                }
            }
            ListAction::Sort(sort) => {
                self.list_reverse = self.list_sort == sort && !self.list_reverse;
                self.list_sort = sort;
            }
        }
    }
    /// Refreshes retain display-time order for existing entities and forget departed ones.
    pub(super) fn reconcile_list(&mut self) {
        let Some(state) = &self.state else {
            self.list_order.clear();
            return;
        };
        self.list_order.retain(|entity| {
            state
                .members
                .iter()
                .flat_map(|m| &m.list)
                .any(|row| row.entity == *entity)
        });
        for row in state.members.iter().flat_map(|m| &m.list) {
            if !self.list_order.contains(&row.entity) {
                self.list_order.push(row.entity);
            }
        }
    }
    #[must_use]
    pub fn list_rows(&self) -> Vec<(u64, &party::PartyListEntity)> {
        let mut rows: Vec<_> = self
            .state
            .iter()
            .flat_map(|s| &s.members)
            .flat_map(|m| m.list.iter().map(move |row| (m.handle, row)))
            .filter(|(_, row)| {
                if row.summon {
                    !self.list_hidden[6]
                } else {
                    !self.list_hidden[0] && !self.list_hidden[usize::from(row.vocation)]
                }
            })
            .collect();
        rows.sort_by(|a, b| {
            let key = match self.list_sort {
                ListSort::Time => self
                    .list_order
                    .iter()
                    .position(|e| *e == a.1.entity)
                    .cmp(&self.list_order.iter().position(|e| *e == b.1.entity)),
                ListSort::Distance => a.1.distance.cmp(&b.1.distance),
                ListSort::Health => {
                    a.1.health_percent
                        .unwrap_or(101)
                        .cmp(&b.1.health_percent.unwrap_or(101))
                }
                ListSort::Name => a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()),
            }
            .then_with(|| a.1.entity.cmp(&b.1.entity));
            if self.list_reverse {
                key.reverse()
            } else {
                key
            }
        });
        rows
    }
}
