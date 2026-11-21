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

    fn online(entry: u64, channel: u8, announce: bool) -> SessionEvent {
        SessionEvent::VipPresence(AppliedDelta {
            server_sequence: u64::from(channel),
            base_revision: u64::from(channel) - 1,
            new_revision: u64::from(channel),
            value: VipPresenceBatch {
                entries: vec![VipPresenceEntry {
                    entry,
                    name: "Bryn".into(),
                    status: VipPresenceStatus::Online,
                    announce,
                    confirmed_for_ms: 10_000,
                    channel: Some([channel; 16]),
                }],
            },
        })
    }

    #[test]
    fn native_panel_expires_the_channel_without_another_presence_frame() {
        let (link, _commands, outbox) = crate::play::play_channel();
        outbox.vip_presence_initialize(true, true);
        outbox.push(vec![online(1, 1, false)]);
        let panel = link.vip_presence_panel();
        assert_eq!(panel.presence.entries[0].channel, Some([1; 16]));
        lock(&outbox.0).vip_presence_confirmed_at =
            tokio::time::Instant::now().checked_sub(std::time::Duration::from_secs(10));
        let panel = link.vip_presence_panel();
        assert_eq!(panel.presence.entries[0].status, VipPresenceStatus::Unconfirmed);
        assert_eq!(panel.presence.entries[0].channel, None);
    }

    #[test]
    fn channel_notice_uses_the_full_list_even_when_the_contact_is_off_page() {
        let (_link, _commands, outbox) = crate::play::play_channel();
        outbox.vip_presence_initialize(true, true);
        {
            let mut mailbox = lock(&outbox.0);
            mailbox.vip.view = Some(r#"{"entries":[{"entry":1,"name":"Alice","notify":true}]}"#.into());
            mailbox.party.channel_names.insert([2; 16], "B".into());
        }
        outbox.push(vec![online(26, 1, false)]);
        outbox.push(vec![online(26, 2, true)]);
        let mailbox = lock(&outbox.0);
        assert_eq!(mailbox.vip_channel_notices, vec![(1, "Bryn has changed to channel B.".into())]);
    }

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
