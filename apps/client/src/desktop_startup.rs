//! Desktop process preparation that does not depend on the presentation layer: the saved
//! interface language and, on Windows, the consent review of a crash report left by an
//! earlier run. The desktop host calls it once before it opens its window.
use std::path::PathBuf;

/// Applies the saved language and reviews a pending crash report; a smoke run skips the
/// review dialog so packaging checks never wait for a person.
pub fn prepare(preferences: Option<PathBuf>, smoke: bool) {
    if let Some(path) = preferences
        && let Ok(store) = crate::play::PreferenceStore::load(path)
    {
        oteryn_client_ui::localization::set_default(store.preferences().language());
    }
    #[cfg(windows)]
    if !smoke {
        crate::crash_report::review_pending();
    }
    #[cfg(not(windows))]
    let _ = smoke;
}
