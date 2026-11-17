//! Display filters never remove messages from the session-owned history.
use oteryn_client_runtime::chat::ChatLine;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presentation {
    pub visible: [bool; 5],
    pub mode_labels: bool,
    pub dropped: bool,
    pub feedback: bool,
    pub limit: u16,
}
impl Default for Presentation {
    fn default() -> Self {
        Self {
            visible: [true; 5],
            mode_labels: true,
            dropped: true,
            feedback: true,
            limit: 1000,
        }
    }
}
impl Presentation {
    pub fn shows(&self, line: &ChatLine) -> bool {
        match line {
            ChatLine::Local { .. } => self.visible[0],
            ChatLine::Private { .. } | ChatLine::PrivateChannel(_) => self.visible[1],
            ChatLine::Guild(_) => self.visible[2],
            ChatLine::Room { .. } => self.visible[3],
            ChatLine::Party { .. } => self.visible[4],
            ChatLine::Dropped => self.dropped,
        }
    }
}
