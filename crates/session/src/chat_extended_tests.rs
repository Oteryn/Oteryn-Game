use super::*;
use oteryn_protocol_oteryn::chat::line_extended::{
    PrivateEcho, append_line_sequence, encode_channel_notice, encode_private_echo,
};
use oteryn_protocol_oteryn::chat::private_channel_extended::encode_channel_list_extended;
use oteryn_protocol_oteryn::chat::{ChatRoomSet, encode_chat_line};

type Check = Result<(), String>;

fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, String> {
    result.map_err(|error| format!("{error:?}"))
}

fn echo_bytes() -> Result<Vec<u8>, String> {
    ok(encode_private_echo(&PrivateEcho {
        speaker_name: "Ari".into(),
        recipient_name: "Bo".into(),
        text: "hi".into(),
    }))
}

#[test]
fn c0493_echo_decodes_with_its_recipient_only_when_selected() -> Check {
    let bytes = echo_bytes()?;
    let Decoded::Line { line, extra } = ok(decode_line(&bytes, &[7, 493]))? else {
        return Err("a line".into());
    };
    assert_eq!(
        line,
        ChatLine::Private {
            speaker_name: "Ari".into(),
            text: "hi".into()
        }
    );
    assert_eq!(extra.recipient_name.as_deref(), Some("Bo"));
    // Another extension selected: the echo field is still refused without 493.
    assert!(decode_line(&bytes, &[7, 496]).is_err());
    // No extension selected: the legacy decoder refuses field 12 as before.
    assert!(decode_line(&bytes, &[7]).is_err());
    Ok(())
}

#[test]
fn c0496_sequence_is_taken_only_when_selected_and_legacy_lines_stay() -> Check {
    let mut bytes = ok(encode_chat_line(&ChatLine::Private {
        speaker_name: "Bo".into(),
        text: "yo".into(),
    }))?;
    let legacy = ok(decode_line(&bytes, &[7]))?;
    assert!(matches!(legacy, Decoded::Line { ref extra, .. } if *extra == LineExtra::default()));
    ok(append_line_sequence(&mut bytes, 9))?;
    let Decoded::Line { extra, .. } = ok(decode_line(&bytes, &[7, 496]))? else {
        return Err("a line".into());
    };
    assert_eq!(extra.line_sequence, Some(9));
    assert!(decode_line(&bytes, &[7, 493]).is_err());
    assert!(decode_line(&bytes, &[7]).is_err());
    Ok(())
}

#[test]
fn c0495_notice_needs_its_capability() -> Check {
    let notice = ChannelNotice {
        channel_id: 120,
        kind: ChannelNoticeKind::Excluded,
        name: "Bo".into(),
    };
    let bytes = ok(encode_channel_notice(&notice))?;
    assert!(
        matches!(ok(decode_line(&bytes, &[7, 80, 495]))?, Decoded::Notice(got) if got == notice)
    );
    assert!(decode_line(&bytes, &[7, 80, 493]).is_err());
    let mut log = crate::ChatLog::default();
    log.push_notice(notice.clone());
    log.push_notice(notice);
    let serials: Vec<u64> = log.channel_notices().map(|entry| entry.serial).collect();
    assert_eq!(serials, [1, 2]);
    Ok(())
}

#[test]
fn c0494_c0495_list_extension_is_taken_only_when_selected() -> Check {
    let mut rooms = ChatRoomSet::default();
    rooms.insert(crate::ChatRoom::World);
    let list = ChatChannelList {
        public_rooms: rooms,
        own_channel: Some(120),
        invited: Vec::new(),
    };
    let extension = ChannelListExtension {
        joined: vec![120],
        participants: vec![ChannelParticipants {
            channel_id: 120,
            names: vec!["Ari".into(), "Bo".into()],
        }],
    };
    let bytes = ok(encode_channel_list_extended(&list, &extension))?;
    let (got, ext) = ok(decode_list(&bytes, &[7, 80, 494, 495]))?;
    assert_eq!((got, ext), (list.clone(), Some(extension)));
    // Joined only: participants arriving unselected is a protocol fault.
    assert!(decode_list(&bytes, &[7, 80, 494]).is_err());
    // No extension selected: legacy decoder, fields 4 and 5 refused.
    assert!(decode_list(&bytes, &[7, 80]).is_err());
    let legacy = ok(encode_channel_list_extended(
        &list,
        &ChannelListExtension::default(),
    ))?;
    assert_eq!(ok(decode_list(&legacy, &[7, 80]))?, (list, None));
    Ok(())
}

#[test]
fn c0493_selection_requires_chat_and_private_channels() {
    assert!(guild_chat::check_selection(&[493]).is_err());
    assert!(guild_chat::check_selection(&[7, 494]).is_err());
    assert!(guild_chat::check_selection(&[7, 80, 494, 495, 496, 493]).is_ok());
}
