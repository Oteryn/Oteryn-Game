use oteryn_client_runtime::chat::{ActorPosition, ChatLine, ChatSpeaker, ChatSpeechMode};

#[test]
fn scripted_item_line_uses_existing_native_chat_panel() {
    let line = ChatLine::Local {
        speaker: ChatSpeaker {
            identity: [9; 16],
            generation: std::num::NonZeroU64::MIN,
        },
        speaker_name: "Citizen".into(),
        mode: ChatSpeechMode::MonsterSay,
        text: "Mmmm!".into(),
        position: ActorPosition {
            x: 2,
            y: 1,
            floor: 7,
        },
    };
    assert_eq!(
        super::line_text(&line).as_deref(),
        Some("[MonsterSay] Citizen: Mmmm!\n")
    );
    let panel = crate::chat::ChatPanel {
        lines: vec![line.clone()],
        ..Default::default()
    };
    assert!(crate::chat::panel_text(&panel).contains("[MonsterSay] Citizen: Mmmm!\n"));
    let mut ordinary = line;
    if let ChatLine::Local { mode, .. } = &mut ordinary {
        *mode = ChatSpeechMode::Say;
    }
    assert_eq!(super::line_text(&ordinary), None);
    let panel = crate::chat::ChatPanel {
        lines: vec![ordinary],
        ..Default::default()
    };
    assert!(crate::chat::panel_text(&panel).contains("[Say] Citizen: Mmmm!\n"));
}
