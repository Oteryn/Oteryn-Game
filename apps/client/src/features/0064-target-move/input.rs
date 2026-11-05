use crate::combat_input::PlayCommand;
use crate::play::{Input, PlayLink, PlayView, mailbox::lock};
use oteryn_renderer::TileCoord;
use oteryn_session::ActorPosition;

impl PlayView {
    pub(in crate::play) fn walk_to(&mut self, tile: TileCoord) {
        self.queue(Input::TargetWalk(ActorPosition {
            x: tile.x,
            y: tile.y,
            floor: self.floor,
        }));
    }
}

impl PlayLink {
    pub(in crate::play) fn request_target_walk(&self, position: ActorPosition) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.world_input_blocked() {
            return true;
        }
        if mailbox.target_walk.pending {
            return false;
        }
        if let Some(position) =
            oteryn_client_ui::target_move::request(&mut mailbox.target_walk, position)
            && self
                .commands
                .send(PlayCommand::TargetWalk(position))
                .is_err()
        {
            mailbox.target_walk.pending = false;
            mailbox.target_walk.feedback = Some("Session ended");
        }
        true
    }
}
