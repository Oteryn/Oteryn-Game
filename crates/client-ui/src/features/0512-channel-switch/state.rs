use oteryn_client_runtime::channel_switch::SwitchStatus;

/// The PvP type of one Channel (§3). Platform publishes it per Channel; `None` in a row means
/// the directory did not carry it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvpType {
    OptionalPvp,
    OpenPvp,
}

/// One Channel of the current World as the directory and the client's own ping measured it.
/// Every optional field stays `None` until its source exists; nothing is guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelRow {
    pub channel: [u8; 16],
    pub name: String,
    pub pvp: Option<PvpType>,
    /// Round trip from this client to the Channel node, measured by the client.
    pub ping_ms: Option<u32>,
    /// Characters online on the Channel, from the server's durable sessions (command 6251).
    pub online: Option<u32>,
    pub full: bool,
}

/// The Change Channel window. `status` is the latest server answer, read at `status_at_ms`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChannelSwitchPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub current: Option<[u8; 16]>,
    pub rows: Vec<ChannelRow>,
    pub status: Option<SwitchStatus>,
    pub status_at_ms: i64,
    /// The target waiting for the player's confirmation.
    pub confirming: Option<[u8; 16]>,
    /// The target of the request in flight, and after an accepted switch the Channel the shell
    /// enters by a fresh admission.
    pub switching: Option<[u8; 16]>,
    /// The error dialog text, if any.
    pub error: Option<String>,
}

impl ChannelSwitchPanel {
    pub(crate) fn row(&self, channel: [u8; 16]) -> Option<&ChannelRow> {
        self.rows.iter().find(|row| row.channel == channel)
    }
}
