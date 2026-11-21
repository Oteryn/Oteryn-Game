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
/// The survey's invitation; No ends it.
pub fn survey_invitation() -> &'static str {
    tr!(
        "Would you like to answer an optional one-question Oteryn client survey? You can decline delivery after reviewing your answer."
    )
}
/// The survey's one question; Yes answers Easy, No answers Difficult.
pub fn survey_question() -> &'static str {
    tr!("Was browser sign-in easy? Yes = Easy; No = Difficult.")
}
/// The exact response text that consent queues, with its delivery reference.
pub fn survey_response(easy_sign_in: bool) -> String {
    crate::support_delivery::preview(&Survey::new(easy_sign_in).preview())
}
/// The consent question under the response it would queue.
pub fn survey_consent(response: &str) -> String {
    lfmt!(
        "{}\nSend exactly this response to the Oteryn game server on your next signed-in play session?",
        response
    )
}
/// Queues a consented response for delivery on the next signed-in play session.
pub fn queue_survey_response(path: &Path, response: &str) -> io::Result<bool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::crash_report::save(response, path, true)
}
/// The sign-in survey's invitation, answer, preview and consent flow.
pub fn review_survey_at(path: &Path, mut ask: impl FnMut(&str) -> bool) -> io::Result<bool> {
    if !ask(survey_invitation()) {
        return Ok(false);
    }
    let response = survey_response(ask(survey_question()));
    if !ask(&survey_consent(&response)) {
        return Ok(false);
    }
    queue_survey_response(path, &response)
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
