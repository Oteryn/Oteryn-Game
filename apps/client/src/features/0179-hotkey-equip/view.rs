#[path = "preferences.rs"]
mod preferences;
use crate::play::{PlayLink, mailbox::lock};
pub use preferences::ItemHotkeys;
pub(super) use preferences::edited;
pub(crate) use preferences::supported_capabilities;
impl PlayLink {
    #[must_use]
    pub fn hotkey_equip_panel(&self) -> oteryn_client_ui::hotkey_equip::HotkeyEquipPanel {
        lock(&self.mailbox).hotkey_equip.clone()
    }
}
#[cfg(any(windows, test))]
#[path = "native.rs"]
pub(in crate::play) mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(all(test, feature = "bevy-shell"))]
pub(crate) mod bound_tests {
    use super::*;
    use oteryn_client_ui::hotkey_equip::HotkeyEquipPanel;
    pub(crate) fn fixture() -> (
        PlayLink,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
        crate::play::Outbox,
    ) {
        let (link, commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        lock(&outbox.0).hotkey_equip = HotkeyEquipPanel {
            selected: true,
            items: vec![256, 512],
            ..Default::default()
        };
        (link, commands, outbox)
    }
    #[test]
    fn hotkey_equip_actions_are_bound_and_confirmed() {
        let (link, mut commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        lock(&outbox.0).hotkey_equip = HotkeyEquipPanel {
            open: true,
            selected: true,
            items: vec![256],
            ..Default::default()
        };
        let before = link.hotkey_equip_snapshot();
        assert!(link.hotkey_equip_bound(&before, HotkeyEquipControl::Assign(0, 256)));
        let assigned = link.hotkey_equip_snapshot();
        assert!(assigned.dirty);
        assert!(!link.hotkey_equip_bound(&assigned, HotkeyEquipControl::Clear(0, false)));
        assert_eq!(link.hotkey_equip_snapshot(), assigned);
        assert!(link.hotkey_equip_bound(&assigned, HotkeyEquipControl::Use(0)));
        assert!(
            matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.capability == 179 && command.command_type == 348 && command.payload == [0,0,1,0])
        );
        assert!(commands.try_recv().is_err());
        let pending = link.hotkey_equip_snapshot();
        assert!(pending.pending);
        assert_eq!(pending.outcome, assigned.outcome);
        assert!(!link.hotkey_equip_bound(&pending, HotkeyEquipControl::Use(0)));
        outbox.hotkey_equip_result(
            &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                command_id: 1,
                status: oteryn_session::CommandStatus::Accepted,
                server_sequence: 1,
                payload: vec![8, 1],
            },
        );
        let settled = link.hotkey_equip_snapshot();
        assert!(settled.outcome.is_some());
        assert!(!settled.pending);
        assert!(link.hotkey_equip_bound(&settled, HotkeyEquipControl::Clear(0, true)));
        let cleared = link.hotkey_equip_snapshot();
        assert!(cleared.bindings[0].is_none());
        assert!(link.hotkey_equip_bound(&cleared, HotkeyEquipControl::Smart(true)));
        let smart = link.hotkey_equip_snapshot();
        assert!(smart.smart && smart.dirty);
        assert!(!smart.smart_active);
        assert!(commands.try_recv().is_err());
        assert!(link.hotkey_equip_bound(&smart, HotkeyEquipControl::Equip(512)));
        assert!(
            matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.payload == [0,0,2,0])
        );
        assert!(commands.try_recv().is_err());
    }
    #[test]
    fn hotkey_equip_rejected_when_session_replaced_or_target_changed() {
        let (link, mut commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        lock(&outbox.0).hotkey_equip = HotkeyEquipPanel {
            open: true,
            selected: true,
            items: vec![256],
            ..Default::default()
        };
        let before = link.hotkey_equip_snapshot();
        let (replacement, mut replacement_commands, replacement_outbox) =
            crate::play::play_channel();
        replacement.use_shared_frontend();
        lock(&replacement_outbox.0).hotkey_equip = lock(&outbox.0).hotkey_equip.clone();
        assert!(!replacement.hotkey_equip_bound(&before, HotkeyEquipControl::Equip(256)));
        assert!(!replacement.hotkey_equip_bound(&before, HotkeyEquipControl::Close));
        assert!(replacement_commands.try_recv().is_err());
        lock(&outbox.0).hotkey_equip.items.clear();
        let changed = link.hotkey_equip_snapshot();
        assert!(!link.hotkey_equip_bound(&before, HotkeyEquipControl::Equip(256)));
        assert_eq!(link.hotkey_equip_snapshot(), changed);
        assert!(commands.try_recv().is_err());
        drop(commands);
        assert!(!link.hotkey_equip_bound(&changed, HotkeyEquipControl::Equip(256)));
    }
    #[test]
    fn hotkey_equip_owner_keys_reject_duplicates_without_mutation()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, mut commands, outbox) = fixture();
        {
            let mut mailbox = lock(&outbox.0);
            mailbox.hotkey_equip.bindings[0] = std::num::NonZeroU32::new(256);
            mailbox.hotkey_equip.bindings[1] = std::num::NonZeroU32::new(512);
        }
        let key = |code| -> Result<_, oteryn_input_actions::InputError> {
            Ok(oteryn_input_actions::NormalizedInputEvent::Key {
                code: oteryn_input_actions::KeyCode::new(code)?,
                state: oteryn_input_actions::ButtonState::Pressed,
                modifiers: oteryn_input_actions::Modifiers::NONE,
                repeat: false,
            })
        };
        assert!(link.hotkey_equip_keys_bound(&[key(58)?]));
        assert!(commands.try_recv().is_ok());
        let pending = link.hotkey_equip_snapshot();
        assert!(link.hotkey_equip_keys_bound(&[key(59)?]));
        assert_eq!(link.hotkey_equip_snapshot(), pending);
        assert!(commands.try_recv().is_err());
        Ok(())
    }
    #[test]
    fn hotkey_equip_close_preserves_pending_when_unavailable() {
        let (link, _commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        lock(&outbox.0).hotkey_equip = HotkeyEquipPanel {
            open: true,
            pending: true,
            selected: false,
            ..Default::default()
        };
        let before = link.hotkey_equip_snapshot();
        assert!(link.hotkey_equip_bound(&before, HotkeyEquipControl::Close));
        assert!(link.hotkey_equip_snapshot().pending);
        assert!(link.hotkey_equip_open_bound());
        assert!(link.hotkey_equip_snapshot().pending);
    }
}

