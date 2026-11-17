use super::{CyclopediaCharacterPanel, Intent, apply_result, panel_text, request};
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{GeneralStats, TabView},
    raw_feature::GatedCommandResult,
};

fn reply(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}

#[test]
fn stamina_minutes_are_decoded_in_general_panel_for_all_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    for (minutes, shown) in [
        (0, "0:00"),
        (1, "0:01"),
        (59, "0:59"),
        (60, "1:00"),
        (840, "14:00"),
        (2340, "39:00"),
        (2520, "42:00"),
    ] {
        let mut panel = CyclopediaCharacterPanel {
            selected: true,
            ..Default::default()
        };
        let command = request(&mut panel, Intent::Tab).ok_or("request")?;
        assert_eq!((command.capability, command.kind), (199, 388));
        let stats = GeneralStats {
            experience: 123456,
            level: 42,
            stamina_minutes: minutes,
            last_vitals: Some((321, 123)),
            ..Default::default()
        };
        let payload = TabView::GeneralStats(stats)
            .encode()
            .map_err(|_| "encode")?;
        assert!(apply_result(&mut panel, &reply(payload)));
        let text = panel_text(&panel);
        assert!(text.contains(&format!("Stamina: {shown}")), "{text}");
        assert!(!text.contains("bytes"));
    }
    Ok(())
}

#[test]
fn stamina_rejects_truncation_trailing_bytes_and_invalid_option_tag()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    let payload = TabView::GeneralStats(GeneralStats::default())
        .encode()
        .map_err(|_| "encode")?;
    assert_eq!(payload.len(), 42);
    for end in 0..payload.len() {
        assert!(request(&mut panel, Intent::Tab).is_some());
        assert!(!apply_result(&mut panel, &reply(payload[..end].to_vec())));
    }
    let mut invalid = payload.clone();
    invalid[33] = 2;
    assert!(request(&mut panel, Intent::Tab).is_some());
    assert!(!apply_result(&mut panel, &reply(invalid)));
    let mut trailing = payload;
    trailing.push(0);
    assert!(request(&mut panel, Intent::Tab).is_some());
    assert!(!apply_result(&mut panel, &reply(trailing)));
    Ok(())
}
