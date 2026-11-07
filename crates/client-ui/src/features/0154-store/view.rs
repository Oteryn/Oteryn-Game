use super::{StorePage, StorePanel};
use std::fmt::Write;
pub fn panel_text(panel: &StorePanel) -> String {
    let mut text = format!("Store - {:?}", panel.page);
    match panel.page {
        StorePage::Catalogue => {
            if let Some(page) = &panel.catalogue {
                let _ = write!(text, "\nBalance: {} Oteryn Coins", page.total_coins);
                for offer in &page.offers {
                    let _ = write!(text, "\n{}: {} Oteryn Coins", offer.title, offer.price);
                }
            }
        }
        StorePage::History => {
            if let Some(page) = &panel.history {
                for entry in &page.entries {
                    let _ = write!(text, "\n{}: {} Oteryn Coins", entry.offer_key, entry.price);
                }
            }
        }
        StorePage::Inbox => {
            if let Some(receipt) = &panel.claim {
                let _ = write!(
                    text,
                    "\n{}: {:?}",
                    receipt.fulfilment_unit_id, receipt.result
                );
            }
        }
    }
    if let Some(result) = &panel.purchase {
        let _ = write!(text, "\nPurchase: {:?}", result.code);
    }
    if let Some(feedback) = &panel.feedback {
        let _ = write!(text, "\n{feedback}");
    }
    text
}
