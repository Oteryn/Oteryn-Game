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
