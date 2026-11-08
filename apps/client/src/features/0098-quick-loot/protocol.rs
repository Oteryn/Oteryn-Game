use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::server_mechanic::{ServerMechanicCommand, ServerMechanicResponse};
impl PlayLink {
    pub(in crate::play) fn discovery_send(&self, command: ServerMechanicCommand) -> bool {
        let capability = command.capability;
        if self
            .commands
            .send(PlayCommand::ServerMechanic(command))
            .is_ok()
        {
            return true;
        }
        let mut mailbox = lock(&self.mailbox);
        if capability == 154 {
            mailbox.store.pending = false;
            mailbox.store.feedback = Some("Session ended".into());
            return false;
        }
        let state = &mut mailbox.discovery;
        match capability {
            98 => {
                state.loot.pending = None;
                state.loot.feedback = "Session ended".into();
            }
            132 => {
                state.inspection.pending = false;
                state.inspection.feedback = "Session ended".into();
            }
            200 => {
                state.leader.pending = false;
                state.leader.submitted = None;
                state.leader.feedback = "Session ended".into();
            }
            232 => {
                state.teams.pending = None;
                state.teams.feedback = "Session ended".into();
            }
            285 => {
                state.history.pending = false;
                state.history.feedback = "Session ended".into();
            }
            _ => {}
        }
        false
    }
}
impl Outbox {
    pub(in crate::play) fn discovery_initialize(&self, session: &impl crate::play::Stepper) {
        let mut mailbox = lock(&self.0);
        let state = &mut mailbox.discovery;
        *state = Default::default();
        state.loot.selected = session.server_mechanic_selected(98);
        state.inspection.invalidate_catalogue();
        state.inspection.selected = session.server_mechanic_selected(132);
        state.leader.selected = session.server_mechanic_selected(200);
        state.teams.selected = session.server_mechanic_selected(232);
        state.history.selected =
            session.server_mechanic_selected(154) && session.server_mechanic_selected(285);
    }
    pub(in crate::play) fn discovery_result(
        &self,
        capability: u32,
        response: &ServerMechanicResponse,
    ) {
        let mut mailbox = lock(&self.0);
        match capability {
            98 => oteryn_client_ui::quick_loot::apply_result(&mut mailbox.discovery.loot, response),
            132 => oteryn_client_ui::inspection::apply_result(
                &mut mailbox.discovery.inspection,
                response,
            ),
            200 => oteryn_client_ui::team_finder_leader::protocol::apply_result(
                &mut mailbox.discovery.leader,
                response,
            ),
            232 => {
                oteryn_client_ui::team_finder::apply_result(&mut mailbox.discovery.teams, response)
            }
            285 => oteryn_client_ui::purchase_history::protocol::apply_result(
                &mut mailbox.discovery.history,
                response,
            ),
            _ => {}
        }
    }
}
