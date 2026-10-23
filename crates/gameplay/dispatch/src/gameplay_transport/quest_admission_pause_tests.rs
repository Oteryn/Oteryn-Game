//! A test-only suspension at the independently resolved fence boundary.
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Default)]
pub(crate) struct Pause {
    pub reached: Notify,
    pub resume: Notify,
}

thread_local! { static PAUSE: RefCell<Option<Arc<Pause>>> = const { RefCell::new(None) }; }

pub(crate) fn install() -> Arc<Pause> {
    let pause = Arc::new(Pause::default());
    PAUSE.set(Some(Arc::clone(&pause)));
    pause
}

pub(super) async fn after_fence() {
    let pause = PAUSE.take();
    if let Some(pause) = pause {
        pause.reached.notify_one();
        pause.resume.notified().await;
    }
}
