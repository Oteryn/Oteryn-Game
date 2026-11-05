pub use crate::chat_public_channels::ChatPanel;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Say,
    Whisper,
    Yell,
    Private,
    Guild,
}
