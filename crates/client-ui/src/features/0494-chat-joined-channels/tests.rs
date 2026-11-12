use crate::chat::{ChannelTab, ChatExtras, ChatPanel};

fn joined(ids: &[u16]) -> ChatExtras {
    ChatExtras {
        joined: Some(ids.to_vec()),
        ..Default::default()
    }
}

#[test]
fn c0494_joined_channels_get_a_tab_after_a_reconnect() {
    let mut panel = ChatPanel::default();
    panel.apply_extras("Ari", None, joined(&[120, 300]));
    assert_eq!(
        panel.console.channels,
        [ChannelTab::Private(120), ChannelTab::Private(300)]
    );
    // The same list again opens nothing new, and a closed tab stays closed while joined.
    panel.console.drop_channel(&ChannelTab::Private(300));
    panel.apply_extras("Ari", None, joined(&[120, 300]));
    assert_eq!(panel.console.channels, [ChannelTab::Private(120)]);
    // Leaving and joining again opens it again.
    panel.apply_extras("Ari", None, joined(&[120]));
    panel.apply_extras("Ari", None, joined(&[120, 300]));
    assert_eq!(panel.console.channels.len(), 2);
}

#[test]
fn c0494_without_the_capability_no_tab_opens_from_the_list() {
    let mut panel = ChatPanel::default();
    panel.apply_extras("Ari", None, ChatExtras::default());
    assert!(panel.console.channels.is_empty());
}

#[test]
fn c0494_a_tab_opened_by_the_join_result_is_not_doubled() {
    let mut panel = ChatPanel::default();
    panel.console.open_channel(ChannelTab::Private(120));
    panel.apply_extras("Ari", None, joined(&[120]));
    assert_eq!(panel.console.channels, [ChannelTab::Private(120)]);
}
