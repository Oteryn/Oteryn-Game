//! Minimal local crash evidence. Panic text, environment values and memory are never collected.
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct CrashReport {
    source: String,
    line: u32,
}
impl CrashReport {
    pub fn capture(source: Option<&str>, line: u32) -> Self {
        let name = source
            .unwrap_or("unknown")
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("unknown");
        Self {
            source: name
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                .take(120)
                .collect(),
            line,
        }
    }
    pub fn preview(&self) -> String {
        format!(
            "Oteryn crash report\nBuild: {}\nOS: {}\nArchitecture: {}\nSource: {}:{}\nNo panic text, account data, memory or environment values collected.\n",
            crate::CLIENT_BUILD,
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.source,
            self.line
        )
    }
    pub fn save(&self, path: &Path, consent: bool) -> io::Result<bool> {
        save(&self.preview(), path, consent)
    }
}
pub(crate) fn save(preview: &str, path: &Path, consent: bool) -> io::Result<bool> {
    if !consent {
        return Ok(false);
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(preview.as_bytes())?;
    file.sync_all()?;
    Ok(true)
}
fn pending_path() -> Option<PathBuf> {
    crate::native_login::settings_path()
        .ok()?
        .parent()
        .map(|p| p.join("pending-crash.txt"))
}
/// Install once at process startup; retain the first crash until the user reviews it.
pub fn install_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(path) = pending_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let location = info.location();
            let report =
                CrashReport::capture(location.map(|l| l.file()), location.map_or(0, |l| l.line()));
            let _ = report.save(&path, true);
        }
        previous(info);
    }));
}
/// Review the retained report, discarding it only after a decision or a successful export.
/// The native login caller supplies its synchronous consent dialog.
pub fn review_pending_at(
    pending: &Path,
    destination: &Path,
    consent: impl FnOnce(&str) -> bool,
) -> io::Result<bool> {
    use std::io::Read;
    let file = std::fs::File::open(pending)?;
    let mut preview = String::new();
    file.take(4097).read_to_string(&mut preview)?;
    if preview.len() > 4096 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Crash report is too large",
        ));
    }
    let approved = consent(&preview);
    if approved {
        save(&preview, destination, true)?;
    }
    std::fs::remove_file(pending)?;
    Ok(approved)
}
/// Review and optional local export run on the UI thread, after a previous crash.
#[cfg(windows)]
pub fn review_pending() {
    let Some(path) = pending_path() else { return };
    if !path.exists() {
        return;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let destination = path.with_file_name(format!("crash-report-{stamp}.txt"));
    match review_pending_at(&path, &destination, |preview| {
        consent_dialog(&format!(
            "{preview}\nSave a local report to share manually? Nothing is uploaded."
        ))
    }) {
        Ok(true) => crate::win_mutex::show_error(&format!(
            "Crash report saved to {}",
            destination.display()
        )),
        Ok(false) => {}
        Err(_) => crate::win_mutex::show_error(
            "Cannot review or save the crash report. The pending report was retained.",
        ),
    }
}
#[cfg(windows)]
// This is the native consent dialog; pointers live for the synchronous call.
#[allow(unsafe_code)]
pub(crate) fn consent_dialog(message: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_YESNO, MessageBoxW,
    };
    let wide = |s: &str| {
        s.replace('\0', "\u{fffd}")
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(message).as_ptr(),
            wide("Oteryn — local report consent").as_ptr(),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES
    }
}
