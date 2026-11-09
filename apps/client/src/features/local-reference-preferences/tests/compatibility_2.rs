use super::*;
#[test]
fn shortcut_order_persists_and_buttons_open_real_panels() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::{PreferenceStore, ReferencePreferences, SHORTCUTS};
    let mut prefs = ReferencePreferences::default();
    let panels = prefs.visible_panels.clone();
    let before = prefs.clone();
    assert!(prefs.add_shortcut("unregistered").is_err());
    assert!(prefs.add_shortcut("battle").is_err());
    assert!(prefs.move_shortcut(0, usize::MAX).is_err());
    assert_eq!(prefs, before);
    prefs.add_shortcut("quest-log")?;
    let last = prefs.shortcuts.len() - 1;
    prefs.move_shortcut(last, 0)?;
    assert_eq!(prefs.shortcuts[0], "quest-log");
    prefs.remove_shortcut(1)?;
    assert!(!prefs.shortcuts.iter().any(|id| id == "battle"));
    assert_eq!(prefs.visible_panels, panels);
    let path =
        std::env::temp_dir().join(format!("clientgap-shortcuts-{}.json", std::process::id()));
    let mut store = PreferenceStore::load(path.clone())?;
    store.apply(prefs.clone())?;
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences().shortcuts,
        prefs.shortcuts
    );
    std::fs::remove_file(path)?;
    let (link, _, _) = crate::play::play_channel();
    assert!(!link.battle_panel().open);
    assert!(link.activate_shortcut("battle"));
    assert!(link.battle_panel().open);
    assert!(link.activate_shortcut("battle"));
    assert!(!link.battle_panel().open);
    assert!(!link.activate_shortcut("unregistered"));
    for &(id, _) in SHORTCUTS {
        assert!(
            link.activate_shortcut(id),
            "shortcut {id} must reach its registered panel entry point"
        );
    }
    let good = prefs.clone();
    prefs.shortcuts.push("unknown".into());
    let mut settings = crate::play::SettingsState::new(None);
    settings.apply(good.clone());
    settings.apply(prefs);
    assert_eq!(settings.preferences, good);
    Ok(())
}

#[test]
fn actor_placement_does_not_select_a_character_preference_file()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.set_own_identity([31; 16]);
    assert_eq!(
        play.preference_character(),
        None,
        "runtime placement is not a durable character id"
    );
    play.set_preference_character([7; 16]);
    play.set_own_identity([32; 16]);
    assert_eq!(play.preference_character(), Some([7; 16]));
    play = play.with_entities(vec![])?;
    assert_eq!(
        play.preference_character(),
        Some([7; 16]),
        "scene rebuilding retains the admitted character"
    );
    let root = std::env::temp_dir().join(format!("stable-character-scope-{}", std::process::id()));
    let path = root.join("preferences.json");
    let mut settings = crate::play::SettingsState::new(Some(path.clone()));
    settings.select_character(play.preference_character());
    let mut prefs = settings.preferences.clone();
    prefs.ui_scale = 140;
    settings.apply(prefs);
    let mut next_admission = view()?;
    next_admission.set_own_identity([33; 16]);
    next_admission.set_preference_character([7; 16]);
    let mut restarted = crate::play::SettingsState::new(Some(path));
    restarted.select_character(next_admission.preference_character());
    assert_eq!(restarted.preferences.ui_scale, 140);
    next_admission.set_preference_character([8; 16]);
    restarted.select_character(next_admission.preference_character());
    assert_eq!(restarted.preferences.ui_scale, 100);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn hud_arcs_project_server_vitals_and_persist_geometry_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = crate::play::play_channel();
    let mut play = view()?;
    let mut prefs = play.preferences().clone();
    prefs.arcs = crate::play::ArcPreferences {
        health: true,
        mana: true,
        distance: 25,
        size: 60,
        opacity: 50,
    };
    play.set_preferences(prefs.clone())?;
    assert!(play.hud_arc_layout().is_none());
    outbox.vitals(Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 25,
        max_mana: 100,
        ..Default::default()
    }));
    play.tick(&link).map_err(|_| "tick")?;
    let layout = play.hud_arc_layout().ok_or("arc layout")?;
    assert_eq!(layout.alpha, 127);
    assert_eq!((layout.width, layout.height), (186, 136));
    assert_eq!(layout.strokes.len(), 2);
    assert_eq!(layout.strokes[0].filled_segments, 40);
    assert_eq!(layout.strokes[1].filled_segments, 20);
    assert!(
        layout
            .strokes
            .iter()
            .flat_map(|stroke| &stroke.points)
            .all(|p| p[0] >= 0 && p[1] >= 0 && p[0] <= layout.width && p[1] <= layout.height)
    );
    let path = std::env::temp_dir().join(format!("arc-preferences-{}.json", std::process::id()));
    let mut settings = crate::play::SettingsState::new(Some(path.clone()));
    settings.apply(prefs.clone());
    assert_eq!(
        crate::play::SettingsState::new(Some(path.clone()))
            .preferences
            .arcs,
        prefs.arcs
    );
    let mut invalid = prefs.clone();
    invalid.arcs.opacity = 101;
    settings.apply(invalid);
    assert_eq!(settings.preferences, prefs);
    prefs.arcs.health = false;
    prefs.arcs.mana = false;
    play.set_preferences(prefs)?;
    assert!(play.hud_arc_layout().is_none());
    std::fs::remove_file(path)?;
    Ok(())
}
