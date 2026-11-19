//! Channel on social rows (capability 513, owner decision 2026-11-17): Party, VIP and Guild rows
//! show the member's Channel of the same World; a Party member on another Channel is greyed
//! out, since shared experience, loot and Party spells need the same Channel; VIP notices name
//! the Channel of a login, logout or Channel change.
mod protocol;
mod view;
pub use protocol::presence_notices;
pub use view::{
    ChannelNames, channel_text, guild_member_text, party_member_on_other_channel,
    party_member_text, vip_entry_text,
};

#[cfg(test)]
mod tests;
