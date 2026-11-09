//! Confirmed settings transfers; no settings lock is held while prompting.
use super::*;
pub(super) unsafe fn apply(action: usize, hwnd: HWND, shared: &Arc<Mutex<Shared>>, path: &str) {
    unsafe {
        let confirmed = action == 111
            || MessageBoxW(
                hwnd,
                wide("Replace your current settings? A backup will be saved first.").as_ptr(),
                wide("Oteryn Settings").as_ptr(),
                MB_YESNO | MB_ICONQUESTION,
            ) == IDYES;
        let mut state = lock(shared);
        if !confirmed || state.shutdown {
            return;
        }
        let path = std::path::PathBuf::from(path);
        let result = match action {
            110 => state.import_from(&path),
            111 => state.export_to(&path),
            112 => state.restore_backup(),
            _ => state.reset_defaults(),
        };
        state.feedback = match result {
                        Ok(()) if action == 111 => "Export saved. Existing files are never overwritten.".into(),
                        Ok(()) => "Settings replaced and saved. The previous settings are available through Restore backup.".into(),
                        Err(error) => format!("Settings transfer failed: {error}"),
                    };
    }
}
