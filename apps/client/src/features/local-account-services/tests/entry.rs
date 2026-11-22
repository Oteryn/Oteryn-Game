use super::*;
use crate::account_services::{Action, test_server::*};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn account_services_selection_uses_separate_scope_and_returns_to_fresh_game_login() -> TestResult {
    test_thread(|| {
        let stack = test_support::Stack::start(Vec::new())?;
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let mut config = stack.config();
        config.platform_url = mock.base.clone();
        let original = config.open_browser.clone();
        let urls = Arc::new(Mutex::new(Vec::new()));
        let opened = urls.clone();
        config.open_browser = Arc::new(move |url| {
            if let Ok(mut urls) = opened.lock() {
                urls.push(url.to_owned());
            }
            original(url)
        });
        let selections = Arc::new(AtomicUsize::new(0));
        let selection = selections.clone();
        config.select_character = Arc::new(move |rows, _, _| {
            if selection.fetch_add(1, Ordering::SeqCst) == 0 {
                crate::character_selection::SelectionAction::AccountServices
            } else {
                crate::character_selection::SelectionAction::Play {
                    character_id: rows[0].character.character_id.clone(),
                    channel_id: None,
                }
            }
        });
        let actions = Arc::new(AtomicUsize::new(0));
        let action = actions.clone();
        config.account_services = Arc::new(move |view, _| {
            if action.fetch_add(1, Ordering::SeqCst) == 0 {
                assert!(view.page.is_some());
                Action::Bid {
                    auction_id: AUCTION.into(),
                    coins: 50,
                    confirmed: true,
                }
            } else {
                assert_eq!(
                    view.receipt.as_ref().map(|r| &r.state),
                    Some(&oteryn_platform_client::native_login::bazaar::OperationState::Committed)
                );
                Action::Back
            }
        });
        let authorized = mock.runtime.block_on(authorize::authorize(
            &client,
            &config,
            CancellationToken::new(),
        ))?;
        assert_eq!(authorized.character_id, CHARACTER);
        assert!(!authorized.training_requested);
        assert_eq!(authorized.character_name, "Ari");
        assert_eq!(selections.load(Ordering::SeqCst), 2);
        let urls = urls.lock().map_err(|_| "urls lock")?;
        assert_eq!(urls.len(), 3);
        assert!(urls[0].contains("scope=game%3Aticket"));
        assert!(urls[1].contains("scope=game%3Aaccount-services"));
        assert!(urls[2].contains("scope=game%3Aticket"));
        assert_eq!(mock.authority.lock().map_err(|_| "lock")?.effects, 1);
        stack.finish();
        Ok(())
    })
}

#[test]
fn entry_cancel_releases_blocked_account_services_browser() -> TestResult {
    use std::sync::mpsc;
    use std::time::Duration;
    test_thread(|| {
        let stack = test_support::Stack::start(Vec::new())?;
        let mock = MockPlatform::start()?;
        let mut config = stack.config();
        config.platform_url = mock.base.clone();
        config.select_character =
            Arc::new(|_, _, _| crate::character_selection::SelectionAction::AccountServices);
        let (entered, ready) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let released = Mutex::new(released);
        let original = config.open_browser.clone();
        config.open_browser = Arc::new(move |url| {
            if !url.contains("scope=game%3Aaccount-services") {
                return original(url);
            }
            let _ = entered.send(());
            assert!(
                released
                    .lock()
                    .is_ok_and(|rx| rx.recv_timeout(Duration::from_secs(5)).is_ok())
            );
            false
        });
        let mut entry =
            crate::reentry::Reentry::new(crate::ClientBootstrap::new()?.with_native_login(config));
        assert!(entry.request());
        ready.recv_timeout(Duration::from_secs(5))?;
        let (started, starting) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let closer = std::thread::spawn(move || {
            let _ = started.send(());
            let _ = finished.send(entry.cancel_entry());
            entry.shutdown();
        });
        starting.recv_timeout(Duration::from_secs(5))?;
        let cancelled = done.recv_timeout(Duration::from_secs(2));
        release.send(())?;
        closer.join().map_err(|_| "entry cancellation thread")?;
        assert!(
            cancelled?,
            "Cancel must return before the callback is released"
        );
        stack.finish();
        Ok(())
    })
}

#[test]
fn fresh_services_oauth_keeps_bazaar_state() -> TestResult {
    fresh_services_oauth(false)
}

#[test]
fn fresh_services_oauth_keeps_prelogin_coin_state() -> TestResult {
    fresh_services_oauth(true)
}

