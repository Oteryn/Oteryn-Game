//! Atomic presentation actions over the retained Wheel owner.
use super::*;
use oteryn_client_ui::wheel_allocation::Intent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WheelControl {
    Close,
    Metadata,
    Points { slot: usize, points: u16 },
    Gem([u8; 16]),
    Vessel(u8),
    Socket,
    Clear,
    Request(Intent),
    Confirm,
    Cancel,
    Preset { action: usize, name: String },
}
fn admitted(m: &crate::play::mailbox::Mailbox, expected: &WheelPanel) -> bool {
    !m.wheel_shutdown
        && m.wheel == *expected
        && (m.wheel.open || m.wheel.presets_open)
        && !m.world_input_blocked()
        && !m.server_modal.is_active()
        && m.wheel.pending.is_none()
        && m.wheel.gems.pending.is_none()
        && m.wheel.diagram.pending.is_none()
}
impl PlayLink {
    pub(crate) fn wheel_window_panel(&self) -> Option<WheelPanel> {
        let m = lock(&self.mailbox);
        (!m.wheel_shutdown && (m.wheel.open || m.wheel.presets_open)).then(|| m.wheel.clone())
    }
    /// Re-prove the complete displayed state under the same lock as the existing writer.
    pub(crate) fn wheel_dispatch_bound(&self, expected: &WheelPanel, action: WheelControl) -> bool {
        let mut m = lock(&self.mailbox);
        if !admitted(&m, expected) || (!m.wheel.selected && action != WheelControl::Close) {
            return false;
        }
        if m.wheel.gems.review.is_some()
            && !matches!(
                action,
                WheelControl::Confirm | WheelControl::Cancel | WheelControl::Close
            )
        {
            return false;
        }
        if m.wheel.retry.is_some()
            && !matches!(
                action,
                WheelControl::Request(Intent::Retry) | WheelControl::Close
            )
        {
            return false;
        }
        let before = m.wheel.clone();
        match action {
            WheelControl::Metadata => {
                super::super::input::wheel_presentation::query(&mut m, &self.commands);
            }
            WheelControl::Close => {
                m.wheel.open = false;
                m.wheel.presets_open = false;
                m.wheel.gems.review = None;
            }
            WheelControl::Points { slot, points } => {
                if !m.wheel.set_points(slot, points) {
                    m.wheel = before;
                    return false;
                }
            }
            WheelControl::Gem(id) => {
                if !m.wheel.ready()
                    || !m
                        .wheel
                        .confirmed_atelier
                        .as_ref()
                        .is_some_and(|a| a.gems.iter().any(|g| g.id == id))
                {
                    return false;
                }
                m.wheel.select_gem(Some(id));
            }
            WheelControl::Vessel(slot) => {
                if !m.wheel.ready() || slot >= m.wheel.vessel_slot_count() {
                    return false;
                }
                m.wheel.vessel_slot = slot;
            }
            WheelControl::Socket | WheelControl::Clear => {
                let a = if action == WheelControl::Socket {
                    super::super::input::gem_presentation::Action::Socket
                } else {
                    super::super::input::gem_presentation::Action::Clear
                };
                super::super::input::gem_presentation::activate(&mut m, &self.commands, a);
            }
            WheelControl::Request(intent) => {
                super::super::input::enqueue(&mut m, &self.commands, intent)
            }
            WheelControl::Confirm => {
                super::super::input::gem_presentation::confirm(&mut m, &self.commands)
            }
            WheelControl::Cancel => m.wheel.gems.review = None,
            WheelControl::Preset { action, name } => {
                let Ok(path) = super::presets::path() else {
                    return false;
                };
                if !preset(&mut m, expected, action, &name, &path) {
                    return false;
                }
            }
        }
        m.wheel != before
    }
    #[cfg(test)]
    pub(crate) fn wheel_preset_bound(
        &self,
        expected: &WheelPanel,
        action: usize,
        name: &str,
        path: &std::path::Path,
    ) -> bool {
        preset(&mut lock(&self.mailbox), expected, action, name, path)
    }
    #[cfg(test)]
    pub(crate) fn wheel_test_update(&self, update: impl FnOnce(&mut WheelPanel, &mut bool)) {
        let mut m = lock(&self.mailbox);
        let crate::play::mailbox::Mailbox {
            wheel,
            wheel_shutdown,
            ..
        } = &mut *m;
        update(wheel, wheel_shutdown);
    }
}
fn preset(
    m: &mut crate::play::mailbox::Mailbox,
    expected: &WheelPanel,
    action: usize,
    name: &str,
    path: &std::path::Path,
) -> bool {
    if !admitted(m, expected) || !m.wheel.ready() || m.wheel.gems.review.is_some() || action > 1 {
        return false;
    }
    // Load through the same format and owner as the native preset window.
    let Ok(book) = oteryn_client_ui::wheel_allocation::presets::PresetBook::load(path) else {
        return false;
    };
    m.wheel.preset_fields[0] = name.to_owned();
    m.wheel.preset_book = book;
    super::presets::submit(m, action, path);
    true
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn wheel_test_result(
        &self,
        response: &oteryn_client_ui::wheel_allocation::WheelResponse,
    ) {
        self.wheel_result(response);
    }
    pub(crate) fn wheel_test_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
}
