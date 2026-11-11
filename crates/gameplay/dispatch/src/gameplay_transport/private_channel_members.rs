//! Capabilities 494 and 495: the list extension and the member notices of the private channels a
//! session is in. Pure functions of two consecutive durable member reads; the reads themselves
//! (`read_private_channel_members`) prove the session fence and hold the World lock.
use crate::durability::private_channel::{ChannelMember, JoinedChannel};
use oteryn_protocol_oteryn::chat::line_extended::{ChannelNotice, ChannelNoticeKind};
use oteryn_protocol_oteryn::chat::private_channel::ChatChannelList;
use oteryn_protocol_oteryn::chat::private_channel_extended::{
    ChannelListExtension, ChannelParticipants, MAX_JOINED_PRIVATE_CHANNELS,
    MAX_PRIVATE_CHANNEL_PARTICIPANTS,
};

fn listed(list: &ChatChannelList, channel_id: u16) -> bool {
    list.own_channel == Some(channel_id)
        || list
            .invited
            .iter()
            .any(|invited| invited.channel_id == channel_id)
}

/// Fields 4 (capability 494) and 5 (capability 495) of the list: the listed channels the session
/// is in, ascending, and the joined members of each, in the read's name order.
pub(super) fn extension(
    list: &ChatChannelList,
    joined: &[JoinedChannel],
    members: &[ChannelMember],
    joined_selected: bool,
    participants_selected: bool,
) -> ChannelListExtension {
    let mut extension = ChannelListExtension::default();
    if joined_selected {
        let mut ids: Vec<u16> = joined
            .iter()
            .map(|interest| interest.channel_id)
            .filter(|channel_id| listed(list, *channel_id))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids.truncate(MAX_JOINED_PRIVATE_CHANNELS);
        extension.joined = ids;
    }
    if participants_selected {
        for member in members
            .iter()
            .filter(|member| member.joined && listed(list, member.channel_id))
        {
            match extension.participants.last_mut() {
                Some(entry) if entry.channel_id == member.channel_id => {
                    if entry.names.len() < MAX_PRIVATE_CHANNEL_PARTICIPANTS {
                        entry.names.push(member.name.clone());
                    }
                }
                Some(entry) if entry.channel_id > member.channel_id => {}
                _ if extension.participants.len() < MAX_JOINED_PRIVATE_CHANNELS => {
                    extension.participants.push(ChannelParticipants {
                        channel_id: member.channel_id,
                        names: vec![member.name.clone()],
                    });
                }
                _ => {}
            }
        }
    }
    extension
}

/// The sessions of the other joined members `before` shows: the read reports which of them are
/// still participants, so a removed row tells an exclusion from a logout.
pub(super) fn probe(viewer: [u8; 16], before: &[ChannelMember]) -> Vec<[u8; 16]> {
    let mut sessions: Vec<[u8; 16]> = before
        .iter()
        .filter(|member| member.joined && member.session != viewer)
        .map(|member| member.session)
        .collect();
    sessions.sort_unstable();
    sessions.dedup();
    sessions
}

