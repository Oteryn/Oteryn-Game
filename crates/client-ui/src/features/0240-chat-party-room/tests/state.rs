use super::*;
use oteryn_client_runtime::chat_public_channels::ChatIntent;
use oteryn_client_runtime::cl25a_protocol::ChatLine;

#[test]
fn pushed_party_line_is_visible_and_send_builds_party_intent() -> Result<(), String> {
    let mut panel = ChatPartyRoomPanel {
        selected: true,
        text: "meet at the depot".into(),
        ..Default::default()
    };
    push_line(
        &mut panel,
        ChatLine::Party {
            speaker_name: "Leader".into(),
            text: "ready".into(),
        },
    );
    assert!(panel_text(&panel).contains("Leader: ready"));
    let intent = request(&mut panel);
    assert_eq!(
        intent,
        Some(ChatIntent::Party {
            text: "meet at the depot".into()
        })
    );
    assert_eq!(
        oteryn_client_runtime::cl25a_protocol::encode_chat_intent(intent.as_ref().ok_or("intent")?)
            .map_err(|error| format!("{error:?}"))?,
        [
            114, 19, 10, 17, b'm', b'e', b'e', b't', b' ', b'a', b't', b' ', b't', b'h', b'e',
            b' ', b'd', b'e', b'p', b'o', b't'
        ]
    );
    Ok(())
}
