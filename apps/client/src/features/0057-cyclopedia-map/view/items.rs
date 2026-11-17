//! Compose catalogue facts and matching read-only market results in the shared host.
use crate::play::mailbox::Mailbox;
use std::fmt::Write;
pub(super) fn text(mailbox: &Mailbox) -> String {
    let panel = &mailbox.discovery.inspection;
    let mut text = oteryn_client_ui::inspection::catalogue_text(panel);
    let Some(definition) = panel
        .catalogue_definition
        .parse::<u32>()
        .ok()
        .filter(|id| panel.catalogue.contains_key(id))
    else {
        return text;
    };
    let market = &mailbox.market;
    if panel.catalogue_market_requested != Some(definition)
        || market.definition != definition
        || market.view != oteryn_client_ui::market::MarketView::Browse
    {
        return text;
    }
    let _ = write!(
        text,
        "Market offers — page {}\r\n",
        market.page.saturating_add(1)
    );
    if market.pending.is_some() {
        text.push_str("Waiting for current market offers\r\n");
        return text;
    }
    if let Some(feedback) = market.feedback {
        let _ = write!(text, "{feedback}\r\n");
    }
    let mut count = 0;
    for row in market
        .rows
        .iter()
        .filter(|row| row.definition == definition && row.side != 0)
    {
        let _ = write!(text, "{}\r\n", oteryn_client_ui::market::row_text(row));
        count += 1;
    }
    if count == 0 && market.feedback.is_none() {
        text.push_str("No offers on this page\r\n");
    }
    text
}
