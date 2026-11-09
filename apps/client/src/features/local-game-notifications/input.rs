use crate::play::PlayView;
use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
impl PlayView {
    /// Preferences apply to queued and future notices in this connection.
    pub fn configure_notifications(&self, banners: bool, popups: bool) {
        let mut center = super::state::lock(&self.game_notifications.0);
        center.banners_enabled = banners;
        center.popups_enabled = popups;
    }
    /// Ctrl+Alt+Shift+F10/F11 toggle banners/popups; F12 dismisses current notices.
    pub fn notification_input(&self, events: &[NormalizedInputEvent]) -> bool {
        let chord = Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift);
        let mut handled = false;
        for event in events {
            if let NormalizedInputEvent::Key {
                code,
                state: ButtonState::Pressed,
                modifiers,
                repeat: false,
            } = event
                && *modifiers == chord
            {
                let mut center = super::state::lock(&self.game_notifications.0);
                match code.get() {
                    67 => center.banners_enabled = !center.banners_enabled,
                    68 => center.popups_enabled = !center.popups_enabled,
                    69 => center.dismiss_all(),
                    _ => continue,
                }
                handled = true;
            }
        }
        handled
    }
}
