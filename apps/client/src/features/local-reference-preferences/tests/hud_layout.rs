#[test]
fn t010_hud_sidebars_drag_dock_resize_hide_reorder_and_survive_restart()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{HudAction, HudDock, HudPanel, SettingsState};
    let path =
        std::env::temp_dir().join(format!("cg-gamewin-hud-layout-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut play = view()?;
    settings.restore(&mut play, [960, 540])?;
    play.hud_action(HudAction::Visible(HudPanel::Conditions, true), [960, 540])?;
    play.hud_action(
        HudAction::Dock(HudPanel::Conditions, HudDock::Left),
        [960, 540],
    )?;
    play.hud_action(
        HudAction::Dock(HudPanel::Resources, HudDock::Left),
        [960, 540],
    )?;
    play.hud_action(HudAction::Earlier(HudPanel::Conditions), [960, 540])?;
    let layout = &play.preferences().hud_layout;
    let conditions = layout
        .rect(HudPanel::Conditions, [960, 540], 100)
        .ok_or("conditions hidden")?;
    let resources = layout
        .rect(HudPanel::Resources, [960, 540], 100)
        .ok_or("resources hidden")?;
    assert!(conditions[1] < resources[1]);
    play.hud_action(
        HudAction::Dragged(HudPanel::Resources, [700, 150, 240, 160]),
        [960, 540],
    )?;
    assert_eq!(
        play.preferences().hud_layout.pane(HudPanel::Resources).dock,
        HudDock::Right
    );
    assert_eq!(
        play.preferences()
            .hud_layout
            .pane(HudPanel::Resources)
            .bounds[2..],
        [240, 160]
    );
    play.hud_action(HudAction::Visible(HudPanel::Resources, false), [960, 540])?;
    assert!(
        play.preferences()
            .hud_layout
            .rect(HudPanel::Resources, [960, 540], 100)
            .is_none()
    );
    let expected = play.preferences().hud_layout.clone();
    settings.capture(&mut play)?;
    assert_eq!(play.preferences().hud_layout, expected);
    assert_eq!(settings.preferences.hud_layout, expected);
    let mut restarted = view()?;
    SettingsState::new(Some(path.clone())).restore(&mut restarted, [960, 540])?;
    assert_eq!(
        restarted.preferences().hud_layout,
        play.preferences().hud_layout
    );
    assert_eq!(restarted.preferences().hud_layout, expected);
    let mut invalid = restarted.preferences().clone();
    invalid.hud_layout.order = [HudPanel::Resources; 4];
    assert!(restarted.set_preferences(invalid).is_err());
    // A newer settings edit must win over a stale native view during capture.
    let mut newer = settings.preferences.clone();
    newer.hud_layout.apply(HudAction::Visible(HudPanel::Resources, true), [960, 540], 100);
    settings.apply(newer);
    settings.capture(&mut play)?;
    assert!(settings.preferences.hud_layout.pane(HudPanel::Resources).visible);
    assert!(play.preferences().hud_layout.pane(HudPanel::Resources).visible);
    std::fs::remove_file(path)?;
    Ok(())
}
