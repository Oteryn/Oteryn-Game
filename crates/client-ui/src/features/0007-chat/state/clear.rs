use super::{ChannelTab, ChatPanel, ConsoleTab};
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Cutoff {
    tab: ConsoleTab,
    pub(super) channel: Option<ChannelTab>,
    received: u64,
    notices: u64,
    channel_notice: Option<u64>,
    npc: u64,
    feedback: Option<String>,
}

impl ChatPanel {
    pub fn clear_current_transcript(&mut self) {
        let cut = Cutoff {
            tab: self.console.tab,
            channel: self.console.current.clone(),
            received: self.received,
            notices: self.console.notices.serial(),
            channel_notice: self.console.channel_notices.last().map(|line| line.serial),
            npc: self.console.npc_serial,
            feedback: self.feedback.clone(),
        };
        self.console.cleared.retain(|old| {
            old.channel != cut.channel || (cut.channel.is_none() && old.tab != cut.tab)
        });
        self.console.cleared.push(cut);
    }

    pub fn transcript_after_clear(
        &self,
        tab: ConsoleTab,
        channel: Option<&ChannelTab>,
    ) -> Cow<'_, Self> {
        let Some(cut) =
            self.console.cleared.iter().find(|cut| {
                cut.channel.as_ref() == channel && (channel.is_some() || cut.tab == tab)
            })
        else {
            return Cow::Borrowed(self);
        };
        let mut panel = self.clone();
        panel.console.cleared.clear();
        let skip = panel.lines.len().saturating_sub(
            usize::try_from(self.received.wrapping_sub(cut.received)).unwrap_or(usize::MAX),
        );
        panel.lines.drain(..skip);
        panel.recipients.drain(..skip.min(panel.recipients.len()));
        panel.console.metadata.skip_receipts(skip);
        panel.console.notices.retain_after(cut.notices);
        if let Some(serial) = cut.channel_notice {
            panel
                .console
                .channel_notices
                .retain(|line| line.serial > serial);
        }
        let skip = panel.console.npc_lines.len().saturating_sub(
            usize::try_from(self.console.npc_serial.saturating_sub(cut.npc)).unwrap_or(usize::MAX),
        );
        panel.console.npc_lines.drain(..skip);
        if panel.feedback == cut.feedback {
            panel.feedback = None;
        }
        Cow::Owned(panel)
    }
}
