use super::*;
use crate::hotkey_equip::{HotkeyEquipPanel, control_request};

fn panel(tiered: bool, upgrade_tier: u8) -> HotkeyEquipPanel {
    HotkeyEquipPanel {
        selected: true,
        tiered,
        upgrade_tier,
        item_definition: 258,
        ..Default::default()
    }
}

fn definition() -> Result<NonZeroU32, &'static str> {
    NonZeroU32::new(258).ok_or("definition")
}

#[test]
fn c0485_selected_appends_the_upgrade_tier_byte() -> Result<(), &'static str> {
    for tier in 0..=10 {
        assert_eq!(
            equip_payload(definition()?, true, tier),
            Some(vec![0, 0, 1, 2, tier])
        );
        let command = control_request(&mut panel(true, tier)).ok_or("request")?;
        assert_eq!(command.payload, [0, 0, 1, 2, tier]);
        assert_eq!((command.capability, command.command_type), (179, 348));
    }
    Ok(())
}

#[test]
fn c0485_unselected_keeps_the_four_byte_payload() -> Result<(), &'static str> {
    assert_eq!(
        equip_payload(definition()?, false, 7),
        Some(vec![0, 0, 1, 2])
    );
    let command = control_request(&mut panel(false, 7)).ok_or("request")?;
    assert_eq!(command.payload, [0, 0, 1, 2]);
    Ok(())
}

#[test]
fn c0485_tier_above_ten_sends_nothing() {
    let mut p = panel(true, 11);
    assert!(control_request(&mut p).is_none());
    assert!(!p.pending);
    assert!(p.feedback.is_some());
}

#[test]
fn c0485_the_tier_applies_to_one_request_only() -> Result<(), &'static str> {
    let mut p = panel(true, 4);
    assert_eq!(
        control_request(&mut p).ok_or("first")?.payload,
        [0, 0, 1, 2, 4]
    );
    p.pending = false;
    assert_eq!(
        control_request(&mut p).ok_or("second")?.payload,
        [0, 0, 1, 2, 0]
    );
    Ok(())
}
