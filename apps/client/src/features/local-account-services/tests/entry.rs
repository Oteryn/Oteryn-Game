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
fn entry_cancel_waits_for_account_services_browser_delivery() -> TestResult {
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
        let early = done.recv_timeout(Duration::from_millis(100));
        release.send(())?;
        closer.join().map_err(|_| "entry cancellation thread")?;
        assert_eq!(
            early,
            Err(mpsc::RecvTimeoutError::Timeout),
            "entry cancellation passed an Account Services browser delivery still in progress"
        );
        assert!(done.try_recv()?);
        stack.finish();
        Ok(())
    })
}
