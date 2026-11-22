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
            Arc::new(move |_| {
                let _ = done_tx.send(());
            }),
            None,
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
            Arc::new(move |_| {
                refreshed.fetch_add(1, Ordering::SeqCst);
            }),
            None,
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

#[test]
fn store_launcher_opens_coin_history_and_transfer_pages_until_the_session_ends() -> TestResult {
    use crate::web_links::WebLink;
    test_thread(|| {
        let stack = test_support::Stack::start(Vec::new())?;
        let mut config = stack.config();
        config.platform_url = "https://oteryn.example/".into();
        let (url_tx, url_rx) = mpsc::channel();
        config.open_browser = Arc::new(move |url| {
            let _ = url_tx.send(url.to_owned());
            true
        });
        let launcher = Launcher::new(config, CHARACTER.into(), Arc::new(|_| {}), None);
        assert!(launcher.open_web(WebLink::CoinHistory));
        assert!(launcher.open_web(WebLink::CoinTransfer));
        let language = oteryn_client_ui::localization::current().code();
        let language = if language == "pl" { "pl" } else { "en" };
        assert_eq!(
            url_rx.try_iter().collect::<Vec<_>>(),
            [
                format!("https://oteryn.example/{language}/shop/history"),
                format!("https://oteryn.example/{language}/shop/transfer"),
            ]
        );
        launcher.end();
        assert!(!launcher.open_web(WebLink::CoinHistory));
        assert!(url_rx.try_recv().is_err());
        Ok(())
    })
}

#[test]
fn store_launcher_fresh_oauth_keeps_request_until_sign_out() -> TestResult {
    use crate::account_services::test_server::coins::PACKAGE;
    use std::time::Instant;
    test_thread(|| {
        let stack = test_support::Stack::start(Vec::new())?;
        let mock = MockPlatform::start()?;
        mock.authority.lock().map_err(|_| "lock")?.coin_post_failure = Some(0);
        let mut config = stack.config();
        config.platform_url = mock.base.clone();
        let (shown_tx, shown_rx) = mpsc::channel();
        let device = config.device_sessions.clone();
        let steps = AtomicUsize::new(0);
        config.coin_shop = Arc::new(move |view, _| {
            let _ = shown_tx.send((
                view.status_text()
                    .lines()
                    .find(|line| line.contains("Unresolved order request"))
                    .map(str::to_owned),
                format!("{view:?}"),
            ));
            if matches!(steps.fetch_add(1, Ordering::SeqCst), 0 | 4) {
                Action::Purchase {
                    package_id: PACKAGE.into(),
                    confirmed: true,
                }
            } else {
                Action::Back
            }
        });
        let (done_tx, done_rx) = mpsc::channel();
        let launcher = Launcher::new(
            config,
            CHARACTER.into(),
            Arc::new(move |_| {
                let _ = done_tx.send(());
            }),
            None,
        );
        assert!(launcher.start());
        let mut original = None;
        for expected in [false, true] {
            let (pending, debug) = shown_rx.recv_timeout(Duration::from_secs(5))?;
            assert_eq!(pending.is_some(), expected);
            original = pending;
            assert!(!debug.contains("services-test-token-"));
        }
        done_rx.recv_timeout(Duration::from_secs(5))?;
        for account in [Some("account-b"), None] {
            mock.authority.lock().map_err(|_| "lock")?.oauth_account = account.map(str::to_owned);
            let deadline = Instant::now() + Duration::from_secs(5);
            while !launcher.can_start() && Instant::now() < deadline {
                std::thread::yield_now();
            }
            assert!(launcher.start());
            assert!(
                shown_rx.recv_timeout(Duration::from_secs(5))?.0.is_none(),
                "another login inherited the pending order"
            );
            done_rx.recv_timeout(Duration::from_secs(5))?;
        }
        mock.authority.lock().map_err(|_| "lock")?.oauth_account = Some("account-a".into());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !launcher.can_start() && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert!(launcher.start());
        let (pending, debug) = shown_rx.recv_timeout(Duration::from_secs(5))?;
        assert_eq!(pending, original);
        let (after_purchase, _) = shown_rx.recv_timeout(Duration::from_secs(5))?;
        assert_eq!(after_purchase, original);
        assert_eq!(mock.with_coins(|coins| coins.creations)?, 0);
        assert!(!debug.contains("services-test-token-"));
        assert!(!format!("{launcher:?}").contains("services-test-token-"));
        done_rx.recv_timeout(Duration::from_secs(5))?;
        mock.runtime.block_on(device.sign_out(&mock.client()?))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while !launcher.can_start() && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert!(launcher.start());
        assert!(shown_rx.recv_timeout(Duration::from_secs(5))?.0.is_none());
        done_rx.recv_timeout(Duration::from_secs(5))?;
        assert_eq!(
            mock.authority.lock().map_err(|_| "lock")?.oauth_exchanges,
            5
        );
        Ok(())
    })
}
