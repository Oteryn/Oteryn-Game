use crate::input::Targetable;
use crate::play::{Input, PlayView};
use oteryn_renderer::{BatchError, TileCoord};

impl PlayView {
    /// A click on `tile`: a drawn object or actor is selected, any other tile becomes the walk
    /// goal.
    pub fn click(&mut self, tile: TileCoord) -> Result<(), BatchError> {
        match self.scene.select_tile(tile)? {
            None => self.walk.set_goal(tile),
            Some(Targetable {
                entity: Some(target),
                ..
            }) => self.queue(Input::Attack(target)),
            Some(_) => {}
        }
        Ok(())
    }
}

pub(in crate::play) fn send_attack(
    link: &crate::play::PlayLink,
    target: oteryn_session::EntityRef,
) {
    link.request_combat(crate::combat_input::CombatCommand::Attack(target));
}
