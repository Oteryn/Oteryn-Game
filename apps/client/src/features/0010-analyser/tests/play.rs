use crate::play::play_channel;
use oteryn_session::{AppliedDelta, SessionEvent, analyser_transport::AnalyserFact};
#[test]
fn analyser_push_reaches_closed_panel_and_end_clears_totals() {
    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 8,
        base_revision: 1,
        new_revision: 2,
        value: vec![AnalyserFact::Kill { race: None }],
    })]);
    assert_eq!(link.analyser_panel().kills, 1);
    assert!(!link.analyser_panel().open);
    outbox.logged_out();
    assert_eq!(link.analyser_panel().kills, 0);
    assert!(!link.analyser_panel().selected);
}

#[test]
fn analyser_selector_projects_session_facts_without_resetting_other_tools() {
    use oteryn_client_ui::analyser::{Tool, panel_text};
    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 8,
        base_revision: 1,
        new_revision: 2,
        value: vec![
            AnalyserFact::Kill { race: None },
            AnalyserFact::Experience { raw: 7, gained: 9 },
        ],
    })]);
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    crate::play::feature_0010_analyser::input::select_tool(&mut mailbox.analyser, 2);
    assert_eq!(mailbox.analyser.tool, Tool::Experience);
    assert!(panel_text(&mailbox.analyser).contains("Experience: 7 raw / 9 gained"));
    assert!(!panel_text(&mailbox.analyser).contains("Kills:"));
    crate::play::feature_0010_analyser::input::select_tool(&mut mailbox.analyser, 1);
    assert!(panel_text(&mailbox.analyser).contains("Kills: 1"));
    crate::play::feature_0010_analyser::input::select_tool(&mut mailbox.analyser, 7);
    assert_eq!(mailbox.analyser.kills, 0);
    assert_eq!(mailbox.analyser.tool, Tool::Hunting);
}
