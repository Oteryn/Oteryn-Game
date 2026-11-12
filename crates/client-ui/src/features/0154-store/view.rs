use super::{StorePage, StorePanel, StoreSort};
use std::fmt::Write;
pub fn panel_text(panel: &StorePanel) -> String {
    let mut text = lfmt!("Store - {:?}", panel.page);
    if let Some(balance) = panel.cosmetic_tokens {
        text.push_str(&format!("\nCosmetic Tokens (this character): {balance}"));
    }
    match panel.page {
        StorePage::Catalogue => {
            if let Some(page) = &panel.catalogue {
                let _ = lwrite!(text, "\nBalance: {} Oteryn Coins", page.total_coins);
                let _ = lwrite!(
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
                    let _ = lwrite!(text, "\n{}: {} Oteryn Coins", entry.offer_key, entry.price);
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
    if let Some(reason) = token_purchase_disabled_reason(panel) {
        let _ = write!(text, "\n{reason}");
    }
    if let Some(result) = &panel.purchase {
        let _ = lwrite!(text, "\nPurchase: {:?}", result.code);
    }
    if let Some(feedback) = &panel.feedback {
        let _ = write!(text, "\n{feedback}");
    }
    text
}

/// Why a Cosmetic Token purchase is disabled: the selected quote costs more than the balance.
pub fn token_purchase_disabled_reason(panel: &StorePanel) -> Option<&'static str> {
    let balance = panel.cosmetic_tokens?;
    let offer = panel.catalogue.as_ref()?.offers.first()?;
    (offer.currency == "COSMETIC_TOKEN" && offer.price > balance)
        .then_some(tr!("not enough Cosmetic Tokens"))
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
    Some(lfmt!(
        "Preview: {} — {} {}\r\nOffer: {}; catalogue revision: {}",
        offer.title,
        offer.price,
        offer.currency,
        offer.offer_key,
        page.catalog_revision
    ))
}

#[cfg(test)]
mod token_reason_tests {
    use super::*;
    use oteryn_client_runtime::cui3::store::{StoreCataloguePage, StoreOffer};
    fn panel(balance: u64, price: u32, currency: &str) -> StorePanel {
        let offer = StoreOffer {
            offer_key: "store:outfit:x".into(),
            title: "Outfit".into(),
            price: price.into(),
            currency: currency.into(),
        };
        StorePanel {
            cosmetic_tokens: Some(balance),
            catalogue: Some(StoreCataloguePage {
                catalog_revision: "r".into(),
                offers: vec![offer],
                next_page_cursor: String::new(),
                total_coins: 0,
                transferable_coins: 0,
            }),
            ..Default::default()
        }
    }
    #[test]
    fn token_quote_above_balance_shows_not_enough_cosmetic_tokens() {
        let p = panel(5, 10, "COSMETIC_TOKEN");
        assert_eq!(
            token_purchase_disabled_reason(&p),
            Some("not enough Cosmetic Tokens")
        );
        assert!(panel_text(&p).contains("not enough Cosmetic Tokens"));
        assert_eq!(
            token_purchase_disabled_reason(&panel(10, 10, "COSMETIC_TOKEN")),
            None
        );
        assert_eq!(
            token_purchase_disabled_reason(&panel(5, 10, "TIBIA_COIN")),
            None
        );
    }
}
