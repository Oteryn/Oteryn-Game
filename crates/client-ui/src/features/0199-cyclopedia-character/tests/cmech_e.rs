use super::*;
use crate::cyclopedia_character::{
    AugmentIntent, augment_request, augment_rows, combat_detail, combat_rows, history_rows,
    misc_bars, overview_bars, promotion_text, promotion_view, protection_heading,
};
use oteryn_client_runtime::cyclopedia_character::{
    combat::{CombatStat, CombatStats},
    offence::{Augment, Effect, Page, Query, Snapshot, Source, SourceKind},
    protection::{Consumption, Forecast, Scenario},
};

fn accepted(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}

fn forecast(promoted: bool) -> Forecast {
    let scenario = Scenario {
        experience_lost: 10,
        equipment_per_mille: 25,
        container_per_mille: 250,
        consumption: Consumption::Regular,
        amulet_consumed: false,
    };
    Forecast {
        observed_at_unix_ms: 1000,
        level: 100,
        experience: 1000,
        regular_blessings: 1,
        twist: false,
        promotion_benefit: promoted,
        red_or_black: false,
        adventurer: false,
        pve: scenario,
        pvp: None,
    }
}

#[test]
fn n007_promotion_view_shows_title_regeneration_and_soul_cap_200() {
    let promoted = promotion_view(&forecast(true));
    assert_eq!(
        (
            promoted.promoted,
            promoted.soul_cap,
            promoted.soul_tick_seconds
        ),
        (true, 200, 15)
    );
    let text = promotion_text(&forecast(true));
    assert!(text.contains("promoted vocation title"));
    assert!(text.contains("faster health/mana regeneration"));
    assert!(text.contains("soul cap 200"));
    let base = promotion_view(&forecast(false));
    assert_eq!((base.promoted, base.soul_cap), (false, 100));
    assert!(promotion_text(&forecast(false)).contains("not promoted"));
    assert!(protection_heading(&forecast(true)).contains("soul cap 200"));
}

#[test]
fn t087_offence_sources_expose_interactive_source_descriptions()
-> Result<(), Box<dyn std::error::Error>> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    let command = augment_request(&mut p, AugmentIntent::Refresh).ok_or("request")?;
    let query = Query::decode(&command.payload).map_err(|_| "query")?;
    let stats = CombatStats::new(CharacterTab::Offence, 9, 1, 2).map_err(|_| "stats")?;
    let facts = Snapshot {
        known_sources: 5,
        sources: vec![
            Source {
                kind: SourceKind::Equipment,
                slot: 5,
                key: "oteryn:item/weapon".into(),
            },
            Source {
                kind: SourceKind::Wheel,
                slot: 0,
                key: "Current Wheel".into(),
            },
        ],
        rows: vec![
            Augment {
                source: 0,
                spell: "A spell".into(),
                effect: Effect::CriticalChance,
                value: 250,
            },
            Augment {
                source: 1,
                spell: "B spell".into(),
                effect: Effect::WheelStage,
                value: 2,
            },
        ],
    };
    let page = Page::from_snapshot([0; 16], query, stats, Some(facts)).map_err(|_| "page")?;
    p.target = query.target;
    assert!(apply_result(
        &mut p,
        &accepted(page.encode().map_err(|_| "encode")?)
    ));
    let rows = augment_rows(&p);
    assert_eq!(rows.len(), 2);
    assert!(
        rows[0]
            .detail
            .contains("Source: Equipment | Equipment slot 5 | oteryn:item/weapon")
    );
    assert!(rows[0].detail.contains("Cast, vocation and target"));
    assert!(rows[1].detail.contains("Source: Wheel | Current Wheel"));
    assert!(rows[1].detail.contains("Current Wheel augment stage"));
    Ok(())
}

#[test]
fn t088_defence_table_lists_every_defence_stat_with_source_and_detail()
-> Result<(), Box<dyn std::error::Error>> {
    let mut stats = CombatStats::new(CharacterTab::Defence, 5, 1, 1).map_err(|_| "stats")?;
    stats
        .set(CombatStat::MitigationEquipment, 20000)
        .map_err(|_| "equipment")?;
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::Defence,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    assert!(apply_result(
        &mut p,
        &accepted(TabView::Combat(stats).encode().map_err(|_| "encode")?)
    ));
    let rows = combat_rows(&p);
    assert_eq!(rows.len(), CombatStat::for_tab(CharacterTab::Defence).len());
    let equipment = rows
        .iter()
        .find(|r| r.key == CombatStat::MitigationEquipment)
        .ok_or("row")?;
    assert_eq!(
        (equipment.source, equipment.value.as_str()),
        ("Equipment", "2.0000%")
    );
    assert!(combat_detail(&p, CombatStat::AbsorbFire).contains("reduces incoming damage"));
    Ok(())
}

#[test]
fn t089_resource_presentation_bars_cap_stamina_and_offline_training() {
    let mut p = CyclopediaCharacterPanel::default();
    assert_eq!(overview_bars(&p), None);
    p.general = Some(GeneralStats {
        stamina_minutes: 5000,
        offline_training_minutes: 100,
        ..Default::default()
    });
    assert_eq!(overview_bars(&p), Some([(2520, 2520), (100, 720)]));
    p.misc_general = p.general;
    p.tab = CharacterTab::Misc;
    assert_eq!(misc_bars(&p), Some([(2520, 2520), (100, 720)]));
    p.tab = CharacterTab::GeneralStats;
    assert_eq!(misc_bars(&p), None);
}

#[test]
fn t090_death_table_keeps_each_confirmed_death_with_time_and_cause()
-> Result<(), Box<dyn std::error::Error>> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::RecentDeaths,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    let view = TabView::RecentDeaths(vec![
        (200, "Died at Level 50 by a troll".into()),
        (100, "Died at Level 49 by a player".into()),
    ]);
    assert!(apply_result(
        &mut p,
        &accepted(view.encode().map_err(|_| "encode")?)
    ));
    let rows = history_rows(&p);
    assert_eq!(rows.iter().map(|r| r.at).collect::<Vec<_>>(), [200, 100]);
    assert!(rows[1].description.contains("by a player"));
    // A death table is not narrowed by a kill-type filter.
    p.kill_filter = Some(KillStatus::GuildWar);
    assert_eq!(history_rows(&p).len(), 2);
    Ok(())
}

#[test]
fn t091_kill_type_filters_select_each_kill_kind() -> Result<(), Box<dyn std::error::Error>> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::RecentPvpKills,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Tab).is_some());
    let view = TabView::RecentPvpKills(vec![
        (1, KillStatus::GuildWar, "war".into()),
        (2, KillStatus::Unjustified, "unjust".into()),
        (3, KillStatus::Unjustified, "unjust two".into()),
    ]);
    assert!(apply_result(
        &mut p,
        &accepted(view.encode().map_err(|_| "encode")?)
    ));
    for (filter, expected) in [
        (Some(KillStatus::Unjustified), 2),
        (Some(KillStatus::GuildWar), 1),
        (None, 3),
    ] {
        p.kill_filter = filter;
        let rows = history_rows(&p);
        assert_eq!(rows.len(), expected);
        assert!(rows.iter().all(|r| filter.is_none() || r.status == filter));
    }
    Ok(())
}
