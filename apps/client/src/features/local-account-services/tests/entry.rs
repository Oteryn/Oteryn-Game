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
fn fresh_services_oauth_discards_bazaar_state() -> TestResult {
    fresh_services_oauth(false)
}

#[test]
fn fresh_services_oauth_discards_prelogin_coin_state() -> TestResult {
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
            if selections.fetch_add(1, Ordering::SeqCst) < 2 {
                SelectionAction::AccountServices
            } else {
                SelectionAction::Play {
                    character_id: CHARACTER.into(),
                    channel_id: None,
                }
            }
        });
        let account = AtomicUsize::new(0);
        config.account_services = Arc::new(move |view, _| {
            let step = account.fetch_add(1, Ordering::SeqCst);
            assert!(!format!("{view:?}").contains("services-test-token-"));
            match (coins, step) {
                (true, 0 | 2) => Action::CoinShop,
                (false, 0) => Action::Bid {
                    auction_id: AUCTION.into(),
                    coins: 50,
                    confirmed: true,
                },
                (false, 1) => {
                    assert!(view.pending());
                    Action::Back
                }
                (false, 2) => {
                    assert!(!view.pending(), "new OAuth inherited the Bazaar request");
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
                    assert!(!view.pending(), "new OAuth inherited the coin request");
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
        let state = mock.authority.lock().map_err(|_| "lock")?;
        assert_eq!(state.oauth_exchanges, 5);
        Ok(())
    })
}

#[test]
fn commerce_oauth_bearer_is_absent_from_debug_and_log_output() -> TestResult {
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "fresh_services_oauth_discards_",
            "store_launcher_fresh_oauth",
            "--nocapture",
        ])
        .env("OTERYN_LOG", "info")
        .output()?;
    assert!(output.status.success(), "commerce test subprocess failed");
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stdout.contains("3 passed"));
    assert!(
        stderr.contains("endpoint=directory"),
        "native logs must be captured"
    );
    assert!(!stdout.contains("services-test-token-"));
    assert!(!stderr.contains("services-test-token-"));
    Ok(())
}
