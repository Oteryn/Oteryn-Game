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
