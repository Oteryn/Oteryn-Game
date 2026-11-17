//! Query the complete server catalogue; local browsing never rewrites a purchase quote.
use super::StorePanel;
use oteryn_client_runtime::{
    cui3::store::{self, browse as b},
    server_mechanic::{CommandStatus, ServerMechanicCommand, ServerMechanicResponse},
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Browser {
    pub page: Option<b::Page>,
    pub category: String,
    pub search: String,
    pub sort: u8,
    pub selected: usize,
    pub cursors: Vec<String>,
    pub gender: usize,
    pub direction: u8,
}
impl Default for Browser {
    fn default() -> Self {
        Self {
            page: None,
            category: "featured".into(),
            search: String::new(),
            sort: 0,
            selected: 0,
            cursors: vec![String::new()],
            gender: 0,
            direction: 2,
        }
    }
}
pub fn browse(
    panel: &mut StorePanel,
    category: &str,
    search: &str,
    sort: u8,
    step: i8,
) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let browser = &mut panel.browser;
    let changed = browser.category != category || browser.search != search || browser.sort != sort;
    if changed || step == 0 {
        browser.cursors = vec![String::new()];
    } else if step > 0 {
        if browser.cursors.len() >= 32 {
            return None;
        }
        let next = &browser.page.as_ref()?.next;
        if next.is_empty() {
            return None;
        }
        browser.cursors.push(next.clone());
    } else {
        if browser.cursors.len() < 2 {
            return None;
        }
        browser.cursors.pop();
    }
    let query = b::Query {
        category: category.into(),
        search: search.into(),
        sort,
        cursor: browser.cursors.last()?.clone(),
    };
    let payload = match query.encode() {
        Ok(payload) => payload,
        Err(_) => {
            panel.feedback = Some("Search must fit within 128 UTF-8 bytes".into());
            return None;
        }
    };
    browser.category = category.into();
    browser.search = search.into();
    browser.sort = sort;
    super::protocol::request(
        panel,
        ServerMechanicCommand {
            capability: 154,
            command_type: b::COMMAND_TYPE,
            payload,
        },
    )
}
pub fn select_product(panel: &mut StorePanel, index: usize) -> bool {
    if panel.pending || panel.confirmed_purchase.is_some() {
        return false;
    }
    let Some(page) = &panel.browser.page else {
        return false;
    };
    let Some(product) = page.products.get(index) else {
        return false;
    };
    panel.browser.selected = index;
    panel.selected_offer = 0;
    panel.purchase_confirmed = false;
    panel.catalogue = Some(store::StoreCataloguePage {
        catalog_revision: product.revision.clone(),
        offers: vec![product.offer.clone()],
        next_page_cursor: String::new(),
        total_coins: page.total,
        transferable_coins: if page.transferable_known {
            page.transferable
        } else {
            0
        },
    });
    true
}
pub fn apply_browse(panel: &mut StorePanel, result: &ServerMechanicResponse) {
    panel.pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("Store browse refused; refresh the page".into());
        return;
    }
    match b::Page::decode(&result.payload) {
        Ok(page) => {
            if panel.confirmed_purchase.is_none() {
                panel.catalogue = None;
            }
            panel.browser.page = Some(page);
            panel.browser.selected = 0;
            select_product(panel, 0);
            panel.feedback = None;
        }
        Err(_) => panel.feedback = Some("Invalid Store browser reply".into()),
    }
}
#[cfg(test)]
#[path = "browser/tests.rs"]
mod tests;
