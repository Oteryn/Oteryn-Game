use super::*;
use crate::cyclopedia_character::{
    CyclopediaCharacterPanel, Intent, TabExtensions, apply_result, overview_cards, promotion_text,
    promotion_view, protection_heading, request,
};
use oteryn_client_runtime::cyclopedia_character::{
    CharacterTab, GeneralStats, TabExtension, TabView,
    protection::{Consumption, Forecast, Scenario},
};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

fn general_reply(vocation: Option<u8>) -> Result<Vec<u8>, &'static str> {
    TabView::GeneralStats(GeneralStats {
        experience: 5000,
        level: 50,
        magic_level: 12,
        skills: [10; 7],
        stamina_minutes: 2520,
        offline_training_minutes: 0,
        last_vitals: Some((100, 50)),
    })
    .encode_extended(&TabExtension {
        vocation_id: vocation,
        deaths: None,
    })
    .map_err(|_| "general stats reply")
}

fn accepted(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}

fn read(extensions: TabExtensions, payload: Vec<u8>) -> Option<CyclopediaCharacterPanel> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        extensions,
        ..Default::default()
    };
    request(&mut p, Intent::Tab)?;
    apply_result(&mut p, &accepted(payload)).then_some(p)
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

const SELECTED: TabExtensions = TabExtensions {
    vocation: true,
    death_details: false,
};

#[test]
fn c0487_every_vocation_id_has_its_exact_title() {
    let titles: Vec<_> = (1..=11).filter_map(vocation_title).collect();
    assert_eq!(titles.len(), 11);
    assert_eq!(vocation_title(3), Some("Elder Druid"));
    assert_eq!(vocation_title(11), Some("Exalted Monk"));
    assert_eq!(vocation_title(0), None);
    assert_eq!(vocation_title(12), None);
}

#[test]
fn c0487_selected_general_stats_reply_stores_the_vocation_and_shows_the_exact_title()
-> Result<(), &'static str> {
    let p = read(SELECTED, general_reply(Some(7))?).ok_or("selected reply decodes")?;
    assert_eq!(p.vocation_id, Some(7));
    assert!(p.general.is_some());
    let cards = overview_cards(&p);
    assert!(cards.contains(&("Vocation", "Elite Knight".into())));
    let text = promotion_text(&forecast(true), p.vocation_id);
    assert!(text.contains("promoted rank (Elite Knight)"), "{text}");
    assert_eq!(
        promotion_view(&forecast(true), p.vocation_id).title,
        Some("Elite Knight")
    );
    assert!(protection_heading(&forecast(true), p.vocation_id).contains("Elite Knight"));
    let base = promotion_text(&forecast(false), Some(6));
    assert!(
        base.contains("not promoted") && base.contains("(Knight)"),
        "{base}"
    );
    Ok(())
}

#[test]
fn c0487_unselected_session_keeps_the_old_reply_and_the_generic_text() -> Result<(), &'static str> {
    let p = read(TabExtensions::default(), general_reply(None)?).ok_or("old reply decodes")?;
    assert_eq!(p.vocation_id, None);
    assert!(
        overview_cards(&p)
            .iter()
            .all(|(label, _)| *label != "Vocation")
    );
    let text = promotion_text(&forecast(true), None);
    assert!(text.contains("promoted vocation title"), "{text}");
    assert_eq!(promotion_view(&forecast(true), None).title, None);
    // A session without 487 never accepts the trailing byte, and a selected one requires it.
    assert!(read(TabExtensions::default(), general_reply(Some(7))?).is_none());
    assert!(read(SELECTED, general_reply(None)?).is_none());
    Ok(())
}

#[test]
fn c0487_an_invalid_vocation_id_is_refused() -> Result<(), &'static str> {
    for invalid in [0, 12, 255] {
        let mut payload = general_reply(Some(1))?;
        *payload.last_mut().ok_or("payload")? = invalid;
        assert!(read(SELECTED, payload).is_none(), "id {invalid}");
    }
    let _ = CharacterTab::GeneralStats;
    Ok(())
}