fn fresh_services_oauth(coins: bool) -> TestResult {
    use crate::character_selection::SelectionAction;
    use crate::coin_shop::Action as CoinAction;
    test_thread(move || {
        let stack = test_support::Stack::start(Vec::new())?;
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let mut config = stack.config();
        config.platform_url = mock.base.clone();
        let selections = AtomicUsize::new(0);
        config.select_character = Arc::new(move |_, _, _| {
            if matches!(selections.fetch_add(1, Ordering::SeqCst), 0 | 1 | 3) {
                SelectionAction::AccountServices
            } else {
                SelectionAction::Play {
                    character_id: CHARACTER.into(),
                    channel_id: None,
                }
            }
        });
        let account = AtomicUsize::new(0);
        let original = Mutex::new(None);
        config.account_services = Arc::new(move |view, _| {
            let step = account.fetch_add(1, Ordering::SeqCst);
            assert!(!format!("{view:?}").contains("services-test-token-"));
            match (coins, step) {
                (true, 0 | 2 | 4) => Action::CoinShop,
                (false, 0) => Action::Bid {
                    auction_id: AUCTION.into(),
                    coins: 50,
                    confirmed: true,
                },
                (false, 1) => {
                    assert!(view.pending());
                    let id = view.pending_id();
                    assert!(id.is_some());
                    *original
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = id.map(str::to_owned);
                    Action::Back
                }
                (false, 2 | 3) => {
                    assert!(view.pending());
                    assert!(!view.can_retry());
                    assert!(view.pending_id().is_none());
                    let original = original
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    assert!(
                        original
                            .as_deref()
                            .is_some_and(|id| !format!("{view:?}").contains(id))
                    );
                    if step == 2 {
                        Action::Bid {
                            auction_id: AUCTION.into(),
                            coins: 55,
                            confirmed: true,
                        }
                    } else {
                        Action::Back
                    }
                }
                (false, 4) => {
                    assert!(view.pending_id().is_none());
                    assert!(!view.can_retry());
                    Action::Back
                }
                _ => Action::Back,
            }
        });
        let coin = AtomicUsize::new(0);
        config.coin_shop = Arc::new(move |view, _| {
            assert!(!format!("{view:?}").contains("services-test-token-"));
            match coin.fetch_add(1, Ordering::SeqCst) {
                0 => CoinAction::Purchase {
                    package_id: crate::account_services::test_server::coins::PACKAGE.into(),
                    confirmed: true,
                },
                1 => {
                    assert!(view.pending());
                    CoinAction::Back
                }
                _ => {
                    assert!(!view.pending());
                    assert!(!view.can_retry());
                    assert!(
                        !view.can_purchase(crate::account_services::test_server::coins::PACKAGE)
                    );
                    CoinAction::Back
                }
            }
        });
        {
            let mut authority = mock.authority.lock().map_err(|_| "lock")?;
            authority.coin_post_failure = Some(0);
            authority.bazaar_post_failure = Some(0);
        }
        mock.runtime.block_on(authorize::authorize(
            &client,
            &config,
            CancellationToken::new(),
        ))?;
        mock.runtime
            .block_on(config.device_sessions.sign_out(&client))?;
        mock.runtime.block_on(authorize::authorize(
            &client,
            &config,
            CancellationToken::new(),
        ))?;
        let state = mock.authority.lock().map_err(|_| "lock")?;
        assert_eq!(state.effects, 0);
        let path = if coins {
            "POST /api/v1/native-account/coin-purchases/orders "
        } else {
            "POST /api/v1/native-account/bazaar/commands "
        };
        assert_eq!(
            state
                .requests
                .iter()
                .filter(|r| r.starts_with(path))
                .count(),
            1
        );
        assert_eq!(state.oauth_exchanges, 8);
        Ok(())
    })
}

#[test]
fn commerce_oauth_bearer_is_absent_from_debug_and_log_output() -> TestResult {
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "fresh_services_oauth_keeps_",
            "store_launcher_fresh_oauth",
            "--nocapture",
        ])
        .env("OTERYN_LOG", "info")
        .output()?;
    assert!(output.status.success(), "commerce test subprocess failed");
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stdout.contains("4 passed"));
    assert!(
        stderr.contains("endpoint=directory"),
        "native logs must be captured"
    );
    assert!(!stdout.contains("services-test-token-"));
    assert!(!stderr.contains("services-test-token-"));
    Ok(())
}
