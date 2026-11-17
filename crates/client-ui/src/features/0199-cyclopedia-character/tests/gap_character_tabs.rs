use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

#[test]
fn gap_character_tabs_reject_truncated_general_stats() {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        result: Some("confirmed".into()),
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    assert!(!apply_result(
        &mut p,
        &GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![1, 0]
        }
    ));
    assert_eq!(p.result.as_deref(), Some("confirmed"));
}

#[test]
fn gap_character_tabs_decode_every_body_and_retain_confirmed_state() {
    use oteryn_client_runtime::cyclopedia_character::{
        CharacterTab, EquippedItem, GeneralStats, KillStatus, StoreSummary, TabView,
        combat::{CombatStat, CombatStats},
    };
    let mut fixtures = vec![
        (
            TabView::GeneralStats(GeneralStats {
                experience: 123456,
                level: 42,
                magic_level: 9,
                skills: [12; 7],
                stamina_minutes: 2000,
                offline_training_minutes: 600,
                last_vitals: Some((100, 50)),
            }),
            "Health: 100",
        ),
        (
            TabView::RecentDeaths(vec![(123, "Died at Level 42".into())]),
            "123: Died at Level 42",
        ),
        (
            TabView::RecentPvpKills(vec![(456, KillStatus::GuildWar, "Killed Bob.".into())]),
            "456: Guild war: Killed Bob.",
        ),
        (
            TabView::Achievements {
                total: 9,
                keys: vec!["Explorer".into()],
            },
            "Total: 9\nExplorer",
        ),
        (
            TabView::Appearances {
                total: 2,
                keys: vec!["mount:horse".into()],
            },
            "Total: 2\nmount:horse",
        ),
        (
            TabView::StoreSummary(StoreSummary {
                blessings: vec!["Wisdom".into()],
                permanent_prey_slots: 1,
                prey_wildcards: 2,
                hirelings: 3,
            }),
            "Blessing: Wisdom\nPermanent prey slots: 1\nPrey wildcards: 2\nHirelings: 3",
        ),
        (
            TabView::Inspection(vec![EquippedItem {
                slot: 5,
                definition: "sword".into(),
                quantity: 2,
            }]),
            "Slot 5: sword × 2",
        ),
    ];
    for (tab, key, value, expected) in [
        (
            CharacterTab::Offence,
            CombatStat::CriticalChance,
            50000,
            "CriticalChance: 5.0000%",
        ),
        (
            CharacterTab::Defence,
            CombatStat::AbsorbFire,
            -12500,
            "AbsorbFire: -1.2500%",
        ),
        (
            CharacterTab::Misc,
            CombatStat::WealthDuplexSeconds,
            60,
            "WealthDuplexSeconds: 60 seconds",
        ),
    ] {
        let mut stats = CombatStats::new(tab, 789, 2, 3).unwrap_or_else(|_| unreachable!());
        assert!(stats.set(key, value).is_ok());
        fixtures.push((TabView::Combat(stats), expected));
    }
    for (view, expected) in fixtures {
        let payload = view.encode().unwrap_or_else(|_| unreachable!());
        let mut p = CyclopediaCharacterPanel {
            selected: true,
            tab: view.tab(),
            ..Default::default()
        };
        assert!(request(&mut p, Intent::Tab).is_some());
        // A UI selection changed while waiting must not change reply correlation.
        p.tab = CharacterTab::Misc;
        let mut reply = GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: payload.clone(),
        };
        assert!(apply_result(&mut p, &reply));
        assert!(panel_text(&p).contains(expected), "{}", panel_text(&p));
        if matches!(view, TabView::Combat(_)) {
            assert!(panel_text(&p).contains("unavailable"));
            assert!(panel_text(&p).contains("Observed at: 789 ms"));
        }
        let confirmed = p.result.clone();
        let mut invalid = vec![vec![view.tab() as u8; 16385]];
        for end in 0..payload.len() {
            invalid.push(payload[..end].to_vec());
        }
        let mut trailing = payload.clone();
        trailing.push(0);
        invalid.push(trailing);
        let mut wrong_tab = payload.clone();
        wrong_tab[0] = if view.tab() == CharacterTab::Misc {
            1
        } else {
            10
        };
        invalid.push(wrong_tab);
        for body in invalid {
            p.tab = view.tab();
            assert!(request(&mut p, Intent::Tab).is_some());
            reply.payload = body;
            assert!(!apply_result(&mut p, &reply));
            assert_eq!(p.result, confirmed);
            assert!(!p.pending);
            assert!(p.feedback.is_some());
        }
    }
}

#[test]
fn gap_character_tabs_reject_invalid_tags_counts_and_text() {
    use oteryn_client_runtime::cyclopedia_character::CharacterTab;
    let mut general = vec![0; 42];
    general[0] = 1;
    general[33] = 2;
    for payload in [
        general,
        vec![2, 51, 0], // Page count exceeds the native limit.
        vec![3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0], // Unowned kill status.
        vec![4, 0, 0, 1, 0, 0], // Count exceeds total.
        vec![5, 1, 0, 1, 0, 1, 255], // Invalid UTF-8.
        vec![7, 1, 5, 1, 2], // Production key exceeds 512 bytes.
    ] {
        let mut p = CyclopediaCharacterPanel {
            selected: true,
            tab: CharacterTab::from_id(payload[0]).unwrap_or_else(|| unreachable!()),
            result: Some("confirmed".into()),
            ..Default::default()
        };
        assert!(request(&mut p, Intent::Tab).is_some());
        assert!(!apply_result(
            &mut p,
            &GatedCommandResult {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload,
            }
        ));
        assert_eq!(p.result.as_deref(), Some("confirmed"));
    }
}
