use super::*;
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Purchase history is unavailable in this session".into();
    }
    let mut text = format!("Store — Purchase history\r\n{}", panel.feedback);
    if let Some(page) = &panel.page {
        if page.entries.is_empty() {
            text.push_str("\r\nNo purchases on this page");
        }
        for row in &page.entries {
            let identity = row
                .order_id
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            text.push_str(&format!(
                "\r\nOrder: {identity}\r\nCatalogue revision: {}\r\nDebit: {} {}",
                row.catalog_revision, row.price, row.currency
            ));
            text.push_str(&format!(
                "\r\n{} — {} {} — {} ms since Unix epoch",
                row.offer_key, row.price, row.currency, row.purchased_at_unix_ms
            ));
        }
        text.push_str(if page.next_page_cursor.is_empty() {
            "\r\nEnd of purchase history"
        } else {
            "\r\nMore purchases: Next page"
        });
    }
    text
}
