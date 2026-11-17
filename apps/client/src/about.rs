//! Product identification and an optional client experience questionnaire.
use std::{io, path::Path};
pub fn about_text() -> String {
    format!(
        "Oteryn Game Client\r\nRelease: {}\r\nBuild: {}\r\n{} / {}\r\nLicense: MPL-2.0\r\nProject: https://github.com/Oteryn/Oteryn-Game\r\nSurvey participation is optional. Responses are queued for delivery to the Oteryn game server only after review and consent.",
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
        lfmt!(
            "Oteryn client survey\nBuild: {}\nQuestion: Was browser sign-in easy?\nAnswer: {}\nNo identity, account information or credentials included.\n",
            crate::CLIENT_BUILD,
            if self.easy_sign_in {
                tr!("Easy")
            } else {
                tr!("Difficult")
            }
        )
    }
    pub fn save(&self, path: &Path, consent: bool) -> io::Result<bool> {
        crate::crash_report::save(&self.preview(), path, consent)
    }
}
/// The sign-in survey's invitation, answer, preview and consent flow.
pub fn review_survey_at(path: &Path, mut ask: impl FnMut(&str) -> bool) -> io::Result<bool> {
    if !ask(tr!(
        "Would you like to answer an optional one-question Oteryn client survey? You can decline delivery after reviewing your answer."
    )) {
        return Ok(false);
    }
    let survey = Survey::new(ask(tr!(
        "Was browser sign-in easy? Yes = Easy; No = Difficult."
    )));
    let preview = crate::support_delivery::preview(&survey.preview());
    if !ask(&lfmt!(
        "{}\nSend exactly this response to the Oteryn game server on your next signed-in play session?",
        preview
    )) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::crash_report::save(&preview, path, true)
}
#[cfg(windows)]
pub(crate) fn run_survey() {
    let result = (|| -> io::Result<Option<std::path::PathBuf>> {
        let settings = crate::native_login::settings_path().map_err(io::Error::other)?;
        let directory = settings
            .parent()
            .ok_or_else(|| io::Error::other(tr!("No configuration directory")))?;
        let path = directory.join("approved-survey.txt");
        Ok(review_survey_at(&path, crate::crash_report::consent_dialog)?.then_some(path))
    })();
    match result {
        Ok(None) => {}
        Ok(Some(_)) => show_information(tr!(
            "Survey response queued for delivery on your next play session."
        )),
        Err(_) => show_information(tr!("Cannot save the response. No response was sent.")),
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
            wide(tr!("About Oteryn")).as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
