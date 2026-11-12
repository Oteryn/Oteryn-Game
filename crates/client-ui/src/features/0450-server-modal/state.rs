use oteryn_client_runtime::server_modal::{Answer, Dialog};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    Query,
    Answer(Answer),
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TerminalReplies {
    #[default]
    Probing,
    Enabled,
    Unsupported,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub pending: Option<Pending>,
    pub active: Option<Dialog>,
    pub feedback: Option<&'static str>,
    pub terminal_replies: TerminalReplies,
    pub logged_out: bool,
}
