use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_client_ui::cyclopedia_character::{
    CyclopediaCharacterPanel, Intent, apply_result, request,
};
#[test]
fn controls_encode_exact_request_and_reply_and_refusal_are_visible() {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        offset: 258,
        limit: 20,
        ..Default::default()
    };
    let c = request(&mut p, Intent::Tab).unwrap_or_else(|| unreachable!());
    assert_eq!(c.payload, [1, 2, 1, 20]);
    let ok = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: oteryn_client_runtime::cyclopedia_character::TabView::GeneralStats(
            Default::default(),
        )
        .encode()
        .unwrap_or_else(|_| unreachable!()),
    };
    assert!(apply_result(&mut p, &ok));
    assert!(p.result.is_some());
    assert!(request(&mut p, Intent::Tab).is_some());
    let refused = GatedCommandResult {
        status: CommandStatus::Rejected,
        ..ok
    };
    assert!(!apply_result(&mut p, &refused));
    assert!(p.feedback.is_some());
}

#[test]
fn character_statistics_decode_real_general_and_combat_results() -> Result<(), &'static str> {
    use crate::combat_input::PlayCommand;
    use crate::play::play_channel;
    use oteryn_client_runtime::cyclopedia_character::{
        CharacterTab, GeneralStats, TabView,
        combat::{CombatStat, CombatStats},
    };
    use oteryn_client_ui::cyclopedia_character::panel_text;
    let (link, mut incoming, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    let general = TabView::GeneralStats(GeneralStats {
        level: 20,
        experience: 98800,
        magic_level: 4,
        skills: [10, 11, 12, 13, 14, 15, 16],
        stamina_minutes: 2400,
        offline_training_minutes: 720,
        last_vitals: Some((365, 150)),
    })
    .encode()
    .map_err(|_| "general stats encode")?;
    link.cyclopedia_character_request(Intent::Tab);
    assert!(
        matches!(incoming.try_recv(), Ok(PlayCommand::Feature(c)) if c.kind == 388 && c.payload[0] == 1)
    );
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: general,
    });
    let text = panel_text(&link.cyclopedia_character_panel());
    assert!(text.contains("Level: 20"));
    assert!(text.contains("Health: 365"));
    assert!(text.contains("Mana: 150"));
    for tab in [
        CharacterTab::Offence,
        CharacterTab::Defence,
        CharacterTab::Misc,
    ] {
        let mut stats = CombatStats::new(tab, 100, 2, 3).map_err(|_| "combat stats")?;
        for key in CombatStat::for_tab(tab) {
            if !matches!(key, CombatStat::CriticalChance) {
                stats.set(*key, 0).map_err(|_| "set")?;
            }
        }
        let mut panel = CyclopediaCharacterPanel {
            selected: true,
            tab,
            ..Default::default()
        };
        request(&mut panel, Intent::Tab).ok_or("tab command")?;
        assert!(apply_result(
            &mut panel,
            &GatedCommandResult {
                command_id: 2,
                status: CommandStatus::Accepted,
                server_sequence: 2,
                payload: TabView::Combat(stats)
                    .encode()
                    .map_err(|_| "combat encode")?,
            }
        ));
        let text = panel_text(&panel);
        assert!(!text.contains("server page:"));
        if tab == CharacterTab::Offence {
            assert!(text.contains("CriticalChance: unavailable"));
        }
    }
    Ok(())
}

#[test]
fn statistics_reject_malformed_results_and_keep_unknown_distinct_from_zero()
-> Result<(), &'static str> {
    use oteryn_client_runtime::cyclopedia_character::{
        CharacterTab, GeneralStats, TabView,
        combat::{CombatStat, CombatStats},
    };
    let general = TabView::GeneralStats(GeneralStats::default())
        .encode()
        .map_err(|_| "encode")?;
    let result = |payload| GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    };
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        result: Some("previous view".into()),
        ..Default::default()
    };
    let apply = |panel: &mut CyclopediaCharacterPanel, reply: &GatedCommandResult| {
        assert!(request(panel, Intent::Tab).is_some());
        apply_result(panel, reply)
    };
    for end in 0..general.len() {
        assert!(!apply(&mut panel, &result(general[..end].to_vec())));
        assert_eq!(panel.result.as_deref(), Some("previous view"));
    }
    let mut unknown_with_value = general.clone();
    unknown_with_value[34] = 1;
    assert!(!apply(&mut panel, &result(unknown_with_value)));
    let mut invalid_tag = general.clone();
    invalid_tag[33] = 2;
    assert!(!apply(&mut panel, &result(invalid_tag)));
    assert!(apply(&mut panel, &result(general)));
    assert!(
        panel
            .result
            .as_deref()
            .is_some_and(|text| text.contains("Health: unavailable"))
    );
    let mut combat = CombatStats::new(CharacterTab::Defence, 100, 2, 3).map_err(|_| "combat")?;
    combat
        .set(CombatStat::AbsorbPhysical, -12500)
        .map_err(|_| "signed resistance")?;
    combat.set(CombatStat::Armor, 0).map_err(|_| "known zero")?;
    let encoded = TabView::Combat(combat)
        .encode()
        .map_err(|_| "combat encode")?;
    assert!(
        !apply(&mut panel, &result(encoded.clone())),
        "reply must match requested tab"
    );
    panel.tab = CharacterTab::Defence;
    assert!(apply(&mut panel, &result(encoded.clone())));
    let text = panel.result.as_deref().ok_or("rendered stats")?;
    assert!(text.contains("AbsorbPhysical: -1.2500%"));
    assert!(text.contains("Armor: 0"));
    assert!(text.contains("Dodge: unavailable"));
    for end in 0..encoded.len() {
        assert!(!apply(&mut panel, &result(encoded[..end].to_vec())));
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(!apply(&mut panel, &result(trailing)));
    Ok(())
}

#[path = "achievements.rs"]
mod achievements;
#[path = "gallery.rs"]
mod gallery;
#[path = "misc.rs"]
mod misc;
#[path = "navigation.rs"]
mod navigation;
