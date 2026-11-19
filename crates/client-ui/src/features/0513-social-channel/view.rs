use oteryn_client_runtime::cl25a_protocol::{VipPresenceEntry, VipPresenceStatus};
use oteryn_client_runtime::guild::guild::GuildMember;
use oteryn_client_runtime::party::party::PartyEntry;
use std::collections::BTreeMap;

/// Channel names of the current World, from the Platform directory (client-directory-v1).
pub type ChannelNames = BTreeMap<[u8; 16], String>;

/// The Channel's name, or "—" when the member is offline or the Channel is not listed.
#[must_use]
pub fn channel_text(names: &ChannelNames, channel: Option<[u8; 16]>) -> String {
    channel
        .and_then(|channel| names.get(&channel))
        .map_or_else(|| "—".to_owned(), Clone::clone)
}

/// A Party member is greyed out when it is known to be on another Channel than ours.
#[must_use]
pub fn party_member_on_other_channel(own: Option<[u8; 16]>, member: &PartyEntry) -> bool {
    matches!((own, member.channel), (Some(own), Some(theirs)) if own != theirs)
}

/// `Name — Channel` for a Party member row.
#[must_use]
pub fn party_member_text(names: &ChannelNames, member: &PartyEntry) -> String {
    format!("{} — {}", member.name, channel_text(names, member.channel))
}

/// `Name — Online — Channel` for an Online VIP row; other statuses keep `Name — Status`.
#[must_use]
pub fn vip_entry_text(names: &ChannelNames, entry: &VipPresenceEntry) -> String {
    match entry.status {
        VipPresenceStatus::Online => format!(
            "{} — {} — {}",
            entry.name,
            tr!("Online"),
            channel_text(names, entry.channel)
        ),
        VipPresenceStatus::Offline => format!("{} — {}", entry.name, tr!("Offline")),
        VipPresenceStatus::Unconfirmed => format!("{} — {}", entry.name, tr!("Unconfirmed")),
    }
}

/// `Name — Channel` for a Guild roster row.
#[must_use]
pub fn guild_member_text(names: &ChannelNames, member: &GuildMember) -> String {
    format!("{} — {}", member.name, channel_text(names, member.channel))
}
