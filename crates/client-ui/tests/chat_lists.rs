use oteryn_client_runtime::chat::ChatLine;
use oteryn_client_ui::chat::{ChatPanel, CommunicationList, panel_text};
#[test]
fn communication_lists_filter_received_messages_and_whitelist_overrides() {
    let mut panel = ChatPanel {
        lines: vec![ChatLine::Private {
            speaker_name: "Alice".into(),
            text: "hello".into(),
        }],
        ..Default::default()
    };
    assert!(
        panel
            .console
            .communication
            .edit(CommunicationList::Ignore, " Alice ", true)
    );
    assert!(!panel_text(&panel).contains("hello"));
    assert!(
        panel
            .console
            .communication
            .edit(CommunicationList::Whitelist, "ALICE", true)
    );
    assert!(panel_text(&panel).contains("hello"));
    assert!(
        panel
            .console
            .communication
            .edit(CommunicationList::Whitelist, "alice", false)
    );
    assert!(!panel_text(&panel).contains("hello"));
    assert!(
        !panel
            .console
            .communication
            .edit(CommunicationList::Blacklist, "", true)
    );
    assert!(
        !panel
            .console
            .communication
            .edit(CommunicationList::Blacklist, "a\0b", true)
    );
    assert_eq!(panel.lines.len(), 1);
}
