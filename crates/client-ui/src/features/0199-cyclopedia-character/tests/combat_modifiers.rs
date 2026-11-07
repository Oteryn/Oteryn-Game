use super::{CyclopediaCharacterPanel, Intent, apply_result, panel_text, request};
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{
        CharacterTab, TabView,
        combat::{CombatStat, CombatStats},
    },
    raw_feature::GatedCommandResult,
};

#[test]
fn combat_modifier_reply_reaches_panel_with_every_field_and_exact_units()
-> Result<(), Box<dyn std::error::Error>> {
    for tab in [
        CharacterTab::Offence,
        CharacterTab::Defence,
        CharacterTab::Misc,
    ] {
        let mut panel = CyclopediaCharacterPanel {
            selected: true,
            tab,
            ..Default::default()
        };
        let command = request(&mut panel, Intent::Tab).ok_or("request")?;
        assert_eq!(command.kind, 388);
        let mut stats = CombatStats::new(tab, 1234, 2, 3).map_err(|_| "stats")?;
        for key in CombatStat::for_tab(tab) {
            stats
                .set(
                    *key,
                    if key.permits_negative() {
                        -12345
                    } else {
                        12345
                    },
                )
                .map_err(|_| "set")?;
        }
        let payload = TabView::Combat(stats).encode().map_err(|_| "encode")?;
        assert!(apply_result(
            &mut panel,
            &GatedCommandResult {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload
            }
        ));
        let text = panel_text(&panel);
        for key in CombatStat::for_tab(tab) {
            assert!(text.contains(&format!("{key:?}")), "{text}");
        }
        let expected: &[&str] = match tab {
            CharacterTab::Offence => &[
                "CriticalChance: 1.2345%",
                "CriticalChanceImbuement: -1.2345%",
                "LifeLeech: 1.2345%",
                "ManaLeech: 1.2345%",
                "Cleave: 1.2345%",
                "PerfectShot1: 12345",
                "Onslaught: 1.2345%",
            ],
            CharacterTab::Defence => &[
                "PhysicalReflection: 12345",
                "AbsorbPhysical: -1.2345%",
                "Dodge: 1.2345%",
            ],
            CharacterTab::Misc => &["Momentum: 1.2345%", "WealthDuplexSeconds: 12345 seconds"],
            _ => return Err("unexpected combat tab".into()),
        };
        for expected in expected {
            assert!(text.contains(expected), "{text}");
        }
        assert!(text.contains("1234"));
        assert!(!text.contains("bytes"));
    }
    Ok(())
}

#[test]
fn combat_modifier_unknown_is_distinct_from_zero_and_malformed_is_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::Offence,
        ..Default::default()
    };
    let mut stats = CombatStats::new(panel.tab, 1, 1, 1).map_err(|_| "stats")?;
    stats
        .set(CombatStat::CriticalChance, 0)
        .map_err(|_| "set")?;
    let payload = TabView::Combat(stats).encode().map_err(|_| "encode")?;
    let reply = |payload| GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    };
    assert!(request(&mut panel, Intent::Tab).is_some());
    assert!(apply_result(&mut panel, &reply(payload.clone())));
    let text = panel_text(&panel);
    assert!(text.contains("CriticalChance: 0.0000%"), "{text}");
    assert!(text.contains("CriticalDamage: unavailable"), "{text}");
    for end in 0..payload.len() {
        assert!(request(&mut panel, Intent::Tab).is_some());
        assert!(!apply_result(&mut panel, &reply(payload[..end].to_vec())));
    }
    let mut trailing = payload;
    trailing.push(0);
    assert!(request(&mut panel, Intent::Tab).is_some());
    assert!(!apply_result(&mut panel, &reply(trailing)));
    Ok(())
}
