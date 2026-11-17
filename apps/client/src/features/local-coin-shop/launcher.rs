//! Session-bound launcher for the Store's Get coins entry; no bearer is retained here.
#![cfg_attr(not(any(windows, test)), allow(dead_code))]
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
    active: AtomicBool,
    ended: AtomicBool,
    cancellation: Mutex<Option<CancellationToken>>,
}
impl std::fmt::Debug for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(tr!("CoinShopLauncher"))
            .field("active", &self.active.load(Ordering::Acquire))
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
            active: AtomicBool::new(false),
            ended: AtomicBool::new(false),
            cancellation: Mutex::new(None),
        })
    }
    pub(crate) fn can_start(&self) -> bool {
        !self.ended.load(Ordering::Acquire) && !self.active.load(Ordering::Acquire)
    }
    /// Opens a Platform web page (coin history, coin transfer) in the system browser.
    pub(crate) fn open_web(&self, link: crate::web_links::WebLink) -> bool {
        !self.ended.load(Ordering::Acquire) && crate::web_links::open(&self.config, link)
    }
    pub(crate) fn end(&self) {
        self.ended.store(true, Ordering::Release);
        if let Ok(token) = self.cancellation.lock()
            && let Some(token) = token.as_ref()
        {
            token.cancel();
        }
    }
    pub(crate) fn start(self: &Arc<Self>) -> bool {
        if self.ended.load(Ordering::Acquire)
            || self
                .active
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return false;
        }
        let cancel = CancellationToken::new();
        {
            let Ok(mut token) = self.cancellation.lock() else {
                self.active.store(false, Ordering::Release);
                return false;
            };
            if self.ended.load(Ordering::Acquire) {
                self.active.store(false, Ordering::Release);
                return false;
            }
            *token = Some(cancel.clone());
        }
        let launcher = self.clone();
        if std::thread::Builder::new()
            .name("coin-shop".into())
            .stack_size(64 << 20)
            .spawn(move || {
                let _active = Active(launcher.clone());
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
            if let Ok(mut token) = self.cancellation.lock() {
                *token = None;
            }
            self.active.store(false, Ordering::Release);
            return false;
        }
        true
    }
}
struct Active(Arc<Launcher>);
impl Drop for Active {
    fn drop(&mut self) {
        if let Ok(mut token) = self.0.cancellation.lock() {
            *token = None;
        }
        self.0.active.store(false, Ordering::Release);
    }
}
