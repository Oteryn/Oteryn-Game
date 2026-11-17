use super::*;
use oteryn_client_runtime::cl25a_protocol::{
    VipPresenceBatch, VipPresenceEntry, VipPresenceStatus,
};
use oteryn_client_runtime::party::party::PartyEntry;

const HERE: [u8; 16] = [1; 16];
const THERE: [u8; 16] = [2; 16];

fn names() -> ChannelNames {
    ChannelNames::from([(HERE, "Aurelia-1".into()), (THERE, "Aurelia-3".into())])
}

fn vip(status: VipPresenceStatus, channel: Option<[u8; 16]>, announce: bool) -> VipPresenceEntry {
    VipPresenceEntry {
        channel,
        entry: 1,
        name: "Bryn".into(),
        status,
        announce,
        confirmed_for_ms: if status == VipPresenceStatus::Online {
            10_000
        } else {
            0
        },
    }
}

fn batch(entry: VipPresenceEntry) -> VipPresenceBatch {
    VipPresenceBatch {
        entries: vec![entry],
    }
}

#[test]
fn c0513_rows_show_the_channel_and_notices_name_it() {
    let names = names();
    let member = |channel| PartyEntry {
        channel,
        list: Vec::new(),
        handle: 1,
        name: "Bryn".into(),
        leader: false,
        expires_at: 0,
        party_spell_remaining_ms: None,
    };
    assert_eq!(
        party_member_text(&names, &member(Some(THERE))),
        "Bryn — Aurelia-3"
    );
    assert!(party_member_on_other_channel(
        Some(HERE),
        &member(Some(THERE))
    ));
    assert!(!party_member_on_other_channel(
        Some(HERE),
        &member(Some(HERE))
    ));
    // Hidden or offline members and peers without 513 are never greyed by Channel.
    assert!(!party_member_on_other_channel(Some(HERE), &member(None)));
    assert_eq!(party_member_text(&names, &member(None)), "Bryn — —");

    let online = vip(VipPresenceStatus::Online, Some(THERE), false);
    assert_eq!(vip_entry_text(&names, &online), "Bryn — Online — Aurelia-3");
    assert_eq!(
        vip_entry_text(&names, &vip(VipPresenceStatus::Offline, None, false)),
        "Bryn — Offline"
    );

    let offline = batch(vip(VipPresenceStatus::Offline, None, false));
    assert_eq!(
        presence_notices(
            &names,
            &offline,
            &batch(vip(VipPresenceStatus::Online, Some(HERE), true))
        ),
        ["Bryn has logged in on channel Aurelia-1."]
    );
    // A switch seen as one correction: Online on another Channel.
    assert_eq!(
        presence_notices(
            &names,
            &batch(vip(VipPresenceStatus::Online, Some(HERE), false)),
            &batch(online.clone())
        ),
        ["Bryn has changed to channel Aurelia-3."]
    );
    assert_eq!(
        presence_notices(
            &names,
            &batch(online.clone()),
            &batch(vip(VipPresenceStatus::Offline, None, true))
        ),
        ["Bryn has logged out of channel Aurelia-3."]
    );
    assert!(presence_notices(&names, &batch(online.clone()), &batch(online)).is_empty());
}
