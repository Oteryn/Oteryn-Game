use super::*;
use crate::account_services::test_server::{CHARACTER, MockPlatform, TestResult, test_thread};
use crate::coin_shop::{Action, launcher::Launcher};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::Duration;

#[test]
fn coin_shop_store_launcher_uses_services_scope_character_context_and_refresh_callback()
-> TestResult {
    test_thread(|| {
        let stack = test_support::Stack::start(Vec::new())?;
        let mock = MockPlatform::start()?;
        let mut config = stack.config();
        config.platform_url = mock.base.clone();
        let browser = config.open_browser.clone();
        let (url_tx, url_rx) = mpsc::channel();
        config.open_browser = Arc::new(move |url| {
            let _ = url_tx.send(url.to_owned());
            browser(url)
        });
        let shown = Arc::new(AtomicUsize::new(0));
        let count = shown.clone();
        config.coin_shop = Arc::new(move |view, _| {
            assert_eq!(
                view.catalogue
                    .as_ref()
                    .and_then(|c| c.character_id.as_deref()),
                Some(CHARACTER)
            );
            assert_eq!(view.balance.as_ref().map(|b| b.total_coins), Some(1000));
            count.fetch_add(1, Ordering::SeqCst);
            Action::Back
        });
        let (done_tx, done_rx) = mpsc::channel();
        let launcher = Launcher::new(
            config,
            CHARACTER.into(),
            Arc::new(move || {
                let _ = done_tx.send(());
            }),
        );
        assert!(launcher.can_start());
        assert!(launcher.start());
        let url = url_rx.recv_timeout(Duration::from_secs(5))?;
        assert!(url.contains("scope=game%3Aaccount-services"));
        done_rx.recv_timeout(Duration::from_secs(5))?;
        assert_eq!(shown.load(Ordering::SeqCst), 1);
        launcher.end();
        assert!(!launcher.can_start());
        assert!(!launcher.start());
        stack.finish();
        Ok(())
    })
}

#[test]
fn coin_shop_session_end_cancels_pending_oauth_without_presenting_or_refreshing() -> TestResult {
    test_thread(|| {
        let stack = test_support::Stack::start(Vec::new())?;
        let mock = MockPlatform::start()?;
        let mut config = stack.config();
        config.platform_url = mock.base.clone();
        let (opened_tx, opened_rx) = mpsc::channel();
        config.open_browser = Arc::new(move |_| {
            let _ = opened_tx.send(());
            true
        });
        let invoked = Arc::new(AtomicUsize::new(0));
        let presented = invoked.clone();
        config.coin_shop = Arc::new(move |_, _| {
            presented.fetch_add(1, Ordering::SeqCst);
            Action::Back
        });
        let refreshed = invoked.clone();
        let launcher = Launcher::new(
            config,
            CHARACTER.into(),
            Arc::new(move || {
                refreshed.fetch_add(1, Ordering::SeqCst);
            }),
        );
        assert!(launcher.start());
        opened_rx.recv_timeout(Duration::from_secs(5))?;
        assert!(!launcher.start());
        launcher.end();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while Arc::strong_count(&launcher) > 1 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            Arc::strong_count(&launcher),
            1,
            "shop thread retained after session end"
        );
        assert_eq!(invoked.load(Ordering::SeqCst), 0);
        assert!(!launcher.start());
        stack.finish();
        Ok(())
    })
}
