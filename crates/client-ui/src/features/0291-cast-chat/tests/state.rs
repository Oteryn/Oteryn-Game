use super::*;
use oteryn_client_runtime::{CastCasterLine, cast::Message};
#[test]
fn s17c_live_text_and_revision_fenced_moderation_are_native_values() -> Result<(), &'static str> {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    let mut control = crate::cast_control::Panel {
        selected: true,
        ..Default::default()
    };
    control.apply(Message::CasterState {
        cast_id: [1; 16],
        revision: 7,
        epoch: 1,
        delay: 300,
        count: 1,
    })?;
    protocol::append(
        &mut panel,
        CastCasterLine {
            handle: [3; 16],
            text: "Viewer-1: hello now".into(),
        },
    );
    assert!(panel_text(&panel).contains("hello now"));
    protocol::roster(
        &mut panel,
        oteryn_client_runtime::CastRoster {
            revision: 7,
            subjects: vec![[3; 16]],
        },
    );
    for action in 1..=4 {
        let request = protocol::command(&panel, &control, "", &"03".repeat(16), action, [9; 16])?;
        assert_eq!((request.capability, request.command_type), (290, 505));
        assert_eq!(
            Message::decode(&request.payload).map_err(|_| "decode")?,
            Message::Moderate {
                revision: 7,
                subject: [3; 16],
                action,
                key: [9; 16]
            }
        );
    }
    assert!(protocol::command(&panel, &control, "", &"04".repeat(16), 3, [9; 16]).is_err());
    let reply = protocol::command(&panel, &control, "Hello viewers", "", 0, [0; 16])?;
    assert_eq!(
        Message::decode(&reply.payload).map_err(|_| "reply")?,
        Message::Reply("Hello viewers".into())
    );
    assert_eq!(
        panel.lines.len(),
        1,
        "replies are shown only when confirmed by the live stream"
    );
    Ok(())
}
