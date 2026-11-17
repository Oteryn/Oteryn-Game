//! Capability 495 `CHAT_CHANNEL_PARTICIPANTS_V1`: a private channel's tab is captioned
//! "<Owner>'s channel", lists its participants and shows who joined, left or was excluded.
//! Without the capability the tab reads "Channel <id>" and carries none of it.

/// What happened to a member of a private channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Joined,
    Left,
    Excluded,
}

/// One join, leave or exclude notice; `serial` orders the notices of a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeLine {
    pub serial: u64,
    pub channel_id: u16,
    pub kind: NoticeKind,
    pub name: String,
}

/// The most notices a console keeps.
pub const MAX_NOTICES: usize = 200;

/// The tab caption of a channel whose owner is known.
#[must_use]
pub fn channel_label(owner: &str) -> String {
    lfmt!("{owner}'s channel", owner = owner)
}

/// The participants line of a channel tab.
#[must_use]
pub fn participants_text(names: &[String]) -> String {
    lfmt!("Participants: {names}", names = names.join(", "))
}

/// A notice as the channel tab prints it.
#[must_use]
pub fn notice_text(notice: &NoticeLine) -> String {
    match notice.kind {
        NoticeKind::Joined => lfmt!("{name} has joined the channel", name = notice.name),
        NoticeKind::Left => lfmt!("{name} has left the channel", name = notice.name),
        NoticeKind::Excluded => lfmt!(
            "{name} has been excluded from the channel",
            name = notice.name
        ),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
