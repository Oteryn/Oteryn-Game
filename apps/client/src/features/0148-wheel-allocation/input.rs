use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::wheel_allocation::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    pub fn wheel_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }

    pub fn wheel_input(&self, events: &[NormalizedInputEvent]) -> bool {
        use oteryn_input_actions::{ButtonState, Modifier, Modifiers};
        let presets = events.iter().any(|event| matches!(event, NormalizedInputEvent::Key {
            code, state: ButtonState::Pressed, modifiers, repeat: false
        } if code.get() == 26 && *modifiers == Modifiers::one(Modifier::Control).with(Modifier::Shift)));
        // The shared frontend draws presets inside the Wheel panel, so a closed
        // panel takes the full open path, which also opens the presets.
        if presets && !(self.uses_shared_frontend() && !lock(&self.mailbox).wheel.open) {
            self.wheel_presets_open();
            return true;
        }
        if !presets && !oteryn_client_ui::wheel_allocation::open_key(events) {
            return false;
        }
        self.wheel_request(Intent::Refresh);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            {
                let mut mailbox = lock(&self.mailbox);
                mailbox.wheel.open = true;
                if mailbox.shared_frontend {
                    mailbox.wheel.diagram.last = None;
                }
            }
            if self.uses_shared_frontend() {
                self.wheel_presets_open();
            }
        }
        true
    }
}

/// The prepared bytes, including occurrence, remain in the panel until the outcome settles.
pub(in crate::play) fn enqueue(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if mailbox.wheel.pending.is_some() {
        return;
    }
    let occurrence = if matches!(intent, Intent::Refresh | Intent::Retry) {
        [0; 16]
    } else if let Some(id) = occurrence() {
        id
    } else {
        mailbox.wheel.feedback = Some(tr!("A request identity could not be created; try again"));
        return;
    };
    if matches!(
        intent,
        Intent::Reveal | Intent::Dismantle | Intent::Reshape | Intent::GradeUp
    ) {
        oteryn_client_ui::wheel_allocation::gems::stage(&mut mailbox.wheel, intent, occurrence);
        return;
    }
    if let Some(command) = request(&mut mailbox.wheel, intent, occurrence)
        && commands.send(PlayCommand::Wheel(command)).is_err()
    {
        mailbox.wheel.pending = None;
        mailbox.wheel.feedback = Some(tr!("The session has ended"));
    }
}

fn occurrence() -> Option<[u8; 16]> {
    let mut id = [0; 16];
    oteryn_platform_client::native_login::secure_random(&mut id).ok()?;
    let milliseconds = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
    .ok()?;
    if milliseconds >= 1_u64 << 48 {
        return None;
    }
    id[..6].copy_from_slice(&milliseconds.to_be_bytes()[2..]);
    id[6] = 0x70 | (id[6] & 0x0f);
    id[8] = 0x80 | (id[8] & 0x3f);
    Some(id)
}

#[path = "gem_presentation.rs"]
pub(in crate::play) mod gem_presentation;

#[cfg(feature = "bevy-shell")]
#[path = "shared.rs"]
pub(crate) mod shared;

#[path = "wheel_presentation.rs"]
pub(in crate::play) mod wheel_presentation;
