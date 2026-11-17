use crate::chat::{ChannelTab, ChatExtras, ChatPanel, panel_text};
use oteryn_client_runtime::chat::ChatLine;

fn private(speaker: &str, text: &str) -> ChatLine {
    ChatLine::Private {
        speaker_name: speaker.into(),
        text: text.into(),
    }
}

fn panel_in(peer: &str, lines: Vec<ChatLine>) -> ChatPanel {
    let mut panel = ChatPanel {
        lines,
        open: true,
        ..Default::default()
    };
    panel
        .console
        .open_channel(ChannelTab::Conversation(peer.into()));
    panel
}

#[test]
fn c0493_own_echo_shows_in_the_recipients_conversation_tab() {
    let lines = vec![private("Bo", "hello"), private("Ari", "hi Bo")];
    let mut panel = panel_in("Bo", lines);
    let extras = ChatExtras {
        recipients: Some(vec![None, Some("Bo".into())]),
        ..Default::default()
    };
    panel.apply_extras("Ari", None, extras);
    let text = panel_text(&panel);
    assert!(text.contains("Bo: hello"), "{text}");
    assert!(text.contains("To Bo: hi Bo"), "{text}");
    // The echo belongs to the recipient's tab only.
    let mut other = panel_in("Cy", panel.lines.clone());
    other.recipients = panel.recipients.clone();
    assert!(!panel_text(&other).contains("hi Bo"));
}

#[test]
fn c0493_without_the_capability_own_lines_are_not_shown() {
    let lines = vec![private("Bo", "hello"), private("Ari", "hi Bo")];
    let mut panel = panel_in("Bo", lines);
    panel.apply_extras("Ari", None, ChatExtras::default());
    assert!(panel.recipients.is_empty());
    let text = panel_text(&panel);
    assert!(text.contains("Bo: hello"), "{text}");
    assert!(!text.contains("hi Bo"), "{text}");
}

#[test]
fn c0493_echo_reads_as_sent_in_the_private_tab_too() {
    let mut panel = ChatPanel {
        lines: vec![private("Ari", "hi Bo")],
        open: true,
        ..Default::default()
    };
    panel.console.select(crate::chat::ConsoleTab::Private);
    panel.apply_extras(
        "Ari",
        None,
        ChatExtras {
            recipients: Some(vec![Some("Bo".into())]),
            ..Default::default()
        },
    );
    assert!(panel_text(&panel).contains("To Bo: hi Bo"));
}
