//! Native confirmation used by the actual close/logout callers.
#![allow(unsafe_code)] // Synchronous message box borrows only terminated local UTF-16 buffers.
pub fn confirm_action(message: &str) -> bool {
    if std::env::args_os().any(|arg| arg == "--smoke") {
        return true;
    }
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IDYES, MB_DEFBUTTON2, MB_YESNO, MessageBoxW,
    };
    let text: Vec<_> = message.encode_utf16().chain(Some(0)).collect();
    let title: Vec<_> = "Oteryn confirmation"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: local NUL-terminated buffers outlive the synchronous call; no owner handle is borrowed.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_DEFBUTTON2,
        ) == IDYES
    }
}
