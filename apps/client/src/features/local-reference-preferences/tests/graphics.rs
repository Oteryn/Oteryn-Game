use super::*;
#[test]
fn t060_graphics_settings_restore_into_real_scene_and_presentation_request()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;
    let path = std::env::temp_dir().join(format!("t060-graphics-{}.json", std::process::id()));
    let mut state = SettingsState::new(Some(path.clone()));
    let mut prefs = state.preferences.clone();
    prefs.ui_scale = 100;
    prefs.graphics.fullscreen = true;
    prefs.graphics.vsync = false;
    prefs.graphics.integer_scaling = true;
    state.apply(prefs);
    let mut state = SettingsState::new(Some(path.clone()));
    let mut play = view()?;
    state.restore(&mut play, [4096, 4096])?;
    assert_eq!(play.scene().view().tile_px(), 64);
    assert!(play.preferences().graphics.fullscreen);
    assert_eq!(
        play.preferences().graphics.presentation(),
        oteryn_renderer::PresentationMode::Unlocked
    );
    let mut changed = play.preferences().clone();
    changed.graphics.integer_scaling = false;
    play.set_preferences(changed)?;
    assert_eq!(play.scene().view().tile_px(), 48);
    assert_eq!(oteryn_renderer::GRAPHICS_ENGINE.label(), "Direct3D 12");
    assert_eq!(oteryn_renderer::ANTIALIASING.samples(), 1);
    let mut integral = play.preferences().clone();
    integral.graphics.integer_scaling = true;
    for zoom in [80, 100, 125, 150, 180] {
        integral.ui_scale = zoom;
        play.set_preferences(integral.clone())?;
        assert_eq!(
            play.scene().view().tile_px() % oteryn_renderer::PAGE_CELL_PX,
            0
        );
    }
    std::fs::remove_file(path)?;
    Ok(())
}
