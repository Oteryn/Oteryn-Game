#[test]
fn analytics_direct_selector_uses_live_facts_and_keeps_tools_separate() {
    use crate::play::{mailbox::lock, play_channel};
    use oteryn_client_ui::{analyser::Tool, cyclopedia_map::Section};
    use oteryn_session::{AppliedDelta, SessionEvent, analyser_transport::AnalyserFact};
    let (link, mut receiver, outbox) = play_channel();
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
    let mut mailbox = lock(&link.mailbox);
    for (index, tool) in Tool::CYCLOPEDIA.iter().enumerate() {
        assert!(super::analytics::select(&mut mailbox, 26 + index));
        assert_eq!(mailbox.analyser.tool, *tool);
        assert_eq!(
            mailbox.properties.cyclopedia_map.section,
            Section::Analytics
        );
        let text = super::section_text(&mailbox);
        assert!(text.contains(&format!("Selected tool: {}", tool.title())));
        assert_eq!(mailbox.analyser.kills, 1);
        assert_eq!(mailbox.analyser.gained_xp, 9);
        if *tool == Tool::Experience {
            assert!(text.contains("Experience: 7 raw / 9 gained"));
            assert!(!text.contains("Kills:"));
        }
        if *tool == Tool::Hunting {
            assert!(text.contains("Kills: 1"));
        }
    }
    assert!(!super::analytics::select(&mut mailbox, 33));
    assert_eq!(mailbox.analyser.kills, 1);
    assert!(receiver.try_recv().is_err());
    drop(mailbox);
    outbox.logged_out();
    let mailbox = lock(&link.mailbox);
    assert!(super::analytics::text(&mailbox).contains("unavailable"));
}
