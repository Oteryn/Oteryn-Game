use crate::chat::{ChannelTab, ChatExtras, ChatPanel};
use oteryn_client_runtime::chat::ChatLine;

fn private(speaker: &str) -> ChatLine {
    ChatLine::Private {
        speaker_name: speaker.into(),
        text: "psst".into(),
    }
}

fn sequences(values: &[u64]) -> ChatExtras {
    ChatExtras {
        sequences: Some(values.iter().map(|value| Some(*value)).collect()),
        ..Default::default()
    }
}

#[test]
fn c0496_an_incoming_private_message_opens_its_tab_once() {
    let mut panel = ChatPanel {
        lines: vec![private("Bo")],
        ..Default::default()
    };
    panel.apply_extras("Ari", None, sequences(&[7]));
    assert_eq!(
        panel.console.channels,
        [ChannelTab::Conversation("Bo".into())]
    );
    // The player closes the tab: the same line does not reopen it.
    panel
        .console
        .drop_channel(&ChannelTab::Conversation("Bo".into()));
    panel.apply_extras("Ari", None, sequences(&[7]));
    assert!(panel.console.channels.is_empty());
    // A newer line from the same sender opens it again.
    panel.lines.push(private("bo"));
    panel.apply_extras("Ari", None, sequences(&[7, 8]));
    assert_eq!(
        panel.console.channels,
        [ChannelTab::Conversation("bo".into())]
    );
    // Already open in another case: no second tab.
    panel.lines.push(private("BO"));
    panel.apply_extras("Ari", None, sequences(&[7, 8, 9]));
    assert_eq!(panel.console.channels.len(), 1);
}

#[test]
fn c0496_own_echoes_and_ignored_senders_open_no_tab() {
    let mut panel = ChatPanel {
        lines: vec![private("Ari"), private("Spam")],
        ..Default::default()
    };
    panel
        .console
        .communication
        .edit(crate::chat::CommunicationList::Ignore, "Spam", true);
    let extras = ChatExtras {
        recipients: Some(vec![Some("Bo".into()), None]),
        ..sequences(&[1, 2])
    };
    panel.apply_extras("Ari", None, extras);
    assert!(panel.console.channels.is_empty());
}

#[test]
fn c0496_without_the_capability_no_tab_opens_by_itself() {
    let mut panel = ChatPanel {
        lines: vec![private("Bo")],
        ..Default::default()
    };
    panel.apply_extras("Ari", None, ChatExtras::default());
    assert!(panel.console.channels.is_empty());
}
