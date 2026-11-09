use super::*;

#[test]
fn analyser_dedicated_activity_views_chart_confirmed_wire_facts()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    // Damage 120 (fire), healing 60, decoded through the session's production codec.
    let mut facts = oteryn_client_runtime::analyser::decode_analyser_facts(&[
        10, 8, 34, 6, 8, 1, 16, 120, 24, 2, 10, 6, 34, 4, 8, 2, 16, 60,
    ])
    .map_err(|_| "analyser facts")?;
    facts.push(AnalyserFact::Kill { race: None });
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: facts,
    })]);
    for (key, total, rate, peak) in [
        ("hunting", "Kills: 1", "Kills/h:", "peak 1"),
        ("damage", "Damage dealt: 120", "DPS:", "peak 120"),
        ("healing", "Healing: 60", "HPS:", "peak 60"),
    ] {
        assert!(link.analyser_select(key));
        let text = link.analyser_text();
        for expected in [total, rate, peak, "Per minute"] {
            assert!(text.contains(expected), "{key} missing {expected}: {text}");
        }
    }
    assert!(!link.analyser_panel().open);
    outbox.logged_out();
    assert_eq!(link.analyser_panel().kills, 0);
    assert!(!link.analyser_panel().selected);
    Ok(())
}
