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
#[cfg(windows)]
pub(crate) fn run_survey() {
    use crate::crash_report::consent_dialog;
    if !consent_dialog(tr!(
        "Would you like to answer an optional one-question Oteryn client survey? You can decline to save your answer after reviewing it. Nothing is uploaded."
    )) {
        return;
    }
    let survey = Survey::new(consent_dialog(tr!(
        "Was browser sign-in easy? Yes = Easy; No = Difficult."
    )));
    if !consent_dialog(&lfmt!(
        "{}\nSave this response as a local file?",
        survey.preview()
    )) {
        return;
    }
    let result = (|| -> io::Result<std::path::PathBuf> {
        let settings = crate::native_login::settings_path().map_err(io::Error::other)?;
        let directory = settings
            .parent()
            .ok_or_else(|| io::Error::other(tr!("No configuration directory")))?;
        std::fs::create_dir_all(directory)?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = directory.join(format!("client-survey-{stamp}.txt"));
        survey.save(&path, true)?;
        Ok(path)
    })();
    match result {
        Ok(path) => show_information(&lfmt!(
            "Survey response saved to {}. You can share or delete this file yourself.",
            path.display()
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
