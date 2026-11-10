//! Minimal crash evidence. Panic text, environment values and memory are never collected.
use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
};

const STARTUP_LOG: &str = "startup.log";
const LAUNCHER_EXIT_CODE: &str = "launcher-exit-code.txt";

fn user_data_file(name: &str) -> Option<PathBuf> {
    crate::native_login::settings_path()
        .ok()?
        .parent()
        .map(|directory| directory.join(name))
}

fn append_startup_at(path: &Path, component: &str, detail: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{component}: {detail}")
}

/// Retain a secret-free startup failure for a window-subsystem process whose stderr is hidden.
pub fn record_startup_failure(component: &str, detail: &str) {
    if let Some(path) = user_data_file(STARTUP_LOG) {
        let _ = append_startup_at(&path, component, detail);
    }
}

/// Publish the launcher's own result so unattended installer diagnostics can distinguish spawn
/// failure from a client that started and exited.
pub fn record_launcher_exit_code(code: u8) {
    let Some(path) = user_data_file(LAUNCHER_EXIT_CODE) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, format!("{code}\n"));
}

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
        lfmt!(
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
/// Review retained evidence, removing it only after declining or durably saving approved text.
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
    let preview = crate::support_delivery::preview(&preview);
    if preview.len() > 4096 {
        return Err(io::Error::other("Crash report is too large"));
    }
    let approved = consent(&preview);
    if approved {
        save(&preview, destination, true)?;
    }
    std::fs::remove_file(pending)?;
    Ok(approved)
}
/// Preview and delivery consent run on the UI thread after a previous crash.
#[cfg(windows)]
pub fn review_pending() {
    let Some(path) = pending_path() else { return };
    if !path.exists() {
        return;
    }
    let destination = path.with_file_name("approved-crash.txt");
    match review_pending_at(&path, &destination, |preview| {
        consent_dialog(&lfmt!(
            "{preview}\nSend exactly this report to the Oteryn game server on your next signed-in play session? Yes queues delivery; No discards it.",
            preview = preview
        ))
    }) {
        Ok(true) => crate::win_mutex::show_error(tr!(
            "Crash report queued for delivery on your next play session."
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
            wide(tr!("Oteryn — local report consent")).as_ptr(),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES
    }
}

#[cfg(test)]
mod tests {
    use super::append_startup_at;

    #[test]
    fn startup_failures_are_appended_for_later_diagnosis() -> std::io::Result<()> {
        let directory =
            std::env::temp_dir().join(format!("oteryn-startup-log-{}", std::process::id()));
        let path = directory.join("nested").join("startup.log");
        let _ = std::fs::remove_dir_all(&directory);
        append_startup_at(&path, "launcher", "spawn failed")?;
        append_startup_at(&path, "client", "window creation failed")?;
        assert_eq!(
            std::fs::read_to_string(&path)?,
            "launcher: spawn failed\nclient: window creation failed\n"
        );
        std::fs::remove_dir_all(directory)
    }
}
