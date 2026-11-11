//! Capability 483: the Wheel panel runs on the v2 Atelier reply and Replace Vessel.
use super::*;
use atelier::v2;

fn v2_reply(revision: u64, in_temple: bool, premium: bool) -> v2::AtelierReply {
    v2::AtelierReply {
        refused: false,
        revision,
        wheel_revision: revision,
        in_temple,
        premium,
        gems: vec![v2::GemView {
            id: [17; 16],
            quality: 2,
            locked: false,
            slot: None,
            modifiers: vec![31, 0, 5],
        }],
        grades: vec![],
        quality_quotes: vec![],
        grade_quotes: vec![],
        vessels: (0..12u8)
            .map(|slot| v2::VesselSlotView {
                quality: slot / 4,
                open: true,
                gem: None,
            })
            .collect(),
        vessel_nodes: vec![3],
        vessel_node_bonus: 2,
        resistance_capped: true,
    }
}

fn synced(in_temple: bool, premium: bool) -> Result<WheelPanel, Error> {
    let mut panel = WheelPanel {
        selected: true,
        atelier_v2: true,
        ..WheelPanel::default()
    };
    let (wheel, _) = current(4);
    request(&mut panel, Intent::Refresh, occurrence(1)).ok_or("refresh")?;
    let response = WheelResponse {
        kind: 161,
        status: CommandStatus::Accepted,
        payload: wire(wheel::encode_wheel_reply(&wheel))?,
        wheel: Some(wire(wheel::encode_wheel_reply(&wheel))?),
        atelier: Some(wire(v2::encode_reply(&v2_reply(4, in_temple, premium)))?),
        resources: None,
    };
    assert!(apply_result(&mut panel, &response));
    Ok(panel)
}

#[test]
fn a_v2_reply_enables_reshape_only_in_the_temple_with_premium() -> Result<(), Error> {
    for (in_temple, premium, blocked) in [
        (true, true, false),
        (false, true, true),
        (true, false, true),
    ] {
        let panel = synced(in_temple, premium)?;
        assert_eq!(panel.in_temple, Some(in_temple));
        assert_eq!(panel.premium, Some(premium));
        let gem = panel.selected_gem().ok_or("gem")?;
        assert_eq!(reshape_block(&panel, gem).is_some(), blocked);
    }
    Ok(())
}

#[test]
fn replace_vessel_sends_the_slot_gem_and_wheel_revision_and_checks_the_quality() -> Result<(), Error>
{
    let mut panel = synced(true, true)?;
    // The gem is Greater (quality 2); slots 8-11 are Greater.
    panel.vessel_slot = 0;
    assert!(request(&mut panel, Intent::ReplaceVessel, occurrence(2)).is_none());
    assert!(panel.feedback.is_some());
    panel.vessel_slot = 9;
    let command = request(&mut panel, Intent::ReplaceVessel, occurrence(3)).ok_or("replace")?;
    assert_eq!((command.kind, command.mutation), (6213, true));
    let decoded = wire(v2::decode_replace_vessel(&command.payload))?;
    assert_eq!(
        (
            decoded.occurrence,
            decoded.expected_wheel_revision,
            decoded.slot,
            decoded.gem
        ),
        (occurrence(3), 4, 9, Some([17; 16]))
    );
    Ok(())
}

#[test]
fn v2_commands_use_the_6210_family_and_save_keeps_the_vessels_unsent() -> Result<(), Error> {
    let mut panel = synced(true, true)?;
    let lock = request(&mut panel, Intent::ToggleLock, occurrence(4)).ok_or("lock")?;
    assert_eq!(lock.kind, 6211);
    panel.pending = None;
    panel.retry = None;
    panel.set_points(14, 1);
    let save = request(&mut panel, Intent::Save, occurrence(5)).ok_or("save")?;
    assert_eq!(save.kind, 161);
    assert_eq!(
        wire(wheel::decode_wheel_request(&save.payload))?.vessels,
        None
    );
    Ok(())
}

#[test]
fn the_panel_text_lists_twelve_vessels_the_node_bonus_and_the_resistance_cap() -> Result<(), Error>
{
    let panel = synced(true, true)?;
    let text = panel_text(&panel);
    assert!(text.contains("Gem Vessel 12"));
    assert!(text.contains("+2 Damage / Healing"));
    assert!(text.contains("capped at 30%"));
    Ok(())
}

#[test]
fn without_483_the_four_draft_vessels_and_old_bytes_remain() -> Result<(), Error> {
    let mut panel = panel();
    assert!(!panel.atelier_v2);
    assert_eq!(panel.vessel_slot_count(), 4);
    let command = request(&mut panel, Intent::ToggleLock, occurrence(6)).ok_or("lock")?;
    assert_eq!(command.kind, 309);
    panel.pending = None;
    assert!(request(&mut panel, Intent::ReplaceVessel, occurrence(7)).is_none());
    Ok(())
}
