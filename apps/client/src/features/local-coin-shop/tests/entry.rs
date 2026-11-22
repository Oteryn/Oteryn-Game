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
    test_thread(|| fresh_oauth(false))
}

#[test]
fn store_launcher_fresh_oauth_releases_definitively_refused_request() -> TestResult {
    test_thread(|| fresh_oauth(true))
}

fn fresh_oauth(refused: bool) -> TestResult {
    use crate::account_services::test_server::coins::{CoinAuthority, EventKind, PACKAGE};
    use std::sync::Mutex;
    let _entry = crate::account_services::test_server::COMMERCE_ENTRY
        .lock()
        .map_err(|_| "commerce entry lock")?;
    let stack = test_support::Stack::start(Vec::new())?;
    let mock = MockPlatform::start()?;
    let mut config = stack.config();
    config.platform_url = mock.base.clone();
    let device = config.device_sessions.clone();
    let actions = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let script = actions.clone();
    let (tx, rx) = mpsc::channel();
    config.coin_shop = Arc::new(move |view, _| {
        assert!(tx.send(view.clone()).is_ok());
        script
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front()
            .unwrap_or(Action::Back)
    });
    let (done, finished) = mpsc::channel();
    let launcher = Launcher::new(
        config,
        CHARACTER.into(),
        Arc::new(move |_| {
            let _ = done.send(());
        }),
        None,
    );
    let open =
        |steps: Vec<Action>| -> Result<Vec<crate::coin_shop::View>, Box<dyn std::error::Error>> {
            actions.lock().map_err(|_| "script")?.extend(steps);
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while !launcher.can_start() && std::time::Instant::now() < deadline {
                std::thread::yield_now();
            }
            assert!(launcher.start());
            finished.recv_timeout(Duration::from_secs(5))?;
            Ok(rx.try_iter().collect())
        };
    let purchase = || Action::Purchase {
        package_id: PACKAGE.into(),
        confirmed: true,
    };
    let posts = || -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let state = mock.authority.lock().map_err(|_| "lock")?;
        state
            .requests
            .iter()
            .filter(|r| r.starts_with("POST /api/v1/native-account/coin-purchases/orders "))
            .map(|r| {
                let (_, body) = r.split_once("\r\n\r\n").ok_or("missing body")?;
                let value: serde_json::Value = serde_json::from_str(body)?;
                Ok(value["request_id"].as_str().ok_or("missing id")?.to_owned())
            })
            .collect()
    };
    if refused {
        mock.authority.lock().map_err(|_| "lock")?.coin_post_failure = Some(409);
    } else {
        mock.with_coins(|coins| {
            coins.drop_after_create = true;
            coins.hide_recent_orders = true;
        })?;
    }
    let first = open(vec![purchase()])?;
    let original = posts()?.pop().ok_or("original POST")?;
    assert!(
        first
            .last()
            .ok_or("pending")?
            .status_text()
            .contains(&original)
    );
    if refused {
        let fresh = open(vec![Action::Retry, purchase()])?;
        assert!(!format!("{fresh:?}").contains(&original));
        let requests = posts()?;
        assert_eq!(requests.len(), 2);
        assert_ne!(requests[1], original);
        let status = format!("GET /api/v1/native-account/coin-purchases/requests/{original} ");
        {
            let state = mock.authority.lock().map_err(|_| "lock")?;
            let lookup = state
                .requests
                .iter()
                .position(|r| r.starts_with(&status))
                .ok_or("original grant status lookup")?;
            let reauth = state
                .requests
                .iter()
                .rposition(|r| r.starts_with("POST /oauth/token "))
                .ok_or("fresh authorization")?;
            assert!(lookup < reauth);
        }
        assert_eq!(mock.with_coins(|c| c.creations)?, 1);
        return Ok(());
    }

    // Give the second sign-in an independent wallet and order listing.
    let mut other = CoinAuthority::new(mock.base.clone())?;
    other.balance.total_coins = 2000;
    other.drop_after_create = true;
    mock.authority
        .lock()
        .map_err(|_| "lock")?
        .coin_wallets_by_grant
        .insert(2, other);
    let fresh = open(vec![Action::Retry, purchase(), Action::Refresh])?;
    assert_eq!(fresh.len(), 4);
    for view in &fresh {
        assert_eq!(view.balance.as_ref().map(|b| b.total_coins), Some(2000));
        assert!(!view.status_text().contains(&original));
        assert!(!format!("{view:?}").contains(&original));
    }
    assert!(!fresh[0].can_retry());
    assert!(fresh[1].can_purchase(PACKAGE));
    assert!(fresh[2].can_retry());
    assert!(!fresh[3].can_retry());
    assert!(fresh[3].can_purchase(PACKAGE));
    let requests = posts()?;
    assert_eq!(requests.len(), 2);
    assert_ne!(requests[1], original);
    assert_eq!(
        fresh[3].order.as_ref().map(|o| &o.request_id),
        Some(&requests[1])
    );
    let hidden = open(vec![Action::Retry])?;
    assert!(
        hidden
            .iter()
            .all(|v| v.can_purchase(PACKAGE) && !format!("{v:?}").contains(&original))
    );
    assert_eq!(hidden.len(), 2);
    for view in hidden {
        assert_eq!(view.balance.as_ref().map(|b| b.total_coins), Some(1000));
        assert!(!view.status_text().contains(&original));
        assert!(!view.can_retry());
    }
    assert_eq!(posts()?, requests);
    let order = mock
        .with_coins(|coins| coins.orders.get(&original).map(|(_, order)| order.clone()))?
        .ok_or("unlisted order")?;
    let (body, signature) = mock.signed_event(&order, EventKind::Paid)?;
    assert_eq!(mock.webhook(&body, &signature)?, 200);
    mock.with_coins(|coins| coins.hide_recent_orders = false)?;
    let reconciled = open(Vec::new())?;
    let view = reconciled.first().ok_or("reconciled")?;
    assert_eq!(view.order.as_ref().map(|o| &o.request_id), Some(&original));
    assert!(view.order.as_ref().is_some_and(|o| o.state.completed()));
    assert!(view.can_purchase(PACKAGE));
    assert!(!view.pending());
    assert_eq!(posts()?, requests);
    assert!(!format!("{view:?}{launcher:?}").contains("services-test-token-"));
    mock.runtime.block_on(device.sign_out(&mock.client()?))?;
    mock.with_coins(|coins| coins.hide_recent_orders = true)?;
    assert!(!format!("{:?}", open(Vec::new())?).contains(&original));
    assert_eq!(
        mock.authority.lock().map_err(|_| "lock")?.oauth_exchanges,
        5
    );
    Ok(())
}
