use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn hotkey_equip_assign(&self, slot: usize, definition: u32) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.hotkey_equip;
        let assigned = oteryn_client_ui::hotkey_equip::assign(panel, slot, definition);
        if assigned {
            super::view::edited(panel);
        }
        assigned
    }

    pub fn hotkey_equip_request(&self, payload: Vec<u8>) {
        let mut mailbox = lock(&self.mailbox);
        let command = oteryn_client_ui::hotkey_equip::command(payload);
        if let Some(command) =
            oteryn_client_ui::hotkey_equip::request(&mut mailbox.hotkey_equip, command)
            && self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
        {
            mailbox.hotkey_equip.pending = false;
            mailbox.hotkey_equip.feedback = Some(tr!("The session has ended").into());
        }
    }
    pub fn hotkey_equip_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        use oteryn_input_actions::{ButtonState, Modifiers, NormalizedInputEvent};
        for event in events {
            if let NormalizedInputEvent::Key {
                code,
                state: ButtonState::Pressed,
                modifiers,
                repeat,
            } = event
                && *modifiers == Modifiers::NONE
                && let Some(slot) = code
                    .get()
                    .checked_sub(58)
                    .map(usize::from)
                    .filter(|slot| *slot < oteryn_client_ui::hotkey_equip::ITEM_HOTKEY_SLOTS)
            {
                let mut mailbox = lock(&self.mailbox);
                let panel = &mut mailbox.hotkey_equip;
                let Some(definition) = panel.bindings[slot] else {
                    continue;
                };
                if !*repeat {
                    panel.item_definition = definition.get();
                    if let Some(command) = oteryn_client_ui::hotkey_equip::control_request(panel)
                        && self
                            .commands
                            .send(PlayCommand::ServerMechanic(command))
                            .is_err()
                    {
                        panel.pending = false;
                        panel.feedback = Some(tr!("The session has ended").into());
                    }
                }
                return true;
            }
        }
        if !oteryn_client_ui::hotkey_equip::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).hotkey_equip.open = true;
        }
        true
    }
}
