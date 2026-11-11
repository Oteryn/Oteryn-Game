use super::protocol::Submission;
use oteryn_client_runtime::server_modal::input::DialogInput;
use oteryn_client_ui::server_modal::Dialog;
use oteryn_client_ui::server_modal_input::Entry;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
pub(super) fn open(
    dialog: Dialog,
    input: Option<DialogInput>,
    (sender, entry): (Sender<Submission>, Arc<Mutex<Entry>>),
) {
    #[cfg(all(windows, not(test)))]
    {
        let _ = std::thread::Builder::new()
            .name("server-modal".into())
            .spawn(move || {
                ACTIVE.store(true, std::sync::atomic::Ordering::Release);
                native::run(dialog, input, sender, entry);
                ACTIVE.store(false, std::sync::atomic::Ordering::Release);
            });
    }
    #[cfg(any(not(windows), test))]
    {
        let _ = (dialog, input, sender, entry);
    }
}
#[cfg(all(windows, not(test)))]
#[path = "view/native.rs"]
mod native;

#[cfg(all(windows, not(test)))]
static ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub(in crate::play) fn active() -> bool {
    #[cfg(all(windows, not(test)))]
    {
        ACTIVE.load(std::sync::atomic::Ordering::Acquire)
    }
    #[cfg(any(not(windows), test))]
    {
        false
    }
}