/// A projection tied to the exact session mailbox; equality includes its identity.
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Debug)]
pub struct HotkeyEquipSnapshot {
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    panel: oteryn_client_ui::hotkey_equip::HotkeyEquipPanel,
}
#[cfg(feature = "bevy-shell")]
impl PartialEq for HotkeyEquipSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session) && self.panel == other.panel
    }
}
#[cfg(feature = "bevy-shell")]
impl Eq for HotkeyEquipSnapshot {}
#[cfg(feature = "bevy-shell")]
impl std::ops::Deref for HotkeyEquipSnapshot {
    type Target = oteryn_client_ui::hotkey_equip::HotkeyEquipPanel;
    fn deref(&self) -> &Self::Target {
        &self.panel
    }
}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyEquipControl {
    Close,
    Assign(usize, u32),
    Clear(usize, bool),
    Use(usize),
    Smart(bool),
    Equip(u32),
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    /// Resolve owner key bindings and enqueue atomically, including when the window is closed.
    pub fn hotkey_equip_keys_bound(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        use oteryn_input_actions::{ButtonState, Modifiers, NormalizedInputEvent};
        for event in events {
            let NormalizedInputEvent::Key {
                code,
                state: ButtonState::Pressed,
                modifiers,
                repeat,
            } = event
            else {
                continue;
            };
            if *modifiers != Modifiers::NONE {
                continue;
            }
            let Some(slot) = code
                .get()
                .checked_sub(58)
                .map(usize::from)
                .filter(|slot| *slot < oteryn_client_ui::hotkey_equip::ITEM_HOTKEY_SLOTS)
            else {
                continue;
            };
            let mut mailbox = lock(&self.mailbox);
            let blocked = self.commands.is_closed() || mailbox.server_modal.is_active();
            let panel = &mut mailbox.hotkey_equip;
            let Some(definition) = panel.bindings[slot] else {
                continue;
            };
            if *repeat || blocked || !panel.selected || panel.pending {
                return true;
            }
            let mut draft = panel.clone();
            draft.item_definition = definition.get();
            if let Some(command) = oteryn_client_ui::hotkey_equip::control_request(&mut draft)
                && self
                    .commands
                    .send(crate::combat_input::PlayCommand::ServerMechanic(command))
                    .is_ok()
            {
                *panel = draft;
            }
            return true;
        }
        false
    }
    pub fn hotkey_equip_snapshot(&self) -> HotkeyEquipSnapshot {
        let mailbox = lock(&self.mailbox);
        HotkeyEquipSnapshot {
            session: std::sync::Arc::downgrade(&self.mailbox),
            panel: mailbox.hotkey_equip.clone(),
        }
    }
    pub fn hotkey_equip_open_bound(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed() || mailbox.server_modal.is_active() {
            return false;
        }
        mailbox.hotkey_equip.open = true;
        true
    }
    /// Admission, persistence edits and command enqueue share the owner lock.
    pub fn hotkey_equip_bound(
        &self,
        expected: &HotkeyEquipSnapshot,
        action: HotkeyEquipControl,
    ) -> bool {
        use HotkeyEquipControl as Kind;
        use oteryn_client_ui::hotkey_equip as ui;
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || self.commands.is_closed()
        {
            return false;
        }
        // Close is session-only: capability, input fences and pending work cannot trap it.
        if action == Kind::Close {
            mailbox.hotkey_equip.open = false;
            return true;
        }
        if mailbox.server_modal.is_active() {
            return false;
        }
        let panel = &mut mailbox.hotkey_equip;
        if panel != &expected.panel || !panel.open || !panel.selected || panel.pending {
            return false;
        }
        let mut draft = panel.clone();
        match action {
            Kind::Assign(slot, definition) => {
                if definition == 0
                    || draft
                        .bindings
                        .get(slot)
                        .is_none_or(|binding| binding.is_some_and(|d| d.get() == definition))
                    || !ui::assign(&mut draft, slot, definition)
                {
                    return false;
                }
                edited(&mut draft);
            }
            Kind::Clear(slot, confirmed) => {
                if !confirmed
                    || draft.bindings.get(slot).is_none_or(Option::is_none)
                    || !ui::assign(&mut draft, slot, 0)
                {
                    return false;
                }
                edited(&mut draft);
            }
            Kind::Smart(smart) => {
                if draft.smart == smart {
                    return false;
                }
                draft.smart = smart;
                edited(&mut draft);
            }
            Kind::Use(_) | Kind::Equip(_) => {
                let definition = match action {
                    Kind::Use(slot) => {
                        let Some(Some(definition)) = draft.bindings.get(slot) else {
                            return false;
                        };
                        definition.get()
                    }
                    Kind::Equip(definition) => definition,
                    _ => return false,
                };
                if definition == 0 {
                    return false;
                }
                draft.item_definition = definition;
                let Some(command) = ui::control_request(&mut draft) else {
                    return false;
                };
                if self
                    .commands
                    .send(crate::combat_input::PlayCommand::ServerMechanic(command))
                    .is_err()
                {
                    return false;
                }
            }
            Kind::Close => return false,
        }
        *panel = draft;
        true
    }
}
