use crate::play::{Outbox, Stepper, mailbox::lock};
use oteryn_session::{ActorPosition, SessionError};

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
