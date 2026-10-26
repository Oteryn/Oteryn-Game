#![allow(clippy::expect_used, clippy::panic, reason = "test fixtures")]
use super::*;

const BASE: SemanticTimeMicros = SemanticTimeMicros::from_micros(1_000_000);

fn session_of(tag: u8) -> GameSessionId {
    GameSessionId::decode(&[
        0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
    ])
    .expect("session")
}

/// A Channel whose sessions `names` are admitted with capability 80.
fn channel(names: &[&str]) -> (ChatRuntime, Vec<GameSessionId>) {
    let chat = ChatRuntime::default();
    let sessions: Vec<_> = (0..names.len())
        .map(|index| session_of(0x10 + u8::try_from(index).expect("few")))
        .collect();
    for (session, name) in sessions.iter().zip(names) {
        chat.admit(*session, name, BASE);
        chat.set_private_channels(*session, true);
    }
    (chat, sessions)
}

fn intent(
    chat: &ChatRuntime,
    session: GameSessionId,
    intent: PrivateChannelIntent,
) -> ChatDisposition {
    // The connection path: every intent but a create is answered by the gate.
    chat.pre_gate(session, &ChatIntent::PrivateChannel(intent), BASE)
        .expect("answered by the gate")
        .disposition
}

fn invite(chat: &ChatRuntime, owner: GameSessionId, name: &str) -> ChatDisposition {
    intent(
        chat,
        owner,
        PrivateChannelIntent::Invite { name: name.into() },
    )
}

fn say(chat: &ChatRuntime, session: GameSessionId, channel_id: u16) -> ChatDisposition {
    intent(
        chat,
        session,
        PrivateChannelIntent::Say {
            channel_id,
            text: " hello ".into(),
        },
    )
}

fn heard(chat: &ChatRuntime, session: GameSessionId) -> Vec<(u16, String, String)> {
    chat.drain(session)
        .into_iter()
        .map(|line| match line {
            ChatLine::PrivateChannel(line) => (line.channel_id, line.speaker_name, line.text),
            other => panic!("unexpected line {other:?}"),
        })
        .collect()
}

