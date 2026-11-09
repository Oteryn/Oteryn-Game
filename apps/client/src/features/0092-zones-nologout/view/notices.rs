//! Bounded native dialog delivery; ordinary HUD messages remain available on overflow.
use oteryn_client_ui::modal_dialogs::NoticeDialog;

pub(super) fn enqueue(notice: NoticeDialog) {
    use std::sync::{
        OnceLock,
        mpsc::{SyncSender, sync_channel},
    };
    static INBOX: OnceLock<Option<SyncSender<NoticeDialog>>> = OnceLock::new();
    let sender = INBOX.get_or_init(|| {
        let (sender, receiver) = sync_channel::<NoticeDialog>(32);
        std::thread::Builder::new()
            .name("server-notice-dialogs".into())
            .spawn(move || {
                while let Ok(notice) = receiver.recv() {
                    crate::play::feature_0115_mail::view::native::dialogs::confirm(
                        std::ptr::null_mut(),
                        &notice.key,
                        "Important server notice",
                        &notice.message,
                    );
                }
            })
            .ok()
            .map(|_| sender)
    });
    if let Some(sender) = sender {
        // Presentation must not block the session or create unbounded native windows.
        // Overflow still remains visible in the ordinary server-message HUD/history.
        let _ = sender.try_send(notice);
    }
}
