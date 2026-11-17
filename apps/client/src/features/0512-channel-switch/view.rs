//! chsw (capability 512): what the shell reads to draw the window and to enter the target.
use crate::play::{PlayLink, PlayView, mailbox::lock};

impl PlayLink {
    #[must_use]
    pub fn channel_switch_panel(&self) -> oteryn_client_ui::channel_switch::ChannelSwitchPanel {
        lock(&self.mailbox).channel_switch.clone()
    }

    /// The Channel the server released this session for: the character and the Channel as
    /// canonical UUID text for the fresh admission, and the row for the arrival line.
    #[must_use]
    pub fn channel_switch_target(&self) -> Option<SwitchTarget> {
        let mailbox = lock(&self.mailbox);
        let target = mailbox.channel_switch_target?;
        let row = mailbox
            .channel_switch
            .rows
            .iter()
            .find(|row| row.channel == target)
            .cloned();
        Some(SwitchTarget {
            character_id: mailbox.channel_switch_character.clone(),
            channel_id: uuid_text(&target),
            row,
        })
    }
}

impl PlayView {
    /// The arrival line after the fresh admission into the target Channel.
    pub fn announce_channel_arrival(&self, row: &oteryn_client_ui::channel_switch::ChannelRow) {
        self.game_notifications.show(
            "channel-switch:arrival",
            &oteryn_client_ui::channel_switch::arrival_text(row),
            false,
        );
    }
}

/// Where an accepted switch goes next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchTarget {
    pub character_id: String,
    pub channel_id: String,
    pub row: Option<oteryn_client_ui::channel_switch::ChannelRow>,
}

fn uuid_text(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

/// The current wall clock for the window's cooldown countdown.
#[cfg(any(feature = "bevy-shell", test))]
#[must_use]
pub fn now_ms() -> i64 {
    super::protocol::now_ms()
}
