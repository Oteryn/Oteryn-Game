//! Product identification and an optional, local-only client experience questionnaire.
use std::{io, path::Path};
pub fn about_text() -> String {
    format!(
        "Oteryn Game Client\r\nRelease: {}\r\nBuild: {}\r\n{} / {}\r\nLicense: MPL-2.0\r\nProject: https://github.com/Oteryn/Oteryn-Game\r\nSurvey participation is optional. Responses are saved locally only after review and consent.",
        crate::RELEASE_ID,
        crate::CLIENT_BUILD,
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}
pub struct Survey {
    easy_sign_in: bool,
}
impl Survey {
    pub fn new(easy_sign_in: bool) -> Self {
        Self { easy_sign_in }
    }
    pub fn preview(&self) -> String {
        format!(
            "Oteryn client survey\nBuild: {}\nQuestion: Was browser sign-in easy?\nAnswer: {}\nNo identity, account information or credentials included.\n",
            crate::CLIENT_BUILD,
            if self.easy_sign_in {
                "Easy"
            } else {
                "Difficult"
            }
        )
    }
    pub fn save(&self, path: &Path, consent: bool) -> io::Result<bool> {
        crate::crash_report::save(&self.preview(), path, consent)
    }
}
/// The sign-in survey's invitation, answer, preview and consent flow.
pub fn review_survey_at(path: &Path, mut ask: impl FnMut(&str) -> bool) -> io::Result<bool> {
    if !ask(
        "Would you like to answer an optional one-question Oteryn client survey? You can decline to save your answer after reviewing it. Nothing is uploaded.",
    ) {
        return Ok(false);
    }
    let survey = Survey::new(ask("Was browser sign-in easy? Yes = Easy; No = Difficult."));
    if !ask(&format!(
        "{}\nSave this response as a local file?",
        survey.preview()
    )) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    survey.save(path, true)
}
#[cfg(windows)]
pub(crate) fn run_survey() {
    let result = (|| -> io::Result<Option<std::path::PathBuf>> {
        let settings = crate::native_login::settings_path().map_err(io::Error::other)?;
        let directory = settings
            .parent()
            .ok_or_else(|| io::Error::other("No configuration directory"))?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = directory.join(format!("client-survey-{stamp}.txt"));
        Ok(review_survey_at(&path, crate::crash_report::consent_dialog)?.then_some(path))
    })();
    match result {
        Ok(None) => {}
        Ok(Some(path)) => show_information(&format!(
            "Survey response saved to {}. You can share or delete this file yourself.",
            path.display()
        )),
        Err(_) => show_information("Cannot save the response. No response was sent."),
    }
}
#[cfg(windows)]
// Native informational dialog; both UTF-16 buffers outlive the synchronous call.
#[allow(unsafe_code)]
pub(crate) fn show_information(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONINFORMATION, MB_OK, MessageBoxW};
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
            wide("About Oteryn").as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