#[test]
fn a_private_channel_lives_from_create_to_its_owner_leaving() {
    let (chat, s) = channel(&["Aela", "Bryn", "Cato"]);
    let (aela, bryn, cato) = (s[0], s[1], s[2]);
    // Negotiation owes every session its (empty) list once.
    for session in [aela, bryn, cato] {
        assert_eq!(
            chat.take_channel_list(session),
            Some(ChatChannelList::default())
        );
        assert_eq!(chat.take_channel_list(session), None);
    }
    assert!(
        chat.pre_gate(
            aela,
            &ChatIntent::PrivateChannel(PrivateChannelIntent::Create),
            BASE
        )
        .is_none()
    );
    assert_eq!(
        chat.create_private_channel(bryn, false).disposition,
        ChatDisposition::NotAllowed
    );
    assert_eq!(
        chat.create_private_channel(aela, true).disposition,
        ChatDisposition::Ok
    );
    assert_eq!(
        chat.create_private_channel(aela, true).disposition,
        ChatDisposition::Rejected
    );
    assert_eq!(
        chat.take_channel_list(aela).map(|list| list.own_channel),
        Some(Some(1))
    );
    assert_eq!(chat.take_channel_list(bryn), None);

    // Not invited: no join and no line.
    let join = PrivateChannelIntent::Join { channel_id: 1 };
    assert_eq!(
        intent(&chat, bryn, join.clone()),
        ChatDisposition::NotInvited
    );
    assert_eq!(say(&chat, bryn, 1), ChatDisposition::NotInvited);
    assert_eq!(say(&chat, bryn, 2), ChatDisposition::NotInvited);

    // The name matches by its key; the invitee is told.
    assert_eq!(invite(&chat, aela, "bRyN"), ChatDisposition::Ok);
    assert_eq!(invite(&chat, aela, "Bryn"), ChatDisposition::Ok);
    assert_eq!(
        chat.take_channel_list(bryn).map(|list| list.invited),
        Some(vec![InvitedChannel {
            channel_id: 1,
            owner_name: "Aela".into()
        }])
    );
    assert_eq!(say(&chat, bryn, 1), ChatDisposition::RoomNotOpen);
    assert_eq!(intent(&chat, bryn, join.clone()), ChatDisposition::Ok);
    assert_eq!(say(&chat, bryn, 1), ChatDisposition::Ok);
    let line = vec![(1, "Bryn".to_owned(), "hello".to_owned())];
    assert_eq!(heard(&chat, aela), line);
    assert_eq!(heard(&chat, bryn), line);
    assert!(heard(&chat, cato).is_empty());
    assert_eq!(
        intent(&chat, cato, join.clone()),
        ChatDisposition::NotInvited
    );

    // Exclusion withdraws the invitation and the membership.
    let exclude = |name: &str| {
        intent(
            &chat,
            aela,
            PrivateChannelIntent::Exclude { name: name.into() },
        )
    };
    assert_eq!(exclude("Cato"), ChatDisposition::NotInvited);
    assert_eq!(exclude("Bryn"), ChatDisposition::Ok);
    assert_eq!(say(&chat, bryn, 1), ChatDisposition::NotInvited);
    assert_eq!(
        chat.take_channel_list(bryn).map(|list| list.invited.len()),
        Some(0)
    );
    assert_eq!(say(&chat, aela, 1), ChatDisposition::Ok);
    assert!(heard(&chat, bryn).is_empty());
    assert_eq!(heard(&chat, aela).len(), 1);

    // A member leaving stays invited; the owner leaving closes the channel.
    assert_eq!(invite(&chat, aela, "Bryn"), ChatDisposition::Ok);
    assert_eq!(intent(&chat, bryn, join.clone()), ChatDisposition::Ok);
    let leave = PrivateChannelIntent::Leave { channel_id: 1 };
    assert_eq!(intent(&chat, bryn, leave.clone()), ChatDisposition::Ok);
    assert_eq!(say(&chat, bryn, 1), ChatDisposition::RoomNotOpen);
    assert_eq!(intent(&chat, bryn, join), ChatDisposition::Ok);
    let _ = chat.take_channel_list(bryn);
    assert_eq!(intent(&chat, aela, leave), ChatDisposition::Ok);
    assert_eq!(say(&chat, bryn, 1), ChatDisposition::NotInvited);
    assert_eq!(
        chat.take_channel_list(bryn),
        Some(ChatChannelList::default())
    );
    assert_eq!(
        chat.take_channel_list(aela).map(|list| list.own_channel),
        Some(None)
    );
    assert_eq!(
        intent(&chat, aela, PrivateChannelIntent::List),
        ChatDisposition::Ok
    );
    assert!(chat.take_channel_list(aela).is_some());
}

#[test]
fn invitations_need_an_own_channel_and_an_online_other_participant() {
    let (chat, s) = channel(&["Aela", "Bryn"]);
    let (aela, bryn) = (s[0], s[1]);
    let outsider = session_of(0x7f);
    chat.admit(outsider, "Dara", BASE);
    assert_eq!(invite(&chat, aela, "Bryn"), ChatDisposition::NotAllowed);
    assert_eq!(
        intent(
            &chat,
            aela,
            PrivateChannelIntent::Exclude {
                name: "Bryn".into()
            }
        ),
        ChatDisposition::NotAllowed
    );
    assert_eq!(
        chat.create_private_channel(aela, true).disposition,
        ChatDisposition::Ok
    );
    // A name of no session of this Channel, or no name at all, is no online character.
    assert_eq!(invite(&chat, aela, "Nobody"), ChatDisposition::NotOnline);
    assert_eq!(invite(&chat, aela, "B1"), ChatDisposition::NotOnline);
    assert_eq!(invite(&chat, aela, "Aela"), ChatDisposition::Rejected);
    // A session without capability 80 can neither be invited nor act.
    assert_eq!(invite(&chat, aela, "Dara"), ChatDisposition::Rejected);
    assert_eq!(
        intent(
            &chat,
            outsider,
            PrivateChannelIntent::Join { channel_id: 1 }
        ),
        ChatDisposition::Rejected
    );
    assert_eq!(
        chat.create_private_channel(outsider, true).disposition,
        ChatDisposition::Rejected
    );
    assert_eq!(chat.take_channel_list(outsider), None);
    // Unusable text takes no line and no limiter step.
    assert_eq!(invite(&chat, aela, "Bryn"), ChatDisposition::Ok);
    assert_eq!(
        intent(
            &chat,
            bryn,
            PrivateChannelIntent::Say {
                channel_id: 1,
                text: "\u{7}".into()
            }
        ),
        ChatDisposition::Rejected
    );
}

