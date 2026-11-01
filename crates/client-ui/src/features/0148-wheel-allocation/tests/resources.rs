use super::*;
use atelier::resources::{ResourceBalances, encode_reply};

fn balances() -> ResourceBalances {
    ResourceBalances {
        bank_gold: 900_001,
        carried_gold: 20_002,
        lesser_gems: 3,
        regular_gems: 4,
        greater_gems: 5,
        basic_fragments: 60,
        supreme_fragments: 70,
    }
}

#[test]
fn live_balances_follow_receipt_replay_and_display_all_seven_resources() -> Result<(), Error> {
    let mut panel = panel();
    panel.confirmed_resources = Some(ResourceBalances::default());
    request(&mut panel, Intent::Reveal, occurrence(1)).ok_or("reveal")?;
    let (_, mut historical) = current(1);
    historical.quality_quotes.clear();
    historical.grade_quotes.clear();
    let (wheel, atelier) = current(6);
    let mut reply = response(
        424,
        CommandStatus::DuplicateReplay,
        wire(atelier::encode_reply(&historical))?,
        &wheel,
        &atelier,
    )?;
    reply.resources = Some(encode_reply(&balances()));
    assert!(apply_result(&mut panel, &reply));
    assert_eq!(panel.confirmed_resources, Some(balances()));
    assert_eq!(panel.confirmed_atelier, Some(atelier));
    assert!(panel.ready());
    let text = panel_text(&panel);
    assert!(text.contains("Gold: 20002 carried; 900001 in bank."));
    assert!(text.contains("Unrevealed gems carried: 3 lesser; 4 regular; 5 greater."));
    assert!(text.contains("Fragments carried and in Supply Stash: 60 basic; 70 supreme."));
    Ok(())
}

#[test]
fn absent_or_malformed_optional_balances_clear_old_values_without_blocking_legacy_controls()
-> Result<(), Error> {
    for bytes in [None, Some(vec![255])] {
        let mut panel = panel();
        panel.confirmed_resources = Some(balances());
        request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
        let (wheel, atelier) = current(5);
        let mut reply = response(
            161,
            CommandStatus::Accepted,
            wire(wheel::encode_wheel_reply(&wheel))?,
            &wheel,
            &atelier,
        )?;
        reply.resources = bytes;
        assert!(apply_result(&mut panel, &reply));
        assert_eq!(panel.confirmed_resources, None);
        assert!(panel.ready());
        assert!(!panel_text(&panel).contains("Gold:"));
        assert!(request(&mut panel, Intent::Reveal, occurrence(2)).is_some());
    }
    Ok(())
}

#[test]
fn stale_core_projection_cannot_publish_a_partial_resource_refresh() -> Result<(), Error> {
    let mut panel = panel();
    let before = (
        panel.confirmed_wheel.clone(),
        panel.confirmed_atelier.clone(),
    );
    panel.confirmed_resources = Some(balances());
    request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
    let (wheel, atelier) = current(3);
    let mut reply = response(
        161,
        CommandStatus::Accepted,
        wire(wheel::encode_wheel_reply(&wheel))?,
        &wheel,
        &atelier,
    )?;
    reply.resources = Some(encode_reply(&ResourceBalances::default()));
    assert!(!apply_result(&mut panel, &reply));
    assert_eq!(
        (
            panel.confirmed_wheel.clone(),
            panel.confirmed_atelier.clone()
        ),
        before
    );
    assert_eq!(panel.confirmed_resources, None);
    assert!(!panel.ready());
    Ok(())
}

#[test]
fn displayed_balances_never_become_client_payment_authority() -> Result<(), Error> {
    let mut panel = panel();
    panel.confirmed_resources = Some(ResourceBalances::default());
    let command = request(&mut panel, Intent::Reveal, occurrence(9)).ok_or("reveal intent")?;
    assert_eq!(command.kind, 424);
    assert_eq!(
        wire(atelier::decode_mutate(&command.payload))?,
        atelier::MutateRequest {
            occurrence: occurrence(9),
            expected_revision: 4,
            mutation: atelier::Mutation::Reveal { quality: 0 },
        }
    );
    assert_eq!(
        panel.confirmed_resources,
        Some(ResourceBalances::default()),
        "submitting an intent cannot spend a displayed balance optimistically"
    );
    Ok(())
}
