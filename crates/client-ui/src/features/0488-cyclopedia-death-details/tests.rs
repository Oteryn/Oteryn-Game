use super::*;
use crate::cyclopedia_character::{
    CyclopediaCharacterPanel, Intent, TabExtensions, apply_result, history_rows, request,
};
use oteryn_client_runtime::cyclopedia_character::{
    CharacterTab, DeathDetails, TabExtension, TabView,
};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

const SELECTED: TabExtensions = TabExtensions {
    vocation: false,
    death_details: true,
};

fn deaths() -> Vec<(i64, String)> {
    vec![
        (10, "Died at Level 90 by a dragon".into()),
        (20, "Died at Level 91 by Alice and Bob".into()),
    ]
}

fn details() -> Vec<DeathDetails> {
    vec![
        DeathDetails {
            level: 90,
            killer: "dragon".into(),
            most_damage: "dragon".into(),
            unjustified: false,
        },
        DeathDetails {
            level: 91,
            killer: String::new(),
            most_damage: "Bob".into(),
            unjustified: true,
        },
    ]
}

fn read(extensions: TabExtensions, payload: Vec<u8>) -> Option<CyclopediaCharacterPanel> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        extensions,
        ..Default::default()
    };
    request(&mut p, Intent::SelectTab(CharacterTab::RecentDeaths))?;
    let result = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    };
    apply_result(&mut p, &result).then_some(p)
}

fn extended() -> Result<Vec<u8>, &'static str> {
    TabView::RecentDeaths(deaths())
        .encode_extended(&TabExtension {
            vocation_id: None,
            deaths: Some(details()),
        })
        .map_err(|_| "death reply")
}

#[test]
fn c0488_selected_rows_carry_the_detailed_death_table() -> Result<(), &'static str> {
    let p = read(SELECTED, extended()?).ok_or("selected reply decodes")?;
    let rows = history_rows(&p);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].death, details().into_iter().next());
    assert_eq!(
        history_label(rows[0]),
        "10\tDeath\t90\tdragon\tdragon\tNo\tDied at Level 90 by a dragon"
    );
    assert_eq!(
        history_label(rows[1]),
        "20\tDeath\t91\tunknown\tBob\tYes\tDied at Level 91 by Alice and Bob"
    );
    assert!(history_columns(true).contains("Killer"));
    assert!(
        p.result
            .as_deref()
            .is_some_and(|text| text.contains("most damage Bob, unjustified"))
    );
    Ok(())
}

#[test]
fn c0488_unselected_session_keeps_the_plain_rows() -> Result<(), &'static str> {
    let plain = TabView::RecentDeaths(deaths())
        .encode()
        .map_err(|_| "plain reply")?;
    let p = read(TabExtensions::default(), plain.clone()).ok_or("old reply decodes")?;
    let rows = history_rows(&p);
    assert_eq!(rows[0].death, None);
    assert_eq!(
        history_label(rows[0]),
        "10\tDeath\tDied at Level 90 by a dragon"
    );
    assert!(!history_columns(false).contains("Killer"));
    // Mixed encodings never decode: the extended rows are refused without 488 and the plain
    // rows are refused with it.
    assert!(read(TabExtensions::default(), extended()?).is_none());
    assert!(read(SELECTED, plain).is_none());
    Ok(())
}

#[test]
fn c0488_pvp_kill_rows_never_carry_death_details() -> Result<(), &'static str> {
    let payload = TabView::RecentPvpKills(vec![(5, KillStatus::Unjustified, "Opponent".into())])
        .encode()
        .map_err(|_| "pvp reply")?;
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        extensions: SELECTED,
        ..Default::default()
    };
    request(&mut p, Intent::SelectTab(CharacterTab::RecentPvpKills)).ok_or("request")?;
    let result = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    };
    assert!(apply_result(&mut p, &result));
    let rows = history_rows(&p);
    assert_eq!(history_label(rows[0]), "5\tUnjustified\tOpponent");
    Ok(())
}
