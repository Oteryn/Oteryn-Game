//! Movement acknowledgements never predict a position; spatial deltas remain authoritative.
use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
use oteryn_session::{ActorPosition, SessionError};
impl Outbox {
    pub(in crate::play) fn target_move_initialize(&self, selected: bool) {
        lock(&self.0).target_move_selected = selected;
    }
    pub(in crate::play) fn target_move_result(
        &self,
        request: &oteryn_client_runtime::server_mechanic::ServerMechanicCommand,
        response: &ServerMechanicResponse,
    ) {
        let mut mailbox = lock(&self.0);
        if matches!(
            response.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) && response.payload == [8, 1]
            && let Ok(oteryn_session::target_move::TargetMoveIntent::Turn(direction)) =
                oteryn_session::target_move::decode_target_move_intent(&request.payload)
        {
            mailbox.movement_turn = Some(match direction {
                oteryn_session::StepDirection::North => crate::input::StepDir::North,
                oteryn_session::StepDirection::East => crate::input::StepDir::East,
                oteryn_session::StepDirection::South => crate::input::StepDir::South,
                oteryn_session::StepDirection::West => crate::input::StepDir::West,
                oteryn_session::StepDirection::NorthEast => crate::input::StepDir::NorthEast,
                oteryn_session::StepDirection::SouthEast => crate::input::StepDir::SouthEast,
                oteryn_session::StepDirection::SouthWest => crate::input::StepDir::SouthWest,
                oteryn_session::StepDirection::NorthWest => crate::input::StepDir::NorthWest,

            });
        }
        mailbox.movement_feedback = Some(match (response.status, response.payload.as_slice()) {
            (CommandStatus::Accepted | CommandStatus::DuplicateReplay, [8, 1]) => {
                tr!("Movement accepted")
            }
            (CommandStatus::Accepted | CommandStatus::DuplicateReplay, [8, 2]) => tr!("No route"),
            (_, [8, 3]) | (CommandStatus::Rejected, _) => tr!("Movement refused"),
            _ => tr!("Invalid movement result"),
        });
    }
}
impl crate::play::PlayLink {
    #[must_use]
    pub fn movement_feedback(&self) -> Option<&'static str> {
        lock(&self.mailbox).movement_feedback
    }
}

impl crate::play::PlayView {
    pub(in crate::play) fn apply_target_turn(
        &mut self,
        link: &crate::play::PlayLink,
    ) -> Result<(), oteryn_renderer::BatchError> {
        if let Some(facing) = lock(&link.mailbox).movement_turn.take() {
            self.facing = facing;
            self.scene.set_facing(facing)?;
        }
        Ok(())
    }
}
impl Outbox {
    pub(in crate::play) fn target_walk_initialize(&self, selected: bool) {
        lock(&self.0).target_walk = oteryn_client_ui::target_move::WalkState {
            selected,
            ..Default::default()
        };
    }

    pub(in crate::play) async fn target_walk_run(
        &self,
        session: &mut impl Stepper,
        position: ActorPosition,
    ) -> Result<(), SessionError> {
        let response = session.target_walk(position).await?;
        oteryn_client_ui::target_move::apply(&mut lock(&self.0).target_walk, response);
        Ok(())
    }
}
