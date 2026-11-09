use super::*;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{CharacterTab, GeneralStats, KillStatus, TabView},
    raw_feature::GatedCommandResult,
};

#[test]
fn character_reply_renders_stats_deaths_and_pvp_through_panel()
-> Result<(), Box<dyn std::error::Error>> {
    for (view, expected) in [
        (
            TabView::GeneralStats(GeneralStats {
                level: 123,
                experience: 987654,
                ..Default::default()
            }),
            vec![
                "Level: 123",
                "Experience: 987654",
                "Health: unavailable",
                "Mana: unavailable",
            ],
        ),
        (
            TabView::RecentDeaths(vec![(12345, "Died at Level 123 by Alice".into())]),
            vec!["12345", "Died at Level 123 by Alice"],
        ),
        (
            TabView::RecentPvpKills(vec![(23456, KillStatus::GuildWar, "Killed Bob.".into())]),
            vec!["23456", "Guild war", "Killed Bob."],
        ),
    ] {
        let mut panel = CyclopediaCharacterPanel {
            selected: true,
            tab: view.tab(),
            ..Default::default()
        };
        let command = request(&mut panel, Intent::Tab).ok_or("request missing")?;
        assert_eq!(command.kind, 388);
        let result = GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: view.encode().map_err(|_| "encoding failed")?,
        };
        assert!(apply_result(&mut panel, &result));
        let text = panel_text(&panel);
        for expected in expected {
            assert!(text.contains(expected), "{text} lacks {expected}");
        }
        assert!(!text.contains("bytes"));
    }
    Ok(())
}

#[test]
fn character_panel_rejects_malformed_trailing_wrong_tab_and_excess_page_rows()
-> Result<(), Box<dyn std::error::Error>> {
    let valid = TabView::RecentDeaths(vec![(5, "Died at Level 8".into())])
        .encode()
        .map_err(|_| "encoding failed")?;
    let mut trailing = valid.clone();
    trailing.push(0);
    let mut bad_utf8 = valid.clone();
    *bad_utf8.last_mut().ok_or("empty")? = 255;
    for bytes in [vec![2], trailing, bad_utf8, vec![3, 0, 0], vec![2, 51, 0]] {
        let mut p = CyclopediaCharacterPanel {
            selected: true,
            tab: CharacterTab::RecentDeaths,
            ..Default::default()
        };
        assert!(request(&mut p, Intent::Tab).is_some());
        assert!(!apply_result(
            &mut p,
            &GatedCommandResult {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: bytes
            }
        ));
        assert!(p.feedback.is_some());
    }
    Ok(())
}

#[test]
fn character_remaining_tabs_decode_completely_and_reject_every_truncation()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cyclopedia_character::{
        EquippedItem, StoreSummary,
        combat::{CombatStat, CombatStats},
    };
    let mut combat = CombatStats::new(CharacterTab::Offence, 123, 2, 3).map_err(|_| "combat")?;
    combat
        .set(CombatStat::CriticalChance, 50_000)
        .map_err(|_| "combat")?;
    for (view, expected) in [
        (
            TabView::Achievements {
                total: 5,
                keys: vec!["achievement.one".into()],
            },
            "achievement.one",
        ),
        (
            TabView::Appearances {
                total: 8,
                keys: vec!["outfit.citizen".into()],
            },
            "outfit.citizen",
        ),
        (
            TabView::StoreSummary(StoreSummary {
                blessings: vec!["spiritual_shielding".into()],
                permanent_prey_slots: 1,
                prey_wildcards: 3,
                hirelings: 2,
            }),
            "Hirelings: 2",
        ),
        (
            TabView::Inspection(vec![EquippedItem {
                slot: 1,
                definition: "item.sword".into(),
                quantity: 1,
            }]),
            "Slot 1: item.sword × 1",
        ),
        (TabView::Combat(combat), "CriticalChance: 5.0000%"),
        (
            TabView::Combat(
                CombatStats::new(CharacterTab::Defence, 123, 2, 3).map_err(|_| "combat")?,
            ),
            "Armor: unavailable",
        ),
        (
            TabView::Combat(CombatStats::new(CharacterTab::Misc, 123, 2, 3).map_err(|_| "combat")?),
            "Blessings: unavailable",
        ),
    ] {
        let payload = view.encode().map_err(|_| "encode")?;
        let mut p = CyclopediaCharacterPanel {
            selected: true,
            tab: view.tab(),
            ..Default::default()
        };
        for end in 0..payload.len() {
            assert!(request(&mut p, Intent::Tab).is_some());
            assert!(!apply_result(
                &mut p,
                &GatedCommandResult {
                    command_id: 1,
                    status: CommandStatus::Accepted,
                    server_sequence: 1,
                    payload: payload[..end].into()
                }
            ));
        }
        assert!(request(&mut p, Intent::Tab).is_some());
        assert!(apply_result(
            &mut p,
            &GatedCommandResult {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload
            }
        ));
        assert!(
            panel_text(&p).contains(expected),
            "{} lacks {expected}",
            panel_text(&p)
        );
    }
    Ok(())
}

#[test]
fn character_navigation_targets_and_pages_are_bounded_and_frozen_while_pending()
-> Result<(), Box<dyn std::error::Error>> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    assert!(set_target(&mut p, "01010101010170018001010101010101"));
    let c = request(&mut p, Intent::SelectTab(CharacterTab::RecentDeaths)).ok_or("request")?;
    assert_eq!(c.payload.len(), 20);
    assert!(!set_target(&mut p, ""));
    assert!(request(&mut p, Intent::Next).is_none());
    assert_eq!(p.offset, 0);
    assert_eq!(p.tab, CharacterTab::RecentDeaths);
    p.pending = false;
    assert!(request(&mut p, Intent::Next).is_some());
    assert_eq!(p.offset, 20);
    p.pending = false;
    assert!(request(&mut p, Intent::Previous).is_some());
    assert_eq!(p.offset, 0);
    p.pending = false;
    assert!(!set_target(&mut p, "invalid"));
    assert!(set_target(&mut p, ""));
    assert_eq!(p.target, None);
    Ok(())
}

#[test]
fn character_history_rejects_complete_reply_exceeding_requested_page()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::RecentDeaths,
        limit: 1,
        result: Some("confirmed".into()),
        ..Default::default()
    };
    let payload =
        TabView::RecentDeaths(vec![(1, "First death".into()), (2, "Second death".into())])
            .encode()
            .map_err(|_| "encode")?;
    assert!(request(&mut panel, Intent::Tab).is_some());
    assert!(!apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload,
        }
    ));
    assert_eq!(panel.result.as_deref(), Some("confirmed"));
    Ok(())
}

mod achievements;
mod history_tables;
mod misc;
mod overview;
mod services;
