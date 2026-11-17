//! The VIP push reducer of capability 513: notices that name the Channel.
use super::view::{ChannelNames, channel_text};
use oteryn_client_runtime::cl25a_protocol::{VipPresenceBatch, VipPresenceStatus};

/// The status notices a new VIP batch causes: an announced login or logout names its Channel,
/// and an Online entry that changed Channel says where it went.
#[must_use]
pub fn presence_notices(
    names: &ChannelNames,
    before: &VipPresenceBatch,
    after: &VipPresenceBatch,
    notify_move: impl Fn(u64) -> bool,
) -> Vec<String> {
    let mut notices = Vec::new();
    for entry in &after.entries {
        let prior = before.entries.iter().find(|old| old.entry == entry.entry);
        match entry.status {
            VipPresenceStatus::Online if entry.announce => notices.push(lfmt!(
                "{name} has logged in on channel {channel}.",
                name = entry.name,
                channel = channel_text(names, entry.channel)
            )),
            VipPresenceStatus::Online => {
                if notify_move(entry.entry)
                    && prior.is_some_and(|old| {
                        old.status == VipPresenceStatus::Online
                            && old.channel.is_some()
                            && entry.channel.is_some()
                            && old.channel != entry.channel
                    })
                {
                    notices.push(lfmt!(
                        "{name} has changed to channel {channel}.",
                        name = entry.name,
                        channel = channel_text(names, entry.channel)
                    ));
                }
            }
            VipPresenceStatus::Offline if entry.announce => notices.push(lfmt!(
                "{name} has logged out of channel {channel}.",
                name = entry.name,
                channel = channel_text(names, prior.and_then(|old| old.channel))
            )),
            VipPresenceStatus::Offline | VipPresenceStatus::Unconfirmed => {}
        }
    }
    notices
}
