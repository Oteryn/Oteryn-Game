use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::hireling::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;

impl PlayLink {
    pub fn hireling_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::hireling::open_key(events) {
            return false;
        }
        if lock(&self.mailbox).hireling_management.selected {
            self.hireling_management_open();
            return true;
        }
        self.hireling_request(Intent::Refresh);
        let mut mailbox = lock(&self.mailbox);
        mailbox.hireling.open = true;
        mailbox.hireling_picking = false;
        true
    }

    /// Leaves an armed summon-location pick; the banker form shows again.
    pub(in crate::play) fn hireling_pick_cancel(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !std::mem::take(&mut mailbox.hireling_picking) {
            return false;
        }
        mailbox.hireling.feedback = None;
        true
    }

    pub fn hireling_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.hireling, intent)
            && self.commands.send(PlayCommand::Hireling(command)).is_err()
        {
            mailbox.hireling.pending = None;
            mailbox.hireling.feedback = Some(tr!("The session has ended"));
        }
    }

    pub fn hireling_target(&self, x: u16, y: u16, floor: i8) {
        lock(&self.mailbox).hireling.target = Some((x, y, floor));
    }
}

impl crate::play::PlayView {
    /// While the banker form waits for a summon location, a primary click on a tile next
    /// to the player becomes the summon target; any other tile keeps the form waiting.
    pub(in crate::play) fn hireling_pick(
        &self,
        tile: oteryn_renderer::TileCoord,
        link: &PlayLink,
    ) -> bool {
        let target = {
            let mut mailbox = lock(&link.mailbox);
            if !mailbox.hireling_picking {
                return false;
            }
            let adjacent = tile != self.own
                && (tile.x - self.own.x).abs() <= 1
                && (tile.y - self.own.y).abs() <= 1;
            match (
                adjacent,
                u16::try_from(tile.x),
                u16::try_from(tile.y),
                i8::try_from(self.floor),
            ) {
                (true, Ok(x), Ok(y), Ok(floor)) => {
                    mailbox.hireling_picking = false;
                    mailbox.hireling.feedback = None;
                    Some((x, y, floor))
                }
                _ => {
                    mailbox.hireling.feedback = Some(tr!("Select an adjacent summon location"));
                    None
                }
            }
        };
        if let Some((x, y, floor)) = target {
            link.hireling_target(x, y, floor);
        }
        true
    }
}
