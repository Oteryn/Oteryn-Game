use crate::play::PlayLink;
use oteryn_client_ui::chat::{ChannelTab, ConsoleTab, Mode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Tab {
    Fixed(ConsoleTab),
    Channel(ChannelTab),
}

pub(super) fn submit(
    link: &PlayLink,
    generation: u64,
    current: u64,
    mode: Mode,
    text: &str,
) -> bool {
    if generation != current || link.chat_panel().pending {
        return false;
    }
    link.chat_text(text.to_owned());
    link.chat_request(mode, "");
    link.chat_panel().pending
}

pub(super) fn select(link: &PlayLink, tab: &Tab) {
    match tab {
        Tab::Fixed(tab) => link.chat_console_tab(*tab),
        Tab::Channel(target) => {
            let panel = link.chat_panel();
            if let Some(index) = panel.console.channels.iter().position(|tab| tab == target) {
                let target = index + 4;
                let count = panel.console.tab_count();
                for _ in 0..(target + count - panel.console.tab_index()) % count {
                    link.chat_channel_cycle(true);
                }
            }
        }
    }
}
