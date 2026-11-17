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
        let (_, id, _, name, training_requested) = mock.runtime.block_on(authorize::authorize(
            &client,
            &config,
            CancellationToken::new(),
        ))?;
        assert_eq!(id, CHARACTER);
        assert!(!training_requested);
        assert_eq!(name, "Ari");
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
