use super::{StorePage, StorePanel, StoreSort};
use std::fmt::Write;
pub fn panel_text(panel: &StorePanel) -> String {
    let mut text = format!("Store - {:?}", panel.page);
    match panel.page {
        StorePage::Catalogue => {
            if let Some(page) = &panel.catalogue {
                let _ = write!(text, "\nBalance: {} Oteryn Coins", page.total_coins);
                let _ = write!(
                    text,
                    "\nSearch/sort cover the loaded category page. Sort: {:?}",
                    panel.sort
                );
                for index in visible_offers(panel) {
                    let offer = &page.offers[index];
                    let _ = write!(
                        text,
                        "\n{}: {} {}",
                        offer.title, offer.price, offer.currency
                    );
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
        StorePage::CoinHistory => text.push_str(&super::coin_history_text(&panel.coin_history)),
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
    if panel.page == StorePage::Catalogue
        && let Some(preview) = offer_preview(panel)
    {
        let _ = write!(text, "\n{preview}");
    }
    if let Some(result) = &panel.purchase {
        let _ = write!(text, "\nPurchase: {:?}", result.code);
    }
    if let Some(feedback) = &panel.feedback {
        let _ = write!(text, "\n{feedback}");
    }
    text
}

pub fn visible_offers(panel: &StorePanel) -> Vec<usize> {
    let Some(page) = &panel.catalogue else {
        return Vec::new();
    };
    let query = panel.search.trim().to_lowercase();
    let mut rows: Vec<_> = page
        .offers
        .iter()
        .enumerate()
        .filter(|(_, offer)| {
            offer.title.to_lowercase().contains(&query)
                || offer.offer_key.to_lowercase().contains(&query)
        })
        .map(|(index, _)| index)
        .collect();
    rows.sort_by(|a, b| {
        let (a, b) = (&page.offers[*a], &page.offers[*b]);
        match panel.sort {
            StoreSort::Catalogue => std::cmp::Ordering::Equal,
            StoreSort::Title => a.title.cmp(&b.title),
            StoreSort::Price => a
                .currency
                .cmp(&b.currency)
                .then_with(|| a.price.cmp(&b.price))
                .then_with(|| a.title.cmp(&b.title)),
        }
    });
    rows
}

pub fn offer_preview(panel: &StorePanel) -> Option<String> {
    if !visible_offers(panel).contains(&panel.selected_offer) {
        return None;
    }
    let page = panel.catalogue.as_ref()?;
    let offer = page.offers.get(panel.selected_offer)?;
    Some(format!(
        "Preview: {} — {} {}\r\nOffer: {}; catalogue revision: {}",
        offer.title, offer.price, offer.currency, offer.offer_key, page.catalog_revision
    ))
}
