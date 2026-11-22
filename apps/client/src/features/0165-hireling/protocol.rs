use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::hireling::{HirelingCommand, HirelingResponse, hireling::Request};

impl Outbox {
    pub(in crate::play) fn hireling_initialize(&self, selected: bool) {
        lock(&self.0).hireling.selected = selected;
    }

    pub(in crate::play) fn wire_hireling(&self, request: HirelingCommand) -> HirelingCommand {
        if matches!(request.0, Request::Summon { .. })
            && let Some(own) = lock(&self.0).wire.own_native()
            && let (Ok(x), Ok(y), Ok(floor)) = (
                u16::try_from(own.x),
                u16::try_from(own.y),
                i8::try_from(own.floor.unsigned_abs()),
            )
        {
            // Command 220 uses the runtime floor (0–15) and requires the actor's own tile.
            return HirelingCommand(Request::Summon { x, y, floor });
        }
        request
    }

    pub(in crate::play) fn hireling_result(&self, response: &HirelingResponse) {
        oteryn_client_ui::hireling::apply_result(&mut lock(&self.0).hireling, response);
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    /// Admits the banker hireling (165) without its management window (250).
    pub(crate) fn bevy_hireling_selected(&self) {
        self.hireling_initialize(true);
    }
}
