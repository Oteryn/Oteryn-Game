//! Canary livestream.cpp:237,249,260: explicit session-local start, settings and stop.
pub use oteryn_client_runtime::cast::Message;
mod input;
pub use input::open_key;
#[derive(Debug, Clone)]
pub struct Panel {
    pub selected: bool,
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
        if !self.selected || self.pending {
            return Err("Cast controls unavailable");
        }
        let message = match action {
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
            _ => return Err("No active cast"),
        };
        message
            .encode()
            .map_err(|_| "Delay must be 0–300 and viewer limit 1–50")?;
        Ok(message)
    }
    pub fn apply(&mut self, message: Message) -> Result<(), &'static str> {
        match message {
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
            _ => return Err("Invalid cast control response"),
        }
        self.pending = false;
        Ok(())
    }
    pub fn text(&self) -> String {
        format!(
            "Casting {} · {} viewers · delay {} s · limit {}\n{}\nSave account preferences and passwords in account settings first. Game checks the saved access revision.",
            if self.active { "on" } else { "off" },
            self.viewers,
            self.delay,
            self.cap,
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
