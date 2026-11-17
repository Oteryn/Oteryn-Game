//! Safe maintenance UI and the install/client mutex handshake.
#![allow(unsafe_code)] // Narrow Windows message-box FFI, no retained pointers.
use super::*;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IDNO, IDYES, MB_OK, MB_YESNOCANCEL, MessageBoxW,
};
fn dialog(text: &str, style: u32) -> i32 {
    let text: Vec<_> = text.encode_utf16().chain(Some(0)).collect();
    let title: Vec<_> = "Oteryn updates and repair"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: both UTF-16 buffers terminate in NUL and live throughout this synchronous call.
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), style) }
}
pub(super) fn acquire_maintenance() -> Result<crate::win_mutex::NamedMutex, UpdateError> {
    let sid = crate::win_mutex::current_user_sid().map_err(|_| UpdateError::Unavailable)?;
    let name = crate::win_mutex::setup_mutex_name(&sid);
    if crate::win_mutex::exists(&name).map_err(|_| UpdateError::Unavailable)? {
        return Err(UpdateError::Unavailable);
    }
    let guard = crate::win_mutex::create(&name).map_err(|_| UpdateError::Unavailable)?;
    if crate::win_mutex::exists(&crate::win_mutex::client_mutex_name(&sid))
        .map_err(|_| UpdateError::Unavailable)?
    {
        return Err(UpdateError::Unavailable);
    }
    Ok(guard)
}
pub(super) fn show(directory: &Path) -> Result<String, UpdateError> {
    let runtime = tokio::runtime::Runtime::new().map_err(|_| UpdateError::Unavailable)?;
    let version = runtime.block_on(maintain(directory, "stable", Action::Check))?;
    let action = match dialog(
        &format!(
            "{version}\n\nYes: download and install update\nNo: verify installed files\nCancel: close"
        ),
        MB_YESNOCANCEL,
    ) {
        IDYES => Action::Update,
        IDNO => Action::Verify,
        _ => return Ok(version),
    };
    let mut result = runtime.block_on(maintain(directory, "stable", action));
    if result.is_err()
        && action == Action::Verify
        && dialog(
            "Verification failed. Download and repair from the signed release?",
            MB_YESNOCANCEL,
        ) == IDYES
    {
        result = runtime.block_on(maintain(directory, "stable", Action::Repair));
    }
    let report = match &result {
        Ok(report) => report.clone(),
        Err(error) => error.to_string(),
    };
    dialog(&report, MB_OK);
    result
}
