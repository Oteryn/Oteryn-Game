use crate::play::{Outbox, mailbox::lock};
pub use oteryn_client_runtime::player_typing::{PlayerTypingCommand, PlayerTypingResponse};
impl Outbox {
    pub(in crate::play) fn player_typing_initialize(&self, selected: bool) {
        lock(&self.0).player_typing.initialize(selected);
    }
    pub(in crate::play) fn player_typing_result(
        &self,
        command: PlayerTypingCommand,
        response: PlayerTypingResponse,
    ) {
        oteryn_client_ui::player_typing::apply_result(
            &mut lock(&self.0).player_typing,
            command,
            response,
        );
    }
}
