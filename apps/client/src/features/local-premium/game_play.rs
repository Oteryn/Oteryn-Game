//! Mounted inside play's Store view to keep mailbox access within its owner.
use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayLink, mailbox::lock};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
impl Outbox {
    pub(crate) fn premium_game_initialize(&self, selected: bool) {
        lock(&self.0).premium_game = crate::premium::game::Panel {
            selected,
            ..Default::default()
        };
    }
    pub(crate) fn premium_game_result(
        &self,
        command: &FeatureCommand,
        result: &GatedCommandResult,
    ) {
        lock(&self.0).premium_game.apply(command, result);
    }
}
impl PlayLink {
    pub fn premium_overview_text(&self) -> String {
        lock(&self.mailbox).premium_game.view.status_text()
    }
    pub fn premium_overview_refresh(&self) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = mailbox.premium_game.request()
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.premium_game.fail("The session has ended.");
        }
    }
    pub fn premium_overview_open(&self) {
        self.premium_overview_refresh();
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::premium_native::open(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).premium_game.open = true;
        }
    }
    pub fn premium_overview_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
        let chord = Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift);
        if events.iter().any(|event|matches!(event,NormalizedInputEvent::Key {code,state:ButtonState::Pressed,modifiers,repeat:false} if code.get()==19&&*modifiers==chord)) {self.premium_overview_open();true}else{false}
    }
}

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn premium_overview_binding(&self) -> crate::premium::game::Binding {
        let identity: std::sync::Arc<dyn Send + Sync> = self.mailbox.clone();
        lock(&self.mailbox)
            .premium_game
            .binding(std::sync::Arc::downgrade(&identity))
    }

    /// Close is session-only: never cancel or discard the retained in-flight observation.
    pub(crate) fn premium_overview_action_bound(
        &self,
        expected: &crate::premium::game::Binding,
        close: bool,
        admitted: bool,
    ) -> bool {
        use std::sync::atomic::Ordering;
        let identity: std::sync::Arc<dyn Send + Sync> = self.mailbox.clone();
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&identity))
        {
            return false;
        }
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.shared_frontend
            || mailbox.quest_shutdown
            || mailbox.custom_hotkeys_ended
            || self.commands.is_closed()
        {
            return false;
        }
        if close {
            if !mailbox.premium_game.open {
                return false;
            }
            mailbox.premium_game.open = false;
            mailbox.premium_game.bevy_revision = mailbox.premium_game.bevy_revision.wrapping_add(1);
            expected.used.store(true, Ordering::Relaxed);
            return true;
        }
        if !admitted
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || expected.used.load(Ordering::Relaxed)
            || !expected.matches(&mailbox.premium_game)
        {
            return false;
        }
        // Opening remains possible without capability selection or during a pending request.
        // Refresh itself is always admitted by the existing owner.
        if mailbox.premium_game.open
            && (!mailbox.premium_game.selected || mailbox.premium_game.pending)
        {
            return false;
        }
        expected.used.store(true, Ordering::Relaxed);
        mailbox.premium_game.open = true;
        mailbox.premium_game.bevy_revision = mailbox.premium_game.bevy_revision.wrapping_add(1);
        if let Some(command) = mailbox.premium_game.request()
            && self.commands.send(PlayCommand::Feature(command)).is_err()
        {
            mailbox.premium_game.fail("The session has ended.");
        }
        true
    }

    /// The default owner's Ctrl+Alt+Shift+P chord, routed only by shared controls.
    pub(crate) fn premium_overview_input_bound(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
        let chord = Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift);
        if !events.iter().any(|event| matches!(event,
            NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
            if code.get() == 19 && *modifiers == chord)) {
            return false;
        }
        let expected = self.premium_overview_binding();
        // Party Effects already uses this chord: preserve its route if Premium is unselected.
        if !expected.selected {
            return false;
        }
        self.premium_overview_action_bound(&expected, expected.open, true)
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "game_bound_tests.rs"]
mod bound_tests;
