use super::*;
use crate::account_services::{
    Action as ServicesAction,
    test_server::{MockPlatform, TestResult, test_thread},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn premium_panel_opens_from_real_services_selection_and_refreshes_without_payment() -> TestResult {
    test_thread(|| {
        let stack = test_support::Stack::start(Vec::new())?;
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let mut config = stack.config();
        config.platform_url = mock.base.clone();
        let selected = Arc::new(AtomicUsize::new(0));
        let selections = selected.clone();
        config.select_character = Arc::new(move |rows, _, _| {
            if selections.fetch_add(1, Ordering::SeqCst) == 0 {
                crate::character_selection::SelectionAction::AccountServices
            } else {
                crate::character_selection::SelectionAction::Play {
                    character_id: rows[0].character.character_id.clone(),
                    channel_id: None,
                }
            }
        });
        let opened = Arc::new(AtomicUsize::new(0));
        let services = opened.clone();
        config.account_services = Arc::new(move |_, _| {
            if services.fetch_add(1, Ordering::SeqCst) == 0 {
                ServicesAction::Premium
            } else {
                ServicesAction::Back
            }
        });
        let shown = Arc::new(AtomicUsize::new(0));
        let panels = shown.clone();
        config.premium = Arc::new(move |view, _| {
            assert!(view.fresh());
            assert!(view.status_text().contains("Private channels [available]"));
            assert!(view.status_text().contains("Expiry: 1800086400"));
            if panels.fetch_add(1, Ordering::SeqCst) == 0 {
                crate::premium::Action::Refresh
            } else {
                crate::premium::Action::Back
            }
        });
        mock.runtime.block_on(authorize::authorize(
            &client,
            &config,
            CancellationToken::new(),
        ))?;
        assert_eq!(shown.load(Ordering::SeqCst), 2);
        let authority = mock.authority.lock().map_err(|_| "lock")?;
        assert_eq!(
            authority
                .requests
                .iter()
                .filter(|r| r.starts_with("GET /api/v1/native-account/premium "))
                .count(),
            2
        );
        assert!(
            !authority
                .requests
                .iter()
                .any(|r| r.contains("coin-purchases/orders"))
        );
        stack.finish();
        Ok(())
    })
}
