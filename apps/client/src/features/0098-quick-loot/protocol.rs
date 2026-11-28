use crate::combat_input::PlayCommand;
use oteryn_session::quick_loot::CAPABILITY_QUICK_LOOT_DESTINATIONS_V1;
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
            mailbox.store.feedback = Some(tr!("Session ended").into());
            return false;
        }
        let state = &mut mailbox.discovery;
        match capability {
            98 | CAPABILITY_QUICK_LOOT_DESTINATIONS_V1 => {
                state.loot.pending = None;
                state.loot.feedback = tr!("Session ended").into();
            }
            132 => {
                state.inspection.pending = false;
                state.inspection.feedback = tr!("Session ended").into();
            }
            200 => {
                state.leader.pending = false;
                state.leader.submitted = None;
                state.leader.feedback = tr!("Session ended").into();
            }
            232 => {
                state.teams.pending = None;
                state.teams.feedback = tr!("Session ended").into();
            }
            285 => {
                state.history.pending = false;
                state.history.feedback = tr!("Session ended").into();
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
        state.loot.destinations_selected =
            session.server_mechanic_selected(CAPABILITY_QUICK_LOOT_DESTINATIONS_V1);
        state.inspection.invalidate_catalogue();
        state.inspection.selected = session.server_mechanic_selected(132);
        state.leader.selected = session.server_mechanic_selected(200);
        state.teams.selected = session.server_mechanic_selected(232);
        state.history.selected =
            session.server_mechanic_selected(154) && session.server_mechanic_selected(285);
    }
    pub(in crate::play) async fn restore_quick_loot(
        &self,
        session: &mut impl crate::play::Stepper,
    ) -> Result<(), oteryn_session::SessionError> {
        use oteryn_client_ui::quick_loot::apply_destinations;
        use oteryn_session::quick_loot::*;
        if !session.server_mechanic_selected(CAPABILITY_QUICK_LOOT_DESTINATIONS_V1) {
            return Ok(());
        }
        let Ok(payload) = encode_quick_loot_request(&quick_loot_filter_review_request()) else {
            return Ok(());
        };
        let response = session
            .server_mechanic(&ServerMechanicCommand {
                capability: CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
                command_type: COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
                payload,
            })
            .await?;
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.discovery.loot;
        if let Err(message) = apply_destinations(panel, &response)
            && panel.pending.is_none()
        {
            panel.feedback = message.into();
        }
        Ok(())
    }

    pub(in crate::play) fn discovery_result(
        &self,
        capability: u32,
        response: &ServerMechanicResponse,
    ) {
        let mut mailbox = lock(&self.0);
        match capability {
            98 | CAPABILITY_QUICK_LOOT_DESTINATIONS_V1 => {
                if oteryn_client_ui::quick_loot::apply_result(&mut mailbox.discovery.loot, response)
                    && mailbox.local_preferences.game_window.loot_messages
                    && let Some(result) = &mailbox.discovery.loot.result
                {
                    let received = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let text = oteryn_client_ui::quick_loot::result_notice(result);
                    mailbox.chat.console.notices.push(
                        oteryn_client_ui::chat::NoticeClass::Loot,
                        &text,
                        received,
                    );
                }
            }
            132 => {
                let panel = &mut mailbox.discovery.inspection;
                if let Some(allow) = panel.permission_pending.take() {
                    use oteryn_session::CommandStatus;
                    panel.pending = false;
                    if matches!(response.status, CommandStatus::Accepted | CommandStatus::DuplicateReplay)
                        && oteryn_client_ui::inspection::decode_reply(&response.payload).is_ok_and(|reply| reply.status == 8)
                    {
                        panel.allow_inspection = allow;
                        panel.permission_retry_at = None;
                    } else {
                        panel.feedback = tr!("Permission denied").into();
                        panel.permission_retry_at = Some((allow, std::time::Instant::now() + std::time::Duration::from_secs(5)));
                    }
                    return;
                }
                crate::play::feature_0068_imbue::protocol::equipment_inspection_result(
                    &mut mailbox,
                    response,
                );
                oteryn_client_ui::inspection::apply_result(
                    &mut mailbox.discovery.inspection,
                    response,
                );
            }
            200 => oteryn_client_ui::team_finder_leader::protocol::apply_result(
                &mut mailbox.discovery.leader,
                response,
            ),
            232 => {
                // A fresh applicants page is the server's current listing for this leader.
                if oteryn_client_ui::team_finder::apply_result(
                    &mut mailbox.discovery.teams,
                    response,
                ) && let Some(oteryn_client_ui::team_finder::TeamFinderResult::Applicants {
                    listing,
                    ..
                }) = mailbox.discovery.teams.result
                {
                    mailbox.discovery.leader.confirmed = listing;
                }
            }
            285 => oteryn_client_ui::purchase_history::protocol::apply_result(
                &mut mailbox.discovery.history,
                response,
            ),
            _ => {}
        }
    }
}
