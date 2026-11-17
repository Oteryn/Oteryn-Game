use super::*;

#[test]
fn overview_cards_and_resource_bars_project_the_confirmed_general_stats()
-> Result<(), Box<dyn std::error::Error>> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    let stats = GeneralStats {
        level: 42,
        experience: 12345,
        magic_level: 8,
        skills: [17; 7],
        stamina_minutes: 1260,
        offline_training_minutes: 360,
        last_vitals: Some((120, 75)),
    };
    let payload = TabView::GeneralStats(stats)
        .encode()
        .map_err(|_| "encode")?;
    assert!(apply_result(
        &mut p,
        &GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload
        }
    ));
    let cards = overview_cards(&p);
    assert!(
        cards
            .iter()
            .any(|(label, value)| *label == "Level" && value == "42")
    );
    assert!(
        cards
            .iter()
            .any(|(label, value)| *label == "Health (last saved)" && value == "120")
    );
    assert_eq!(overview_bars(&p), Some([(1260, 2520), (360, 720)]));
    assert_eq!(p.general, Some(stats));
    assert!(request(&mut p, Intent::Tab).is_some());
    let unknown = GeneralStats {
        last_vitals: None,
        ..stats
    };
    let payload = TabView::GeneralStats(unknown)
        .encode()
        .map_err(|_| "encode")?;
    assert!(apply_result(
        &mut p,
        &GatedCommandResult {
            command_id: 2,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload
        }
    ));
    assert!(
        overview_cards(&p)
            .iter()
            .any(|(label, value)| *label == "Health (last saved)" && value == "Unavailable")
    );
    Ok(())
}
