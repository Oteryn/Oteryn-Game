use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_input_actions::NormalizedInputEvent;
use oteryn_protocol_oteryn::newhaven_vocation::NewhavenVocationRequest;

impl PlayLink {
    /// The portal chooser owns input while open, before movement and spell hotkeys.
    pub fn newhaven_vocation_input(&self, events: &[NormalizedInputEvent]) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.vocation.open {
            return false;
        }
        if let Some(vocation) =
            oteryn_client_ui::newhaven_vocation::choice_key(events).and_then(|vocation| {
                oteryn_client_ui::newhaven_vocation::request(&mut mailbox.vocation, vocation)
            })
            && let Some(payload) = (NewhavenVocationRequest { vocation }).encode()
            && self
                .commands
                .send(PlayCommand::NewhavenVocation(payload))
                .is_err()
        {
            mailbox.vocation.result(false);
        }
        true
    }
}
