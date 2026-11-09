//! Canary livestream.cpp:237,249,260: explicit session-local start, settings and stop.
pub use oteryn_client_runtime::cast::Message;
mod input;
pub use input::open_key;
#[derive(Debug, Clone)]
pub struct Panel {
    pub selected: bool,
    pub consent_selected: bool,
    pub consent_known: bool,
    pub consent_revision: u64,
    pub allow_shown: bool,
    pub open: bool,
    pub pending: bool,
    pub active: bool,
    pub revision: u64,
    pub epoch: u64,
    pub delay: u16,
    pub cap: u8,
    pub password: bool,
    pub access_revision: u64,
    pub viewers: u8,
    pub feedback: String,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            selected: false,
            consent_selected: false,
            consent_known: false,
            consent_revision: 0,
            allow_shown: false,
            open: false,
            pending: false,
            active: false,
            revision: 0,
            epoch: 0,
            delay: 0,
            cap: 50,
            password: false,
            access_revision: 1,
            viewers: 0,
            feedback: String::new(),
        }
    }
}
impl Panel {
    pub fn command(&self, action: usize) -> Result<Message, &'static str> {
        if self.pending
            || ((action <= 1 || action == 5) && !self.selected)
            || ((2..=4).contains(&action) && !self.consent_selected)
        {
            return Err(tr!("Cast controls unavailable"));
        }
        let message = match action {
            5 => Message::ControlQuery,
            2 => Message::ConsentQuery,
            3 | 4 if self.consent_known => Message::ConsentSet {
                revision: self.consent_revision,
                allowed: action == 3,
            },
            3 | 4 => return Err(tr!("Refresh character consent first")),
            0 if self.active => Message::Update {
                revision: self.revision,
                access_revision: self.access_revision,
                delay: self.delay,
                cap: self.cap,
                password: self.password,
            },
            0 => Message::Start {
                revision: 0,
                access_revision: self.access_revision,
                delay: self.delay,
                cap: self.cap,
                password: self.password,
            },
            1 if self.active => Message::Stop {
                revision: self.revision,
            },
            _ => return Err(tr!("No active cast")),
        };
        message
            .encode()
            .map_err(|_| tr!("Delay must be 0–300 and viewer limit 1–50"))?;
        Ok(message)
    }
    pub fn apply(&mut self, message: Message) -> Result<(), &'static str> {
        match message {
            Message::ConsentState { revision, allowed } => {
                self.consent_known = true;
                self.consent_revision = revision;
                self.allow_shown = allowed;
                self.feedback.clear();
            }
            Message::CasterState {
                revision,
                epoch,
                delay,
                count,
                ..
            } => {
                self.active = true;
                self.revision = revision;
                self.epoch = epoch;
                self.delay = delay;
                self.viewers = count;
                self.feedback.clear();
            }
            Message::End => {
                self.active = false;
                self.revision = 0;
                self.epoch = 0;
                self.viewers = 0;
            }
            _ => return Err(tr!("Invalid cast control response")),
        }
        self.pending = false;
        Ok(())
    }
    pub fn text(&self) -> String {
        lfmt!(
            "Casting {} · {} viewers · delay {} s · limit {}\nAllow me to be shown in casts: {}\n{}\nSave account preferences and passwords in account settings first. Game checks the saved access revision.",
            if self.active { tr!("on") } else { tr!("off") },
            self.viewers,
            self.delay,
            self.cap,
            if !self.consent_known {
                tr!("unknown — refresh")
            } else if self.allow_shown {
                tr!("on")
            } else {
                tr!("off")
            },
            self.feedback
        )
    }
}
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;

pub fn command(panel: &Panel, action: usize) -> Result<Message, &'static str> {
    panel.command(action)
}
pub fn panel_text(panel: &Panel) -> String {
    panel.text()
}

/// Moderation shares the confirmed caster revision, never viewer-supplied identity.
pub fn moderate(
    chat: &crate::cast_chat::Panel,
    control: &Panel,
    subject: &str,
    action: u8,
    key: [u8; 16],
) -> Result<oteryn_client_runtime::server_mechanic::ServerMechanicCommand, &'static str> {
    crate::cast_chat::protocol::command(chat, control, "", subject, action, key)
}
