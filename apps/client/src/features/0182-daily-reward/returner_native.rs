//! A confirmed entitlement opens a native welcome notification; the form submits the real claim.
use crate::play::PlayLink;
pub(super) fn open_reward(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    crate::play::feature_0115_mail::view::native::open(link, super::returner_form(link));
}
pub(in crate::play) fn notify() -> bool {
    std::thread::Builder::new().name("returner-reward-notice".into()).spawn(|| {
        let title:Vec<u16>=tr!("Welcome back!").encode_utf16().chain(Some(0)).collect();
        let text:Vec<u16>=tr!("A welcome-back gift is available. Open Daily Reward (Ctrl+D), then Welcome-back reward, to claim it.").encode_utf16().chain(Some(0)).collect();
        // This thread owns both UTF-16 buffers until the native message box closes.
        #[allow(unsafe_code)]
        unsafe {windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(std::ptr::null_mut(),text.as_ptr(),title.as_ptr(),windows_sys::Win32::UI::WindowsAndMessaging::MB_OK | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION);}
    }).is_ok()
}
