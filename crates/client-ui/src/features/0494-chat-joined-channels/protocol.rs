//! Capability 494 `CHAT_JOINED_CHANNELS_V1`: the channel list names the private channels the
//! character is in, so their tabs exist again after a reconnect. Without the capability a tab
//! opens only from the accepted Join result.
use std::collections::BTreeSet;

/// The channels of `joined` that were not in it at the previous list, in list order. A channel
/// that left the list is forgotten, so joining it again opens a tab again, while a tab the
/// player closed stays closed for as long as the channel stays joined.
pub fn newly_joined(seen: &mut BTreeSet<u16>, joined: &[u16]) -> Vec<u16> {
    seen.retain(|id| joined.contains(id));
    joined
        .iter()
        .copied()
        .filter(|id| seen.insert(*id))
        .collect()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