#[test]
fn private_channel_invitations_are_bounded() {
    const NAMES: [&str; 19] = [
        "Aa", "Bb", "Cc", "Dd", "Ee", "Ff", "Gg", "Hh", "Ii", "Jj", "Kk", "Ll", "Mm", "Nn", "Oo",
        "Pp", "Qq", "Rr", "Ss",
    ];
    let (chat, s) = channel(&NAMES);
    // CHATPM-RL-01: one channel invites at most 16.
    assert_eq!(
        chat.create_private_channel(s[0], true).disposition,
        ChatDisposition::Ok
    );
    for name in &NAMES[1..=MAX_PRIVATE_CHANNEL_INVITES] {
        assert_eq!(invite(&chat, s[0], name), ChatDisposition::Ok);
    }
    assert_eq!(invite(&chat, s[0], NAMES[17]), ChatDisposition::Rejected);
    // CHATPM-RL-02: one character holds at most 16 invitations (the first channel is full).
    let target = NAMES[18];
    for owner in &s[1..=MAX_INVITED_PRIVATE_CHANNELS] {
        let _ = chat.create_private_channel(*owner, true);
        assert_eq!(invite(&chat, *owner, target), ChatDisposition::Ok);
    }
    let last = s[MAX_INVITED_PRIVATE_CHANNELS + 1];
    assert_eq!(
        chat.create_private_channel(last, true).disposition,
        ChatDisposition::Ok
    );
    assert_eq!(invite(&chat, last, target), ChatDisposition::Rejected);
    let list = chat.take_channel_list(s[18]).expect("invited");
    assert_eq!(list.invited.len(), MAX_INVITED_PRIVATE_CHANNELS);
    assert!(oteryn_protocol_oteryn::chat::private_channel::encode_channel_list(&list).is_ok());
}

#[test]
fn a_flood_on_a_private_channel_is_limited() {
    let (chat, s) = channel(&["Aela"]);
    let _ = chat.create_private_channel(s[0], true);
    let mut refused = None;
    for _ in 0..64 {
        let disposition = say(&chat, s[0], 1);
        if disposition != ChatDisposition::Ok {
            refused = Some(disposition);
            break;
        }
    }
    assert!(matches!(
        refused,
        Some(ChatDisposition::Exhausted | ChatDisposition::Muted)
    ));
}

#[test]
fn an_ended_session_closes_its_channel_and_leaves_the_others() {
    let (chat, s) = channel(&["Aela", "Bryn", "Cato"]);
    let (aela, bryn, cato) = (s[0], s[1], s[2]);
    let _ = chat.create_private_channel(aela, true);
    let _ = chat.create_private_channel(cato, true);
    assert_eq!(invite(&chat, aela, "Bryn"), ChatDisposition::Ok);
    assert_eq!(invite(&chat, cato, "Bryn"), ChatDisposition::Ok);
    for session in [aela, bryn, cato] {
        let _ = chat.take_channel_list(session);
    }
    chat.forget(aela);
    assert_eq!(
        chat.take_channel_list(bryn).map(|list| list
            .invited
            .iter()
            .map(|channel| channel.channel_id)
            .collect::<Vec<_>>()),
        Some(vec![2])
    );
    // Its number is free again, and nothing of it is left.
    assert_eq!(
        chat.create_private_channel(bryn, true).disposition,
        ChatDisposition::Ok
    );
    assert_eq!(
        chat.take_channel_list(bryn).map(|list| list.own_channel),
        Some(Some(1))
    );
    // A renegotiation without capability 80 leaves every channel, as an ended session does.
    chat.set_private_channels(bryn, false);
    assert_eq!(chat.take_channel_list(bryn), None);
    assert_eq!(invite(&chat, cato, "Bryn"), ChatDisposition::Rejected);
    assert_eq!(
        intent(&chat, cato, PrivateChannelIntent::Join { channel_id: 1 }),
        ChatDisposition::NotInvited
    );
}
