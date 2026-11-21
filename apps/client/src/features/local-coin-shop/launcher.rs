//! Session-bound launcher for the Store's Get coins entry; no bearer is retained here.
#![cfg_attr(not(test), allow(dead_code))]
use crate::NativeLoginConfig;
use oteryn_foundation::CancellationToken;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

pub(crate) struct Launcher {
    config: NativeLoginConfig,
    character_id: String,
    after: Arc<dyn Fn() + Send + Sync>,
    ended: AtomicBool,
    cancellation: Mutex<Option<Arc<CancellationToken>>>,
}
impl std::fmt::Debug for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(tr!("CoinShopLauncher"))
            .field(
                "active",
                &self
                    .cancellation
                    .lock()
                    .is_ok_and(|active| active.is_some()),
            )
            .finish_non_exhaustive()
    }
}
impl Launcher {
    pub(crate) fn new(
        config: NativeLoginConfig,
        character_id: String,
        after: Arc<dyn Fn() + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            character_id,
            after,
            ended: AtomicBool::new(false),
            cancellation: Mutex::new(None),
        })
    }
    pub(crate) fn can_start(&self) -> bool {
        !self.ended.load(Ordering::Acquire)
            && self
                .cancellation
                .lock()
                .is_ok_and(|active| active.is_none())
    }
    /// Opens a Platform web page (coin history, coin transfer) in the system browser.
    pub(crate) fn open_web(&self, link: crate::web_links::WebLink) -> bool {
        !self.ended.load(Ordering::Acquire) && crate::web_links::open(&self.config, link)
    }
    pub(crate) fn end(&self) {
        self.ended.store(true, Ordering::Release);
        self.cancel_active();
    }
    pub(crate) fn cancel_active(&self) {
        if let Ok(mut active) = self.cancellation.lock()
            && let Some(cancel) = active.take()
        {
            cancel.cancel();
        }
    }
    pub(crate) fn start(self: &Arc<Self>) -> bool {
        let cancel = Arc::new(CancellationToken::new());
        {
            let Ok(mut active) = self.cancellation.lock() else {
                return false;
            };
            if self.ended.load(Ordering::Acquire) || active.is_some() {
                return false;
            }
            *active = Some(cancel.clone());
        }
        let launcher = self.clone();
        let worker_cancel = cancel.clone();
        if std::thread::Builder::new()
            .name("coin-shop".into())
            .stack_size(64 << 20)
            .spawn(move || {
                let _active = Active(launcher.clone(), worker_cancel.clone());
                let cancel = (*worker_cancel).clone();
                let result = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime.block_on(crate::native_entry::open_coin_shop(
                        &launcher.config,
                        Some(launcher.character_id.clone()),
                        cancel.clone(),
                    )),
                    Err(_) => Err(crate::GameplayEntryError::Rejected(
                        oteryn_platform_client::native_login::PublicClass::TemporarilyUnavailable,
                    )),
                };
                if let Err(error) = result
                    && !cancel.is_cancelled()
                {
                    let view = super::View {
                        unavailable: true,
                        feedback: lfmt!(
                            "Coin shop sign-in was not completed. {}",
                            crate::native_login::error_text(error)
                        ),
                        ..super::View::default()
                    };
                    let _ = (launcher.config.coin_shop)(&view, cancel.clone());
                }
                if !cancel.is_cancelled() {
                    (launcher.after)();
                }
            })
            .is_err()
        {
            self.finish_active(&cancel);
            return false;
        }
        true
    }

    fn finish_active(&self, cancel: &Arc<CancellationToken>) {
        if let Ok(mut active) = self.cancellation.lock()
            && active
                .as_ref()
                .is_some_and(|token| Arc::ptr_eq(token, cancel))
        {
            *active = None;
        }
    }
}
struct Active(Arc<Launcher>, Arc<CancellationToken>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.finish_active(&self.1);
    }
}
