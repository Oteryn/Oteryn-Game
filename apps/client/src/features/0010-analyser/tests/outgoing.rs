use super::*;
#[test]
fn outgoing_and_incoming_metrics_reach_real_selectors_through_wire_facts_and_mailbox()
-> Result<(), &'static str> {
    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    let facts = oteryn_client_runtime::analyser::decode_analyser_facts(&[
        10, 8, 34, 6, 8, 1, 16, 120, 24, 2, 10, 6, 34, 4, 8, 2, 16, 60, 10, 10, 42, 8, 8, 60, 16,
        2, 24, 1, 32, 7,
    ])
    .map_err(|_| "impact/input wire")?;
    outbox.push(vec![SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: facts,
    })]);
    assert!(link.analyser_select("damage"));
    let damage = link.analyser_text();
    assert!(
        damage.contains("DPS:")
            && damage.contains("Damage per minute")
            && damage.contains("Fire: 120")
    );
    assert!(link.analyser_select("healing"));
    let healing = link.analyser_text();
    assert!(
        healing.contains("HPS:")
            && healing.contains("Healing per minute")
            && healing.contains("Healing: 60")
    );
    assert!(link.analyser_select("impact"));
    let impact = link.analyser_text();
    assert!(impact.contains("DPS:") && impact.contains("HPS:") && impact.contains("66.66%"));
    assert!(link.analyser_select("input"));
    let input = link.analyser_text();
    assert!(input.contains("Incoming DPS:") && input.contains("Creature race #7: 60"));
    assert_eq!(
        (
            link.analyser_panel().damage,
            link.analyser_panel().healing,
            link.analyser_panel().received
        ),
        (120, 60, 60)
    );
    outbox.logged_out();
    assert_eq!(link.analyser_panel().damage, 0);
    Ok(())
}
