use super::*;

#[test]
fn store_summary_cards_and_blessings_come_from_the_validated_reply()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_character::StoreSummary;
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::StoreSummary,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    let payload = TabView::StoreSummary(StoreSummary {
        blessings: vec!["Wisdom".into(), "Protection".into()],
        permanent_prey_slots: 1,
        prey_wildcards: 7,
        hirelings: 2,
    })
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
    assert_eq!(
        store_cards(&p),
        vec![
            ("Permanent prey slots", "1".into()),
            ("Prey wildcards", "7".into()),
            ("Hirelings", "2".into()),
            ("Blessings", "2".into())
        ]
    );
    assert_eq!(store_blessings(&p), "Wisdom\r\nProtection");
    Ok(())
}

#[test]
fn blessings_overview_distinguishes_empty_protection_from_unloaded_state()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_character::StoreSummary;
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::StoreSummary,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    let payload = TabView::StoreSummary(StoreSummary {
        blessings: vec![],
        permanent_prey_slots: 0,
        prey_wildcards: 0,
        hirelings: 0,
    })
    .encode()
    .map_err(|_| "encode")?;
    assert!(apply_result(
        &mut p,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload
        }
    ));
    let text = p.result.as_deref().ok_or("missing overview")?;
    assert!(text.contains("Blessings / death protection"));
    assert!(text.contains("No blessings owned"));
    assert!(text.contains("Loss percentages are unavailable"));
    Ok(())
}