/// The notices between two reads, for the channels (and epochs) the viewer is in at both: each
/// other member that joined, left (or whose session ended or dropped capability 80), or was
/// excluded (its membership row is gone while its session still participates). Per channel,
/// ascending: departures in the earlier read's order, then arrivals in the later read's order.
pub(super) fn notices(
    viewer: [u8; 16],
    before: &[ChannelMember],
    after: &[ChannelMember],
    active: &[[u8; 16]],
) -> Vec<ChannelNotice> {
    if before == after {
        return Vec::new();
    }
    let inside = |rows: &[ChannelMember], channel_id: u16, epoch: u64| {
        rows.iter().any(|row| {
            row.session == viewer
                && row.channel_id == channel_id
                && row.epoch == epoch
                && row.joined
        })
    };
    let find = |rows: &[ChannelMember], member: &ChannelMember| {
        rows.iter()
            .find(|row| {
                row.channel_id == member.channel_id
                    && row.epoch == member.epoch
                    && row.session == member.session
            })
            .map(|row| row.joined)
    };
    let mut channels: Vec<(u16, u64)> = after
        .iter()
        .filter(|row| row.session == viewer && row.joined)
        .map(|row| (row.channel_id, row.epoch))
        .filter(|(channel_id, epoch)| inside(before, *channel_id, *epoch))
        .collect();
    channels.sort_unstable();
    channels.dedup();
    let mut out = Vec::new();
    for (channel_id, epoch) in channels {
        let other = |row: &ChannelMember| {
            row.channel_id == channel_id
                && row.epoch == epoch
                && row.session != viewer
                && row.joined
        };
        for member in before.iter().filter(|row| other(*row)) {
            let kind = match find(after, member) {
                Some(true) => continue,
                Some(false) => ChannelNoticeKind::Left,
                None if active.contains(&member.session) => ChannelNoticeKind::Excluded,
                None => ChannelNoticeKind::Left,
            };
            out.push(ChannelNotice {
                channel_id,
                kind,
                name: member.name.clone(),
            });
        }
        for member in after.iter().filter(|row| other(*row)) {
            if find(before, member) != Some(true) {
                out.push(ChannelNotice {
                    channel_id,
                    kind: ChannelNoticeKind::Joined,
                    name: member.name.clone(),
                });
            }
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::chat::ChatRoomSet;
    use oteryn_protocol_oteryn::chat::private_channel::InvitedChannel;

    const VIEWER: [u8; 16] = [1; 16];
    const BRYN: [u8; 16] = [2; 16];
    const CORA: [u8; 16] = [3; 16];

    fn member(session: [u8; 16], name: &str, joined: bool) -> ChannelMember {
        ChannelMember {
            channel_id: 100,
            epoch: 7,
            session,
            joined,
            name: name.into(),
        }
    }

    fn notice(kind: ChannelNoticeKind, name: &str) -> ChannelNotice {
        ChannelNotice {
            channel_id: 100,
            kind,
            name: name.into(),
        }
    }

    #[test]
    fn a_join_a_leave_and_an_exclusion_are_told_apart() {
        let before = vec![
            member(VIEWER, "Aldric", true),
            member(BRYN, "Bryn", true),
            member(CORA, "Cora", false),
        ];
        // Cora joins; Bryn leaves (its row stays, not joined).
        let after = vec![
            member(VIEWER, "Aldric", true),
            member(BRYN, "Bryn", false),
            member(CORA, "Cora", true),
        ];
        assert_eq!(
            notices(VIEWER, &before, &after, &[BRYN]),
            vec![
                notice(ChannelNoticeKind::Left, "Bryn"),
                notice(ChannelNoticeKind::Joined, "Cora"),
            ]
        );
        // Cora's row is gone while its session still participates: the owner excluded it.
        let excluded = vec![member(VIEWER, "Aldric", true), member(BRYN, "Bryn", false)];
        assert_eq!(
            notices(VIEWER, &after, &excluded, &probe(VIEWER, &after)),
            vec![notice(ChannelNoticeKind::Excluded, "Cora")]
        );
        // Gone with its session (logout, capability 80 dropped): it left.
        assert_eq!(
            notices(VIEWER, &after, &excluded, &[]),
            vec![notice(ChannelNoticeKind::Left, "Cora")]
        );
        // An unchanged read, the first read and the viewer's own changes notify nothing.
        assert!(notices(VIEWER, &after, &after, &[]).is_empty());
        let outside = vec![member(BRYN, "Bryn", true)];
        assert!(notices(VIEWER, &outside, &after, &[]).is_empty());
        assert!(notices(VIEWER, &after, &outside, &[]).is_empty());
        // A channel recreated under the same id is another epoch: nothing carries over.
        let recreated: Vec<ChannelMember> = after
            .iter()
            .cloned()
            .map(|mut row| {
                row.epoch = 8;
                row
            })
            .collect();
        assert!(notices(VIEWER, &before, &recreated, &[]).is_empty());
    }

    #[test]
    fn the_extension_lists_the_joined_channels_and_their_joined_members() {
        let list = ChatChannelList {
            public_rooms: ChatRoomSet::default(),
            own_channel: Some(100),
            invited: vec![InvitedChannel {
                channel_id: 200,
                owner_name: "Cora".into(),
            }],
        };
        let joined = [
            JoinedChannel {
                channel_id: 200,
                epoch: 1,
                since_micros: 5,
            },
            JoinedChannel {
                channel_id: 100,
                epoch: 7,
                since_micros: 5,
            },
        ];
        let mut members = vec![
            member(VIEWER, "Aldric", true),
            member(BRYN, "Bryn", false),
            member(CORA, "Cora", true),
        ];
        members.push(ChannelMember {
            channel_id: 200,
            epoch: 1,
            session: VIEWER,
            joined: true,
            name: "Aldric".into(),
        });
        let shown = extension(&list, &joined, &members, true, true);
        assert_eq!(shown.joined, vec![100, 200]);
        assert_eq!(
            shown.participants,
            vec![
                ChannelParticipants {
                    channel_id: 100,
                    names: vec!["Aldric".into(), "Cora".into()],
                },
                ChannelParticipants {
                    channel_id: 200,
                    names: vec!["Aldric".into()],
                },
            ]
        );
        assert_eq!(
            extension(&list, &joined, &members, false, false),
            ChannelListExtension::default()
        );
        assert!(
            extension(&list, &joined, &members, true, false)
                .participants
                .is_empty()
        );
        assert!(
            oteryn_protocol_oteryn::chat::private_channel_extended::encode_channel_list_extended(
                &list, &shown
            )
            .is_ok()
        );
    }
}
