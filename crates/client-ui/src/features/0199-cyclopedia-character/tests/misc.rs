use super::*;
use oteryn_client_runtime::cyclopedia_character::combat::{CombatStat, CombatStats};
#[test]
fn misc_table_and_resources_keep_the_selected_tab_and_confirmed_sources() -> Result<(), &'static str>
{
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::SelectTab(CharacterTab::Misc)).is_some());
    let mut stats = CombatStats::new(CharacterTab::Misc, 50, 2, 3).map_err(|_| "stats")?;
    stats
        .set(CombatStat::Momentum, 25000)
        .map_err(|_| "momentum")?;
    stats
        .set(CombatStat::Blessings, 0)
        .map_err(|_| "blessings")?;
    let reply = |payload| GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    };
    assert!(apply_result(
        &mut p,
        &reply(
            TabView::Combat(stats.clone())
                .encode()
                .map_err(|_| "misc")?
        )
    ));
    let rows = combat_rows(&p);
    assert!(
        rows.iter()
            .any(|r| r.key == CombatStat::Momentum && r.value == "2.5000%")
    );
    assert!(
        rows.iter()
            .any(|r| r.key == CombatStat::Blessings && r.value == "0")
    );
    assert!(
        rows.iter()
            .any(|r| r.key == CombatStat::WealthDuplexSeconds && r.value == "Unavailable")
    );
    assert!(misc_cards(&p).is_empty());
    let command = request(&mut p, Intent::MiscResources).ok_or("resource query")?;
    assert_eq!(command.payload[0], CharacterTab::GeneralStats as u8);
    let general = GeneralStats {
        magic_level: 8,
        skills: [17; 7],
        stamina_minutes: 1260,
        offline_training_minutes: 360,
        ..Default::default()
    };
    assert!(apply_result(
        &mut p,
        &reply(
            TabView::GeneralStats(general)
                .encode()
                .map_err(|_| "general")?
        )
    ));
    assert_eq!(p.tab, CharacterTab::Misc);
    assert_eq!(p.combat, Some(stats));
    assert_eq!(misc_cards(&p).len(), 10);
    assert!(
        misc_cards(&p)
            .iter()
            .any(|(label, value)| *label == "Sword" && value == "17")
    );
    assert_eq!(misc_bars(&p), Some([(1260, 2520), (360, 720)]));
    assert!(request(&mut p, Intent::MiscResources).is_none());
    Ok(())
}
#[test]
fn profile_changes_clear_previous_typed_views_and_refusals_do_not_restore_them() {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::Misc,
        general: Some(GeneralStats::default()),
        misc_general: Some(GeneralStats::default()),
        ..Default::default()
    };
    assert!(set_target(&mut p, "01010101010101010101010101010101"));
    assert!(p.general.is_none());
    assert!(p.misc_general.is_none());
    assert!(request(&mut p, Intent::Tab).is_some());
    assert!(!apply_result(
        &mut p,
        &GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Rejected,
            server_sequence: 1,
            payload: vec![]
        }
    ));
    assert!(misc_cards(&p).is_empty());
    assert!(!p.misc_resources_due);
}
#[test]
fn refused_or_malformed_auxiliary_reads_leave_misc_confirmed_and_do_not_retry()
-> Result<(), &'static str> {
    for status in [CommandStatus::Rejected, CommandStatus::Accepted] {
        let mut p = CyclopediaCharacterPanel {
            selected: true,
            ..Default::default()
        };
        assert!(request(&mut p, Intent::SelectTab(CharacterTab::Misc)).is_some());
        let stats = CombatStats::new(CharacterTab::Misc, 1, 1, 1).map_err(|_| "stats")?;
        let payload = TabView::Combat(stats.clone())
            .encode()
            .map_err(|_| "encode")?;
        let result = GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload,
        };
        assert!(apply_result(&mut p, &result));
        assert!(request(&mut p, Intent::MiscResources).is_some());
        assert!(!apply_result(
            &mut p,
            &GatedCommandResult { status, ..result }
        ));
        assert_eq!(p.combat, Some(stats));
        assert_eq!(p.tab, CharacterTab::Misc);
        assert!(misc_cards(&p).is_empty());
        assert!(request(&mut p, Intent::MiscResources).is_none());
    }
    Ok(())
}
