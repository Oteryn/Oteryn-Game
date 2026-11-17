use crate::play::{
    SettingsState,
    help::{Report, Topic},
    mailbox::lock,
    play_channel,
};

#[test]
fn help_navigation_opens_reviewed_report_through_live_caller()
-> Result<(), Box<dyn std::error::Error>> {
    let mut state = SettingsState::new(None);
    for topic in Topic::ALL {
        assert!(!state.help_text(topic).is_empty());
    }
    assert!(state.help_text(Topic::Faq).contains("server"));
    let (link, mut commands, _) = play_channel();
    state.request_typed_help_report(Report::Bug);
    assert!(!state.open_typed_help_report(&link));
    assert!(!link.player_report_panel().bug.open);
    lock(&link.mailbox).properties.player_report.selected = true;
    state.request_typed_help_report(Report::Bug);
    assert!(state.open_typed_help_report(&link));
    assert!(link.player_report_panel().bug.open);
    lock(&link.mailbox).properties.player_report.bug.fields = [
        "100,200,0".into(),
        "The map tile cannot be reached".into(),
        String::new(),
    ];
    link.bug_report_action(0);
    link.bug_report_action(4);
    assert!(commands.try_recv().is_err());
    link.bug_report_action(5);
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Feature(command) if command.capability == 76)
    );
    state.request_typed_help_report(Report::Player);
    assert!(state.open_typed_help_report(&link));
    assert!(link.player_report_panel().open);
    Ok(())
}
