//! Native controls for Store.
#[path = "state/coin_history.rs"]
mod coin_history;
mod input;
mod protocol;
mod state;
mod view;
pub use coin_history::{CoinHistory, coin_history, coin_history_filter, coin_history_text};
pub use input::{categories, open_key, select_category};
pub use protocol::{
    apply_result, catalogue, choose_payment, claim, command, confirmation_quote, history, purchase,
    request,
};
pub use state::{GiftDraft, StorePage, StorePanel, StoreSort};
pub use view::{offer_preview, panel_text, visible_offers};

#[path = "state/browser.rs"]
mod browser;
pub use browser::{Browser, select_product};
pub use protocol::browse_request as browse;

#[path = "state/gift.rs"]
mod gift;
pub use gift::{GiftPanel, gift_control, gift_result};
