use super::*;
use crate::account_services::test_server::coins::{Event, EventKind, PACKAGE};
use crate::account_services::test_server::{CHARACTER, MockPlatform, TestResult, test_thread};
use std::sync::Mutex;
mod checkout;
mod reversals;

mod validation;

#[test]
fn coin1_shop_shows_one_available_balance() {
    let view = View {
        balance: Some(CoinBalance {
            version: 1,
            revision: 1,
            total_coins: 100,
            available_coins: 95,
            transferable_coins: 90,
            review_required: false,
        }),
        ..Default::default()
    };
    let text = view.status_text();
    assert!(text.contains("Balance: 95 coins"), "{text}");
    assert!(!text.to_lowercase().contains("transferable"), "{text}");
}
