use super::*;

#[test]
fn recent_deaths_table_keeps_timestamp_and_cause_in_separate_columns()
-> Result<(), Box<dyn std::error::Error>> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::RecentDeaths,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    let payload = TabView::RecentDeaths(vec![(12345, "Died at Level 42: dragon".into())])
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
    let rows = history_rows(&p);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].at, 12345);
    assert_eq!(rows[0].description, "Died at Level 42: dragon");
    assert!(rows[0].status.is_none());
    Ok(())
}

#[test]
fn pvp_history_table_filters_confirmed_kill_kinds_without_changing_the_page()
-> Result<(), Box<dyn std::error::Error>> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::RecentPvpKills,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    let payload = TabView::RecentPvpKills(vec![
        (1, KillStatus::GuildWar, "Killed A.".into()),
        (2, KillStatus::Unjustified, "Killed B.".into()),
    ])
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
    p.kill_filter = Some(KillStatus::GuildWar);
    assert_eq!(history_rows(&p).len(), 1);
    assert_eq!(history_rows(&p)[0].description, "Killed A.");
    p.kill_filter = Some(KillStatus::Unjustified);
    assert_eq!(history_rows(&p)[0].description, "Killed B.");
    p.kill_filter = None;
    assert_eq!(history_rows(&p).len(), 2);
    assert_eq!(p.offset, 0);
    Ok(())
}
