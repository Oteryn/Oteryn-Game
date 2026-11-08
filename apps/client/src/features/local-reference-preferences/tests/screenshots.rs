use crate::play::{
    SettingsState,
    screenshot::{CaptureSource, Pixels},
};

struct Source(Vec<bool>);
impl CaptureSource for Source {
    fn capture(&mut self, game_only: bool) -> std::io::Result<Pixels> {
        self.0.push(game_only);
        Pixels::new(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255])
    }
}

#[test]
fn manual_screenshot_caller_saves_real_pixels_and_selected_scope()
-> Result<(), Box<dyn std::error::Error>> {
    let folder = std::env::temp_dir().join(format!("t070-{}", std::process::id()));
    std::fs::create_dir_all(&folder)?;
    let mut state = SettingsState::new(Some(folder.join("settings.json")));
    let mut preferences = state.preferences.clone();
    preferences.screenshots.folder = folder.to_string_lossy().into_owned();
    preferences.screenshots.game_only = true;
    state.apply(preferences);
    let mut source = Source(Vec::new());
    assert!(state.take_screenshot(&mut source)?.is_none());
    state.request_screenshot();
    let path = state
        .take_screenshot(&mut source)?
        .ok_or("missing screenshot")?;
    let bytes = std::fs::read(&path)?;
    assert_eq!(&bytes[..2], b"BM");
    assert_eq!(&bytes[54..60], &[0, 0, 255, 0, 255, 0]);
    assert_eq!(source.0, [true]);
    let mut preferences = state.preferences.clone();
    preferences.screenshots.game_only = false;
    state.apply(preferences);
    state.request_screenshot();
    assert_ne!(
        state
            .take_screenshot(&mut source)?
            .ok_or("missing window capture")?,
        path
    );
    assert_eq!(source.0, [true, false]);
    assert!(
        !SettingsState::new(Some(folder.join("settings.json")))
            .preferences
            .screenshots
            .game_only
    );
    std::fs::remove_dir_all(folder)?;
    Ok(())
}
