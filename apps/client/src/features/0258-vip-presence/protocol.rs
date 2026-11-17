use crate::play::{Outbox, Pushed, mailbox::lock};
use oteryn_session::{AppliedDelta, VipPresenceBatch};
impl Outbox {
    pub(in crate::play) fn vip_presence_initialize(&self, selected: bool, social: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.vip_presence.selected = selected;
        mailbox.social_channel_selected = social;
    }
}
pub(in crate::play) fn vip_presence(pushed: &mut Pushed, delta: AppliedDelta<VipPresenceBatch>) {
    pushed.vip_presence = Some(delta.value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_session::{SessionEvent, VipPresenceEntry, VipPresenceStatus};

    #[test]
    fn disabled_vip_never_announces_a_channel_move() {
        let (_link, _commands, outbox) = crate::play::play_channel();
        outbox.vip_presence_initialize(true, true);
        lock(&outbox.0).vip.view = Some(r#"{"entries":[{"entry":1,"name":"Bryn","notify":false}]}"#.into());
        for channel in [1, 2] {
            outbox.push(vec![SessionEvent::VipPresence(AppliedDelta {
                server_sequence: channel, base_revision: channel-1, new_revision: channel,
                value: VipPresenceBatch { entries: vec![VipPresenceEntry {
                    entry: 1, name: "Bryn".into(), status: VipPresenceStatus::Online,
                    announce: false, confirmed_for_ms: 10000, channel: Some([channel as u8;16]),
                }] },
            })]);
        }
        assert!(lock(&outbox.0).vip_channel_notices.is_empty());
    }

    #[test]
    fn legacy_presence_does_not_announce_a_channel() {
        let (_link, _commands, outbox) = crate::play::play_channel();
        outbox.vip_presence_initialize(true, false);
        outbox.push(vec![SessionEvent::VipPresence(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: VipPresenceBatch { entries: vec![VipPresenceEntry {
                entry: 1, name: "Bryn".into(), status: VipPresenceStatus::Online,
                announce: true, confirmed_for_ms: 10000, channel: None,
            }] },
        })]);
        assert!(lock(&outbox.0).vip_channel_notices.is_empty());
    }
}
