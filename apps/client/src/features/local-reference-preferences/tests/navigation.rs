#[test]
fn basic_options_navigate_without_changing_preferences() {
    use crate::play::{SettingsPage, SettingsState};
    let mut state = SettingsState::new(None);
    let before = state.preferences.clone();
    assert_eq!(state.page, SettingsPage::Gameplay);
    for page in SettingsPage::ALL {
        state.page = page;
        assert!(!page.label().is_empty());
        assert_eq!(state.preferences, before);
    }
    assert_eq!(SettingsPage::ALL.len(), 22);
    for added in [
        SettingsPage::Screenshots,
        SettingsPage::HelpGuide,
        SettingsPage::MinimapTransfer,
        SettingsPage::ConsoleFilters,
        SettingsPage::ActionBarLayout,
        SettingsPage::Language,
    ] {
        assert!(SettingsPage::ALL.contains(&added));
    }
    assert_eq!(SettingsPage::ALL[9], SettingsPage::Shortcuts);
    assert_eq!(SettingsPage::ALL[10], SettingsPage::Graphics);
    assert_eq!(SettingsPage::ALL[11], SettingsPage::Effects);
    assert_eq!(SettingsPage::ALL[13], SettingsPage::Mixer);
    assert_eq!(SettingsPage::ALL[14], SettingsPage::Battle);
}
