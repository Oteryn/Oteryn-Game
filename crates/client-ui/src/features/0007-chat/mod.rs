//! Local speech and named private messages over the session chat owner.
#[path = "../../components/console_display.rs"]
mod display;
pub use display::{ConsoleDisplay, Metadata};
#[path = "../../components/console_filters.rs"]
mod filters;
pub use filters::{ConsoleFilters, ConsoleNotices, NoticeClass, NoticeScope};
#[path = "state/channel_dialog.rs"]
mod channel_dialog;
mod input;
pub use channel_dialog::{ChannelDialog, DialogAction, dialog_rows};
#[path = "state/extensions.rs"]
mod extensions;
pub use extensions::ChatExtras;
#[path = "state/presentation.rs"]
mod presentation;
mod protocol;
pub use presentation::Presentation;
mod state;
mod view;
pub use crate::chat_public_channels::apply_result;
pub use input::{default_channel_key, open_key, temporary_mode_key};
pub use protocol::{
    channel_request, close_current, console_request, open_conversation, open_entry, open_help,
    private_result, request,
};
pub use state::{
    ChannelTab, ChatPanel, CommunicationList, CommunicationLists, ComposerMode, Console,
    ConsoleTab, Mode, PendingFocus,
};
pub use view::{
    ChannelEntry, channel_entries, channel_list_text, dialog_entries, mirror_text, panel_text,
    write_log,
};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
