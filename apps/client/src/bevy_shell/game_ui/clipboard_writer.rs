//! Serialize native clipboard writes and keep the selection owner alive off the UI thread.
use bevy::{clipboard::Clipboard, prelude::Resource};
use std::sync::{Arc, OnceLock, mpsc};

type Write = (String, Arc<OnceLock<bool>>);

#[derive(Resource)]
pub(super) struct ClipboardWriter(mpsc::SyncSender<Write>);

impl Default for ClipboardWriter {
    fn default() -> Self {
        let (send, receive) = mpsc::sync_channel::<Write>(16);
        let _ = std::thread::Builder::new()
            .name("client-clipboard".into())
            .spawn(move || {
                let mut clipboard = None;
                for (text, result) in receive {
                    let clipboard = clipboard.get_or_insert_with(Clipboard::default);
                    let _ = result.set(clipboard.set_text(text).is_ok());
                }
            });
        Self(send)
    }
}

impl ClipboardWriter {
    pub(super) fn write(&self, value: String) -> Arc<OnceLock<bool>> {
        let result = Arc::new(OnceLock::new());
        if self.0.try_send((value, result.clone())).is_err() {
            let _ = result.set(false);
        }
        result
    }
}
